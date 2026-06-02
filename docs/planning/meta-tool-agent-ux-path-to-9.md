# Meta-Tool Agent UX — Readiness, Browse & Structured Invoke (Path to 9)

**Last Updated:** Jun 2, 2026
**Status:** Planned — not started
**Branch:** TBD (off `dev`)
**Base branch:** `dev`
**Depends on:** Shipped lean-core ([`meta-surface-lean-core.md`](./meta-surface-lean-core.md)) + invoke ergonomics ([`meta-tool-invoke-ergonomics.md`](./meta-tool-invoke-ergonomics.md)) — `default_params`, `required_params`, bare/qualified invoke, hidden-but-callable surface
**Unblocks:** Agent rating from **7/10 → 9/10** against a real 35-server install (PostHog 375 tools, Supabase 29, GWorkspace 122) without expanding the 4-tool advertised surface

---

## Problem

An AI client exercised the post-lean-core meta surface against a live install and scored it **7/10**, up from ~4 pre-fixes. The wins are real: search → invoke works without reading schemas for common cases, `filter` shapes payloads, parallel cross-server invokes work, and `list_servers` gives an at-a-glance roster. Five friction points cap it below 9:

| # | Feedback | Root cause today |
| - | -------- | ---------------- |
| 1 | **`required_params` only, not optional** — "for PostHog's 375-tool server I have no idea what optional filters/pagination exist without a separate schema read" | `entry_to_json` emits `required_params` only; optional params live solely in `get_tool_schema` |
| 2 | **No auth/health signal before you try** — "MongoDB needed a connect step, PostHog returned a 402. I only found out by trying." | `list_servers` reports `enabled_via_binding \| inactive` — a binding flag, not a live connection/auth state. No health reaches the agent until a call fails. |
| 3 | **Search relevance at scale is load-bearing** — "if search misses the right tool there's no graceful fallback short of paginating blind. For large servers a `list_tools` scoped to a `server_id` would help." | Browse (empty query + `server_id`) already returns all matches alphabetically, but it's undocumented, capped at the search default (20), and not named as a browse path. |
| 4 | **Still two hops when you already know the tool** — "if I know I want `getJiraIssue` I have to search first to confirm the `bare_name` before invoking." | Qualified/bare invoke already works post round-2, but nothing tells the agent it can skip search; no copy-paste invoke shape is handed back. |
| 5 | **Inactive servers are a trap** — "firebase-dev, sonarqube, langfuse appear in the list but silently fail until you try to invoke. A `bindable` vs `ready` distinction would help." | `derive_server_status` returns `enabled_via_binding` for a *bound* server even when its pool connection is down / needs auth / missing inputs. Bound ≠ reachable. |

