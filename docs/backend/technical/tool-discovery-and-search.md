# Tool Discovery & Search

**Synthesizes:** [`meta-gateway-invoke.md`](../reference/meta-gateway-invoke.md), [`search-tools-hybrid-semantic-ranking.md`](../reference/search-tools-hybrid-semantic-ranking.md), [`search-tools-embedding-search-read-path.md`](../reference/search-tools-embedding-search-read-path.md), [`search-tools-latency-and-root-race.md`](../reference/search-tools-latency-and-root-race.md), [`mcpmux-diagnose-server.md`](../reference/mcpmux-diagnose-server.md)

**Last Updated:** Jun 5, 2026

---

## The Meta Surface

AI clients connected to McpMux see **4 `mcpmux_*` meta tools** in `tools/list` at startup — the hot path for every session. The remaining 7 meta tools are registered (always callable) but hidden from the advertised list. Agents reach hidden tools through the error/hint recovery strings that name them exactly when needed.

```
tools/list (core — always advertised):
├── mcpmux_list_servers              server roster with readiness, connection, health, blocking_reason
├── mcpmux_search_tools              search by intent; browse mode; optional_params; invoke_example on browse
├── mcpmux_get_tool_schema           single or batch schema fetch
├── mcpmux_invoke_tool               single invoke entry point; optional preflight
└── [0–N surfaced backend tools]     opt-in per FeatureSet member; default zero

hidden-but-callable (registered, not advertised — reached via recovery strings):
├── mcpmux_list_feature_sets         → named in search_tools empty-results hint
├── mcpmux_bind_current_workspace    → named in server-inactive error + inactive-tool redirect
├── mcpmux_search_resources          → named in direct read_resource redirect
├── mcpmux_read_resource             → named in direct read_resource redirect
├── mcpmux_search_prompts            → named in direct get_prompt redirect
├── mcpmux_fetch_prompt              → named in direct get_prompt redirect
└── mcpmux_diagnose_server           → operator tool; human-callable directly
```

Advertisement and dispatch are decoupled: `list_as_tools()` filters to the 4 core tools; `MetaToolRegistry::call()` gates on `registry.contains(name)` — unchanged, covering all 11. This eliminates the `notifications/tools/list_changed` dependency that dynamic surface options required, and saves **~940 Claude-est tokens** of startup context vs advertising all 11 meta tools (re-measure: `pnpm count-tokens`, Jun 2026).

Non-surfaced backend tools are **not** in `tools/list`. Attempting to call a backend tool directly returns a redirect error pointing at `mcpmux_invoke_tool`.

---

## Search → Schema → Invoke

The canonical agent workflow (one to three steps depending on tool complexity):

```
1. mcpmux_search_tools({ query: "list issues", server_id: "github", detail_level: "description" })
      → ranked hits with qualified_name, bare_name, required_params, optional_params, server_readiness, schema_complex

2. mcpmux_get_tool_schema({ tools: ["github_list_issues"] })   ← optional when required_params is enough
      → full JSON input schema; compact: true strips examples/descriptions

3. mcpmux_invoke_tool({ server_id: "github", tool: "list_issues" | "github_list_issues", args: { … } })
      → backend tool result (optionally shaped via filter)
```

**Direct invoke (no search hop):** When you already know the tool name, skip step 1 and call `mcpmux_invoke_tool` with `bare_name` or `qualified_name` from a prior session or browse hit. Bare and qualified names route identically.

**Browse mode:** Per-server: `mcpmux_search_tools({ server_id: "posthog" })` or `{ server_id: "posthog", mode: "browse" }`. Whole Space: `{ mode: "browse" }` without `server_id`. All browse paths return a paginated A–Z catalog (default limit **50**). Each browse hit includes an **`invoke_example`** object — copy-paste into `mcpmux_invoke_tool` (required args as `<type>` placeholders).

**Opt-in preflight:** `mcpmux_invoke_tool({ server_id, tool, preflight: true })` returns `{ ready: true }` or the same structured `not_ready` error as a failed invoke, without calling the backend.

