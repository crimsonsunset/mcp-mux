# Meta-Tools Module Split — Phase 2 (Tail Cleanup)

**Last Updated:** Jun 5, 2026
**Status:** Shipped — Jun 5, 2026 (Phases 1–10 on `feat/meta-surface-lean-core`)
**Branch:** `feat/meta-surface-lean-core` (or `feat/meta-tools-module-split-phase-2` off current branch)
**Base branch:** `main`
**Depends on:** [`meta-tools-module-split.md`](./meta-tools-module-split.md) Phases 1–5 complete (`tools.rs` / monolithic `invoke.rs` deleted; flat sibling layout landed)
**Unblocks:** All `meta_tools/` and `tool_discovery` service modules under ~300 prod lines; `invoke.rs` shim removed; test modules colocated via `#[path]` without inflating source files
**Estimated effort:** ~12–16 hours (Phases 1–4 ~6h immediate tail; Phases 5–9 ~6–10h remaining splits)

---

## Problem

Phase 1 eliminated the 1,400-line monoliths but left **four files above the repo's ~200–300 line maintainability target**. Two are prod+test combos; one is a deliberate shim; one bundles view builders with the MCP tool handler.

| File | Lines (Jun 2026) | Contents |
| ---- | ---------------: | -------- |
| [`invoke_result_filter.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_result_filter.rs) | ~907 | Filter struct + apply (~200 prod) + payload parse helpers (~180) + shaping helpers (~250) + `#[cfg(test)]` block (~440) |
| [`diagnose.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs) | ~669 | `ServerHealth`, `ConfigView`, runtime view builders (~320) + `DiagnoseServerTool` (~130) + tests (~220) |
| [`invoke_tool.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_tool.rs) | ~535 | Handler + alias resolution + response builders + tests — **acceptable; defer further split** |
| [`approval.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/approval.rs) | ~539 | Broker + types + tests — **acceptable; defer** |
| [`search_tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs) | ~496 | Index helpers + handler — **acceptable; defer** |
| [`disclosure.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs) | ~481 | 4 related tools — **intentionally grouped (Phase 1 decision)** |
| [`invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | ~12 | Re-export shim from Phase 1 — **delete in Phase 3** |

`tool_discovery.rs` (~782 lines) lives outside `meta_tools/` and is **out of scope** — separate planning doc if/when that service layer needs splitting.

**Pure refactor.** No agent-visible behavior changes, no new meta tools, no token-budget changes.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Scope | **Tail cleanup only** — `invoke_result_filter`, `diagnose`, `invoke.rs` shim | Highest ROI files; rest are borderline or intentionally grouped |
| 2 | Module layout | **Flat sibling modules** under `meta_tools/` (same as Phase 1) | No nested directories; `mod.rs` stays registry factory |
| 3 | Test module strategy | **Option A — `#[path = "foo_tests.rs"] mod tests`** | Drops file line counts without moving tests to integration crate; keeps `cargo test -p mcpmux-gateway` coverage local |
| 4 | `invoke_result_filter` split | **Three prod modules + one test file** | `invoke_result_filter` (public API), `invoke_payload_parse`, `invoke_result_shaping`, `invoke_result_filter_tests.rs` |
| 5 | `diagnose` split | **View module + tool module + test file** | `diagnose_view` (types + builders), `diagnose_server` (tool only), `diagnose_tests.rs` |
| 6 | `invoke.rs` shim | **Delete in Phase 3** | Phase 1 kept shim for one release; update `meta_gateway_invoke.rs` to import `invoke_tool` / `invoke_result_filter` directly |
| 7 | Public API | **Preserve `services/mod.rs` re-exports** | External crates import via `meta_tools::invoke_result_filter::` etc.; no path changes at crate boundary |
| 8 | `invoke_tool.rs` further split | **Phase 6 — `invoke_alias.rs` + `#[path]` tests** | Handler ~275 prod lines alone; alias fns are the churn surface |
| 9 | `search_tools` index extract | **Phase 7 — `search_tools_index.rs`** | Index/cache helpers (~120 lines) separate from `SearchToolsTool` handler |
| 10 | `approval.rs` split | **Phase 8 — types vs broker** | Only if approval UX work is active; otherwise mechanical move when touched |
| 11 | `disclosure.rs` split | **Phase 9 — 4 tools → 2×2 or per-file** | Lowest priority; split when disclosure tools start diverging in churn |
| 12 | `tool_discovery.rs` split | **Phase 5 — separate epic inside this plan** | 782 lines outside `meta_tools/`; higher blast radius, touches `search_tools.rs` |
| 13 | Test strategy | **No integration test rewrites** — same `meta_tools` + `meta_gateway_invoke` gate | Refactor-only; unit tests move via `#[path]`, logic unchanged |
| 14 | Git granularity | **One commit per phase** | Easier bisect than squash |

---

## Scope

**In:**

- Split `invoke_result_filter.rs` into filter API + payload parse + result shaping + external test module
- Split `diagnose.rs` into view builders + tool handler + external test module
- Delete `invoke.rs` re-export shim; retarget the one integration-test import site
- Update `mod.rs` module declarations and `pub use` surface
- Refresh [`tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) Architecture (maintainers) paragraph after each phase group
- `pnpm validate` + integration gates per phase (see Phase 10)
- **Phase 5:** Split `tool_discovery.rs` (~782 lines) into types, index build, search execution
- **Phase 6:** Split `invoke_tool.rs` — `invoke_alias.rs` + `invoke_tool_tests.rs` via `#[path]`
- **Phase 7:** Split `search_tools.rs` — `search_tools_index.rs` for index/cache/embedding helpers
- **Phase 8:** Split `approval.rs` — approval types vs `ApprovalBroker`
- **Phase 9:** Split `disclosure.rs` — 4 resource/prompt tools (2×2 grouped or one per file)

**Out:**

| Item | Reason |
| ---- | ------ |
| Agent-facing API / schema changes | Refactor only |
| Nested `invoke/`, `diagnose/`, or `tool_discovery/` directories | Flat siblings are the repo norm |
| Search latency / ranking algorithm changes | File moves only; no behavior edits |

---

## Target layout

```text
crates/mcpmux-gateway/src/services/meta_tools/
├── mod.rs                          # update mod declarations + pub use
├── invoke_result_filter.rs         # InvokeResultFilter, parse_invoke_filter, apply_invoke_result_filter (~200)
├── invoke_payload_parse.rs         # NEW — text→JSON/YAML, fence extraction, bracketed keys (~180)
├── invoke_result_shaping.rs        # NEW — shape_json_value, shape_object/array, byte limits (~250)
├── invoke_result_filter_tests.rs   # NEW — #[path] test module (~440)
├── diagnose_view.rs                # NEW — ServerHealth, ConfigView, DiagnoseArgs, build_*_view (~320)
├── diagnose_server.rs              # NEW — DiagnoseServerTool + MetaTool impl (~130)
├── diagnose_tests.rs               # NEW — #[path] test module (~220)
├── invoke_tool.rs                  # slim handler only (Phase 6)
├── invoke_alias.rs                 # NEW (Phase 6) — alias resolution
├── invoke_tool_tests.rs            # NEW (Phase 6) — #[path] tests
├── search_tools_index.rs           # NEW (Phase 7) — index/cache helpers
├── approval_types.rs               # NEW (Phase 8)
├── approval_broker.rs              # NEW (Phase 8)
├── disclosure_search.rs            # NEW (Phase 9) — search resources/prompts
├── disclosure_read.rs              # NEW (Phase 9) — read/fetch
├── invoke.rs                       # DELETED (Phase 3)
└── … (Phase 1 modules otherwise unchanged)

crates/mcpmux-gateway/src/services/   # Phase 5 — outside meta_tools/
├── tool_discovery_types.rs           # NEW
├── tool_discovery_index.rs           # NEW
├── tool_discovery_search.rs          # NEW
├── tool_discovery_tests.rs           # NEW — #[path] tests
└── tool_discovery.rs                 # slim façade or DELETED
```

### Import graph (post-split)

```text
invoke_payload_parse  ←  invoke_result_filter
invoke_result_shaping ←  invoke_result_filter

invoke_result_filter  ←  invoke_tool
diagnose_view         ←  diagnose_server

mod.rs  ←  diagnose_server, invoke_result_filter, invoke_tool (pub re-exports)
```

### Test module pattern (Option A)

```rust
// invoke_result_filter.rs (tail)
#[cfg(test)]
#[path = "invoke_result_filter_tests.rs"]
mod tests;
```

Same pattern for `diagnose_server.rs` → `diagnose_tests.rs`.

### Public API surface (unchanged at crate boundary)

```rust
// services/mod.rs — no path changes for external consumers
pub use meta_tools::{
    InvokeToolTool,
    invoke_result_filter::{InvokeResultFilter, apply_invoke_result_filter, shape_json_value, …},
    invoke_tool::{normalize_invoke_tool_name, resolve_invoke_server_id, …},
};
```

After Phase 3, `pub mod invoke` is removed from `meta_tools/mod.rs`; callers use `invoke_tool` / `invoke_result_filter` directly.

---

## Phases

### Phase 1 — Split `invoke_result_filter` (~2 hours)

- Create `invoke_payload_parse.rs` — move `coalesce_structured_payload`, `parse_structured_payload_from_text`, `normalize_parsed_payload`, fence/JSON extraction helpers, bracketed-array key normalization
- Create `invoke_result_shaping.rs` — move `shape_json_value`, `shape_content_blocks`, `shape_object`, `shape_array`, `apply_fields_filter`, `enforce_byte_limit`, truncation helpers
- Slim `invoke_result_filter.rs` to `InvokeResultFilter`, `parse_invoke_filter`, `apply_invoke_result_filter` + imports from parse/shaping modules
- Extract `#[cfg(test)] mod tests` to `invoke_result_filter_tests.rs` via `#[path]`
- Add `mod` declarations in `mod.rs`; keep existing `pub use` from `invoke_result_filter`

**Outcome:** `invoke_result_filter.rs` ≤ ~220 prod lines; parse and shaping modules each ≤ ~280 lines; all invoke filter unit tests pass via `cargo test -p mcpmux-gateway invoke_result_filter`. `cargo check -p mcpmux-gateway` clean.

---

### Phase 2 — Split `diagnose.rs` (~2 hours)

- Create `diagnose_view.rs` — move `ServerHealth`, `ConfigView`, `DiagnoseArgs`, `build_config_view_from_definition`, `parse_diagnose_args`, `build_runtime_view`
- Create `diagnose_server.rs` — move `DiagnoseServerTool` + `MetaTool` impl only
- Extract tests to `diagnose_tests.rs` via `#[path]` on `diagnose_server.rs`
- Delete `diagnose.rs`
- Update `mod.rs` to `mod diagnose_view; mod diagnose_server;` + `pub use diagnose_server::DiagnoseServerTool`

**Outcome:** No file under diagnose split exceeds ~320 lines; `DiagnoseServerTool` handler file ≤ ~150 prod lines. Diagnose unit tests pass; `mcpmux_diagnose_server` integration behavior unchanged.

---

### Phase 3 — Remove `invoke.rs` shim (~30 minutes)

- Delete [`invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs)
- Remove `mod invoke;` from `mod.rs`
- Update [`meta_gateway_invoke.rs`](../../tests/rust/tests/integration/meta_gateway_invoke.rs) — `use mcpmux_gateway::services::meta_tools::invoke::{…}` → direct `invoke_tool::` / `invoke_result_filter::` imports
- Grep for remaining `meta_tools::invoke::` references; retarget or add `pub use` at `mod.rs` level only if external crate paths require it (prefer direct paths)

**Outcome:** Zero `invoke.rs` shim; one integration test file updated; `cargo check -p mcpmux-gateway` and `meta_gateway_invoke` integration tests pass.

---

### Phase 4 — Verify + doc touch (~1 hour)

- `pnpm validate`
- `cargo nextest run -p tests --profile ci-integration -E 'test(meta_tools) or test(meta_gateway_invoke)'`
- `cargo test -p mcpmux-gateway invoke_result_filter diagnose` — confirm `#[path]` test modules run
- `pnpm count-tokens` — confirm `measure_meta_tool_token_budget()` output unchanged
- Update Architecture (maintainers) paragraph in [`tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) with Phase 2 module map; remove `invoke.rs` shim reference

**Outcome:** Tail-cleanup refactor green; 95 integration tests pass; token budget unchanged; safe to proceed to Phase 5.

---

### Phase 5 — Split `tool_discovery.rs` (~4–6 hours)

Largest remaining file in the gateway services layer (~782 lines). Lives outside `meta_tools/` but is the backing service for `SearchToolsTool` — higher blast radius than Phases 1–4.

- Create `tool_discovery_types.rs` — `DetailLevel`, `SearchContext`, `ToolIndexEntry`, `SearchToolsResult`
- Create `tool_discovery_index.rs` — index build, embedding hydration, cache key helpers
- Create `tool_discovery_search.rs` — `ToolDiscoveryService` search/rank execution methods
- Slim `tool_discovery.rs` to service struct + public façade re-exports, or delete and wire via `services/mod.rs`
- Extract `#[cfg(test)]` block to `tool_discovery_tests.rs` via `#[path]`
- Update `search_tools.rs` and `discovery_rank.rs` imports if paths change

**Outcome:** No `tool_discovery*` file exceeds ~300 prod lines; hybrid search integration tests pass; `SearchToolsTool` behavior unchanged.

---

### Phase 6 — Split `invoke_tool.rs` (~1–2 hours)

Only if invoke ergonomics churn continues; otherwise mechanical cleanup after Phase 5.

- Create `invoke_alias.rs` — `normalize_invoke_tool_name`, `resolve_invoke_server_id`, `resolve_invoke_tool`, `resolve_invoke_tool_args`, `first_nonempty_str`, `feature_matches_tool_name`
- Slim `invoke_tool.rs` to `InvokeToolTool` + `MetaTool` impl + response builders (`invoke_error`, `invoke_not_ready`, `invoke_preflight_ok`, `merge_default_params`)
- Extract alias-resolution + handler tests to `invoke_tool_tests.rs` via `#[path]`

**Outcome:** `invoke_tool.rs` ≤ ~200 prod lines; alias module ≤ ~120 lines; `meta_gateway_invoke` integration tests pass.

---

### Phase 7 — Split `search_tools.rs` (~2 hours)

Borderline at 496 lines; index helpers are cohesive but separable from the MCP handler.

- Create `search_tools_index.rs` — `build_active_index`, `build_and_cache_active_index`, `hydrate_active_embeddings`
- Slim `search_tools.rs` to `SearchToolsTool` + `MetaTool` impl only
- Update imports in `search_tools.rs` and any callers of index helpers

**Outcome:** Handler file ≤ ~380 lines; index module ≤ ~140 lines; `browse_mode_*` and `list_servers_*` integration tests pass.

---

### Phase 8 — Split `approval.rs` (~2 hours)

Broker is self-contained; split when approval UX work is active or file grows past ~600.

- Create `approval_types.rs` — `ApprovalPayload`, `ApprovalRequest`
- Create `approval_broker.rs` — `ApprovalBroker` impl + broker tests (or `approval_broker_tests.rs` via `#[path]`)
- Delete slimmed `approval.rs` or reduce to re-exports

**Outcome:** Broker ≤ ~350 lines; types ≤ ~120 lines; bind approval integration tests pass.

---

### Phase 9 — Split `disclosure.rs` (~2 hours)

Lowest priority — Phase 1 intentionally grouped 4 related tools. Split when disclosure tools diverge in churn.

- **Default:** 2×2 grouped — `disclosure_search.rs` (`SearchResourcesTool`, `SearchPromptsTool`) + `disclosure_read.rs` (`ReadResourceTool`, `FetchPromptTool`)
- **Alternative:** one file per tool if any single tool exceeds ~200 lines after edits
- Shared helpers stay inline or move to `meta_tool_common` only if duplicated

**Outcome:** No disclosure file exceeds ~280 lines; resource/prompt redirect integration tests pass.

---

### Phase 10 — Final verify + doc touch (~1 hour)

Run after Phases 1–9 (or after whichever phase subset ships).

- `pnpm validate`
- `cargo nextest run -p tests --profile ci-integration -E 'test(meta_tools) or test(meta_gateway_invoke)'`
- `cargo test -p mcpmux-gateway` — all `#[path]` test modules discovered
- `pnpm count-tokens` — confirm `measure_meta_tool_token_budget()` output unchanged
- Update Architecture (maintainers) in [`tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) with full post-Phase-9 module map
- Mark [`meta-tools-module-split-phase-2.md`](./meta-tools-module-split-phase-2.md) status **Shipped**

**Outcome:** Full validate green; entire refactor series complete; no file under `meta_tools/` or `tool_discovery*` above ~400 lines.

---

## Files to create / modify

| File | Change |
| ---- | ------ |
| [`invoke_payload_parse.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_payload_parse.rs) | **Create** — payload text parsing helpers (Phase 1) |
| [`invoke_result_shaping.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_result_shaping.rs) | **Create** — JSON/content shaping helpers (Phase 1) |
| [`invoke_result_filter_tests.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_result_filter_tests.rs) | **Create** — `#[path]` test module (Phase 1) |
| [`invoke_result_filter.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_result_filter.rs) | **Slim** — public filter API only (Phase 1) |
| [`diagnose_view.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose_view.rs) | **Create** — health types + view builders (Phase 2) |
| [`diagnose_server.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose_server.rs) | **Create** — `DiagnoseServerTool` (Phase 2) |
| [`diagnose_tests.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose_tests.rs) | **Create** — `#[path]` test module (Phase 2) |
| [`diagnose.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs) | **Delete** after Phase 2 |
| [`invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | **Delete** (Phase 3) |
| [`mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs) | Module declarations + `pub use` updates (Phases 1–3) |
| [`meta_gateway_invoke.rs`](../../tests/rust/tests/integration/meta_gateway_invoke.rs) | Retarget imports off `invoke::` shim (Phase 3) |
| [`tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) | Architecture paragraph refresh (Phases 4 + 10) |
| [`tool_discovery_types.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery_types.rs) | **Create** — search types (Phase 5) |
| [`tool_discovery_index.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery_index.rs) | **Create** — index build + embeddings (Phase 5) |
| [`tool_discovery_search.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery_search.rs) | **Create** — search/rank execution (Phase 5) |
| [`tool_discovery_tests.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery_tests.rs) | **Create** — `#[path]` test module (Phase 5) |
| [`tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs) | **Slim or delete** (Phase 5) |
| [`invoke_alias.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_alias.rs) | **Create** — alias resolution fns (Phase 6) |
| [`invoke_tool_tests.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_tool_tests.rs) | **Create** — `#[path]` test module (Phase 6) |
| [`invoke_tool.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_tool.rs) | **Slim** — handler only (Phase 6) |
| [`search_tools_index.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools_index.rs) | **Create** — index/cache helpers (Phase 7) |
| [`search_tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs) | **Slim** — handler only (Phase 7) |
| [`approval_types.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/approval_types.rs) | **Create** — payload/request types (Phase 8) |
| [`approval_broker.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/approval_broker.rs) | **Create** — broker impl (Phase 8) |
| [`approval.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/approval.rs) | **Delete or slim** (Phase 8) |
| [`disclosure_search.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/disclosure_search.rs) | **Create** — search resources/prompts (Phase 9) |
| [`disclosure_read.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/disclosure_read.rs) | **Create** — read/fetch tools (Phase 9) |
| [`disclosure.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs) | **Delete** after Phase 9 |

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`invoke_result_filter.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_result_filter.rs) | Tests start ~line 467; parse helpers ~62–195; shaping ~219–466 |
| [`diagnose.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs) | View builders ~39–329; tool ~330–446; tests ~447+ |
| [`invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | 12-line shim — sole external caller is `meta_gateway_invoke.rs` |
| [`meta-tools-module-split.md`](./meta-tools-module-split.md) | Phase 1 plan (complete); layout and deferral rationale |
| [`tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) | Maintainer module map added in Phase 1 P5 — update after Phases 4 + 10 |
| [`tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs) | ~782 lines; `ToolDiscoveryService` ~line 95; tests ~752+ |
| [`invoke_tool.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_tool.rs) | Handler ~81–355; alias fns ~30–77; tests ~400+ |
| [`search_tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs) | Index helpers ~18–135; handler ~136+ |
| [`approval.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/approval.rs) | Types ~91–132; broker ~133+; tests ~364+ |
| [`disclosure.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs) | 4 tools at ~75, ~179, ~267, ~371 |

---

## Risks & mitigations

| Risk | Mitigation |
| ---- | ------ |
| Circular imports (filter ↔ parse ↔ shaping) | One-way graph: filter imports parse + shaping; parse/shaping have no filter imports |
| `#[path]` tests not discovered by `cargo test` | Run `cargo test -p mcpmux-gateway` in Phase 4; `#[path]` modules are standard Rust |
| `pub(crate)` visibility drift on moved helpers | Grep before move; keep parse/shaping helpers `pub(crate)` only if siblings need them |
| Accidental behavior change | Move-only commits; no logic edits mixed with splits |
| Integration test import breakage | Phase 3 scoped to one file (`meta_gateway_invoke.rs`); grep confirms single `invoke::` site |
| `tool_discovery` split breaks `search_tools` | Phase 5 runs `cargo test -p mcpmux-gateway` + search integration tests before commit |
| Over-splitting disclosure adds import noise | Phase 9 defaults to 2×2 grouping; per-file only if a single tool exceeds ~200 lines |

---

## Related documentation

- [`meta-tools-module-split.md`](./meta-tools-module-split.md) — Phase 1 (complete)
- [`meta-surface-lean-core.md`](./meta-surface-lean-core.md) — 4-tool advertised surface (must not change)
- [`meta-tool-invoke-ergonomics.md`](./meta-tool-invoke-ergonomics.md) — invoke aliases in `invoke_tool.rs`
- [`tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) — agent-facing docs (no schema changes expected)

---

## Open questions (non-blocking)

- **Branch name** — continue on `feat/meta-surface-lean-core` vs cut `feat/meta-tools-module-split-phase-2`; default: same branch if PR not yet merged
- **Phase 5+ gating** — ship Phases 1–4 as one PR, Phases 5–9 as follow-up PR(s), or run all 10 phases in one branch; default: 1–4 first, 5–9 after merge
- **Disclosure split shape** — 2×2 grouped (default) vs four files; decide at Phase 9 based on per-tool line counts
- **`tool_discovery.rs` façade** — keep slim `tool_discovery.rs` re-export shim (like Phase 1 `invoke.rs`) vs delete and wire `services/mod.rs` directly; default: delete shim, no permanent re-export file

---

## Phase priority summary

| Priority | Phases | Trigger |
| -------- | ------ | ------- |
| **Immediate** | 1–4 | Files >500 lines or test-inflated (`invoke_result_filter`, `diagnose`, `invoke` shim) |
| **Next epic** | 5 | `tool_discovery.rs` (782 lines) — largest remaining monolith |
| **Churn-gated** | 6–8 | `invoke_tool`, `search_tools`, `approval` — split when file is actively edited or crosses ~600 lines |
| **Lowest** | 9 | `disclosure` — grouped tools still under 500; split only on divergence |
| **Always last** | 10 | Full validate + doc after whichever phase subset ships |
