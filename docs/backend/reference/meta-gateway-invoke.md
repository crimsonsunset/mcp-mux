> **Synthesis:** This doc is synthesized into [`../technical/tool-discovery-and-search.md`](../technical/tool-discovery-and-search.md). Read that doc first; come here for the original design decisions, phasing history, and QA results.

# Meta-Gateway Invoke (Search → Schema → Invoke)

**Last Updated:** Jun 2, 2026
**Status:** ✅ Phases A–D implemented; lean-core + invoke ergonomics shipped on fork `dev` — see [`meta-gateway-invoke-qa.md`](../../testing/meta-gateway-invoke-qa.md), [`meta-surface-lean-core.md`](../../planning/meta-surface-lean-core.md), [`meta-tool-invoke-ergonomics.md`](../../planning/meta-tool-invoke-ergonomics.md)
**Branch:** `feat/meta-surface-lean-core` → fork `dev` (PR [#4](https://github.com/crimsonsunset/mcp-mux/pull/4))
**Base branch:** `dev` on personal fork; upstream contribution is topic-stacked, not `main`
**Issue:** Fork-only; upstream megapr [#155](https://github.com/mcpmux/mcp-mux/pull/155) closed — use #154 stack for meta-tools upstream
**Depends on:** [`feature-set-consent-model.md`](./feature-set-consent-model.md) (bind-only activation; session enable/create removed); workspace bindings / FeatureSets from PR #151
**Supersedes:** Token-budget approach in [`tool-level-session-pin.md`](./tool-level-session-pin.md) — pin filtered a bloated `tools/list`; this doc replaces that model with a fixed meta surface + invoke path. Session pin may return as an invoke ACL in Phase F (very optional, last).
**Unblocks:** Agent-usable McpMux sessions at scale (240+ backend tools installed, ~12 tools in client context); homelab + multi-clone installs without context-window collapse

---

## Problem

Routing every AI client through one McpMux gateway endpoint solved config duplication and credential sprawl. It introduced a different bottleneck: **tool definition bloat in the client context window**.

Concrete symptoms from a May 2026 Cursor session against a real install:

| Symptom                                                             | Number               |
| ------------------------------------------------------------------- | -------------------- |
| Installed servers in Space                                          | 34                   |
| Tools in operator `mcpmux_list_all_tools` dump (not registered)     | 1,581 (~855 KB JSON) |
| Tools exposed in Cursor session (GWorkspace × 2 clones)             | 240                  |
| GitHub tools invokable after workspace FeatureSet bind                | 41                   |
| GitHub tool schemas in Cursor MCP descriptor folder                 | 0                    |
| Approximate tokens consumed by 240 tool definitions                 | ~30–50k              |

The consent model ([`feature-set-consent-model.md`](./feature-set-consent-model.md)) replaced session enable/disable: agents activate capability by binding an existing FeatureSet with `mcpmux_bind_current_workspace` (hidden, error-hinted), not `mcpmux_enable_server`. **`tools/list` still advertises only meta tools + optional surfaced backend tools** — never the full catalog. The LLM must guess parameter names without schema-on-demand unless it uses `mcpmux_search_tools` / `mcpmux_get_tool_schema`.

Competing gateways solve this with a **fixed meta surface** and **progressive disclosure**: search → load schema → invoke. McpMux ships that model with a **lean advertised core** (4 meta tools) and 7 hidden-but-callable meta tools for bind, disclosure, and diagnose paths.

The user-facing ask (May 2026 session):

> I'd rather 1–2 more calls that actually work well than hundreds of tool defs I can't call correctly.

This doc defines that model for McpMux while preserving its product strengths: OS keychain credentials, Spaces, FeatureSets, per-client auth, and the server registry.

---

## Decisions

| #   | Decision                    | Choice                                                                                                                                                        | Rationale                                                                                                                                                                                                                                                                                                                                              |
| --- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Client `tools/list` shape   | **Meta tools + optional surfaced backend tools only** — never the full backend catalog                                                                        | Fixes context bloat. Backend tools are invoked through `mcpmux_invoke_tool`, not registered in the client tool list (except surfaced exceptions).                                                                                                                                                                                                      |
| 2   | Discovery API               | **`mcpmux_search_tools` with `detail_level`**: `name` \| `description` \| `schema`\*\*                                                                        | Replaces dumping `mcpmux_list_all_tools` for agent workflows. Supports server_id filter, pagination, and query string. Start with substring + server_id filter; TF-IDF semantic rank is Phase D optional.                                                                                                                                              |
| 3   | Schema API                  | **`mcpmux_get_tool_schema`** — single or batch via `tools` (string or array); aliases **`tool_name`** / **`tool`** for a single name; accepts **bare names** (`list_issues`) or **qualified names** (`github_list_issues`); optional `compact: true` | Agents must read schemas before invoke without relying on Cursor descriptor JSON files. Batch load for multi-tool workflows (e.g. issue read + comment write). |
| 4   | Invoke API                  | **`mcpmux_invoke_tool({ server_id, tool, args, filter? })`** — one entry point for all backend calls                                                          | Mirrors `gateway_invoke`. Routes through existing `RoutingService::call_tool` after permission checks. Optional `filter` arg activates result shaping (Phase B).                                                                                                                                                                                       |
| 5   | FeatureSet semantics        | **FeatureSets define what is _invokable_, not what appears in `tools/list`**                                                                                  | Binding / grant / session-enable controls the candidate pool for search + invoke. Security boundary stays meaningful without polluting client context.                                                                                                                                                                                                 |
| 6   | Surfaced tools escape hatch | **FeatureSet members may mark tools `surfaced: true` (0–N per set)** — promoted into `tools/list` for one-hop hot paths                                       | Default: **zero surfaced everywhere**, including built-in bundles. No bundle auto-promotes backend tools. Opt-in only via FeatureSet editor (Phase C).                                                                                                                                                                                                 |
| 7   | Invoke authorization        | **Fail closed** — `invoke_tool` rejects when target server/tool is outside effective permission set                                                           | Binding FeatureSet grants control the candidate pool for search + invoke. Inactive servers error with `→ mcpmux_bind_current_workspace`.                                                                                                                                                                                                                 |
| 8   | Server activation           | **`mcpmux_bind_current_workspace` only** — bind an existing FeatureSet to the workspace root (hidden from `tools/list`, error-hinted)                         | Session `mcpmux_enable_server` / `mcpmux_disable_server` and agent `mcpmux_create_feature_set` were removed per consent model. Humans author bundles in UI; agents bind existing ones.                                                                                                                                                                    |
| 9   | Error messages              | **Actionable, bounded errors** — no dumping full available-tool lists                                                                                         | e.g. `"github inactive → mcpmux_bind_current_workspace"`, `"unknown tool → did you mean list_issues?"` (bare names). Levenshtein suggestions on invoke/read/fetch.                                                                                                                                                                                       |
| 10  | Rollout                     | **Hard cut — no legacy opt-out**                                                                                                                              | Non-surfaced backend tools never appear in `tools/list` and direct `call_tool` is rejected with a redirect to `mcpmux_invoke_tool`. **Exception:** FeatureSet members marked `surfaced: true` are promoted into `tools/list` and callable in one hop.                                                                                                                                                                   |
| 11  | Advertised meta surface     | **4 core tools in `tools/list`** — `search_tools`, `invoke_tool`, `get_tool_schema`, `list_servers`; 7 hidden-but-callable                                                   | See [`meta-surface-lean-core.md`](../../planning/meta-surface-lean-core.md). `mcpmux_list_all_tools` is not registered on the agent surface. Hidden tools reached via error/hint recovery strings.                                                                                                                                                        |
| 12  | Result shaping scope        | **Phase B only on `invoke_tool`** — opt-in via explicit `filter`: `max_rows`, `max_bytes`, `fields`, `format: summary`. Omit filter → backend response as-is. | Agents pass `filter` when they know a tool returns large payloads. No default truncation.                                                                                                                                                                                                                                                              |
| 13  | REST / OpenAPI capabilities | **Out of scope here** — Phase E / separate planning doc                                                                                                       | [`docs/guide/gateway.mdx`](../guide/gateway.mdx) covers admin REST, not REST→MCP capability YAML. No conflict; different layer.                                                                                                                                                                                                                        |

---

## The Model

### What the agent sees (current — Jun 2026)

```text
tools/list — 4 advertised mcpmux_* meta tools
├── mcpmux_search_tools
├── mcpmux_invoke_tool
├── mcpmux_get_tool_schema
└── mcpmux_list_servers

Hidden-but-callable (not in tools/list; reachable by name or error hints)
├── mcpmux_list_feature_sets
├── mcpmux_bind_current_workspace
├── mcpmux_search_resources / mcpmux_read_resource
├── mcpmux_search_prompts / mcpmux_fetch_prompt
└── mcpmux_diagnose_server          (operator/debug)

resources/list → surfaced backend resources only (default zero)
prompts/list   → surfaced backend prompts only (default zero)
[0–N surfaced backend tools]        (optional, from FeatureSet)
```

See [`tool-discovery-and-search.md`](../technical/tool-discovery-and-search.md) for the live model.

### Agent workflow (GitHub read example — current)

```text
1. mcpmux_list_servers                          → github: inactive or enabled_via_binding
2. (if inactive) mcpmux_bind_current_workspace  → bind FeatureSet that includes github
3. mcpmux_search_tools({
     query: "list issues",
     server_id: "github",
     detail_level: "description"
   })                                           → bare_name + required_params inline
4. mcpmux_invoke_tool({                         → schema optional for simple tools
     server_id: "github",
     tool: "list_issues",                       → bare or qualified (github_list_issues)
     args: { owner: "mcpmux", repo: "mcp-mux", state: "OPEN" }
   })
```

Two to three meta calls before the backend call when search hits include `required_params`; `get_tool_schema` remains for complex shapes.

### Permission composition (current)

```text
1. (space, feature_set_ids) ← FeatureSetResolverService (workspace binding)
2. binding_servers          ← servers_for(space, feature_set_ids)
3. invokable_tools          ← Tool features for binding_servers ∩ FeatureSet members
4. tools/list               ← CORE_META_TOOLS (4) ∪ surfaced(invokable_tools)
5. search_tools / invoke    ← scoped to invokable_tools only
```

Session overrides (`SessionOverrideRegistry`, enable/disable) were removed with the consent model. Prompts and resources: **hard cut in Phase D** — agents use hidden disclosure meta tools (`search_*` → `read_*` / `fetch_*`).

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

| File                                                                  | Purpose                                                                          | Status             |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ------------------ |
| `crates/mcpmux-gateway/src/services/tool_discovery.rs`                | Index + search + schema lookup over Space tool features                          | ✅ Done            |
| `crates/mcpmux-gateway/src/services/meta_tools/invoke.rs`             | `InvokeToolTool` impl — permission check, routing, error mapping, result shaping | ✅ Done            |
| `crates/mcpmux-gateway/src/services/meta_tools/invoke_backend.rs`     | `InvokeToolBackend` trait + `RoutingService` adapter for testable invoke routing | ✅ Done            |
| `tests/rust/src/canned_invoke_backend.rs`                             | Canned backend for filter e2e integration tests                                  | ✅ Done            |
| `tests/rust/tests/integration/meta_gateway_invoke.rs`                 | Search, schema, invoke, disclosure, polish                                       | ✅ Done (36 tests) |
| `docs/planning/meta-gateway-invoke-qa.md`                             | Manual QA runbook (Phases A–D)                                                   | ✅ Done            |
| `docs/planning/meta-gateway-invoke.md`                                | This doc                                                                         | ✅ Done            |
| `crates/mcpmux-gateway/src/services/resource_discovery.rs`            | Resource index + search                                                          | ✅ Done            |
| `crates/mcpmux-gateway/src/services/prompt_discovery.rs`              | Prompt index + search                                                            | ✅ Done            |
| `crates/mcpmux-gateway/src/services/discovery_rank.rs`                | TF-IDF rank + Levenshtein helpers                                                | ✅ Done            |
| `crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs`         | Search/read/fetch meta tools                                                     | ✅ Done            |
| `crates/mcpmux-gateway/src/services/meta_tools/disclosure_backend.rs` | Pool adapter for read/fetch                                                      | ✅ Done            |

## Files to modify

| File                                                                                                                           | Change                                                                                             | Status  |
| ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- | ------- |
| [`crates/mcpmux-gateway/src/services/mod.rs`](../../crates/mcpmux-gateway/src/services/mod.rs)                                 | `pub mod tool_discovery;`                                                                          | ✅ Done |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs)       | `SearchToolsTool`, `GetToolSchemaTool`; extend `ListAllToolsTool` with optional `server_id` filter | ✅ Done |
| [`crates/mcpmux-gateway/src/services/meta_tools/mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs)           | Register new tools; wire `ToolDiscoveryService` + `InvokeToolBackend` into `MetaToolContext`       | ✅ Done |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | Extend `MetaToolContext` with discovery + invoke backend handles                                   | ✅ Done |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs)                 | Split into `get_advertised_tools_for_grants` vs `get_invokable_tools_for_grants`                   | ✅ Done |
| [`crates/mcpmux-gateway/src/pool/features/resolution.rs`](../../crates/mcpmux-gateway/src/pool/features/resolution.rs)         | `resolve_surfaced_feature_ids` for surfaced promotion                                              | ✅ Done |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs)                                 | `format_direct_call_redirect`; actionable invoke errors                                            | ✅ Done |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs)                                   | Advertised-only `tools/list`, `resources/list`, `prompts/list`; invoke/read/fetch redirect gates   | ✅ Done |
| [`crates/mcpmux-core/src/domain/feature_set.rs`](../../crates/mcpmux-core/src/domain/feature_set.rs)                           | `surfaced: bool` on `FeatureSetMember`                                                             | ✅ Done |
| [`apps/desktop/src/features/featuresets/FeatureSetPanel.tsx`](../../apps/desktop/src/features/featuresets/FeatureSetPanel.tsx) | Per-feature "Surface in client" toggle (tools, resources, prompts) + explainer tooltip             | ✅ Done |
| [`apps/desktop/src/features/settings/SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx)             | Meta-tools copy for search → schema → invoke workflow                                              | ✅ Done |
| [`README.md`](../../README.md)                                                                                                 | Agent-facing search → schema → invoke flow; checkbox vs Surface in Feature Sets                    | ✅ Done |
| [`docs/guide/feature-sets.mdx`](../guide/feature-sets.mdx)                                                                     | Included vs Surface editor explainer; invoke ACL semantics                                         | ✅ Done |

---

## Phasing

### Phase A — Meta invoke core

**Effort:** ~3–4 days  
**Status:** ✅ Implemented — manual QA sections 0–4 pass ([`meta-gateway-invoke-qa.md`](../../testing/meta-gateway-invoke-qa.md))

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

| Payload shape                                                      | Applicable filter keys                      | Behavior                                                                                                                  |
| ------------------------------------------------------------------ | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Plain text (`content[].text` non-JSON, non-YAML document)          | `max_bytes` only                            | Returns `{ returned, total, truncated, text }` envelope when over limit. `max_rows` / `fields` / `format` ignored.        |
| YAML mapping/sequence in `content[].text`                          | `max_rows`, `fields`, `format`, `max_bytes` | Parsed via `yaml_serde` → `serde_json::Value`, then same shaping as JSON. Keys like `results[16]` normalize to `results`. |
| Top-level JSON array                                               | `max_rows`, `fields`, `format`, `max_bytes` | When `total > max_rows`: `{ returned, total, truncated, items: [...] }`                                                   |
| JSON object with heavy array key (`issues`, `items`, `results`, …) | same                                        | Metadata merged at object top-level; array under original key name                                                        |
| JSON serialized inside text content block                          | same                                        | Parsed then shaped; re-serialized into `text`                                                                             |
| `structured_content` on `CallToolResult`                           | same                                        | Shaped independently via `apply_invoke_result_filter`                                                                     |

**`format` semantics (requires `max_rows`):**

- `full` — sample size = `max_rows`
- `summary` — sample size = `min(max_rows, 5)` (no effect when `max_rows ≤ 5`)

**Envelope fields:** `returned` (rows or bytes after truncation), `total` (pre-truncation count/bytes), `truncated: true`, plus `items`/`issues`/… or `text`.

### Phase C — FeatureSet as invoke ACL + surfaced tools

**Effort:** ~3 days  
**Status:** ✅ Implemented — manual QA sections 8–9 pass ([`meta-gateway-invoke-qa.md`](../../testing/meta-gateway-invoke-qa.md))

- [x] FeatureSet member model: tools invokable by default when server in set; optional `surfaced: true` promotes into `tools/list`
- [x] Search + invoke respect FeatureSet member filter (not just server-all)
- [x] Workspaces UI: per-tool "Surface in client" toggle in FeatureSet editor (`FeatureSetPanel.tsx`)
  - **Checkbox** = invoke ACL member (search + `mcpmux_invoke_tool`)
  - **Surface button** = promote that included tool into client `tools/list` for direct one-hop calls
  - User-facing explainer: [`docs/guide/feature-sets.mdx`](../guide/feature-sets.mdx#included-vs-surface-featureset-editor)
- [x] FeatureSet authoring is **UI-only** — `mcpmux_create_feature_set` removed from agent surface (consent model)
- [x] Integration tests: partial FeatureSet binding limits search; surfaced vs invokable gate; advertised set promotion

### Phase D — Resource/prompt hard cut + invoke polish

**Effort:** ~4 days  
**Status:** ✅ Implemented — GAIT v2 Run 2 **SHIP** (May 26); Issue #4 clone routing fixed @ `a4a212a`

- [x] **Resource progressive disclosure** — slim `resources/list` to surfaced only; `mcpmux_search_resources` / `mcpmux_read_resource`
- [x] **Prompt progressive disclosure** — slim `prompts/list` to surfaced only; `mcpmux_search_prompts` / `mcpmux_fetch_prompt`
- [x] `ResourceDiscoveryService` + `PromptDiscoveryService` (grant-filtered indexes)
- [x] Facade: `get_advertised_*` vs `get_readable_*` / `get_fetchable_*` for resources and prompts
- [x] Handler gates: direct `read_resource` / `get_prompt` redirect to meta path when not surfaced
- [x] FeatureSet UI: **Surface** toggle for resources and prompts (same semantics as tools)
- [x] Levenshtein "did you mean?" on invoke / read / fetch errors (`strsim`)
- [x] TF-IDF ranking in search (tools, resources, prompts) when query present
- [x] Better empty-search / ACL errors (inactive server, not-in-binding hints)
- [x] Integration tests: 36 in `meta_gateway_invoke.rs` (includes clone `read_resource` routing)
- [ ] Bundle hygiene: trim PostHog skill resources from `bundle:gait` (operator config interim)
- [ ] Delta responses, auto-summarize, parallel invoke batching — deferred
- [ ] Sandboxed code execution (`gateway_execute_code`) — deferred

**Outcome:** Workspace binding verified — Cursor mux line shows **4** advertised meta tools + surfaced backend tools (see [`meta-gateway-invoke-qa.md`](../../testing/meta-gateway-invoke-qa.md)). Hidden meta tools remain callable by name.

### Phase D (deferred items)

**Effort:** TBD

- [ ] Delta responses, auto-summarize, parallel invoke batching
- [ ] Sandboxed code execution (abdullah-style `gateway_execute_code`)

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

| Step          | Command                                                  | Purpose                                     |
| ------------- | -------------------------------------------------------- | ------------------------------------------- |
| Full validate | `pnpm validate`                                          | fmt, clippy, check, eslint, typecheck       |
| Rust tests    | `pnpm test:rust`                                         | unit + `meta_gateway_invoke.rs` integration |
| TS tests      | `pnpm test:ts`                                           | vitest                                      |
| Manual smoke  | Cursor against live gateway — full runbook sections 0–11 | Agent UX verification — ✅ complete May 25  |

---

## Out of scope

| Item                                             | Reason                                                                        |
| ------------------------------------------------ | ----------------------------------------------------------------------------- |
| [`docs/guide/gateway.mdx`](../guide/gateway.mdx) | Remote admin UI — parallel track, no overlap                                  |
| Full abdullah 15-layer stack                     | Phase D picks winners after A+B prove value                                   |
| `mcpmux_enable_server` / `mcpmux_disable_server` | Removed with consent model — use `mcpmux_bind_current_workspace`              |
| Agent `mcpmux_create_feature_set`                | Removed — humans author bundles in desktop/web UI                             |
| Auto-enable server on failed invoke              | Silent enable defeats audit trail — rejected in consent model                 |
| Tool-poisoning validator / SHA-256 pinning       | MikkoParkkola feature; valuable follow-up for registry trust, not invoke core |
| Cursor descriptor JSON sync                      | Client-side concern; schema-on-demand makes it non-blocking                   |

---

## Key files referenced

| File                                                                                                                                       | Why                                                                                             |
| ------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs)                             | Materialization chokepoint — must split advertised vs invokable                                 |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs)                                             | Existing `call_tool` path invoke reuses                                                         |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs)                 | Invoke meta tool + result shaping                                                               |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke_backend.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_backend.rs) | Pluggable invoke routing trait                                                                  |
| [`tests/rust/src/canned_invoke_backend.rs`](../../tests/rust/src/canned_invoke_backend.rs)                                                 | Test double for filter e2e                                                                      |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs)                                               | `tools/list` + `call_tool` — advertised set, surfaced one-hop, invoke redirect for non-surfaced |
| [`docs/planning/feature-set-consent-model.md`](./feature-set-consent-model.md)                             | Bind-only activation; removed enable/create meta tools                                          |
| [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md)                     | Historical session-toggle design (superseded by consent model)                                  |
| [`docs/planning/tool-level-session-pin.md`](./tool-level-session-pin.md)                                                                   | Superseded for token budget; Phase F very optional rework                                       |

