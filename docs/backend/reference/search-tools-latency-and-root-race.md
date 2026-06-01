> **Synthesis:** This doc is synthesized into [`../technical/tool-discovery-and-search.md`](../technical/tool-discovery-and-search.md). Read that doc first; come here for the root-race fix, inactive scan SQL rewrite, and per-session cache implementation detail.

# search_tools Latency & Root-Race Fixes

**Last Updated:** May 29, 2026
**Status:** Shipped — Phases 6–8 committed on `docs/feature-set-consent-model` (4195944, 494c693, 16d5fff)
**Branch:** `docs/feature-set-consent-model` (Phase 6 of consent-model work)
**Base branch:** `docs/feature-set-consent-model`
**Depends on:** [`feature-set-consent-model.md`](./feature-set-consent-model.md) Phases 1–5 (shipped on this branch) — `include_inactive` discovery, bind layering, session-override removal

---

## Problem

Manual QA of the consent-model PR (May 29, 2026) surfaced two independent performance bugs that together required 6 meta-tool calls to post one Jira comment:

| Symptom                                                                                                                                                                          | Root cause                                                                                                                                                                                                                                                                     |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `search_tools("jira get issue")` → `total: 0, scope: active_only` even though `bundle:gait` (488 members including 37 Atlassian tools) was already on the GAIT workspace binding | `ensure_roots_probed` is called on `tools/list` / `call_tool` / `resources/list` / `prompts/list` but **never on meta-tool invocations**. First `search_tools` fires before the 300 ms root probe completes → resolver sees `PendingRoots` → empty grants → zero active tools. |
| `mcpmux_search_tools({ include_inactive: true })` (no `server_id`) hung for ~84 s against a PostHog clone (451 tools per bundle)                                                 | `list_inactive_discovery_tools` iterates every FeatureSet in the space and calls `resolve_feature_sets` per FS — O(FS × tools) DB work. 9 bundles × ~450 tool resolution passes each = thousands of ops before returning.                                                      |
| `search_tools` calls feel slow even when binding is correct                                                                                                                      | Active index is rebuilt from DB on every call (2 round-trips + in-memory sort); no cross-call cache.                                                                                                                                                                           |

All three are gateway-side bugs. None require changes to the consent-model data model or approval flow.

---

## Decisions

| #   | Decision                                      | Choice                                                                                                                                                                                           | Rationale                                                                                                                                                                                                                                                 |
| --- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Root-probe call site                          | Call `ensure_roots_probed` in `handler.rs` **before** dispatching to `meta_tool_registry.call(...)`                                                                                              | The handler already has `peer`, `session_id`, and `client_id` at the meta-tool dispatch site (~line 739). No new abstractions needed — same one-liner used for `tools/list`.                                                                              |
| 2   | Inactive scan rewrite                         | Replace per-FS `resolve_feature_sets` loop with a **single JOIN query** across `feature_set_members → server_features`, excluding invokable keys                                                 | O(1) DB round-trips regardless of FS count. Nested FS-type members handled with a follow-up pass (rare in practice — current bundles use flat feature members). Add a soft hint when result set > 50 tools: "Narrow with `server_id` for faster results." |
| 3   | Active index cache                            | **Per-session cache** keyed on `(session_id, feature_set_ids_fingerprint)`, stored in `MetaToolContext`                                                                                          | Sessions are long-lived; binding rarely changes mid-session. Fingerprint (hash of `feature_set_ids`) already computed by `record_resolution` in nearby code. Invalidate on `WorkspaceBindingChanged` event and session disconnect.                        |
| 4   | Cache invalidation events                     | `WorkspaceBindingChanged` → evict entry for `(space_id, affected_roots)`; `ServerFeaturesDiscovered` for a server in the cached binding → evict that session's entry; session disconnect → evict | Covers all state-change paths without polling.                                                                                                                                                                                                            |
| 5   | Wide inactive scan (no `server_id`)           | **Not blocked** — single JOIN query makes it fast; `server_id` filter remains optional                                                                                                           | Hard-requiring `server_id` would break the discovery use-case ("which bundle has a Jira tool?"). With the SQL rewrite, wide scans are acceptable.                                                                                                         |
| 6   | Nested FeatureSet resolution in inactive scan | Two-pass: first the JOIN for flat `feature`-type members (99% of cases), then a second small pass for `feature_set`-type members                                                                 | Keeps the hot path simple without dropping correctness for composed bundles.                                                                                                                                                                              |

---

## What this is NOT

- Not a change to the consent/bind model, approval flow, or FeatureSet authoring
- Not lazy / invoke-time server connects (separate doc: [`gateway-warm-pool-startup.md`](./gateway-warm-pool-startup.md))
- Not exposing required-params in `search_tools` results (separate small enhancement, no planning doc needed)
- Not caching `list_inactive_discovery_tools` results (covered by the JOIN rewrite; cross-call inactive cache deferred until measured)

---

## Architecture

### Phase 1 fix — root probe at meta-tool dispatch

