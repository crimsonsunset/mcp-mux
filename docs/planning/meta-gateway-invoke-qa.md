# Meta-Gateway Invoke — Manual QA Runbook

**Last Updated:** May 25, 2026  
**Branch:** `feat/meta-gateway-invoke`  
**Related:** [`meta-gateway-invoke.md`](./meta-gateway-invoke.md)

One-session checklist for validating Phases A–C (search → schema → invoke, result shaping, FeatureSet ACL + surfaced tools).

---

## Quick prep

- [x] Rebuild/restart gateway if you haven't since the branch (`pnpm dev` or run the built app)
- [x] Cursor → MCP → **Reload tools**
- [x] Confirm McpMux endpoint: `http://localhost:45818/mcp`
- [x] Have at least one OAuth server (GitHub) **installed and connected** — `QA: meta-gateway invoke` FeatureSet bound in UI (May 25)
- [x] Workspace binding with GWorkspace (or target server) configured in UI — **not** via agent `mcpmux_bind_current_workspace`
- [x] Optional for Phase C tests: create a FeatureSet with 1–2 GitHub tools, bind to workspace; leave surfaced off until test 8 — `QA: meta-gateway invoke` (`list_issues` + `get_me`, surfaced off, bound May 25)

**FeatureSet editor controls (tests 8–9):**

| Control | Role in QA |
| ------- | ---------- |
| **Checkbox** | Include tool in invoke ACL → search + `mcpmux_invoke_tool` |
| **Surface** button | Promote included tool into client `tools/list` → direct one-hop call (test 9 only) |
| **Server header toggle** | Bulk include/exclude — not Surface |

After any Surface change: **Cursor → MCP → Reload tools**.