Search hits always include `bare_name` (use as `invoke_tool.tool` when unsure), `required_params` and `optional_params` (capped ~8) as `{ name, type }` objects, `server_readiness` (`bindable` | `bound` | `ready`), and `schema_complex` (call `get_tool_schema` when true). At `detail_level=schema`, full `input_schema` is included instead of the shallow param lists.

For parameter-light tools, an agent can skip step 2 and invoke from search using `bare_name` or `qualified_name`. When `schema_complex: true`, `mcpmux_get_tool_schema` is the source of truth.

`mcpmux_invoke_tool` accepts **bare or qualified** `tool` values (strips a leading `{server_id}_` prefix when present). Passing the `qualified_name` from search no longer produces a double-prefixed permission error.

Sticky per-server tool arguments (`cloudId`, `projectKey`, etc.) can be preset via **`default_params`** so agents do not repeat them every invoke — see [`server-config-lanes.md`](../guides/server-config-lanes.md#default_params).

This replaces the prior model of dumping all tool definitions into context. An agent makes 1–3 predictable meta calls per backend tool instead of guessing parameter names from a stale descriptor file.

### Example agent session (validated Jun 2, 2026)

Context7 documentation lookup without `get_tool_schema`:

```
mcpmux_search_tools({ server_id: "com.context7-mcp-npx", query: "resolve library", limit: 3 })
  → bare_name: "resolve-library-id", required_params: [{ name: "libraryName", type: "string" }, { name: "query", type: "string" }]

mcpmux_invoke_tool({
  server_id: "com.context7-mcp-npx",
  tool: "resolve-library-id",   // bare_name from search
  args: { libraryName: "react", query: "hooks" }
})
  → library ID /reactjs/react.dev

mcpmux_invoke_tool({
  server_id: "com.context7-mcp-npx",
  tool: "query-docs",
  args: { libraryId: "/reactjs/react.dev", query: "useEffect cleanup" },
  filter: { max_bytes: 2000 }
})
  → truncated doc snippets
```

The same `tool` field accepts `com.context7-mcp-npx_resolve-library-id` (qualified) with identical routing.

---

## ToolDiscoveryService

`crates/mcpmux-gateway/src/services/tool_discovery.rs`

Maintains a per-Space in-memory index of `ToolIndexEntry` records built from `server_feature_repo::list_for_space`. The index is rebuilt on feature-change events. A per-session active-index cache (`search_cache: Arc<DashMap<session_id, (fingerprint, ToolIndex)>>` on `SessionRootsRegistry`) avoids repeated DB round-trips within a session — evicted on `WorkspaceBindingChanged` or session disconnect.

**Ranking pipeline (active-set query):**

```
search_tools({ query })
  │
  ├─ build / fetch cached ToolIndexEntry[]         (per-session cache)
  ├─ LEXICAL: token-overlap filter + TF-IDF score   (discovery_rank.rs)
  │     token-overlap OR match replaces contiguous-substring gate
  │     AND-boost when all query tokens present
  ├─ SEMANTIC (model Ready):
  │     read doc vectors from global DashMap (content_hash keys)
  │     embed query inline via spawn_blocking (~ms)
  │     cosine similarity per doc
  ├─ FUSE: 0.4 × norm(lexical) + 0.6 × semantic
  └─ sort → paginate → annotate ranking: "hybrid" | "lexical"
```

When the embedding model is not yet `Ready` (downloading or absent), search degrades cleanly to lexical-only and annotates `ranking: "lexical"` in the response. An agent or UI can read this field to know.

### Browse mode

Empty or absent `query` with `server_id` set (or explicit `mode: "browse"`) lists all matching tools alphabetically by `qualified_name`. Default **limit 50** (ranked search defaults to 20). Response includes `"mode": "browse"`. Each hit carries **`invoke_example`** for one-hop invoke:

```json
{
  "invoke_example": {
    "server_id": "github",
    "tool": "list_issues",
    "args": { "owner": "<string>", "repo": "<string>" }
  }
}
```

Ranked search hits do **not** include `invoke_example` (token budget).

### Search hit shape

| Field | When | Notes |
| ----- | ---- | ----- |
| `required_params` | always (except `detail_level=schema`) | `{ name, type }[]` |
| `optional_params` | always (except schema level) | capped ~8; shallow types only |
| `schema_complex` | always | `true` → call `get_tool_schema` |
| `server_readiness` | always | `bindable` \| `bound` \| `ready` — point-in-time pool snapshot |
| `invoke_example` | browse mode only | copy-paste into `mcpmux_invoke_tool` |

### Server readiness (`list_servers`)

Replaces the old binary `status` field:

| `readiness` | Meaning |
| ----------- | ------- |
| `bindable` | Not in active binding; `bindable_feature_set_ids` lists FeatureSets that can activate it |
| `bound` | In binding but not invokable — see `blocking_reason` (`auth_required`, `needs_setup`, `disconnected`, `error`) |
| `ready` | Bound + connected + no missing required inputs |

Each entry also includes `connection`, `health`, and `missing_inputs` when setup is incomplete.

**`include_inactive: true`** widens search to tools in installed-but-unbound FeatureSets. Inactive matches carry `{ status: "inactive", bindable_feature_set_id }`. The wide scan uses an optimized single JOIN query (replaced a per-FS `resolve_feature_sets` loop that caused 84 s hangs on large bundles).

---

## Invoke Authorization

`InvokeToolTool` (`meta_tools/invoke_tool.rs`) fails closed:

```
effective_servers = (binding_servers ∪ session_enabled) − session_disabled
invokable_tools   = Tool features for effective_servers ∩ FeatureSet members
```

Invoking a tool outside the effective set returns an actionable error, not a silent proxy. Examples:

- Structured **not_ready** before backend dispatch: `{ error: "not_ready", reason: "inactive"|"bound_offline"|"auth_required"|"needs_setup", action, tool }` — `tool` names `mcpmux_bind_current_workspace` or `mcpmux_diagnose_server`
- `"unknown tool → did you mean list_issues?"` (Levenshtein on bare `feature_name`)

**Preflight:** `preflight: true` runs the same readiness and permission gates; on success returns `{ ready: true }` without backend dispatch.

### Per-server default tool arguments

Shallow merge at `mcpmux_invoke_tool`: `{ ...default_params, ...caller_args }` (caller wins). Full lane guide — when to use vs `env_overrides` / `extra_headers`, Atlassian example, UI path: [`server-config-lanes.md`](../guides/server-config-lanes.md#default_params).

---

## Result Shaping (invoke filter)

`mcpmux_invoke_tool` accepts an optional `filter` argument for known-heavy payloads. Omit `filter` to return the backend response unchanged.

| Payload | Applicable keys | Behavior |
| ------- | --------------- | -------- |
| Plain text | `max_bytes` | Truncation envelope `{ returned, total, truncated, text }` |
| YAML mapping/sequence | `max_rows`, `fields`, `format`, `max_bytes` | Parsed → shaped like JSON |
| Top-level JSON array | `max_rows`, `fields`, `format`, `max_bytes` | Envelope with `items` array |
| JSON object with heavy array key | same | Metadata at object top-level; array under original key |
| `structured_content` | same | Shaped independently |

`format: summary` samples `min(max_rows, 5)` rows. `format: full` samples exactly `max_rows`.

---

## FeatureSet as Invoke ACL

FeatureSets define what is **invokable**, not what appears in `tools/list`. The `surfaced: bool` flag on `FeatureSetMember` is the only way a backend tool enters `tools/list` directly. Binding a large bundle does not promote any tools into context — startup context stays lean regardless of bundle size. This invariant is enforced by the `get_advertised_tools_for_grants` vs `get_invokable_tools_for_grants` split in `facade.rs`.

---

## Root-Race Fix

`ensure_roots_probed` is called in `handler.rs` before `MetaToolRegistry::call(…)`. Without this, `search_tools` called as the first meta-tool in a session (before `tools/list` triggered root resolution) would see `PendingRoots` and return zero results even when a valid binding existed.

---

## Resource & Prompt Discovery

Same progressive disclosure model as tools:

- `resources/list` and `prompts/list` are advertised-only (surfaced items, default zero).
- `mcpmux_search_resources` → `mcpmux_read_resource` for resource access.
- `mcpmux_search_prompts` → `mcpmux_fetch_prompt` for prompt access.
- Direct `read_resource` / `get_prompt` on non-surfaced items redirects to the meta path.

`ResourceDiscoveryService` and `PromptDiscoveryService` are grant-filtered indexes parallel to `ToolDiscoveryService`.

---

## Diagnostics: mcpmux_diagnose_server

`DiagnoseServerTool` (`meta_tools/diagnose.rs`) — read-only, no approval required.

One call returns:
- Runtime connection status + flow ID
- Transport config: type, command, args, env/input/header **keys** (no secret values)
- Missing required inputs
- Recent log tail (default 50 entries, configurable)
- Tool count

Called with no `server_id`: returns all unhealthy servers in the caller's Space. Called with `server_id`: returns that server regardless of health.

Health buckets: `healthy` | `error` | `auth_required` | `needs_setup` | `disconnected`.

---

## Observability

All search operations use a `query_id` correlation key minted at `SearchToolsTool::call` entry. A single `grep query_id` in gateway logs (`RUST_LOG=mcpmux_gateway=debug`) traces the full path: entry → cache decision → embed state → lexical pass → fusion → result summary. Raw query text is logged at `debug` only, never at `info`.

Key log targets: `[search]` for ranking/fusion, `[embed]` for the embedding service.

---

## Architecture (maintainers)

Meta-tool implementations live under `crates/mcpmux-gateway/src/services/meta_tools/` as flat sibling modules (see [`meta-tools-module-split.md`](../../planning/meta-tools-module-split.md) for the full rationale). `mod.rs` registers tools via `build_default_registry`; shared helpers (`caller_space_id`, readiness, `with_approval`, etc.) are in `meta_tool_common.rs`. Per-tool files: `list_servers.rs`, `search_tools.rs` (+ index/cache helpers), `feature_set_tools.rs` (`ListFeatureSetsTool`, `GetToolSchemaTool`), `bind_workspace.rs`. Invoke is split across `invoke_tool.rs` (handler + alias resolution + permission gates), `invoke_result_filter.rs` (shaping/parsing + unit tests), and `invoke.rs` (thin re-export shim). `disclosure.rs`, `diagnose.rs`, `registry.rs`, and `token_budget.rs` are unchanged in role.

## Key Source Locations

| Path | Role |
| ---- | ---- |
| `crates/mcpmux-gateway/src/services/tool_discovery.rs` | `ToolDiscoveryService`, `ToolIndexEntry`, hybrid search, `rank_with_hybrid` |
| `crates/mcpmux-gateway/src/services/discovery_rank.rs` | Token-overlap lexical match, TF-IDF score, score fusion |
| `crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs` | `SearchToolsTool`, `hydrate_active_embeddings`, active-index cache |
| `crates/mcpmux-gateway/src/services/meta_tools/feature_set_tools.rs` | `GetToolSchemaTool`, `ListFeatureSetsTool` |
| `crates/mcpmux-gateway/src/services/meta_tools/invoke_tool.rs` | `InvokeToolTool`, alias resolution, permission check |
| `crates/mcpmux-gateway/src/services/meta_tools/invoke_result_filter.rs` | Result shaping (`filter` arg), YAML/JSON parsers |
| `crates/mcpmux-gateway/src/services/meta_tools/meta_tool_common.rs` | Shared helpers: readiness, `text_result`, `with_approval` |
| `crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs` | `DiagnoseServerTool`, health classification, config view |
| `crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs` | Resource/prompt search, read, fetch meta tools |
| `crates/mcpmux-gateway/src/pool/features/facade.rs` | `get_advertised_tools_for_grants` vs `get_invokable_tools_for_grants` |
| `crates/mcpmux-gateway/src/pool/features/resolution.rs` | Inactive scan JOIN query; `resolve_surfaced_feature_ids` |
| `crates/mcpmux-gateway/src/mcp/handler.rs` | `ensure_roots_probed` call; advertised-only `tools/list`; redirect gate |
| `crates/mcpmux-gateway/src/services/session_roots.rs` | `SessionRootsRegistry`, `search_cache` (active-index per session) |