---

## Related documentation

- [`docs/planning/feature-set-consent-model.md`](./feature-set-consent-model.md) — bind-only agent activation (current)
- [`docs/planning/meta-surface-lean-core.md`](../../planning/meta-surface-lean-core.md) — 4 advertised / 7 hidden meta tools
- [`docs/planning/meta-tool-invoke-ergonomics.md`](../../planning/meta-tool-invoke-ergonomics.md) — default_params, bare/qualified invoke, required_params
- [`docs/planning/meta-tool-agent-ux-path-to-9.md`](../../planning/meta-tool-agent-ux-path-to-9.md) — round 3 follow-up (readiness, browse, structured errors)
- [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) — historical session-toggle design (superseded)
- [`docs/planning/tool-level-session-pin.md`](./tool-level-session-pin.md) — superseded; Phase F may revive as invoke ACL only if needed
- [`docs/planning/server-account-clones.md`](./server-account-clones.md) — origin of 240-tool bloat evidence
- [`docs/guide/gateway.mdx`](../guide/gateway.mdx) — remote operator UI (orthogonal)
- [MikkoParkkola/mcp-gateway](https://github.com/MikkoParkkola/mcp-gateway) — `gateway_search_tools` / `gateway_invoke` reference
- [abdullah1854/MCPGateway](https://github.com/abdullah1854/MCPGateway) — `gateway_get_tool_schema` / result filtering reference

---

## Reconciliation

This doc is the source of truth for the meta-gateway invoke model. Phases A–D are implemented on fork **`dev`** and manually QA complete. Evidence: [`meta-gateway-invoke-qa.md`](../../testing/meta-gateway-invoke-qa.md). Mark [`tool-level-session-pin.md`](./tool-level-session-pin.md) **Status** as _Superseded_ when contributing upstream.

**Decision record (May 25, 2026):** Hard cut to invoke-only for non-surfaced backend tools — no legacy full-catalog `tools/list`. Surfaced tools default zero everywhere (bundles included); opt-in per FeatureSet member for one-hop hot paths. FeatureSets redefine as invoke ACL + optional surfaced promotion. Session pin deferred to Phase F (very optional, last). Competitor analysis (MikkoParkkola + abdullah1854) informed Phase A–B scope; REST capabilities in Phase E / separate doc.

**Handler fix (May 25, 2026):** `call_tool` probes workspace roots before routing (matches `list_tools`) and allows direct calls when the tool is in `get_advertised_tools_for_grants` (surfaced). Non-surfaced backend names still get `use_invoke_tool` redirect.

**Design revision (May 25, 2026):** Removed default smart truncation — `filter` is opt-in only. Rationale: plain-text MCP backends (GWorkspace) don't map cleanly to JSON row truncation; agents should explicitly bound payloads when needed.

**QA ergonomics (May 25, 2026):** Bind FeatureSets in Workspaces UI before agent QA — session enable alone is insufficient without binding ACL. Do **not** call `mcpmux_bind_current_workspace` during routine QA (triggers Space-wide approval modal). Reload MCP tools after UI binding or Surface changes.

**Test coverage (May 26, 2026):** Phase B filter shaping — 13 unit tests in `invoke.rs`, 36 integration tests in `meta_gateway_invoke.rs`, manual QA sections 0–11 pass on live gateway; GAIT v2 Run 2 covers Phase D §0–§7.

**Phase D (May 26, 2026):** Resource/prompt hard cut shipped — `resources/list` and `prompts/list` advertised-only (surfaced escape hatch); 4 new meta tools; TF-IDF search rank; Levenshtein invoke suggestions; FeatureSet Surface toggle for resources/prompts. Meta tool count **14**. GAIT v2 Run 2 **SHIP**; Issue #4 (`read_resource` clone routing) fixed in `a4a212a`.

**Lean meta surface (Jun 2026):** `tools/list` advertises 4 core `mcpmux_*` tools; 7 remain callable by name. See [`meta-surface-lean-core.md`](../../planning/meta-surface-lean-core.md).

**Invoke ergonomics (Jun 1–2, 2026):** Per-server `default_params` on `installed_servers` — shallow-merged under caller args in `mcpmux_invoke_tool` only. Search hits include `bare_name`, `qualified_name`, and `required_params: [{ name, type }]` at all detail levels. `invoke_tool` accepts bare or qualified `tool` (no double-prefix). Invoke "did you mean" suggestions use bare `feature_name`. Agent-validated: Context7 search→invoke without `get_tool_schema`. Lane guide: [`server-config-lanes.md`](../guides/server-config-lanes.md). Design: [`meta-tool-invoke-ergonomics.md`](../../planning/meta-tool-invoke-ergonomics.md). Flow: [`tool-discovery-and-search.md`](../technical/tool-discovery-and-search.md).

**Search UX + agent visibility (Jun 19, 2026):** Query-side stopwords + synonyms in lexical/hybrid rank; zero-result **`inactive_preview`** for ready-but-unbound tools; generic miss hint leads with **`mcpmux_list_servers`**; search hits add **`display_name`** and **`prefilled: true`** on pre-configured required params; **`list_servers`** exposes **`prefilled_params`**; **`get_tool_schema`** accepts **`tool_name`** / **`tool`** aliases and resolves **bare names** (feature_name) in addition to qualified names — passing `getJiraIssue` now works; invoke denial messages append display name when known. Round 3 table: [`meta-tool-invoke-ergonomics.md`](../../planning/meta-tool-invoke-ergonomics.md#round-3-jun-2026--search-ux--agent-visibility).

**Manual QA progress (May 26, 2026):** Overall **Ship** (Phases A–D). Full section results in [`meta-gateway-invoke-qa.md`](../../testing/meta-gateway-invoke-qa.md). Highlights:

| QA section                                | Result  | Notes                                                                                    |
| ----------------------------------------- | ------- | ---------------------------------------------------------------------------------------- |
| 0 — Sanity (meta-only surface)            | ✅ Pass | 4 advertised `mcpmux_*` tools (lean core); 34 servers listed; bind to activate              |
| 1 — Happy path (GitHub read)              | ✅ Pass | search → invoke (schema optional when required_params inline); bind when inactive         |
| 2 — Fail-closed + recovery                | ✅ Pass | Inactive server → bind hint; bind → retry                                                   |
| 3 — Search detail levels + compact schema | ✅ Pass | compact omits top-level description only                                                 |
| 4 — Binding toggle (list size unchanged)  | ✅ Pass | search empty when unbound; 4 advertised meta tools stable                                  |
| 5 — Pass-through without filter (Phase B) | ✅ Pass | GWorkspace `list_drive_items`: 100 items, no metadata envelope                           |
| 6 — Explicit filter (Phase B)             | ✅ Pass | Plain-text `max_bytes` + live `github_list_issues` JSON filter                           |
| 7 — Clone disambiguation                  | ✅ Pass | Personal vs work clone scoped correctly                                                  |
| 8 — FeatureSet ACL (Phase C)              | ✅ Pass | Partial GitHub tool set; invoke deny outside ACL                                         |
| 9 — Surfaced promotion (Phase C)          | ✅ Pass | `github_list_issues` in tools/list + direct one-hop; `get_me` invoke-only                |
| 10 — Diagnostic list vs search            | ✅ Pass | 120 tools both paths for GWorkspace Personal                                             |
| 11 — End-to-end agent task                | ✅ Pass | Meta-only workflow; schema-first; filter truncation metadata                             |
| 12 — Resources hard cut (Phase D)         | ✅ Pass | GAIT v2 Run 2 — 14/0/0 surface; search 91 URIs; read content                             |
| 13 — Prompts hard cut (Phase D)           | ⏭ Skip | No fetchable prompts in GAIT binding (§8 SKIP)                                           |
| 14 — Surfaced resource/prompt             | ⏭ Skip | Not configured in GAIT binding (§9 SKIP)                                                 |