**Tester:** Cursor agent (Composer)  
**Date:** May 25, 2026  
**McpMux version / commit:** `feat/meta-gateway-invoke` @ `433e7bd` (PR [#155](https://github.com/mcpmux/mcp-mux/pull/155))

---

## 0. Sanity — meta-only surface

**Prompt:**

```
You have McpMux meta tools only — no direct backend tools like github_*.

1. Call mcpmux_list_servers and show installed servers and active/inactive status.
2. Tell me how many tools you see in your available tool list total, and list their names.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `mcpmux_list_servers` returns installed servers | ☑ | ☐ | 34 servers returned |
| Only **10** `mcpmux_*` tools exposed (no backend names) | ☑ | ☐ | Verified via MCP descriptor folder |
| Backend servers show **inactive** until enabled | ☑ | ☐ | All inactive at session start |
| Tool list count stable (~10 meta + Cursor/plugin tools) | ☑ | ☐ | No backend tools leaked |

---

## 1. Happy path — GitHub read (Phase A)

**Prompt** (swap repo if needed):

```
Use ONLY the McpMux meta workflow — do not guess backend tool names or params.

Goal: list open issues in mcpmux/mcp-mux.

Steps you must follow explicitly:
1. mcpmux_list_servers — check if github is active
2. If inactive: mcpmux_enable_server for github
3. mcpmux_search_tools with query "list issues", server_id "github", detail_level "description"
4. mcpmux_get_tool_schema for the best match
5. mcpmux_invoke_tool with exact args from the schema

Show each step briefly, then the first 5 issues.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Agent enabled github when inactive | ☐ | ☐ | N/A — github was `enabled_via_binding` |
| Search before invoke (no param guessing) | ☑ | ☐ | Found `github_list_issues` via search |
| Schema read before invoke | ☑ | ☐ | Used `owner`/`repo`/`state`/`perPage` from schema |
| Invoke succeeded with correct param names | ☑ | ☐ | 5 open issues returned for mcpmux/mcp-mux |
| `tools/list` still ~10 meta tools after enable | ☑ | ☐ | Still exactly 10 `mcpmux_*` tools |

---

## 2. Fail-closed + recovery (Phase A errors)

**Prompt:**

```
Try to invoke a GitHub tool WITHOUT enabling github first (disable it if needed).

1. mcpmux_invoke_tool on github with tool list_issues and dummy args
2. Show the exact error message
3. Follow whatever it tells you to do
4. Retry invoke successfully
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Invoke denied when server inactive | ☑ | ☐ | After `mcpmux_disable_server` → `disabled_via_session` |
| Error mentions `mcpmux_enable_server` with server_id | ☑ | ☐ | `server 'github' is disabled for this session → mcpmux_enable_server({ "server_id": "github" })` |
| Recovery via enable → retry works | ☑ | ☐ | enable + invoke returned 3 issues |

---

## 3. Search detail levels + compact schema (Phase A)

**Prompt:**

```
On github (enabled):

1. mcpmux_search_tools query "list" detail_level "name" limit 5
2. Same query detail_level "description"
3. Pick one tool — mcpmux_get_tool_schema compact: true
4. Same tool — compact: false

What did compact strip?
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `name` level omits descriptions | ☑ | ☐ | `github_list_issues` — no `description` key |
| `description` level includes descriptions | ☑ | ☐ | Full tool description present |
| `compact: true` strips descriptions/examples | ☑ | ☐ | Strips **top-level** tool `description`; property descriptions in `input_schema` kept |
| Batch schema (array of tools) works if agent tries it | ☑ | ☐ | `tools: ["github_list_issues"]` returned schemas array |

---

## 4. Session toggle — list size unchanged (Phase A)

**Prompt:**

```
1. Enable github — confirm search finds github tools
2. Disable github via mcpmux_disable_server
3. Search again for github tools
4. Report tools/list count before and after — must stay the same
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Search empty / no github matches when disabled | ☑ | ☐ | `total: 0`, `tools: []` after session disable |
| Meta tool count unchanged across enable/disable | ☑ | ☐ | 10 `mcpmux_*` before and after |

---

## 5. Pass-through without filter (Phase B)

**Setup:** GWorkspace Personal bound (`taylorwilsdon.google-workspace-mcp-uvx`) or any heavy server in FeatureSet ACL.

**Prompt:**

```
Find a list tool via search (e.g. GWorkspace list_drive_items), read schema, invoke WITHOUT filter.

Confirm the full backend response is returned with no { returned, total, truncated } metadata.
Paste rough char count.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Full backend response returned | ☑ | ☐ | GWorkspace `list_drive_items` `page_size: 100` → 100 items + `nextPageToken` |
| No truncation metadata without filter | ☑ | ☐ | Plain text only; no `{ returned, total, truncated }` (opt-in filter @ `433e7bd`) |

---

## 6. Explicit filter (Phase B)

**Setup:** Same tool as test 5, or GitHub `list_issues` for JSON row truncation.

**Prompt:**

```
Invoke with filter: { "max_rows": 3, "format": "summary" }

For plain-text tools (GWorkspace), also try filter: { "max_bytes": 4096 }.

Then fields projection if the tool returns JSON objects with id/name/title fields.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `max_rows: 3` honored (JSON tools) | ☑ | ☐ | Live `github_list_issues` → `{ returned: 3, total: 5, truncated: true, issues: [3 items] }` (5 open issues in repo) |
| `max_bytes` honored with metadata (plain text) | ☑ | ☐ | GWorkspace `list_drive_items` `max_bytes: 4096` → `{ returned: 4110, total: 7660, truncated: true, text: "…[truncated]" }` |
| `format: summary` applied | ☑ | ☐ | JSON: metadata envelope present; with 5 total issues and `max_rows: 3` → 3 returned (summary no-op when max_rows ≤ 5) |
| `fields` projection limits keys per row (if tested) | ☑ | ☐ | `fields: ["id","title","number"]` → rows kept `title` + `number` only (`id` absent in GitHub payload) |

---

## 7. Clone disambiguation (server_id filter)

**Setup:** You have GWorkspace ×2 clones — enable **only one**.

**Prompt:**

```
Enable ONLY taylorwilsdon.google-workspace-mcp-uvx (not the s2h clone).

mcpmux_search_tools query "drive" or "list files" with server_id set explicitly.
Confirm results are scoped to that server_id only.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `server_id` filter scopes search | ☑ | ☐ | `server_id: taylorwilsdon.google-workspace-mcp-uvx` + query `"drive"` → 24 hits, all Personal prefix |
| Other clone's tools not in results | ☑ | ☐ | S2H clone inactive; search with `server_id: …-s2h` → `total: 0` |

---

## 8. FeatureSet ACL — partial tool set (Phase C)

**Setup:** FeatureSet with 1–2 GitHub tools **checked** (included), bound to workspace, **Surface off** on all rows.

**Prompt:**

```
I bound a FeatureSet that only allows specific GitHub tools.

1. mcpmux_search_tools query "github" detail_level "name"
2. Try mcpmux_invoke_tool on a tool NOT in the FeatureSet
3. Invoke one tool that IS included
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Search only finds allowed tools | ☑ | ☐ | `query: "github"` + empty query → 2 hits: `github_get_me`, `github_list_issues` only (not 41) |
| Invoke denied for disallowed tool | ☑ | ☐ | `create_issue` → `tool 'github_create_issue' is not invokable with current grants` |
| Invoke succeeds for allowed tool | ☑ | ☐ | `list_issues` (3 open issues) + `get_me` (`crimsonsunset`) both succeeded |

---

## 9. Surfaced tool promotion (Phase C)

**Setup:** In FeatureSet editor, leave **`list_issues` checked** and click **Surface** (blue) on that row only; leave other included tools checked but Surface off. Save, then **Cursor → MCP → Reload tools**.

**Prompt:**

```
1. List all tools available — identify mcpmux_* vs surfaced backend
2. Call the surfaced tool directly (one hop)
3. Call a different tool on same server via mcpmux_invoke_tool
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Surfaced tool appears in client tool list | ☑ | ☐ | After Cursor MCP reload: 10 `mcpmux_*` + `github_list_issues` only; `github_get_me` not listed |
| Surfaced tool callable without invoke wrapper | ☑ | ☐ | Direct `github_list_issues` → 2 open issues (no `use_invoke_tool` redirect) after handler fix + binding reload May 25 |
| Non-surfaced backend still requires invoke | ☑ | ☐ | `get_me` absent from tools/list; `mcpmux_invoke_tool` → `crimsonsunset` OK |

---

## 10. Diagnostic — list_all_tools vs search

**Prompt:**

```
mcpmux_list_all_tools with server_id "github" (or one enabled server).
Compare count to mcpmux_search_tools with query "" and same server_id.
Explain why agents should prefer search.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `server_id` filter on list_all_tools works | ☑ | ☐ | GWorkspace Personal: 120 tools; all `server_id` matches filter |
| Agent recommends search over full dump | ☑ | ☐ | Same count (120); search supports query/detail_level/pagination — list_all_tools dumps ~42 KB with full descriptions |

---

## 11. End-to-end agent task (realism)

**Prompt:**

```
Brief status report on mcpmux/mcp-mux repo:
- open issue count
- 3 most recent issue titles
- one paragraph summary

Rules: McpMux meta tools only, read schemas before invoke, note truncation if any.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Completed without backend tool name guessing | ☐ | ☐ | |
| Schema-first invoke pattern | ☐ | ☐ | |
| Sensible output despite truncation | ☐ | ☐ | |

---

## Red flags (stop and file a bug)

- [ ] Backend tools (`github_*`, etc.) appear in `tools/list` without surfacing
- [ ] Agent can call backend tools directly (bypassing `mcpmux_invoke_tool`)
- [ ] Enable server expands `tools/list` beyond meta + surfaced
- [ ] Search returns tools from inactive or unbound servers
- [ ] Invoke succeeds for tools outside FeatureSet ACL
- [ ] Invoke with explicit filter fails to truncate or return metadata
- [ ] Opaque errors (no enable/invoke redirect hints)

---

## Sign-off

| Area | Result |
| ---- | ------ |
| Phase A — meta invoke core | ☑ Pass ☐ Fail |
| Phase B — result shaping | ☑ Pass ☐ Fail |
| Phase C — ACL + surfaced | ☑ Pass ☐ Fail ☐ Skipped |
| Overall | ☐ Ship ☐ Block |

**Blockers / issues filed:**

```
- section 6 JSON rows: manual pass May 25 after binding QA FeatureSet — github_list_issues filter verified live
- beeper 401 on get_accounts/search_chats — auth expired; not blocking meta-gateway QA
- test 9: surfaced direct one-hop + invoke-only non-surfaced — pass May 25 live (`github_list_issues` direct → 2 issues; `get_me` via invoke only)
```