```text
handler.rs call_tool (meta-tool branch, ~line 739):

  BEFORE:
    registry.call(name, client_id, session_id, args)

  AFTER:
    ensure_roots_probed(&context.peer, session_id, &oauth_ctx.client_id).await;
    registry.call(name, client_id, session_id, args)
```

`ensure_roots_probed` has a 1 s throttle internally — repeated calls within the same second are no-ops. Budget is 300 ms; on a warm session (roots already resolved) it returns immediately.

### Phase 2 fix — inactive scan single JOIN

```sql
-- Replaces the per-FS resolve loop in resolution.rs
SELECT
    fm.feature_set_id     AS bindable_feature_set_id,
    sf.server_id,
    sf.feature_name,
    sf.feature_type,
    sf.display_name,
    sf.description,
    fs.feature_set_type,
    fs.name               AS fs_name
FROM feature_set_members fm
JOIN feature_sets fs ON fs.id = fm.feature_set_id
JOIN server_features sf ON sf.id = fm.member_id
WHERE sf.space_id = ?
  AND sf.is_available = 1
  AND fm.member_type = 'feature'
  AND fm.mode = 'include'
  AND fs.space_id = ?
  AND fs.deleted_at IS NULL
  AND NOT EXISTS (
      SELECT 1 FROM invokable_keys ik
      WHERE ik.server_id = sf.server_id
        AND ik.feature_name = sf.feature_name
  )
ORDER BY fs.feature_set_type DESC,  -- custom before builtin
         sf.server_id, sf.feature_name;
```

`invokable_keys` passed as a temp table or multi-value bind (SQLite supports this via `WITH` CTE). Dedupe by `(server_id, feature_name)` in Rust after the query — first FS row wins (custom-first order from ORDER BY).

### Phase 3 — per-session active index cache

```text
MetaToolContext gains:
  search_cache: Arc<DashMap<String, (FsFingerprint, ToolIndex)>>
                                 ^session_id

SearchToolsTool.call():
  1. fingerprint = hash(sorted feature_set_ids)
  2. if cache[session_id] == Some((fp, idx)) && fp == fingerprint → use cached idx
  3. else → rebuild (DB round-trips), cache[session_id] = (fingerprint, new_idx)

Cache eviction (two sites):
  - WorkspaceBindingChanged consumer → remove entries for roots under changed space
  - SessionRootsRegistry::remove() (disconnect) → cache.remove(session_id)
```

Cache is `DashMap` (same concurrency primitive used by `SessionRootsRegistry` already). Memory bounded by session count × index size; typical session index is a few hundred KB.

---

## Files to modify

