# Tool Discovery & Search

**Synthesizes:** [`meta-gateway-invoke.md`](../reference/meta-gateway-invoke.md), [`search-tools-hybrid-semantic-ranking.md`](../reference/search-tools-hybrid-semantic-ranking.md), [`search-tools-embedding-search-read-path.md`](../reference/search-tools-embedding-search-read-path.md), [`search-tools-latency-and-root-race.md`](../reference/search-tools-latency-and-root-race.md), [`mcpmux-diagnose-server.md`](../reference/mcpmux-diagnose-server.md)

**Last Updated:** Jun 1, 2026

---

## The Meta Surface

AI clients connected to McpMux see **4 `mcpmux_*` meta tools** in `tools/list` at startup — the hot path for every session. The remaining 7 meta tools are registered (always callable) but hidden from the advertised list. Agents reach hidden tools through the error/hint recovery strings that name them exactly when needed.

```
tools/list (core — always advertised):
├── mcpmux_list_servers              server roster with per-server status + bindable_feature_set_ids
├── mcpmux_search_tools              search by intent; returns qualified_name, bare_name, required_params (name+type)
├── mcpmux_get_tool_schema           single or batch schema fetch
├── mcpmux_invoke_tool               single invoke entry point for all backend calls
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

Advertisement and dispatch are decoupled: `list_as_tools()` filters to the 4 core tools; `MetaToolRegistry::call()` gates on `registry.contains(name)` — unchanged, covering all 11. This eliminates the `notifications/tools/list_changed` dependency that dynamic surface options required, and saves ~800 Claude tokens of startup context for the common tool-only binding case.

Non-surfaced backend tools are **not** in `tools/list`. Attempting to call a backend tool directly returns a redirect error pointing at `mcpmux_invoke_tool`.

---

## Search → Schema → Invoke

The canonical agent workflow (one to three steps depending on tool complexity):

```
1. mcpmux_search_tools({ query: "list issues", server_id: "github", detail_level: "description" })
      → ranked hits with qualified_name, bare_name, required_params [{ name, type }, …], status, description

2. mcpmux_get_tool_schema({ tools: ["github_list_issues"] })   ← optional when required_params is enough
      → full JSON input schema; compact: true strips examples/descriptions

3. mcpmux_invoke_tool({ server_id: "github", tool: "list_issues" | "github_list_issues", args: { … } })
      → backend tool result (optionally shaped via filter)
```

Search hits always include `bare_name` (use as `invoke_tool.tool` when unsure) and `required_params` as `{ name, type }` objects (from cached `inputSchema.required` + `properties`), at every `detail_level` except `detail_level=schema` (which adds full `input_schema` instead). For parameter-light tools, an agent can skip step 2 and invoke from search using `bare_name` or `qualified_name`. For complex shapes or optional params, `mcpmux_get_tool_schema` remains the source of truth.

`mcpmux_invoke_tool` accepts **bare or qualified** `tool` values (strips a leading `{server_id}_` prefix when present). Passing the `qualified_name` from search no longer produces a double-prefixed permission error.

Sticky per-server tool arguments (`cloudId`, `projectKey`, etc.) can be preset via **`default_params`** so agents do not repeat them every invoke — see [`server-config-lanes.md`](../guides/server-config-lanes.md#default_params).

This replaces the prior model of dumping all tool definitions into context. An agent makes 2–4 predictable meta calls per backend tool instead of guessing parameter names from a stale descriptor file.

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

**`include_inactive: true`** widens search to tools in installed-but-unbound FeatureSets. Inactive matches carry `{ status: "inactive", bindable_feature_set_id }`. The wide scan uses an optimized single JOIN query (replaced a per-FS `resolve_feature_sets` loop that caused 84 s hangs on large bundles).

---

## Invoke Authorization

`InvokeToolTool` (`meta_tools/invoke.rs`) fails closed:

```
effective_servers = (binding_servers ∪ session_enabled) − session_disabled
invokable_tools   = Tool features for effective_servers ∩ FeatureSet members
```

Invoking a tool outside the effective set returns an actionable error, not a silent proxy. Examples:

- `"github inactive → mcpmux_enable_server('github')"` (pre-consent-model only; now: bind a FeatureSet)
- `"unknown tool → did you mean list_issues?"` (Levenshtein on bare `feature_name` — paste directly into the `tool` arg)

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

## Key Source Locations

| Path | Role |
| ---- | ---- |
| `crates/mcpmux-gateway/src/services/tool_discovery.rs` | `ToolDiscoveryService`, `ToolIndexEntry`, hybrid search, `rank_with_hybrid` |
| `crates/mcpmux-gateway/src/services/discovery_rank.rs` | Token-overlap lexical match, TF-IDF score, score fusion |
| `crates/mcpmux-gateway/src/services/meta_tools/tools.rs` | `SearchToolsTool`, `GetToolSchemaTool`, `hydrate_active_embeddings` |
| `crates/mcpmux-gateway/src/services/meta_tools/invoke.rs` | `InvokeToolTool`, result shaping, permission check |
| `crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs` | `DiagnoseServerTool`, health classification, config view |
| `crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs` | Resource/prompt search, read, fetch meta tools |
| `crates/mcpmux-gateway/src/pool/features/facade.rs` | `get_advertised_tools_for_grants` vs `get_invokable_tools_for_grants` |
| `crates/mcpmux-gateway/src/pool/features/resolution.rs` | Inactive scan JOIN query; `resolve_surfaced_feature_ids` |
| `crates/mcpmux-gateway/src/mcp/handler.rs` | `ensure_roots_probed` call; advertised-only `tools/list`; redirect gate |
| `crates/mcpmux-gateway/src/services/session_roots.rs` | `SessionRootsRegistry`, `search_cache` (active-index per session) |
