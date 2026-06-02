# Meta-Surface Lean Core — Hidden-but-Callable Tool Trimming

**Last Updated:** Jun 2, 2026
**Status:** Shipped on `feat/meta-surface-lean-core` (through `9532ce0`) — PR [#4](https://github.com/crimsonsunset/mcp-mux/pull/4) → `dev`; agent-validated in Cursor (Jun 2, 2026)
**Branch:** `feat/meta-surface-lean-core`
**Base branch:** `dev`
**Depends on:** Nothing — builds on the shipped invoke-ergonomics model (default_params, required_params, bare-name suggestions)
**Unblocks:** Leaner agent startup context; removes the open `list_changed` reliability question as a blocker

---

## Problem

The `mcpmux_*` meta surface advertises **11 tools flat** in every `tools/list` response regardless of what the binding actually needs. For the common case — a tool-only binding (GitHub, Firebase, Atlassian, etc.) — 7 of those 11 tools are either irrelevant (resource/prompt quartet, diagnose) or only needed once and then forgotten (bind, list_feature_sets). They're burning ~800 Claude tokens of startup context every session for no benefit.

The naive fix is a dynamic surface (Option 2 — hide resource/prompt tools when the binding has no resources/prompts). That requires `notifications/tools/list_changed` to fire reliably after bind/unbind so Cursor re-renders. Cursor's handling of that notification has been inconsistent, and testing it per-build is overhead we don't want.

Reading the dispatch path more carefully surfaced a cleaner path: the handler already gates `call_tool` on `registry.contains(name)`, **not** on whether the tool appeared in `tools/list`. Advertisement and callability are already decoupled. A tool can be registered (always callable) without being advertised (visible in `tools/list`). That hidden-but-callable quadrant is unused today but costs nothing to use.

Combined with the existing recovery strings — every path that fails because a hidden tool is needed already names that tool by name in its error message — agents reach hidden tools through the error channel exactly when they need them. No `list_changed` required, no stranding risk.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Trim approach | **Option 6 — lean advertised core + hidden-but-callable remainder** | Dodges the `list_changed` reliability blocker entirely. Static advertised set, no dynamic re-rendering, no notification dependency. |
| 2 | Core tools (always advertised) | **`search_tools`, `invoke_tool`, `get_tool_schema`, `list_servers`** | The hot path every session: discover → schema → invoke. `list_servers` is 90 tokens and exposes `bindable_feature_set_ids` needed for the bind flow. |
| 3 | `list_feature_sets` | **Hidden** | Named in the `search_tools` empty-results hint (`"call mcpmux_list_feature_sets then mcpmux_bind_current_workspace"`). Agents reach it on demand; no need to advertise cold. |
| 4 | `bind_current_workspace` | **Hidden** | `format_server_inactive_error` already says `"→ mcpmux_bind_current_workspace with a FeatureSet"`. Agents get here only when a server is inactive — the error delivers the tool name. |
| 5 | `diagnose_server` | **Hidden** | Operator/debug tool, not a daily-driver. Not reachable from normal agent flow — acceptable; a human operator can call it directly when needed. |
| 6 | Resource/prompt quartet | **Hidden** | For tool-only bindings (the majority) these are always irrelevant. The `search_resources`/`search_prompts` calls are self-discoverable via `mcpmux_search_tools` hints when a binding actually has resources/prompts. |
| 7 | Dispatch path | **No change** | Handler gates on `registry.contains(name)`, not on advertisement. Hidden tools are callable today without any code change to the dispatch path. |
| 8 | `list_changed` | **Not required** | Advertised set is static (always the 4 core tools). No bind/unbind changes it. Eliminates the blocking question from the prior scoping session. |
| 9 | Recovery string quality | **Audit-and-patch as needed** | Each hidden tool must be reachable via at least one named error/hint. Most already are (verified in scoping); Phase 2 audits and fills gaps. |
| 10 | Description trimming | **Deferred** — not part of this plan | `invoke_tool`'s `filter` sub-schema is the next-largest target (~150–200 tokens). Worth doing but orthogonal; track separately. |

---

## Scope

**In:**

- Filter `list_as_tools()` to advertise only the 4 core tools
- Audit every hidden tool's recovery string path — verify agents can reach it from a named error or hint
- Patch any recovery string that doesn't name its corresponding hidden tool
- Update `tool-discovery-and-search.md` to reflect the new advertised count and hidden-tool model

**Out:**

| Item | Reason |
| ---- | ------ |
| Option 2 capability-gate (dynamic surface based on binding resources/prompts) | Superseded by Option 6 — same bloat reduction for tool-only bindings, simpler, no `list_changed` dependency |
| `list_changed` notification path changes | Not needed for this option; static advertised set |
| Description trimming on core tools (`invoke_tool` filter sub-schema) | ~150–200 tokens additional; separate pass, different risk profile |
| `diagnose_server` settings gate (`gateway.meta_tools_diagnose_enabled`) | Was considered under Option 3; not needed — hidden is sufficient |
| Any changes to the dispatch path or `MetaToolRegistry::call` | Already works; zero changes required |
| Surfacing hidden tools in any UI affordance | Out — the agent-facing change is the entire scope here |

---

## The Model

### Advertisement vs. dispatch (current)

```
tools/list  ←  list_as_tools()  ←  iterates all registered tools (11 today)
call_tool   ←  registry.call()  ←  gates on registry.contains(name)  ← (separate check)
```

Both sets are identical today because `list_as_tools()` emits everything registered. They don't need to be.

### Advertisement vs. dispatch (post-change)

```
tools/list  ←  list_as_tools()  ←  filters to CORE_META_TOOLS (4 tools)
call_tool   ←  registry.call()  ←  gates on registry.contains(name)  ← unchanged (all 11 callable)
```

The 7 hidden tools remain in the registry — fully callable if named. The dispatch path has no knowledge of which tools are advertised.

### Core set

```rust
const CORE_META_TOOLS: &[&str] = &[
    "mcpmux_search_tools",
    "mcpmux_invoke_tool",
    "mcpmux_get_tool_schema",
    "mcpmux_list_servers",
];
```

### Token budget (tiktoken cl100k_base; ×1.1 for Claude estimate)

| Surface | tiktoken | ~Claude est. |
| ------- | -------: | -----------: |
| All 11 (current) | 1,471 | ~1,618 |
| Core 4 (post) | 738 | ~812 |
| **Saved** | **733** | **~806** |

### Recovery string coverage (hidden tools → how agents reach them)

| Hidden tool | Named in | Location |
| ----------- | -------- | -------- |
| `bind_current_workspace` | `"server '{id}' is inactive → mcpmux_bind_current_workspace…"` | `routing.rs:62` |
| `list_feature_sets` | `"call mcpmux_list_feature_sets then mcpmux_bind_current_workspace"` | `tools.rs:762` |
| `search_resources` | `search_tools` hint when resources exist but aren't surfaced (audit Phase 2) | TBD |
| `read_resource` | `"Use mcpmux_read_resource instead: mcpmux_read_resource({\"uri\": \"…\"})"` | `routing.rs:88` |
| `search_prompts` | `search_tools` hint (audit Phase 2) | TBD |
| `fetch_prompt` | `"Use mcpmux_fetch_prompt instead: mcpmux_fetch_prompt({…})"` | `routing.rs:96` |
| `diagnose_server` | Not in agent recovery path — operator tool, human-callable directly | n/a |

---

## Phases

### Phase 1 — Filter `list_as_tools()` to core set (~1 hr) — **Done**

- Define `CORE_META_TOOLS: &[&str]` constant in `meta_tools/mod.rs` (or `registry.rs`) with the 4 core names
- Update `MetaToolRegistry::list_as_tools()` to filter `self.tools` to only entries in `CORE_META_TOOLS`
- No changes to `MetaToolRegistry::call()`, `registry.register()`, or `build_default_registry`
- `pnpm validate` clean

**Outcome:** `tools/list` returned to any connected MCP client contains exactly 4 `mcpmux_*` entries. A direct `call_tool` for `mcpmux_bind_current_workspace` (or any other hidden tool) still succeeds — the dispatch path is unaffected. Confirm by checking `list_as_tools()` unit-level: 4 tools returned; `registry.contains("mcpmux_bind_current_workspace")` still true.

---

### Phase 2 — Recovery string audit (~half day) — **Done**

- Walk each of the 6 hidden agent-facing tools (excludes `diagnose_server`):
  - Trace the exact call path that first fails without the tool
  - Verify the resulting error/hint names the hidden tool explicitly
- For any gap (primarily `search_resources` and `search_prompts`): add a terse usage hint to the relevant empty-results or no-results path — format: `"Use mcpmux_search_resources to discover readable resources."`
- Do not add hints to `diagnose_server` — operator, not agent-facing

**Outcome:** Every agent-reachable hidden tool is named in at least one error or hint string that fires when the agent needs it. An agent with only the 4 core tools can reach `bind`, `list_feature_sets`, `read_resource`, and `fetch_prompt` without prior knowledge of their names — the error channel delivers them on first need.

---

### Phase 3 — Doc update + cleanup (~half day) — **Done**

- Update `docs/backend/technical/tool-discovery-and-search.md`:
  - "The Meta Surface" section: update prose count (now 4 advertised, 7 hidden-but-callable)
  - Replace the flat ASCII `tools/list` diagram with one that shows core vs. hidden-but-callable split
  - Add a "Hidden-but-callable tools" subsection explaining the dispatch vs. advertisement split and the recovery string model
- Remove `scripts/count-meta-tool-tokens.sh` (superseded by the `.py` version)
- Add `count-tokens` script to `mcp-mux/package.json` pointing at `scripts/count-meta-tool-tokens.py`
- `pnpm validate` clean

**Outcome:** Doc accurately reflects the post-change surface. The `.py` token counter is runnable via `pnpm count-tokens` and the `.sh` file is gone. Anyone reading the doc understands why `tools/list` shows 4 tools while more are callable.

---

## Files to create / modify

| File | Change |
| ---- | ------ |
| [`crates/mcpmux-gateway/src/services/meta_tools/mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs) | **Add** `CORE_META_TOOLS` constant |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | **Modify** `list_as_tools()` to filter on `CORE_META_TOOLS` |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | **Modify** (if needed) — add usage hints for `search_resources` / `search_prompts` in empty-result paths |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | **Modify** (if needed) — patch `search_tools` empty-result hints for resource/prompt discovery paths |
| [`docs/backend/technical/tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) | **Modify** — meta surface count, ASCII diagram, hidden-but-callable section |
| `scripts/count-meta-tool-tokens.py` | **Keep** — already written; add `pnpm` entry |
| `scripts/count-meta-tool-tokens.sh` | **Delete** — superseded |
| `mcp-mux/package.json` | **Modify** — add `count-tokens` script |

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | `list_as_tools()` (L224, context-free today); `is_enabled()` master switch; `call()` dispatch (gates on `contains`, not advertisement) |
| [`crates/mcpmux-gateway/src/services/meta_tools/mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs) | `build_default_registry` — all 11 `registry.register(...)` calls |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs) | `list_as_tools()` call (L701) and `is_meta_tool` + `contains` dispatch gate (L738–740) — confirmed advertisement and dispatch are independent |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | `format_server_inactive_error` (names `bind`); `format_direct_read_redirect` (names `read_resource`); `format_direct_fetch_prompt_redirect` (names `fetch_prompt`) |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | `search_tools` empty-result hints naming `list_feature_sets` + `bind` (L762–792) |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs) | `get_readable_resources_for_grants` / `get_fetchable_prompts_for_grants` — the free capability signal (Option 2's mechanism; not used here but available if Option 6 ever needs gating) |
| [`docs/backend/technical/tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) | "The Meta Surface" section — stale count fixed in prior session; diagram + hidden-tool model to be added in Phase 3 |
| `scripts/count-meta-tool-tokens.py` | tiktoken-based per-tool token count (baseline for confirming the ~800 token saving post-implementation) |

---

## Agent validation (Jun 2, 2026)

Cursor session against a live `pnpm dev:restart` build on `localhost:45818`:

| Check | Result |
| ----- | ------ |
| `tools/list` meta count | 4 — `search_tools`, `invoke_tool`, `get_tool_schema`, `list_servers` |
| Search hit shape | `bare_name` + `required_params: [{ name, type }]` present |
| Invoke with `qualified_name` as `tool` | OK — no `server_server_tool` double-prefix |
| Invoke with `bare_name` as `tool` | OK — same routing |
| `get_tool_schema` skipped | Context7 `resolve-library-id` → `query-docs` succeeded using search `required_params` only |
| Hidden tools | Not in `tools/list`; still reachable by name when needed |

Bundled in the same branch: invoke ergonomics round 2 ([`meta-tool-invoke-ergonomics.md`](./meta-tool-invoke-ergonomics.md)).

---

## Open questions (deferred, not blocking)

- **Description trimming on core tools** — `invoke_tool`'s nested `filter` sub-schema is ~150–200 tokens of the remaining 738. Highest-leverage trim on the core set. Separate pass; different risk (modifying a tool agents actively use).
- **`diagnose_server` discoverability** — no agent recovery string names it today. Acceptable for an operator tool, but if it ever needs to be agent-discoverable, add a terse mention to `search_tools`'s no-unhealthy-servers hint path.
- **Option 2 capability-gate as a future layer** — if resource/prompt bindings become more common and the `list_changed` reliability question is resolved for a future Cursor build, Option 2 (advertise the quartet only when the binding has resources/prompts) could layer on top of Option 6 to bring the core from 4 to 4–8 dynamically. Not blocking anything now.

---

## Related documentation

- [`docs/planning/meta-tool-invoke-ergonomics.md`](./meta-tool-invoke-ergonomics.md) — the prior invoke ergonomics work this plan follows; `required_params`, `default_params`, bare-name suggestions are all shipped
- [`docs/backend/technical/tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) — primary doc updated in Phase 3
- [`docs/backend/reference/dynamic-mcp-toggle-meta-tools.md`](../backend/reference/dynamic-mcp-toggle-meta-tools.md) — the `gateway.meta_tools_enabled` master switch; the new filter sits alongside it, not replacing it
