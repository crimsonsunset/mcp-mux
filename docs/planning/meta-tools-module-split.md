# Meta-Tools Module Split — `tools.rs` + `invoke.rs` Co-Extract

**Last Updated:** Jun 5, 2026
**Status:** Planned — follow-up to PR [#4](https://github.com/crimsonsunset/mcp-mux/pull/4) review (`propose-opts-brainstorm` Option 5)
**Branch:** `feat/meta-tools-module-split` (new, off `dev` after #4 merges)
**Base branch:** `dev`
**Depends on:** Shipped lean-core + agent-UX work on `feat/meta-surface-lean-core` (readiness, browse, structured invoke, PR review fixes `2ae904e`)
**Unblocks:** Maintainable meta-tool crate layout; files under ~300 lines; safer follow-up edits without 1,400-line diffs

---

## Problem

Two meta-tool implementation files blew past the repo's ~200–300 line maintainability target after PR #4:

| File | Lines (Jun 2026) | Contents |
| ---- | ---------------: | -------- |
| [`tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | ~1,385 | 5 registered tools + shared helpers + search index builders + write approval path |
| [`invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | ~1,433 | Alias resolution, result shaping (~750 lines + tests), `InvokeToolTool::call` |

PR review brainstorm rated **Option 1** (extract `invoke-result-filter.rs` only) at 8/10 for ROI, but **Option 5** (co-extract both files) was selected: fix both oversized modules in one refactor so we don't land two competing layouts (`tools.rs` still monolithic while `invoke/` is split).

Cross-cutting helpers (`caller_space_id`, `derive_server_readiness`, `classify_invoke_denial`, `text_result`, `with_approval`) are imported by `invoke.rs`, `disclosure.rs`, and `diagnose.rs` today via `super::tools::` — any split must extract shared code first to avoid circular deps.

**This is a pure refactor.** No agent-visible behavior changes, no new meta tools, no token-budget changes.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Split strategy | **Option 5 — co-extract `tools.rs` + `invoke.rs`** | Both files are unmaintainable; splitting only invoke leaves the larger file untouched |
| 2 | Module layout | **Flat sibling modules under `meta_tools/`** (not a nested `tools/` directory) | Matches existing `disclosure.rs`, `diagnose.rs`, `invoke_backend.rs` pattern; `mod.rs` stays the registry factory |
| 3 | Shared helpers | **New `meta_tool_common.rs`** | `caller_*`, readiness, `text_result`, emit helpers, `with_approval`, `parse_uuid_arg` — imported by invoke, disclosure, diagnose, and per-tool files |
| 4 | One tool per file (large tools) | **`list_servers.rs`, `search_tools.rs`, `invoke_tool.rs`** | Largest implementations + most churn; each lands under ~350 lines with helpers extracted |
| 5 | Group small tools | **`feature_set_tools.rs`** — `ListFeatureSetsTool` + `GetToolSchemaTool` + schema parse helpers | Each is <200 lines; splitting further adds import noise without file-size win |
| 6 | Invoke shaping | **`invoke_result_filter.rs`** | `InvokeResultFilter`, `parse_invoke_filter`, `apply_invoke_result_filter`, `shape_json_value`, YAML/JSON parsers, unit tests (~570 lines) |
| 7 | Invoke handler | **`invoke_tool.rs`** (or slim `invoke.rs`) | `InvokeToolTool` + alias resolution (`resolve_invoke_*`, `normalize_invoke_tool_name`) + denial/response builders |
| 8 | `tools.rs` fate | **Delete after migration** — replace with `mod.rs` re-exports only | No permanent shim file; `token_budget.rs` and tests import concrete tool structs by module path |
| 9 | `ListAllToolsTool` | **Move to `meta_tool_common.rs` or delete** | `#[allow(dead_code)]` desktop-only path; keep if still referenced, otherwise remove in same PR |
| 10 | Test strategy | **No test rewrites** — same integration tests, `cargo nextest run -p tests` green | Refactor-only; existing `meta_tools.rs` + `meta_gateway_invoke.rs` are the regression gate |
| 11 | Timing | **After PR #4 merges to `dev`** | Avoid stacking a large file-move diff on an already 81-file PR |

---

## Scope

**In:**

- Extract `meta_tool_common.rs` with all cross-tool helpers currently in `tools.rs` lines 26–234 and write-path helpers (`with_approval`, `parse_uuid_arg`)
- Split `tools.rs` into per-concern modules (see target layout below)
- Split `invoke.rs` into `invoke_result_filter.rs` + slim invoke handler module
- Update `mod.rs`, `token_budget.rs`, and internal `use super::tools::` imports across `meta_tools/`
- `cargo fmt`, `cargo clippy --workspace -- -D warnings`, `pnpm test:rust:int` on meta-tool tests

**Out:**

| Item | Reason |
| ---- | ------ |
| Splitting `disclosure.rs` (481 lines) | Under limit; 4 related tools belong together (mirrors current pattern) |
| Splitting `registry.rs`, `approval.rs`, `diagnose.rs` | Already reasonably sized |
| Splitting `tool_discovery.rs` | Separate service layer; out of meta_tools scope |
| Agent-facing API / schema changes | Refactor only |
| `tools/` nested directory with `mod.rs` per tool | Flat siblings are the repo norm; nested dir adds path churn without benefit |
| Option 1-only partial split | Superseded by Option 5 selection |

---

## Target layout

```text
crates/mcpmux-gateway/src/services/meta_tools/
├── mod.rs                      # CORE_META_TOOLS, build_default_registry, pub use re-exports
├── registry.rs                 # unchanged
├── meta_tool_common.rs         # NEW — shared helpers + with_approval (~220 lines)
├── list_servers.rs             # NEW — ListServersTool (~160 lines)
├── search_tools.rs             # NEW — SearchToolsTool + index/cache helpers (~400 lines)
├── feature_set_tools.rs        # NEW — ListFeatureSetsTool + GetToolSchemaTool (~280 lines)
├── bind_workspace.rs           # NEW — BindCurrentWorkspaceTool (~160 lines)
├── invoke_result_filter.rs     # NEW — shaping + #[cfg(test)] block (~580 lines)
├── invoke_tool.rs              # NEW — InvokeToolTool + alias resolution (~320 lines)
├── invoke.rs                   # DELETED or thin re-export: pub use invoke_tool::InvokeToolTool
├── disclosure.rs               # unchanged imports → meta_tool_common
├── diagnose.rs                 # unchanged imports → meta_tool_common
├── token_budget.rs             # update tool struct imports
└── … (approval, diff, backends unchanged)
```

### Import graph (post-split)

```text
meta_tool_common  ←  list_servers, search_tools, feature_set_tools, bind_workspace
                 ←  invoke_tool, disclosure, diagnose

invoke_result_filter  ←  invoke_tool

mod.rs  ←  all tool structs for build_default_registry
token_budget  ←  tool structs (direct module paths)
```

### Public API surface (unchanged for external crates)

```rust
// services/mod.rs continues to re-export from meta_tools::mod
pub use meta_tools::{
    build_default_registry, CORE_META_TOOLS, InvokeToolTool, /* … */
};

// invoke public helpers stay reachable:
pub use invoke_result_filter::{InvokeResultFilter, apply_invoke_result_filter, shape_json_value, …};
pub use invoke_tool::{normalize_invoke_tool_name, resolve_invoke_server_id, …};
```

---

## Phases

### Phase 1 — Extract `meta_tool_common.rs` (~2 hours)

- Move from `tools.rs`: `text_result`, `caller_space_id`, `caller_resolution`, `derive_server_readiness`, `classify_invoke_denial`, `format_invoke_not_ready_action`, `blocking_reason_from_health`, `is_query_empty`, `build_server_readiness_map`, `emit_tools_list_changed`, `emit_workspace_binding_changed`, `with_approval`, `parse_uuid_arg`
- Update `invoke.rs`, `disclosure.rs`, `diagnose.rs` to `use super::meta_tool_common::…`
- Leave tool structs in `tools.rs` temporarily (mechanical move only)

**Outcome:** `tools.rs` drops ~200 lines; `invoke.rs` no longer imports readiness helpers from the monolith. `cargo check -p mcpmux-gateway` passes; integration tests unchanged.

---

### Phase 2 — Split `search_tools.rs` + `list_servers.rs` (~3 hours)

- Move `ListServersTool` + impl to `list_servers.rs`
- Move `SearchToolsTool`, `build_active_index`, `build_and_cache_active_index`, `hydrate_active_embeddings` to `search_tools.rs`
- `tools.rs` retains `ListFeatureSetsTool`, `GetToolSchemaTool`, `BindCurrentWorkspaceTool` only

**Outcome:** The two largest post-lean-core tools are isolated. `browse_mode_*` and `list_servers_*` integration tests pass without modification.

---

### Phase 3 — Split remaining tools (~2 hours)

- Create `feature_set_tools.rs` (`ListFeatureSetsTool`, `GetToolSchemaTool`, schema name parsers)
- Create `bind_workspace.rs` (`BindCurrentWorkspaceTool`)
- Delete `tools.rs`
- Add `mod.rs` declarations + `pub use` for tool structs consumed by `token_budget.rs` and `build_default_registry`

**Outcome:** No file under `meta_tools/` exceeds ~400 lines. `registry_advertises_core_tools_read_only_in_list` and bind approval tests pass.

---

### Phase 4 — Split `invoke.rs` (~3 hours)

- Create `invoke_result_filter.rs` — move `InvokeResultFilter`, all shaping/parsing helpers, `apply_invoke_result_filter`, `shape_json_value`, and the `#[cfg(test)] mod tests` block
- Create `invoke_tool.rs` — `InvokeToolTool`, alias resolution fns, `merge_default_params`, `invoke_error` / `invoke_not_ready` / `invoke_preflight_ok`
- Replace `invoke.rs` with thin re-exports **or** delete `invoke.rs` and `pub mod invoke_tool` in `mod.rs` (pick one; prefer keeping `invoke.rs` as re-export shim for one release to avoid breaking `meta_tools::invoke::` paths in tests)

**Outcome:** `invoke_result_filter` unit tests run in-module; `meta_gateway_invoke.rs` integration tests pass; `pnpm count-tokens` budget unchanged.

---

### Phase 5 — Verify + doc touch (~1 hour)

- `pnpm validate`
- `cargo nextest run -p tests --profile ci-integration -E 'test(meta_tools) or test(meta_gateway_invoke)'`
- Add one line to [`tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) architecture section pointing maintainers at the new module map (optional, 1 paragraph)

**Outcome:** Full validate green; no diff in `measure_meta_tool_token_budget()` output; PR ready for review as refactor-only.

---

## Files to create / modify

| File | Change |
| ---- | ------ |
| [`meta_tool_common.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/meta_tool_common.rs) | **Create** — shared helpers (Phase 1) |
| [`list_servers.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/list_servers.rs) | **Create** — `ListServersTool` (Phase 2) |
| [`search_tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs) | **Create** — `SearchToolsTool` + index helpers (Phase 2) |
| [`feature_set_tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/feature_set_tools.rs) | **Create** — list FS + get schema (Phase 3) |
| [`bind_workspace.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs) | **Create** — bind write tool (Phase 3) |
| [`invoke_result_filter.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_result_filter.rs) | **Create** — result shaping (Phase 4) |
| [`invoke_tool.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_tool.rs) | **Create** — `InvokeToolTool` handler (Phase 4) |
| [`tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | **Delete** after Phase 3 |
| [`invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | **Replace** with re-exports or delete (Phase 4) |
| [`mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs) | Module declarations + `build_default_registry` import paths |
| [`token_budget.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/token_budget.rs) | Update tool struct import paths |
| [`disclosure.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs) | `meta_tool_common` imports |
| [`diagnose.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs) | `meta_tool_common` imports |
| [`tests/rust/tests/integration/meta_gateway_invoke.rs`](../../tests/rust/tests/integration/meta_gateway_invoke.rs) | Update `use meta_tools::invoke::` paths if shim removed |

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | Current monolith — section markers at lines 386 (`list_servers`), 663 (`search_tools`), 1088 (`get_tool_schema`), 1232 (`bind`) |
| [`invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | Handler ~lines 290–561; shaping ~lines 18–287 + 563–855; tests ~857–1431 |
| [`disclosure.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs) | Reference pattern — 4 tools, 481 lines, one file (target size for grouped small tools) |
| [`mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs) | `build_default_registry` registers 7 tools from `tools::` today |
| [`token_budget.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/token_budget.rs) | Instantiates all 11 tool unit structs for byte measurement |
| PR [#4 review](https://github.com/crimsonsunset/mcp-mux/pull/4) | Option 5 brainstorm origin; Option 1 documented as lower-blast-radius alternative |

---

## Risks & mitigations

| Risk | Mitigation |
| ---- | ---------- |
| Circular imports (`invoke_tool` ↔ `meta_tool_common` ↔ `search_tools`) | Common module has zero tool-specific imports; tools only depend on common + registry |
| `pub(crate)` visibility drift | Grep for `pub(crate) fn` in `tools.rs` before move; preserve visibility per symbol |
| Test import breakage | Keep `invoke.rs` re-export shim for one release; deprecate in comment |
| Clippy `too_many_lines` / module count | Run `cargo clippy --workspace -- -D warnings` in Phase 5 |
| Accidental behavior change during move | Move-only commits per phase; no logic edits mixed with file splits |

---

## Related documentation

- [`meta-surface-lean-core.md`](./meta-surface-lean-core.md) — 4-tool advertised surface (must not change)
- [`meta-tool-agent-ux-path-to-9.md`](./meta-tool-agent-ux-path-to-9.md) — readiness/browse work currently living in `tools.rs`
- [`meta-tool-invoke-ergonomics.md`](./meta-tool-invoke-ergonomics.md) — invoke aliases + `default_params` in `invoke.rs`
- [`tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) — agent-facing docs (no schema changes expected)

---

## Open questions (non-blocking)

- **`invoke.rs` shim duration** — keep re-export file for one release vs delete immediately; default: keep shim, remove in a follow-up cleanup PR
- **`ListAllToolsTool`** — confirm desktop/admin still needs it before moving; if unused, delete instead of carrying dead code
- **Phase granularity in git** — one commit per phase vs single squash; prefer one commit per phase for easier bisect
