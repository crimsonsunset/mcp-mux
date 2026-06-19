# Meta-Tool Invoke Ergonomics — Default Params & First-Call UX

**Last Updated:** Jun 19, 2026
**Status:** Phase 1–3 on `dev` (`a92111c`–`b58c693`); **round 2** shipped on `feat/meta-surface-lean-core` (`9532ce0`); **round 3** (search UX + agent visibility) on `feat/meta-surface-lean-core` (Jun 2026)
**Branch:** merged to `dev`
**Base branch:** `dev`
**Depends on:** nothing — builds on the shipped consent/invoke model
**Unblocks:** agents invoking parameter-heavy servers (Atlassian, anything needing org/projectKey/cloudId) without a guaranteed first-call failure

---

## Round 2 (PR #4 — invoke ergonomics follow-up)

Agent feedback on the lean-core surface: search returned `qualified_name` but invoke expected bare `tool`, producing double-prefixed errors (`github_github_*`). **Shipped in this branch:**

| Change | Detail |
| ------ | ------ |
| `bare_name` in search hits | Same as `feature_name` — the value to pass to `mcpmux_invoke_tool.tool` |
| `required_params` shape | `[{ "name": "owner", "type": "string" }, …]` at default `detail_level` (required keys only) |
| `invoke_tool.tool` | Accepts bare **or** qualified; strips `{server_id}_` prefix when present |
| Deferred | optional-param inlining; full schema in search |

**Agent validation (Jun 2, 2026):** `mcpmux_search_tools` → `mcpmux_invoke_tool` on Context7 without `get_tool_schema`: `resolve-library-id` with `bare_name` + `required_params` types; then `query-docs` with `/reactjs/react.dev`. GitHub `github_search_code` and bare `search_code` both invoked successfully. Wrong-tool errors suggest bare names only (no double-prefix).

---

## Round 3 (Jun 2026 — search UX + agent visibility)

Follow-up from agent sessions (Atlassian/Jira workflows). Shipped on `feat/meta-surface-lean-core`:

| Change | Detail |
| ------ | ------ |
| Lexical query expansion | Stopwords filtered; query-side synonyms (e.g. `jira`→`atlassian`, `ticket`→`issue`) in `discovery_rank.rs`; applies to tool/resource/prompt search |
| Zero-result `inactive_preview` | Active search returns 0 → up to 3 **ready** but unbound tools in separate `inactive_preview[]` with bind hint (not mixed into `tools[]`) |
| Zero-result hint | Generic miss leads with `mcpmux_list_servers` before suggesting `include_inactive: true` |
| `prefilled_params` on `list_servers` | Lists keys from server `default_params` (e.g. `["cloudId"]`) |
| `prefilled: true` on search hits | Required params covered by `default_params` are marked in `required_params[]` |
| `display_name` on search hits | Human server label alongside `server_id` |
| Invoke denial `action` | Appends display name when known, e.g. `… (Jira - S2H)` |
| `get_tool_schema` aliases | Accepts `tool_name` or `tool` (single name) in addition to `tools` — mirrors `invoke_tool` |

**Operator setup unchanged:** configure `default_params` in **Servers → Configure**; agents learn what's pre-filled via `list_servers` / search hits, not by calling `getAccessibleAtlassianResources`. Full lane guide: [`server-config-lanes.md`](../backend/guides/server-config-lanes.md#default_params).

---

## Problem

An AI client exercised the `mcpmux_*` meta-tool surface and flagged a cluster of friction points. The headline one is a hard failure, not just friction: **every Atlassian tool requires a `cloudId`, but it's never surfaced upfront**, so the first invoke of every conversation fails until the agent round-trips to discover the value.

We confirmed from the McpMux DB that `com.atlassian-mcp` is the **official Atlassian remote MCP** (`https://mcp.atlassian.com/v1/mcp`, `hosting_type: remote`, `auth: oauth`). That rules out the obvious fixes:

- **No env var.** `JIRA_URL` / `ATLASSIAN_OAUTH_CLOUD_ID` belong to the self-hosted `sooperset/mcp-atlassian` stdio server — they don't exist for a remote server we don't host.
- **No header.** `X-Atlassian-Cloud-Id` is also a `sooperset` feature, not the official remote one. McpMux *can* inject `extra_headers`, but the official server takes `cloudId` as a **per-call tool argument**.

So the only place to kill the round-trip for this class of server is **the gateway itself**: a per-server map of default arguments that McpMux merges into tool calls before forwarding. That single feature subsumes the "cloudId gotcha" and the "no session memory" complaints, and generalizes to any sticky parameter (`org`, `projectKey`, etc.).