| File                                                                                                                           | Change                                                                                                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs)                                   | Add `ensure_roots_probed` call before meta-tool registry dispatch (~line 739)                                                                            |
| [`crates/mcpmux-gateway/src/pool/features/resolution.rs`](../../crates/mcpmux-gateway/src/pool/features/resolution.rs)         | Replace per-FS `resolve_feature_sets` loop in `list_inactive_tools_for_discovery` with single JOIN query; add second pass for `feature_set`-type members |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs)                 | Update `list_inactive_discovery_tools` callsite if signature changes                                                                                     |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | Add `search_cache: Arc<DashMap<String, (u64, ToolIndex)>>` to `MetaToolContext`; expose cache eviction helper                                            |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs)       | `SearchToolsTool::call` — check/populate per-session cache before DB round-trips; add `server_id`-filter hint to large inactive result sets              |
| [`crates/mcpmux-gateway/src/consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs)             | On `WorkspaceBindingChanged` — evict affected session entries from search cache (via new registry helper)                                                |
| `tests/rust/tests/integration/meta_tools.rs`                                                                                   | Tests for root-race fix (search on first call without prior `tools/list`), cache hit/miss, inactive scan perf (large FS doesn't hang)                    |

---

## Phasing

### Phase 6 — Root-race fix

**Effort:** ~2 hours

- Add `ensure_roots_probed` call in `handler.rs` before `registry.call(...)` at the meta-tool dispatch site
- Add integration test: `search_tools` called as the **first** meta-tool invocation in a session (no prior `tools/list`) on a workspace with a valid binding resolves the binding and returns active tools — not `PendingRoots` / total 0
- `pnpm validate` + `pnpm test:rust` green

**Outcome:** `mcpmux_search_tools` called in a fresh Cursor session finds active tools on the first call, without the agent needing to call `list_feature_sets → bind` first to trigger root resolution. The "already bound, search returned 0, wasted call" failure mode is impossible.

### Phase 7 — Inactive scan SQL rewrite

**Effort:** ~1 day

- Implement single JOIN query in `resolution.rs::list_inactive_tools_for_discovery` replacing the per-FS loop; add second-pass for nested `feature_set`-type members
- Update `facade.rs` callsite if needed
- Add `server_id`-filter hint to `SearchToolsTool` response when inactive result set > 50 tools
- Integration test: `include_inactive: true` on a space with a PostHog-scale bundle (400+ tools) completes in < 2 s
- `pnpm validate` + `pnpm test:rust` green

**Outcome:** `search_tools({ include_inactive: true })` without `server_id` completes in under 2 s regardless of bundle size. The 84 s hang on PostHog clones is gone.

### Phase 8 — Per-session active index cache

**Effort:** ~1 day

- Add `search_cache: Arc<DashMap<String, (u64, ToolIndex)>>` to `MetaToolContext`
- `SearchToolsTool::call` reads from cache on fingerprint match; populates on miss
- Wire cache eviction into `WorkspaceBindingChanged` consumer and `SessionRootsRegistry::remove`
- Integration tests: (a) second `search_tools` call in same session with unchanged binding hits cache (no DB round-trips), (b) cache entry evicted after `WorkspaceBindingChanged`, (c) session disconnect evicts entry
- `pnpm validate` + `pnpm test:rust` green

**Outcome:** Repeated `search_tools` calls within a session (the tool-discovery loop before invoke) return immediately from in-memory cache. DB is only consulted when the binding changes or on the first call per session.

---

## Pre-PR validation

| Step          | Command                                                                                                                                        | Purpose                               |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| Full validate | `pnpm validate`                                                                                                                                | fmt, clippy, check, eslint, typecheck |
| Rust tests    | `pnpm test:rust`                                                                                                                               | unit + integration                    |
| Manual smoke  | Fresh Cursor session on GAIT folder → `search_tools("jira")` on first call returns Atlassian tools without prior `list_feature_sets` or `bind` | Root-race regression                  |
| Perf check    | `search_tools({ include_inactive: true })` on PostHog-scale space < 2 s                                                                        | Inactive scan regression              |

---

## Out of scope

| Item                                                       | Reason                                                                            |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Required-params in `search_tools` results                  | Small enhancement, no architectural decision needed; file as a standalone issue   |
| Caching `list_inactive_discovery_tools` results cross-call | Not needed after Phase 7 SQL rewrite makes it cheap; revisit if measured as hot   |
| `server_id` hard-required for `include_inactive`           | Rejected (Decision 5) — breaks valid "which bundle has this tool?" discovery flow |
| Gateway warm-pool tiered startup                           | Separate doc: [`gateway-warm-pool-startup.md`](./gateway-warm-pool-startup.md)    |
| `cloudId` auto-resolution for Atlassian MCP                | Upstream Atlassian MCP server issue; McpMux proxies the schema as-is              |

---

## Key files referenced

| File                                                                                                                             | Notes                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs)                                     | `ensure_roots_probed` impl (~line 202); meta-tool dispatch site (~line 739)      |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs)               | `SessionRootsRegistry` — DashMap, `probe_lock`, 300 ms budget, 1 s throttle      |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | `resolve()` — `PendingRoots` path returns empty grants when roots not yet probed |
| [`crates/mcpmux-gateway/src/pool/features/resolution.rs`](../../crates/mcpmux-gateway/src/pool/features/resolution.rs)           | `list_inactive_tools_for_discovery` — the per-FS loop being replaced             |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs)   | `MetaToolContext`, `MetaToolCall`, `MetaToolRegistry::call` dispatch             |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs)         | `SearchToolsTool::call`, `caller_resolution`, active index build                 |

---

## Related work

- [`feature-set-consent-model.md`](./feature-set-consent-model.md) — the PR this extends; Phases 1–5 already on `docs/feature-set-consent-model`
- [`gateway-warm-pool-startup.md`](./gateway-warm-pool-startup.md) — related startup perf work (tiered connect, binding-priority warm) — separate PR
- [`meta-gateway-invoke.md`](./meta-gateway-invoke.md) — original `search_tools` design; TF-IDF ranking introduced in Phase D
- [`search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md) — relevance rework (token-overlap + embedding rerank) that layers onto this doc's Phase 8 per-session cache

---

## Reconciliation

**Shipped May 29, 2026** on commits `4195944` (Phase 6), `494c693` (Phase 7), `16d5fff` (Phase 8).

| Phase | Planned                                         | Shipped                                                                                                                               | Deviation                                                                                                                         |
| ----- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 6     | `ensure_roots_probed` before meta-tool dispatch | Same — `handler.rs` ~745                                                                                                              | None                                                                                                                              |
| 7     | Single JOIN SQL in `resolution.rs`              | Two-pass in-memory scan: `list_for_space` + `list_by_space`, flat includes first, `resolve_members` second pass for nested/exclude    | Avoided new storage-layer SQL; achieves O(1) repo round-trips vs per-FS `resolve_feature_sets`. Same outcome, different layer.    |
| 8     | `search_cache` on `MetaToolContext`             | Shared `Arc` on `SessionRootsRegistry`, wired into `MetaToolContext`; eviction in `remove()` + `MCPNotifier::WorkspaceBindingChanged` | Cache lives on `SessionRootsRegistry` (not standalone on context) so session disconnect eviction is colocated with root lifecycle |

**Validation:** `pnpm validate` + `pnpm test:rust` green (May 29, 2026).

**Manual QA still required:** Fresh Cursor session smoke (`search_tools("jira")` first call) and PostHog-scale inactive scan perf check per Pre-PR validation table.
