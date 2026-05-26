# Meta-Gateway Invoke (Search → Schema → Invoke)

**Last Updated:** May 25, 2026
**Status:** ✅ Phases A–C implemented and manually QA complete — ready to merge ([`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md) **Ship**)
**Branch:** `dev` on [crimsonsunset/mcp-mux](https://github.com/crimsonsunset/mcp-mux) (see [`fork-integration.md`](./fork-integration.md))
**Base branch:** `feat/dynamic-mcp-toggle-meta-tools` on fork; upstream contribution is topic-stacked, not `main`
**Issue:** Fork-only; upstream megapr [#155](https://github.com/mcpmux/mcp-mux/pull/155) closed — use #154 stack for meta-tools upstream
**Depends on:** [`dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) (session overrides + meta-tool registry); benefits from workspace bindings / FeatureSets from PR #151
**Supersedes:** Token-budget approach in [`tool-level-session-pin.md`](./tool-level-session-pin.md) — pin filtered a bloated `tools/list`; this doc replaces that model with a fixed meta surface + invoke path. Session pin may return as an invoke ACL in Phase F (very optional, last).
**Unblocks:** Agent-usable McpMux sessions at scale (240+ backend tools installed, ~12 tools in client context); homelab + multi-clone installs without context-window collapse

---

## Problem

Routing every AI client through one McpMux gateway endpoint solved config duplication and credential sprawl. It introduced a different bottleneck: **tool definition bloat in the client context window**.

Concrete symptoms from a May 2026 Cursor session against a real install:

| Symptom | Number |
| ------- | ------ |
| Installed servers in Space | 34 |
| Tools in `mcpmux_list_all_tools` dump | 1,581 (~855 KB JSON) |
| Tools exposed in Cursor session (GWorkspace × 2 clones) | 240 |
| GitHub tools available but usable only after `mcpmux_enable_server` | 41 |
| GitHub tool schemas in Cursor MCP descriptor folder | 0 |
| Approximate tokens consumed by 240 tool definitions | ~30–50k |

Session meta-tools ([`dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md)) let the LLM enable/disable servers mid-conversation, but **`tools/list` still advertises every backend tool** once a server is in the effective set. The LLM must guess parameter names (`issueNumber` vs `issue_number`) because schemas are not exposed through discovery APIs — only through client-side descriptor files that lag behind dynamic enablement.

Competing gateways ([MikkoParkkola/mcp-gateway](https://github.com/MikkoParkkola/mcp-gateway), [abdullah1854/MCPGateway](https://github.com/abdullah1854/MCPGateway)) solve this with a **fixed meta surface** (~14–19 tools) and **progressive disclosure**: search → load schema → invoke. McpMux already has half the plumbing (`mcpmux_list_servers`, `mcpmux_enable_server`, `mcpmux_list_all_tools`) but lacks search-with-schema and a single invoke entry point.

The user-facing ask (May 2026 session):

> I'd rather 1–2 more calls that actually work well than hundreds of tool defs I can't call correctly.

This doc defines that model for McpMux while preserving its product strengths: OS keychain credentials, Spaces, FeatureSets, per-client auth, and the server registry.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Client `tools/list` shape | **Meta tools + optional surfaced backend tools only** — never the full backend catalog | Fixes context bloat. Backend tools are invoked through `mcpmux_invoke_tool`, not registered in the client tool list (except surfaced exceptions). |
| 2 | Discovery API | **`mcpmux_search_tools` with `detail_level`**: `name` \| `description` \| `schema`** | Replaces dumping `mcpmux_list_all_tools` for agent workflows. Supports server_id filter, pagination, and query string. Start with substring + server_id filter; TF-IDF semantic rank is Phase D optional. |
| 3 | Schema API | **`mcpmux_get_tool_schema`** — single or batch, optional `compact: true`** | Agents must read schemas before invoke without relying on Cursor descriptor JSON files. Batch load for multi-tool workflows (e.g. issue read + comment write). |
| 4 | Invoke API | **`mcpmux_invoke_tool({ server_id, tool, args, filter? })`** — one entry point for all backend calls | Mirrors `gateway_invoke`. Routes through existing `RoutingService::call_tool` after permission checks. Optional `filter` arg activates result shaping (Phase B). |
| 5 | FeatureSet semantics | **FeatureSets define what is *invokable*, not what appears in `tools/list`** | Binding / grant / session-enable controls the candidate pool for search + invoke. Security boundary stays meaningful without polluting client context. |
| 6 | Surfaced tools escape hatch | **FeatureSet members may mark tools `surfaced: true` (0–N per set)** — promoted into `tools/list` for one-hop hot paths | Default: **zero surfaced everywhere**, including built-in bundles. No bundle auto-promotes backend tools. Opt-in only via FeatureSet editor (Phase C). |
| 7 | Invoke authorization | **Fail closed** — `invoke_tool` rejects when target server/tool is outside effective permission set | Same composition as today: `(binding_servers ∪ session_enabled) − session_disabled`, then FeatureSet member filter. Empty effective set → invoke denied with actionable error, not silent proxy. |
| 8 | Session enable/disable | **Keep existing `mcpmux_enable_server` / `mcpmux_disable_server`** — they gate invoke/search eligibility, not `tools/list` size | Mental model unchanged: "turn on github" expands what search/invoke can reach. `tools/list` size stays ~constant. |
| 9 | Error messages | **Actionable, bounded errors** — no dumping full available-tool lists | e.g. `"github inactive → mcpmux_enable_server('github')"`, `"unknown tool → did you mean github_list_issues?"`. Optional Levenshtein suggestions (Phase D). |
| 10 | Rollout | **Hard cut — no legacy opt-out** | Non-surfaced backend tools never appear in `tools/list` and direct `call_tool` is rejected with a redirect to `mcpmux_invoke_tool`. **Exception:** FeatureSet members marked `surfaced: true` are promoted into `tools/list` and callable in one hop. No `expose_backend_tools_in_list` setting. Ship in one release; document migration in CHANGELOG. |
| 11 | `mcpmux_list_all_tools` | **Keep as operator/diagnostic tool** — not the primary agent discovery path | Still useful for FeatureSet authoring and UI. Doc + descriptions steer agents to `search_tools`. Consider server_id filter arg in Phase A to avoid 855 KB dumps. |
| 12 | Result shaping scope | **Phase B only on `invoke_tool`** — opt-in via explicit `filter`: `max_rows`, `max_bytes`, `fields`, `format: summary`. Omit filter → backend response as-is. | Agents pass `filter` when they know a tool returns large payloads. No default truncation. |
| 13 | REST / OpenAPI capabilities | **Out of scope here** — Phase E / separate planning doc | [`web-admin-remote-access.md`](./web-admin-remote-access.md) covers admin REST, not REST→MCP capability YAML. No conflict; different layer. |

---

## The Model

### What the agent sees

```text
tools/list (fixed ~10–15 tools)
├── mcpmux_list_servers
├── mcpmux_enable_server / mcpmux_disable_server
├── mcpmux_search_tools
├── mcpmux_get_tool_schema
├── mcpmux_invoke_tool
├── mcpmux_list_feature_sets / mcpmux_create_feature_set / mcpmux_bind_current_workspace
├── mcpmux_list_all_tools          (diagnostic — not primary discovery)
└── [0–N surfaced backend tools]   (optional, from FeatureSet)
```

### Agent workflow (GitHub read example)

```text
1. mcpmux_list_servers                          → github: inactive
2. mcpmux_enable_server({ server_id: "github" })
3. mcpmux_search_tools({
     query: "list issues",
     server_id: "github",
     detail_level: "description"
   })
4. mcpmux_get_tool_schema({ tools: ["github_list_issues"] })
5. mcpmux_invoke_tool({
     server_id: "github",
     tool: "list_issues",
     args: { owner: "mcpmux", repo: "mcp-mux", state: "OPEN" }
   })
```

Three to four meta calls before the backend call — predictable schemas, bounded context.

### Permission composition (unchanged server layer, new tool-list layer)

```text
1. (space, feature_set_ids) ← FeatureSetResolverService
2. binding_servers          ← servers_for(space, feature_set_ids)
3. session_on/off           ← SessionOverrideRegistry
4. effective_servers        ← (binding ∪ session_on) − session_off
5. invokable_tools          ← Tool features for effective_servers ∩ FeatureSet members
6. tools/list               ← meta_tools ∪ surfaced(invokable_tools)
7. search_tools / invoke    ← scoped to invokable_tools only
```

Prompts and resources: unchanged in Phases A–C — still materialized per grants. Invoke model is tool-specific. **Resource list bloat** (e.g. ~124 PostHog skill URIs in GAIT QA) tracked in **Phase D**.

### What this is NOT

- Not replacing the desktop app, registry, or Spaces model
- Not removing FeatureSets — they become invoke ACLs
- Not implementing abdullah's full 15-layer optimization stack in v1
- Not REST capability YAML / OpenAPI import (separate future doc)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Cursor / Claude / VS Code                                      │
│  tools/list → ~12 meta tools (+ optional surfaced)              │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  McpMux Gateway (:45818)                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ MetaToolRegistry                                          │  │
│  │  search_tools → ToolDiscoveryService (index from Space)   │  │
│  │  get_tool_schema → ServerFeature.input_schema             │  │
│  │  invoke_tool → RoutingService::call_tool (existing path)  │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ FeatureService::get_tools_for_grants                      │  │
│  │  → meta tools + surfaced only (hard cut — no backend list)  │  │
│  └───────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         ▼                   ▼                   ▼
    github (stdio)    google-workspace     posthog-personal
```

**New components:**

- `ToolDiscoveryService` — in-memory index built from `server_feature_repo::list_for_space`, rebuilt on feature change events. Powers search + schema lookup.
- `InvokeToolTool` — validates invokable set, forwards to `RoutingService::call_tool`, maps errors to actionable messages.

**Chokepoints (existing):**

- `FeatureService::get_tools_for_grants` — change what gets advertised in `tools/list`
- `RoutingService::call_tool` — reuse for invoke; add invokable-set check if not already covered by grant lookup
- `MetaToolRegistry` — register three new tools

---

## Files to create

| File | Purpose | Status |
| ---- | ------- | ------ |
| `crates/mcpmux-gateway/src/services/tool_discovery.rs` | Index + search + schema lookup over Space tool features | ✅ Done |
| `crates/mcpmux-gateway/src/services/meta_tools/invoke.rs` | `InvokeToolTool` impl — permission check, routing, error mapping, result shaping | ✅ Done |
| `crates/mcpmux-gateway/src/services/meta_tools/invoke_backend.rs` | `InvokeToolBackend` trait + `RoutingService` adapter for testable invoke routing | ✅ Done |
| `tests/rust/src/canned_invoke_backend.rs` | Canned backend for filter e2e integration tests | ✅ Done |
| `tests/rust/tests/integration/meta_gateway_invoke.rs` | Search, schema, invoke, permission deny, surfaced tools, filter shaping, e2e filter via canned backend | ✅ Done (16 tests) |
| `docs/planning/meta-gateway-invoke-qa.md` | Manual QA runbook for Phases A–C | ✅ Done |
| `docs/planning/meta-gateway-invoke.md` | This doc | ✅ Done |

## Files to modify

| File | Change | Status |
| ---- | ------ | ------ |
| [`crates/mcpmux-gateway/src/services/mod.rs`](../../crates/mcpmux-gateway/src/services/mod.rs) | `pub mod tool_discovery;` | ✅ Done |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | `SearchToolsTool`, `GetToolSchemaTool`; extend `ListAllToolsTool` with optional `server_id` filter | ✅ Done |
| [`crates/mcpmux-gateway/src/services/meta_tools/mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs) | Register new tools; wire `ToolDiscoveryService` + `InvokeToolBackend` into `MetaToolContext` | ✅ Done |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | Extend `MetaToolContext` with discovery + invoke backend handles | ✅ Done |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs) | Split into `get_advertised_tools_for_grants` vs `get_invokable_tools_for_grants` | ✅ Done |
| [`crates/mcpmux-gateway/src/pool/features/resolution.rs`](../../crates/mcpmux-gateway/src/pool/features/resolution.rs) | `resolve_surfaced_feature_ids` for surfaced promotion | ✅ Done |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | `format_direct_call_redirect`; actionable invoke errors | ✅ Done |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs) | `tools/list` uses advertised set only; non-surfaced direct `call_tool` rejected with invoke redirect; surfaced tools allowed one-hop; `ensure_roots_probed` before routing in `call_tool` | ✅ Done |
| [`crates/mcpmux-core/src/domain/feature_set.rs`](../../crates/mcpmux-core/src/domain/feature_set.rs) | `surfaced: bool` on `FeatureSetMember` | ✅ Done |
| [`apps/desktop/src/features/featuresets/FeatureSetPanel.tsx`](../../apps/desktop/src/features/featuresets/FeatureSetPanel.tsx) | Per-tool "Surface in client" toggle + explainer tooltip | ✅ Done |
| [`apps/desktop/src/features/settings/SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx) | Meta-tools copy for search → schema → invoke workflow | ✅ Done |
| [`README.md`](../../README.md) | Agent-facing search → schema → invoke flow; checkbox vs Surface in Feature Sets | ✅ Done |
| [`docs/guide/feature-sets.mdx`](../guide/feature-sets.mdx) | Included vs Surface editor explainer; invoke ACL semantics | ✅ Done |

---

## Phasing

### Phase A — Meta invoke core

**Effort:** ~3–4 days  
**Status:** ✅ Implemented — manual QA sections 0–4 pass ([`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md))

- [x] `ToolDiscoveryService` — build index from Space features; search by query + optional `server_id`; return matches at `detail_level`
- [x] `mcpmux_search_tools` meta tool — pagination (`limit`, `cursor`), `detail_level` enum
- [x] `mcpmux_get_tool_schema` — single + batch; `compact` strips descriptions/examples
- [x] `mcpmux_invoke_tool` — `{ server_id, tool, args }`; delegates to `RoutingService::call_tool`; fail closed on permission miss
- [x] `FeatureService` split: **advertised** = meta tools + surfaced only (hard cut — no backend tools in list)
- [x] Handler rejects **non-surfaced** direct backend `call_tool` — redirect to `mcpmux_invoke_tool`; surfaced tools pass through
- [x] Actionable error mapping: inactive server, unknown tool, permission denied, param validation passthrough from backend
- [x] Optional `server_id` filter on `mcpmux_list_all_tools`
- [x] Integration tests: GitHub read path (enable → search → schema → invoke); deny when server inactive; non-surfaced direct call rejected

**Outcome:** Cursor session shows **10** `mcpmux_*` tools (verified May 25, 2026). Agent completes `github_list_issues` on `mcpmux/mcp-mux` via search → schema → invoke with zero param guessing.

### Phase B — Result shaping on invoke

**Effort:** ~2 days  
**Status:** ✅ Implemented — manual QA section 6 pass (May 25)

- [x] Extend `mcpmux_invoke_tool` args with optional `filter: { max_rows?, max_bytes?, fields?, format? }`
- [x] Post-process JSON/text results in gateway when `filter` is provided
- [x] Opt-in truncation only — omit `filter` to return backend response unchanged (May 25 design revision)
- [x] Unit tests (13): top-level arrays, nested `issues`/`items` keys, JSON-in-text blocks, `structured_content`, `fields`, `format: summary` vs `full`, `parse_invoke_filter` edge cases
- [x] Integration tests: pure-fn filter shaping + `invoke_tool_applies_filter_end_to_end` via `CannedInvokeBackend`

**Outcome:** Agents pass `filter` on known-heavy tools (GWorkspace drive lists, GitHub issues, PostHog events). Plain-text and JSON backends both supported when filter is explicit.

#### Filter behavior reference

| Payload shape | Applicable filter keys | Behavior |
| ------------- | ---------------------- | -------- |
| Plain text (`content[].text` non-JSON) | `max_bytes` only | Returns `{ returned, total, truncated, text }` envelope when over limit. `max_rows` / `fields` / `format` ignored. |
| Top-level JSON array | `max_rows`, `fields`, `format`, `max_bytes` | When `total > max_rows`: `{ returned, total, truncated, items: [...] }` |
| JSON object with heavy array key (`issues`, `items`, `results`, …) | same | Metadata merged at object top-level; array under original key name |
| JSON serialized inside text content block | same | Parsed then shaped; re-serialized into `text` |
| `structured_content` on `CallToolResult` | same | Shaped independently via `apply_invoke_result_filter` |

**`format` semantics (requires `max_rows`):**

- `full` — sample size = `max_rows`
- `summary` — sample size = `min(max_rows, 5)` (no effect when `max_rows ≤ 5`)

**Envelope fields:** `returned` (rows or bytes after truncation), `total` (pre-truncation count/bytes), `truncated: true`, plus `items`/`issues`/… or `text`.

### Phase C — FeatureSet as invoke ACL + surfaced tools

**Effort:** ~3 days  
**Status:** ✅ Implemented — manual QA sections 8–9 pass ([`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md))

- [x] FeatureSet member model: tools invokable by default when server in set; optional `surfaced: true` promotes into `tools/list`
- [x] Search + invoke respect FeatureSet member filter (not just server-all)
- [x] Workspaces UI: per-tool "Surface in client" toggle in FeatureSet editor (`FeatureSetPanel.tsx`)
  - **Checkbox** = invoke ACL member (search + `mcpmux_invoke_tool`)
  - **Surface button** = promote that included tool into client `tools/list` for direct one-hop calls
  - User-facing explainer: [`docs/guide/feature-sets.mdx`](../guide/feature-sets.mdx#included-vs-surface-featureset-editor)
- [x] `mcpmux_create_feature_set` accepts optional `surfaced_tools[]` (subset of `tool_qualified_names`; UI path also available)
- [x] Integration tests: partial FeatureSet binding limits search; surfaced vs invokable gate; advertised set promotion

### Phase D — Advanced optimizations (defer)

**Effort:** TBD

- [ ] **Resource progressive disclosure** — extend invoke model to `resources/list` context bloat
  - **Problem:** Phases A–C fixed tool definition bloat (`tools/list` → ~10 meta tools) but `resources/list` still materializes every granted resource URI + metadata to the client. GAIT workspace QA (May 25) surfaced **~124 resources** — mostly PostHog `posthog://skills/...` from `bundle:gait`. Lighter per entry than tool schemas, but still UI noise and potential client context tax if the host injects resource catalogs into agent prompts.
  - **Not an ACL bug:** `get_resources_for_grants` already respects FeatureSets; the gap is **full list materialization**, same class of problem tools had pre–Phase A.
  - **Proposed direction (pick subset to ship):**
    - Slim client `resources/list` to meta-only (or empty) + `mcpmux_search_resources` / `mcpmux_read_resource` meta path — mirror search → read workflow
    - Grant-filtered search index (reuse `ToolDiscoveryService` patterns or sibling `ResourceDiscoveryService`)
    - Optional **surfaced resources** escape hatch (parallel to surfaced tools) for hot URIs
    - Bundle hygiene: exclude PostHog skill resources from default `bundle:gait` until search path exists
  - **Evidence:** [`meta-gateway-invoke-gait-qa.md`](./meta-gateway-invoke-gait-qa.md) — Run 1 §0, Resources note; not a ship blocker
  - **Files:** [`handler.rs` `list_resources`](../../crates/mcpmux-gateway/src/mcp/handler.rs), [`facade.rs` `get_resources_for_grants`](../../crates/mcpmux-gateway/src/pool/features/facade.rs)
- [ ] Levenshtein "did you mean?" on invoke errors
- [ ] TF-IDF / semantic rank in search
- [ ] Delta responses, auto-summarize, parallel invoke batching
- [ ] Sandboxed code execution (abdullah-style `gateway_execute_code`)

**Outcome:** Incremental token/latency wins for power users. Resource disclosure is the highest-priority Phase D item after GAIT QA context feedback. Each item is independently shippable.

### Phase E — REST capabilities (separate initiative)

**Effort:** TBD — requires its own planning doc

- [ ] OpenAPI → capability definition in registry or gateway-local YAML
- [ ] Invoke through same `mcpmux_invoke_tool` path

**Outcome:** Non-MCP HTTP APIs join the gateway without a separate MCP server process. Not blocked by Phases A–D.

### Phase F — Session pin as invoke ACL (very optional)

**Effort:** ~1 day — **only if** a concrete use case remains after Phases A–C

- [ ] Re-scope [`tool-level-session-pin.md`](./tool-level-session-pin.md): `mcpmux_pin_this_session` restricts **invokable set** for the session, not `tools/list` membership
- [ ] Ship only on evidence that search + invoke + FeatureSet ACL is insufficient (e.g. agent repeatedly invokes disallowed tools and needs a tighter session knob)

**Outcome:** Temporary invoke ACL ("only these 12 tools invokable for this session") without re-expanding `tools/list`. Skip entirely if Phase A–C covers the GWorkspace clone case.

---

## Pre-PR validation

| Step | Command | Purpose |
| ---- | ------- | ------- |
| Full validate | `pnpm validate` | fmt, clippy, check, eslint, typecheck |
| Rust tests | `pnpm test:rust` | unit + `meta_gateway_invoke.rs` integration |
| TS tests | `pnpm test:ts` | vitest |
| Manual smoke | Cursor against live gateway — full runbook sections 0–11 | Agent UX verification — ✅ complete May 25 |

---

## Out of scope

| Item | Reason |
| ---- | ------ |
| [`web-admin-remote-access.md`](./web-admin-remote-access.md) | Remote admin UI — parallel track, no overlap |
| Full abdullah 15-layer stack | Phase D picks winners after A+B prove value |
| Removing `mcpmux_enable_server` | Still gates invoke eligibility; still needed when server not in binding |
| Auto-enable server on failed invoke | Silent enable defeats audit trail — rejected in dynamic-toggle doc |
| Tool-poisoning validator / SHA-256 pinning | MikkoParkkola feature; valuable follow-up for registry trust, not invoke core |
| Cursor descriptor JSON sync | Client-side concern; schema-on-demand makes it non-blocking |

---

## Key files referenced

| File | Why |
| ---- | --- |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs) | Materialization chokepoint — must split advertised vs invokable |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | Existing `call_tool` path invoke reuses |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | Invoke meta tool + result shaping |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke_backend.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_backend.rs) | Pluggable invoke routing trait |
| [`tests/rust/src/canned_invoke_backend.rs`](../../tests/rust/src/canned_invoke_backend.rs) | Test double for filter e2e |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs) | `tools/list` + `call_tool` — advertised set, surfaced one-hop, invoke redirect for non-surfaced |
| [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) | Session enable/disable — kept, semantics updated |
| [`docs/planning/tool-level-session-pin.md`](./tool-level-session-pin.md) | Superseded for token budget; Phase F very optional rework |

---

## Related documentation

- [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) — session overrides (complete)
- [`docs/planning/tool-level-session-pin.md`](./tool-level-session-pin.md) — superseded; Phase F may revive as invoke ACL only if needed
- [`docs/planning/server-account-clones.md`](./server-account-clones.md) — origin of 240-tool bloat evidence
- [`docs/planning/web-admin-remote-access.md`](./web-admin-remote-access.md) — remote operator UI (orthogonal)
- [MikkoParkkola/mcp-gateway](https://github.com/MikkoParkkola/mcp-gateway) — `gateway_search_tools` / `gateway_invoke` reference
- [abdullah1854/MCPGateway](https://github.com/abdullah1854/MCPGateway) — `gateway_get_tool_schema` / result filtering reference

---

## Reconciliation

This doc is the source of truth for the meta-gateway invoke model. Phases A–C are implemented on `feat/meta-gateway-invoke` and manually QA complete ([`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md) — **Ship**). Mark [`tool-level-session-pin.md`](./tool-level-session-pin.md) **Status** as *Superseded* when this branch merges to main.

**Decision record (May 25, 2026):** Hard cut to invoke-only for non-surfaced backend tools — no legacy full-catalog `tools/list`. Surfaced tools default zero everywhere (bundles included); opt-in per FeatureSet member for one-hop hot paths. FeatureSets redefine as invoke ACL + optional surfaced promotion. Session pin deferred to Phase F (very optional, last). Competitor analysis (MikkoParkkola + abdullah1854) informed Phase A–B scope; REST capabilities in Phase E / separate doc.

**Handler fix (May 25, 2026):** `call_tool` probes workspace roots before routing (matches `list_tools`) and allows direct calls when the tool is in `get_advertised_tools_for_grants` (surfaced). Non-surfaced backend names still get `use_invoke_tool` redirect.

**Design revision (May 25, 2026):** Removed default smart truncation — `filter` is opt-in only. Rationale: plain-text MCP backends (GWorkspace) don't map cleanly to JSON row truncation; agents should explicitly bound payloads when needed.

**QA ergonomics (May 25, 2026):** Bind FeatureSets in Workspaces UI before agent QA — session enable alone is insufficient without binding ACL. Do **not** call `mcpmux_bind_current_workspace` during routine QA (triggers Space-wide approval modal). Reload MCP tools after UI binding or Surface changes.

**Test coverage (May 25, 2026):** Phase B filter shaping — 13 unit tests in `invoke.rs`, 16 integration tests in `meta_gateway_invoke.rs`, manual QA sections 0–11 pass on live gateway.

**Manual QA progress (May 25, 2026):** Overall **Ship**. Full section results in [`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md). Highlights:

| QA section | Result | Notes |
| ---------- | ------ | ----- |
| 0 — Sanity (meta-only surface) | ✅ Pass | 10 `mcpmux_*` tools; 34 servers listed; all inactive until enabled |
| 1 — Happy path (GitHub read) | ✅ Pass | search → schema → invoke returned 5 open issues; enable step N/A (`enabled_via_binding`) |
| 2 — Fail-closed + recovery | ✅ Pass | Session disable → actionable error → enable → retry |
| 3 — Search detail levels + compact schema | ✅ Pass | compact omits top-level description only |
| 4 — Session toggle (list size unchanged) | ✅ Pass | search empty when disabled; 10 meta tools stable |
| 5 — Pass-through without filter (Phase B) | ✅ Pass | GWorkspace `list_drive_items`: 100 items, no metadata envelope |
| 6 — Explicit filter (Phase B) | ✅ Pass | Plain-text `max_bytes` + live `github_list_issues` JSON filter |
| 7 — Clone disambiguation | ✅ Pass | Personal vs S2H clone scoped correctly |
| 8 — FeatureSet ACL (Phase C) | ✅ Pass | Partial GitHub tool set; invoke deny outside ACL |
| 9 — Surfaced promotion (Phase C) | ✅ Pass | `github_list_issues` in tools/list + direct one-hop; `get_me` invoke-only |
| 10 — Diagnostic list vs search | ✅ Pass | 120 tools both paths for GWorkspace Personal |
| 11 — End-to-end agent task | ✅ Pass | Meta-only workflow; schema-first; filter truncation metadata |