The remaining flags are smaller: an internal inconsistency in invoke error suggestions, and a missing-schema gap in search results that forces an extra call.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | cloudId fix vehicle | **Gateway-layer per-server `default_params`** — not env var, not server swap | The official remote server has no env/header path. A default-args map is generic (any server, any param) and reuses the existing per-server config pattern. |
| 2 | Injection semantics | **Explicit call args always win** — defaults are fallback-only, deep-merged under the caller's arguments | Never mask an intentional value. An agent that passes its own `cloudId` must override the default. |
| 3 | Session-cache idea (original pain point #5) | **Dropped** — static per-server defaults strictly dominate | Caching a discovered value per session still fails the first call and evaporates each conversation. Static defaults set once, survive restarts, never fail first-call. |
| 4 | Storage shape | **New `default_params` JSON column on `installed_servers`**, sibling to `env_overrides` / `extra_headers` / `args_append` | Same per-install config lane already exists; no new table. `cloudId`/`projectKey` are not secrets — store as plaintext JSON like `env_overrides`, not in the encrypted `input_values` lane. |
| 5 | Name-suggestion consistency (pain point #2) | **Invoke "did you mean" suggestions emit bare `feature_name`s**, matching the `tool` arg | `invoke.rs` matches on bare `feature_name` but suggests `qualified_name()` (prefixed). The suggestion you copy must be the string invoke accepts. |
| 6 | Search schema gap (pain point #3) | **Inline `required_params` in search** — Phase 3 shipped names on `dev`; round 2 adds `{ name, type }` for required keys only | Collapses search→invoke for simple tools; full/optional shapes stay in `get_tool_schema`. |
| 7 | Latency (pain point #4) | **Defer** — profiling task, not a feature | ~200–400ms/hop is "not a dealbreaker." Needs a measured profiling pass (resolver/active-index cost per call), not speculative optimization. |

---

## Scope

**In:**

- Per-server `default_params` storage, plumbing, and invoke-time merge
- A way to set `default_params` (server config editor surface)
- Bare-name invoke suggestions
- `required_params` names in search results

**Out:**

- **Session-level cloudId caching** — superseded by static defaults (Decision 3).
- **Env-var / header injection for the official Atlassian server** — not supported by a remote server we don't host. (If a user *swaps* to the `sooperset` stdio server, that path already works via the existing `env_overrides` lane — no new code.)
- **Full tool-schema inlining in search** — names only; `mcpmux_get_tool_schema` remains for full shapes.
- **Invoke latency optimization** — deferred to a separate profiling pass (see Open Questions).

---

## The Model

### Storage

`installed_servers` gains one column, mirroring the existing config lanes:

```
env_overrides   TEXT NOT NULL DEFAULT '{}'   -- (existing) child-process env
extra_headers   TEXT NOT NULL DEFAULT '{}'   -- (existing) HTTP/SSE headers
args_append     TEXT NOT NULL DEFAULT '[]'   -- (existing) stdio args
default_params  TEXT NOT NULL DEFAULT '{}'   -- (NEW) default tool-call arguments
```

`default_params` is a flat JSON object of argument-name → default-value, applied to **every** tool call routed to that server.

### Invoke-time merge

The merge happens in `mcpmux_invoke_tool`, immediately before dispatch to the backend (`invoke.rs`, just ahead of `backend.call_tool(...)`):

```
effective_args = deep_merge(server.default_params, call_args)
                                    ▲                    ▲
                              fallback only        caller wins on key collision
```

Concretely, for the Atlassian server with `default_params = { "cloudId": "<S2H cloud id>" }`, a call to `getJiraIssue` with only `{ "issueIdOrKey": "S2H-1305" }` is forwarded as `{ "cloudId": "...", "issueIdOrKey": "S2H-1305" }` — first call succeeds, no discovery round-trip.

### Suggestion consistency

`invoke.rs` builds `candidates` from `f.qualified_name()` (e.g. `atlassian_getJiraIssue`) but the `tool` argument is matched against bare `f.feature_name` (`getJiraIssue`). The fix is to build suggestion candidates from `feature_name`, so the "did you mean" list is copy-paste-ready into the `tool` field.

### Search result shape

`mcpmux_search_tools` hits include `bare_name`, `qualified_name`, and `required_params: [{ name, type }, …]` for required keys only (from cached `inputSchema`). Optional params and full shapes stay in `mcpmux_get_tool_schema`. `invoke_tool.tool` accepts bare or qualified names (round 2).

---

## Phases

### Phase 1 — Per-server `default_params` injection (~half day) — **P0**

- Migration `022_installed_server_default_params.sql` — add `default_params TEXT NOT NULL DEFAULT '{}'`
- Add `default_params` to the `InstalledServer` domain entity and the storage repository read/write
- Plumb through the `application/server.rs` service layer + the admin write route so it's settable
- Surface `default_params` in the server config editor (Monaco) so a user can set it without raw SQL
- Merge defaults under caller args in `invoke.rs` before `backend.call_tool` (explicit args win)

**Outcome:** With `cloudId` set once on the Atlassian server, `mcpmux_invoke_tool { tool: "getJiraIssue", arguments: { issueIdOrKey: "S2H-1305" } }` succeeds on the **first** call of a fresh session — no `getAccessibleAtlassianResources` round-trip, no cloudId in the call. Verified against both `com.atlassian-mcp` and its `-gait` clone.

---

### Phase 2 — Invoke suggestion consistency (~1 hr) — **P1**

- In `invoke.rs`, build `levenshtein_suggestions` candidates from bare `feature_name` instead of `qualified_name()`
- Audit the parallel suggestion sites in `disclosure.rs` (resource/prompt "did you mean") for the same prefixed-vs-bare mismatch and align them

**Outcome:** A failed/misspelled invoke returns "did you mean `getJiraIssue`, `searchJiraIssues`…" — names that paste directly into the `tool` field with no reformatting. No trial call to discover the expected format.

---

### Phase 3 — `required_params` in search results (~half day) — **P2** (names on `dev`; types in round 2)

- Extend the `mcpmux_search_tools` result shape with `required_params`, sourced from the cached tool schema in `tool_discovery.rs`
- Round 2 (`9532ce0`): `[{ name, type }]` for required keys; also `bare_name` and qualified/bare `invoke_tool.tool`

**Outcome:** A search hit for a parameter-light tool exposes required param names and types inline, so the agent invokes it directly — collapsing search→schema→invoke into search→invoke for the common case. `mcpmux_get_tool_schema` still answers for complex/optional shapes.

---

## Files to create / modify

| File | Change |
| ---- | ------ |
| `crates/mcpmux-storage/src/migrations/022_installed_server_default_params.sql` | **Create** — add `default_params` column |
| [`crates/mcpmux-core/src/domain/installed_server.rs`](../../crates/mcpmux-core/src/domain/installed_server.rs) | Add `default_params` field to the entity |
| [`crates/mcpmux-storage/src/repositories/installed_server_repository.rs`](../../crates/mcpmux-storage/src/repositories/installed_server_repository.rs) | Read/write the new column |
| [`crates/mcpmux-core/src/application/server.rs`](../../crates/mcpmux-core/src/application/server.rs) | Plumb `default_params` through the service layer |
| [`crates/mcpmux-gateway/src/admin/command_bridge/write.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/write.rs) | Accept `default_params` on the server update route |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | Merge defaults under call args (Phase 1); bare-name suggestions (Phase 2) |
| [`crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/disclosure.rs) | Align resource/prompt suggestion names (Phase 2) |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs) | Add `required_params` to search hits (Phase 3) |
| `apps/desktop/src/features/servers/ServersPage.tsx` | Expose `default_params` in the server config editor (Phase 1) |

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | Invoke path; injection site (~`backend.call_tool`) and suggestion bug (line ~354) |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke_backend.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke_backend.rs) | `InvokeToolBackend::call_tool` — `arguments: Value` is what gets merged |
| [`crates/mcpmux-storage/src/migrations/001_initial.sql`](../../crates/mcpmux-storage/src/migrations/001_initial.sql) | `installed_servers` schema; existing `env_overrides`/`extra_headers`/`args_append` lanes |
| [`docs/backend/technical/tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) | Search → schema → invoke flow this plan tightens |
| [`docs/backend/technical/consent-and-binding.md`](../backend/technical/consent-and-binding.md) | Invoke ACL the merge sits behind |

---

## Open questions (deferred, not blocking)

- **Invoke latency (pain point #4)** — profile `mcpmux_invoke_tool` to attribute the ~200–400ms/hop (resolver resolve, active-index lookup, transport). Optimize only what the profile justifies; not part of this plan.
- **`default_params` secrecy** — assumed non-secret (`cloudId`, `projectKey`). If a future use wants a secret default, route it through the encrypted `input_values` lane instead of `default_params`.
- **Per-tool vs per-server defaults** — this plan is per-server (one map applied to all the server's tools). If a single server ever needs different defaults per tool, revisit a `tool → params` nesting; no evidence it's needed yet.