The reviewer's own path to 9: **solve #1 (inline optional params) and #3 (per-server tool list); the rest is polish.** We take the recommended combo — the clarity of an explicit browse mode + a real readiness model (brainstorm Option 2), plus structured invoke errors at the failure boundary (brainstorm Option 4, on-failure only) — which lands all five without adding a fifth advertised tool.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Overall vehicle | **Enrich the existing core four + structured invoke errors** — no new advertised meta tool | Lean-core's 4-tool budget is the whole point of PR #4. Every fix rides on `search_tools` / `list_servers` / `invoke_tool` payloads or the invoke error channel. |
| 2 | Optional params (#1) | **Inline `optional_params: [{ name, type }]` (capped ~8) at the default `detail_level`, plus `schema_complex: true`** | Fixes the headline gripe without making agents learn a new `detail_level`. Cap protects the token budget; the flag points to `get_tool_schema` when the shallow view isn't enough. |
| 3 | Type resolution depth | **Shallow only — reuse `schema_property_type`; set `schema_complex` when a type is `unknown` / `oneOf` / `$ref` / nested object** | Deep JSON-Schema resolution is a rabbit hole. A truthful "this one needs the full schema" flag beats a wrong inline type. |
| 4 | Browse at scale (#3) | **Recognize empty/absent `query` + `server_id` as browse inside `search_tools`** — alphabetical, default `limit` 50, add a `mode: "browse"` doc alias | Delivers the reviewer's "`list_tools` scoped to `server_id`" verbatim behavior without a new advertised tool. The path already exists in `filter_and_rank` (no query tokens ⇒ all pass); this formalizes, documents, and right-sizes it. |
| 5 | Readiness model (#5) | **Replace binary status with `readiness: bindable \| bound \| ready` + `blocking_reason`** on `list_servers`; only `ready` implies invokable | Bound ≠ connected today. A three-state model surfaces the trap (bound-but-offline / auth_required / needs_setup) the agent currently discovers by failing. |
| 6 | Health placement (#2) | **Full health block lives in `list_servers`; search hits carry a lightweight `server_readiness` enum only** | Concentrates the heavy diagnose data in one tool; keeps per-hit search payloads lean (one enum field, not a health object × N hits). |
| 7 | Health source | **Reuse `diagnose.rs::classify_health` + `connection_status_label` + `ServerManager::get_all_statuses`** — already on `MetaToolContext` | No parallel health logic. `server_manager` is already injected into the meta-tool context; `classify_health` already maps connection status + missing inputs to a bucket. |
| 8 | Pre-invoke failure (#2, #5) | **Structured invoke denial `{ error, reason, action, tool }`** distinguishing `inactive` / `bound_offline` / `auth_required` / `needs_setup`, each naming the next tool (`bind` / `diagnose`) | Fail fast with one actionable next step at the boundary instead of a backend 402/500 the agent has to interpret. |
| 9 | Known-tool one-hop (#4) | **Document direct qualified/bare invoke (already works); browse hits carry an `invoke_example` object** | Collapses the redundant search-to-confirm-bare-name hop without a new passthrough tool. The example is copy-paste-ready into `invoke_tool`. |
| 10 | Opt-in preflight | **`mcpmux_invoke_tool { preflight: true }` returns readiness without a backend call** — Phase 5, opt-in only | Lets a cautious agent fail-fast before a wasted call. Opt-in so the happy path keeps its single round-trip. |
| 11 | Default preflight on every invoke | **Rejected** | Adds a readiness round-trip to every call; latency is already a flagged concern. On-failure structured errors (Decision 8) cover the reactive case for free. |
| 12 | Tool-level prereq orchestration (MongoDB `connect` step) | **Out — surface via readiness/health only, no auto-connect** | Auto-running a server's connect tool is a different, riskier feature. Readiness/`blocking_reason` tells the agent *why*; it doesn't drive the remedy. |

---

## Scope

**In:**

- `readiness` (`bindable | bound | ready`) + `blocking_reason` + a health block on `list_servers`
- Lightweight `server_readiness` enum on each `search_tools` hit
- `optional_params` + `schema_complex` on search hits (capped)
- Browse recognition (empty query + `server_id`) with a `mode: "browse"` alias and a 50-item default limit
- Structured invoke denial payload that names the next tool
- `invoke_example` on browse hits (Phase 5)
- `preflight: true` opt-in on `mcpmux_invoke_tool` (Phase 5)
- Doc + QA updates

**Out:**

| Item | Reason |
| ---- | ------ |
| A new advertised meta tool (e.g. a separate `mcpmux_list_tools`) | Breaks the lean-core 4-tool budget (PR #4's core win). Browse mode inside `search_tools` covers #3 without it. |
| Default/automatic preflight on every invoke | Latency on the happy path (Decision 11); opt-in flag + on-failure errors cover it. |
| Full tool-schema inlining in search results | `get_tool_schema` remains the full-shape path; `schema_complex` points there. |
| Deep `oneOf` / `$ref` / nested type resolution for inline params | Flagged via `schema_complex` instead; full resolution is a separate, optional pass. |
| Auto-calling a server's `connect`/setup tool (MongoDB-style prereq) | Decision 12 — out; readiness surfaces *why*, not the remedy. |
| Raw per-hop invoke latency optimization | Tracked in [`meta-tool-invoke-ergonomics.md`](./meta-tool-invoke-ergonomics.md) Open Questions (profiling task, not a feature). The *two-hops* part of #4 is solved here via `invoke_example` + direct invoke. |

---

## The Model

### Readiness (server-level, `list_servers`)

Today: `status = enabled_via_binding | inactive` (binding flag only). After:

```
readiness:
  bindable  →  not in the active binding, but a FeatureSet can activate it
               (carries existing bindable_feature_set_ids)
  bound     →  in the binding, but the pool connection is not Connected
               (blocking_reason: auth_required | needs_setup | disconnected | error)
  ready     →  bound + Connected + no missing required inputs  ⇒ safe to invoke
```

`readiness` is derived by crossing the existing binding membership (`derive_server_status`) with the live pool state:

```
binding membership   ×   ServerManager::get_all_statuses + classify_health
        │                              │
        └────────────► readiness + blocking_reason + health bucket
```

Per-server `list_servers` entry (additions in **bold**):

```jsonc
{
  "id": "posthog",
  "name": "PostHog",
  "tool_count": 375,
  "readiness": "bound",            // bindable | bound | ready   (replaces `status`)
  "connection": "auth_required",   // connection_status_label
  "health": "auth_required",       // ServerHealth bucket
  "blocking_reason": "auth_required",
  "missing_inputs": [],            // present only when health = needs_setup
  "bindable_feature_set_ids": []   // present only when readiness = bindable
}
```

### Search hit shape (additions in **bold**)

```jsonc
{
  "server_id": "posthog",
  "qualified_name": "posthog_insights_get_all",
  "bare_name": "insights_get_all",
  "server_readiness": "ready",     // lightweight enum only — full health is in list_servers
  "required_params": [{ "name": "project_id", "type": "string" }],
  "optional_params": [             // NEW — names + shallow types, capped ~8
    { "name": "limit", "type": "integer" },
    { "name": "offset", "type": "integer" }
  ],
  "schema_complex": false          // NEW — true ⇒ call get_tool_schema for the full shape
}
```

### Browse mode (`search_tools`)

Empty/absent `query` + a `server_id` is treated as a deterministic catalog browse: alphabetical by `qualified_name`, `limit` defaults to **50** (vs 20 for ranked search), paginated by the existing cursor. `mode: "browse"` is an explicit, documented alias for the same behavior.

```
search_tools({ server_id: "posthog" })                 // browse: all PostHog tools, A–Z, 50/page
search_tools({ server_id: "posthog", query: "funnel" }) // ranked search, unchanged
```

### Structured invoke denial (`invoke_tool`)

When a call can't proceed, return a structured payload naming the remedy instead of leaking a backend error:

```jsonc
{
  "error": "not_ready",
  "reason": "bound_offline",        // inactive | bound_offline | auth_required | needs_setup
  "action": "Server 'mongodb' is bound but not connected. Run mcpmux_diagnose_server to see why.",
  "tool": "mcpmux_diagnose_server"  // or mcpmux_bind_current_workspace for `inactive`
}
```

### `invoke_example` on browse hits (Phase 5)

```jsonc
"invoke_example": {
  "server_id": "atlassian",
  "tool": "getJiraIssue",
  "args": { "issueIdOrKey": "<string>" }   // required params as placeholders
}
```

---

## Phases

### Phase 1 — Server readiness model on `list_servers` (~half day) — **P0**

- Replace the binary `status` with `readiness` (`bindable | bound | ready`) in `ListServersTool`
- Cross binding membership (`derive_server_status`) with `ServerManager::get_all_statuses` + `classify_health` (make the diagnose helpers `pub(crate)` if not already)
- Add `connection`, `health`, `blocking_reason`, and conditional `missing_inputs` to each entry
- Keep `bindable_feature_set_ids` on `bindable` entries; keep `cloned_from`
- Update the tool description to document the readiness states

**Outcome:** `mcpmux_list_servers` distinguishes a server that's safe to invoke (`ready`) from one that's bound-but-broken (`bound` + `blocking_reason: auth_required`) from one that needs activation (`bindable`). An agent can skip firebase-dev/sonarqube/langfuse before wasting a call — the trap in feedback #5 is visible up front. Verify: a bound server with its pool disconnected reports `readiness: "bound"`, `blocking_reason: "disconnected"`, not `ready`.

---

### Phase 2 — Optional params + `schema_complex` in search (~half day) — **P0**

- Add `extract_optional_param_specs` in `tool_discovery.rs`, mirroring `extract_required_param_specs`, capped at ~8 entries
- Set `schema_complex: true` when any param type resolves to `unknown`, or the schema uses `oneOf`/`anyOf`/`$ref`/nested objects beyond the shallow `type` read
- Emit `optional_params` + `schema_complex` from `entry_to_json` at the default (`description`) detail level
- Update `SearchToolsTool` description: optional params are inline for simple tools; `schema_complex: true` means call `get_tool_schema`

**Outcome:** A PostHog tool hit shows `limit`/`offset`/filter param names inline, so the agent uses pagination/filters without a separate schema read (feedback #1). A tool with a `oneOf` body comes back `schema_complex: true`, and the agent knows to fetch the full schema rather than guess. Verify against a known-complex schema and a known-flat one.

---

### Phase 3 — Browse mode at scale (~half day) — **P1**

- In `SearchToolsTool`, detect empty/absent `query` + present `server_id` as browse: default `limit` 50, alphabetical sort (already the no-query path in `filter_and_rank`)
- Accept `mode: "browse"` as an explicit alias and document it in the schema
- Ensure the existing cursor pagination carries the larger page size
- Add the lightweight `server_readiness` enum to each hit (cheap cross-reference to the Phase 1 readiness map)

**Outcome:** `mcpmux_search_tools({ server_id: "posthog" })` returns a deterministic, paginated A–Z slice of all 375 PostHog tools — the graceful fallback for when ranked search misses (feedback #3). Each hit also says whether its server is `ready`. Verify: browse returns 50/page with a working `next_cursor`, and a ranked query against the same server is unaffected.

---

### Phase 4 — Structured invoke denial (~half day) — **P1**

- In `invoke.rs`, before dispatch, classify the failure: `inactive` (not in binding) / `bound_offline` (bound, not Connected) / `auth_required` / `needs_setup` (missing inputs), reusing the Phase 1 readiness derivation
- Return a structured `{ error, reason, action, tool }` payload that names `mcpmux_bind_current_workspace` (for `inactive`) or `mcpmux_diagnose_server` (for the bound-but-broken cases)
- Add a `bound_offline` variant alongside `format_server_inactive_error` in `routing.rs`
- Keep the existing permission-denied / unknown-tool suggestion path intact

**Outcome:** Invoking a bound-but-disconnected MongoDB tool returns `{ reason: "bound_offline", tool: "mcpmux_diagnose_server", … }` instead of a raw backend error — the agent gets one named next step rather than discovering the problem by trial (feedback #2, #5 at the call boundary). Verify each reason maps to the correct remedy tool.

---

### Phase 5 — Polish: `invoke_example` + opt-in preflight (~half day) — **P2**

- Add `invoke_example` (`{ server_id, tool: bare_name, args: { <required placeholders> } }`) to browse hits
- Add `preflight: true` to `mcpmux_invoke_tool`: returns the Phase 4 readiness payload (or `{ ready: true }`) without calling the backend
- Document that a known qualified/bare tool can be invoked directly — no search-to-confirm hop

**Outcome:** An agent that knows it wants `getJiraIssue` invokes it in one hop using the browse `invoke_example` (or directly), and a cautious agent can `preflight` before committing — closing the redundant-second-hop friction in feedback #4. Verify a `preflight` call performs no backend dispatch.

---

## Files to create / modify

| File | Change |
| ---- | ------ |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | `ListServersTool` readiness model (Phase 1); `SearchToolsTool` browse mode + `mode` alias + 50-default + `server_readiness` (Phase 3) |
| [`crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs) | Expose `classify_health` / `connection_status_label` / `ServerHealth` as `pub(crate)` for reuse (Phase 1) |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs) | `extract_optional_param_specs` + `schema_complex` in `entry_to_json` (Phase 2); `server_readiness` + `invoke_example` (Phase 3/5) |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | Structured denial classification (Phase 4); `preflight` flag (Phase 5) |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | Add `bound_offline` denial variant alongside `format_server_inactive_error` (Phase 4) |
| [`docs/backend/technical/tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md) | Readiness model, browse mode, optional-param/`schema_complex` shape, structured invoke errors |
| [`docs/testing/meta-gateway-invoke-qa.md`](../testing/meta-gateway-invoke-qa.md) | Add QA rows for readiness, browse, optional params, structured denial |
| [`tests/rust/tests/integration/meta_tools.rs`](../../tests/rust/tests/integration/meta_tools.rs) | Readiness states, browse limit/sort, `server_readiness` on hits |
| [`tests/rust/tests/integration/meta_gateway_invoke.rs`](../../tests/rust/tests/integration/meta_gateway_invoke.rs) | Optional params + `schema_complex`, structured denial reasons, `preflight` |

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | `ListServersTool` (`derive_server_status`, `binding_servers`, `inactive_by_server`); `SearchToolsTool` (scope/limit/detail parsing, empty-result hints) |
| [`crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/diagnose.rs) | `ServerHealth`, `classify_health` (status × missing-inputs → bucket), `connection_status_label`, `parse_missing_required_inputs` |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | `MetaToolContext.server_manager` is already injected — the live status source for readiness |
| [`crates/mcpmux-gateway/src/pool/server_manager.rs`](../../crates/mcpmux-gateway/src/pool/server_manager.rs) | `get_all_statuses(space_id)` → `(ConnectionStatus, flow_id, has_connected_before, error)` per server |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs) | `extract_required_param_specs` / `schema_property_type` (templates for the optional-param extractor); `entry_to_json` (hit shape) |
| [`crates/mcpmux-gateway/src/services/discovery_rank.rs`](../../crates/mcpmux-gateway/src/services/discovery_rank.rs) | `filter_and_rank_inner` — empty `query_tokens` ⇒ all entries pass, sorted by haystack (the browse path Phase 3 formalizes) |
| [`crates/mcpmux-gateway/src/services/meta_tools/invoke.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/invoke.rs) | Binding/`is_available` gate before `backend.call_tool`; current `format_server_inactive_error` / `format_invoke_permission_denied` use |
| [`docs/planning/meta-surface-lean-core.md`](./meta-surface-lean-core.md) | The 4-tool advertised budget this plan must not break |
| [`docs/planning/meta-tool-invoke-ergonomics.md`](./meta-tool-invoke-ergonomics.md) | `required_params`, `default_params`, bare/qualified invoke — the shape Phase 2/4/5 extend |

---

## Open questions (deferred, not blocking)

- **Optional-param cap value** — ~8 is a guess; revisit once we see real PostHog/GWorkspace hit sizes. The cap trades completeness against the token budget lean-core protects.
- **`server_readiness` freshness on search hits** — readiness is a point-in-time pool snapshot; a server can drop between `search_tools` and `invoke_tool`. The Phase 4 structured denial is the backstop, so a slightly stale hit enum is acceptable.
- **Invoke latency (reviewer's separate note)** — the *two-hops* friction is solved here; raw ~200–400ms/hop profiling stays in [`meta-tool-invoke-ergonomics.md`](./meta-tool-invoke-ergonomics.md) Open Questions.
- **`mode: "browse"` vs implicit detection** — shipping both (explicit alias + empty-query inference); if agents never use the explicit alias, drop it in a later cleanup.
