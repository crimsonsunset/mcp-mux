# GAIT Workspace — Meta-Gateway Invoke Capability Test

**Last Updated:** May 25, 2026  
**Status:** **SHIP WITH ISSUES** — Run 2 confirms fixes #1 and #3; **Issue #2 filter still failing** (fix not deployed to running gateway)  
**Branch:** `dev` (Issue #2 fix in `crates/mcpmux-gateway/services/meta_tools/invoke.rs` — rebuild/restart required)  
**Related:** [`meta-gateway-invoke.md`](./meta-gateway-invoke.md), [`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md), [`meta-gateway-invoke-retest.md`](./meta-gateway-invoke-retest.md)

**Source of truth for:** GAIT workspace binding QA, clone isolation, meta-gateway invoke DX, what passed/failed, what was fixed in code, and what still needs a live re-run.

---

## Current verdict

| Phase | Verdict | Notes |
| ----- | ------- | ----- |
| **Run 1** (2026-05-25, generAIt workspace) | **SHIP WITH ISSUES** | Core binding + clone isolation + E2E pass; §2 filter, §3 ACL reporting failed |
| **Run 2** (2026-05-25, generAIt workspace) | **SHIP WITH ISSUES** | §3 ACL + batch `missing` fixed; §2 filter step still fails (16 full rows, no envelope) |

**Target for ship:** Rebuild gateway with Issue #2 fix deployed → re-run §2 step 4 only → overall **SHIP**.

---

## What this test validates

| Area | Sections | Pass signal |
| ---- | -------- | ----------- |
| Meta-only client surface | §0 | ~10 `mcpmux_*` tools; no backend catalog in `tools/list` |
| Jira clone isolation | §1 | GAIT email/site; S2H server inactive or zero hits |
| PostHog clone isolation | §2 | Project **433907**; S2H `posthog-work` invoke denied |
| Invoke filter (Phase B) | §2 step 4 | `{ returned, total, truncated }` + field projection |
| Search / ACL DX (Phase C) | §3 | `total_invokable` matches search; batch schema `missing` |
| Fail-closed errors | §4 | Actionable `mcpmux_enable_server` hint + recovery |
| Realistic agent workflow | §5 | Jira + PostHog brief via search → schema → invoke |
| Supabase (optional) | §6 | GAIT projects visible; personal leak documented if unscoped |

**Out of scope for Phases A–C / not Run 2 blockers:**

- **124 resources** in Cursor mux UI — resources still fully materialized per grants (tools-only hard cut). **Tracked: Phase D** in [`meta-gateway-invoke.md`](./meta-gateway-invoke.md#phase-d--advanced-optimizations-defer)
- **Supabase `com.supabase-mcp-npx`** — one PAT, all org projects; not clone-scoped
- **PostHog project display name** — still "Default project" in PostHog UI (cosmetic)
- **`projects-get` accepts `{}`** despite schema listing `context` — PostHog MCP server behavior

---

## Run 1 — Sign-off (2026-05-25)

Workspace: `/Users/joe/Desktop/Repos/Contracts/generAIt`

| Section | Result | Notes |
| ------- | ------ | ----- |
| §0 Sanity | **Pass** | 10 meta tools; no backend catalog |
| §1 Jira GAIT | **Pass** | `jsangiorgio@generaitsolutions.com` / `generait1.atlassian.net`; S2H search 0 hits |
| §2 PostHog GAIT | **Fail** (filter only) | Project 433907 correct; insights correct; **filter step failed** |
| §3 Search DX | **Fail** | `total_invokable: 0` vs search 331 + working invoke |
| §4 Fail-closed | **Pass** | Disable → enable hint → recovery |
| §5 E2E task | **Pass** | Jira + PostHog brief delivered |
| §6 Supabase | **Pass** | GAIT + personal projects (unscoped server) |
| **Overall** | **Ship w/ issues** | |

**Clone isolation verified:**

- [x] Jira GAIT ≠ S2H
- [x] PostHog GAIT (433907) ≠ Personal (345911) ≠ S2H (311512)

**Red flags from Run 1:**

| Flag | Run 1 | Run 2 | Resolution |
| ---- | ----- | ----- | ---------- |
| Backend tools in `tools/list` without Surface | Clear | Clear | — |
| Wrong clone data when `server_id` filtered | Clear | Clear | — |
| `list_all_tools` invokable reporting broken | **Hit** | Clear | Fixed — Issues #1 **confirmed** |
| Opaque invoke errors | Clear | Clear | — |
| Schema batch omits empty string from `missing` | **Hit** | Clear | Fixed — Issues #3 **confirmed** |
| Param guessing without schema | Clear | Clear | — |
| `invoke_tool` filter not applied on `insights-list` | **Hit** | **Hit** | Fix in code — Issues #2 **not deployed** |

---

## Run 2 — Sign-off (2026-05-25)

Workspace: `/Users/joe/Desktop/Repos/Contracts/generAIt`

| Section | Result | Notes |
| ------- | ------ | ----- |
| §0 Sanity | **Pass** | 10 meta tools; no backend catalog |
| §1 Jira GAIT | **Pass** | `jsangiorgio@generaitsolutions.com` / `generait1.atlassian.net`; S2H search 0 hits |
| §2 PostHog GAIT | **Fail** (filter only) | Project 433907 correct; insights correct; **filter step still failed** (same as Run 1) |
| §3 Search DX | **Pass** | `total_installed: 338`, `total_invokable: 331` = search total; `missing: [""]` present |
| §4 Fail-closed | **Pass** | Disable → enable hint → recovery |
| §5 E2E task | **Pass** | Jira (GAIT-163/165/160) + PostHog brief delivered |
| §6 Supabase | **Pass** | GAIT + personal projects (unscoped server) |
| **Overall** | **Ship w/ issues** | Blocked on Issue #2 gateway deploy |

**Clone isolation verified:**

- [x] Jira GAIT ≠ S2H
- [x] PostHog GAIT (433907) ≠ Personal (345911) ≠ S2H (311512)

**Re-test checklist results:**

| Step | Result | Evidence |
| ---- | ------ | -------- |
| 1 `list_all_tools` invokable counts | **Pass** | `total_invokable: 331`, 331 rows `invokable: true` |
| 2 `insights-list` + filter | **Fail** | 16 full rows, all fields; no `{ returned, total, truncated }` envelope |
| 3 batch schema `""` → `missing` | **Pass** | `"missing": [""]` + message |
| 4 §1 + §4 smoke | **Pass** | No regressions |

---

## Issues tracker

### Confirmed fixed (Run 2)

| # | Symptom (Run 1) | Root cause | Fix | Files | Run 2 |
| - | --------------- | ---------- | --- | ----- | ----- |
| **1** | `list_all_tools`: all 338 rows `invokable: false`, `total_invokable: 0`; search 331 + invoke OK | Compared `qualified_name` strings; invokable set uses **prefix alias** (`posthog-personal_*`), catalog uses **server_id** (`posthog-personal-gait_*`) | Match invokable ACL on `(server_id, feature_name)` | `services/meta_tools/tools.rs` | **Pass** |
| **3** | `get_tool_schema(["…", ""])` → no `missing: [""]` | Empty strings silently dropped when parsing `tools` array | Preserve invalid entries in `missing` | `services/meta_tools/tools.rs` | **Pass** |

### Fixed in code — pending deploy (Run 2 still fails)

| # | Symptom | Root cause | Fix | Files | Run 2 |
| - | ------- | ---------- | --- | ----- | ----- |
| **2** | `insights-list` + `filter: { max_rows: 3, fields: [...] }` → 16 full rows, no envelope | Filter shaped `structuredContent` but agents read `content[].text`; PostHog plain text in content + JSON in structured | Mirror shaped structured into text content; add `insights` to heavy-array keys; aggregate multi-block list payloads | `services/meta_tools/invoke.rs` | **Fail** — running gateway lacks fix |

**Regression tests:** `tests/rust/tests/integration/meta_gateway_invoke.rs` — `list_all_tools_invokable_uses_server_id_not_prefix_alias`, `invoke_filter_shapes_structured_insights_payload`, `invoke_filter_aggregates_multi_block_content`, `get_tool_schema_reports_empty_string_in_missing`

### Open (non-blocking / follow-up)

| Item | Severity | Owner | Notes |
| ---- | -------- | ----- | ----- |
| Deploy Issue #2 fix + re-run §2 step 4 | **Required before ship** | Eng | Rebuild/restart gateway; expect `{ returned: 3, total: 16, truncated: true, insights: [...] }` |
| Resource list bloat (~124 PostHog skill URIs) | Medium | **Phase D** | [`meta-gateway-invoke.md` Phase D](./meta-gateway-invoke.md#phase-d--advanced-optimizations-defer) — progressive disclosure for `resources/list`; interim: trim skills from `bundle:gait` |
| Supabase hard project isolation | Optional | Config | Needs 4 clones with `--project-ref` or accept unscoped |
| Rename PostHog project 433907 | Cosmetic | PostHog UI | Still "Default project" |
| PR #155 merge + CHANGELOG | Process | Eng | After Issue #2 deploy + §2 filter **Pass** |
| Phase D meta-gateway polish | Deferred | Eng | Better errors, search, batch invoke |

### Closed — not bugs

| Observation | Why closed |
| ----------- | ---------- |
| Supabase returns personal + GAIT projects | `com.supabase-mcp-npx` is unscoped; one Management API PAT |
| `projects-get` schema lists `context` but `{}` works | Backend MCP validation, not mux |
| 124 resources in Cursor | Phases A–C spec; **Phase D** tracks meta search/read path for resources |

---

## Re-test checklist (Run 2) — completed 2026-05-25

**Prep:** gateway running on `localhost:45818`; Cursor on **generAIt**; `user-mcpmux` connected.

| Step | Action | Expected | Run 2 |
| ---- | ------ | -------- | ----- |
| 1 | `mcpmux_list_all_tools({ server_id: "posthog-personal-gait" })` | `total_invokable` ≈ 331 | **Pass** — 331/338, hint present |
| 2 | `insights-list` + `filter: { max_rows: 3, fields: ["name","short_id"] }` | Filter envelope + ≤3 rows | **Fail** — 16 full rows |
| 3 | `get_tool_schema({ tools: ["posthog-personal-gait_projects-get", ""] })` | `missing: [""]` | **Pass** |
| 4 | §1 + §4 smoke | No regressions | **Pass** |

**Next:** deploy Issue #2 → re-run step 2 only → flip Overall to **SHIP**.

---

## Prep (required before any tests)

1. Gateway running (`pnpm dev` or desktop app) on `http://localhost:45818/mcp`
2. Open **`/Users/joe/Desktop/Repos/Contracts/generAIt`** in Cursor (not the mcp-mux repo — GAIT binding applies there)
3. Cursor → MCP → **Reload tools**; confirm `user-mcpmux` connected
4. GAIT workspace binding includes: `bundle:core`, `bundle:comms-personal`, `bundle:browser`, `bundle:gait`, `bundle:db-personal`
5. **`com.atlassian-mcp-gait`** OAuth connected → `enabled_via_binding`
6. **`posthog-personal-gait`** connected with project **`433907`** → `enabled_via_binding`

**Expected GAIT stack:**

| Server ID | Scope |
| --------- | ----- |
| `com.atlassian-mcp-gait` | generAIt Jira — `generait1.atlassian.net`, `jsangiorgio@generaitsolutions.com` |
| `posthog-personal-gait` | PostHog project **433907** ("Default project") |
| `com.supabase-mcp-npx` | Unscoped — all 4 projects via `bundle:db-personal` (§6 optional) |

**Must NOT leak in GAIT-scoped calls:**

| Server ID | Wrong data if seen |
| --------- | ------------------ |
| `com.atlassian-mcp` | S2H — `sync2hire.atlassian.net`, `jsangiorgio@sync2hire.com` |
| `posthog-personal` | When.Band — project `345911` |
| `posthog-work` | Sync2Hire — project `311512` |

**FeatureSet editor reminder:**

| Control | Role |
| ------- | ---- |
| **Checkbox** | Invoke ACL (search + `mcpmux_invoke_tool`) |
| **Surface** button | Promote into client `tools/list` for direct one-hop calls |
| **Server header toggle** | Bulk checkbox only — not Surface |

After any Surface change: **Cursor → MCP → Reload tools**.

**Environment constraint:** Opening the **mcp-mux repo** binds **`All`** — not valid for GAIT isolation tests. Always use generAIt folder.

---

## Agent Prompt

Copy everything inside the fence into a **fresh Cursor agent** (generAIt workspace, prep complete):

```markdown
# GAIT workspace — McpMux meta-gateway invoke capability test

You are validating the **GAIT workspace binding** on McpMux (`http://localhost:45818/mcp` via `user-mcpmux`). Use **meta tools only** for backend calls unless §9 explicitly tests surfaced one-hop.

**Expected GAIT stack (from prior config):**
- `com.atlassian-mcp-gait` → generAIt Jira (`generait1.atlassian.net`, account `jsangiorgio@generaitsolutions.com`)
- `posthog-personal-gait` → PostHog project **433907** ("Default project")
- Other clones (S2H, Personal) must **not** leak into GAIT-scoped searches when filtered by `server_id`

**Meta workflow rules (from meta-gateway-invoke spec):**
1. `mcpmux_list_servers` before assuming a server is active
2. `mcpmux_search_tools` → `mcpmux_get_tool_schema` → `mcpmux_invoke_tool`
3. No param guessing — read schema first
4. Prefer `search_tools` over `list_all_tools` for agent discovery
5. Pass `filter` only when testing truncation (Phase B)

---

## §0 — Sanity (meta-only surface)

```
1. mcpmux_list_servers — show all servers; highlight GAIT-related rows and status (enabled_via_binding vs inactive)
2. Count tools in your direct client tool list — list names
3. Confirm: only ~10 `mcpmux_*` meta tools + optional surfaced backend (if any); no full backend catalog
```

**Pass:** GAIT Jira + GAIT PostHog show `enabled_via_binding` (or note if inactive). No hundreds of `posthog_*` / `atlassian_*` in direct tool list.

---

## §1 — Jira GAIT (clone isolation)

```
1. mcpmux_search_tools({ query: "user info", server_id: "com.atlassian-mcp-gait", detail_level: "description" })
2. mcpmux_get_tool_schema({ tools: ["com.atlassian-mcp-gait_atlassianUserInfo"] }) — adjust qualified name if search returns different prefix
3. mcpmux_invoke_tool({ server_id: "com.atlassian-mcp-gait", tool: "atlassianUserInfo", args: {} })
4. mcpmux_invoke_tool({ server_id: "com.atlassian-mcp-gait", tool: "getAccessibleAtlassianResources", args: {} })
```

**Pass criteria:**
- Email is **`jsangiorgio@generaitsolutions.com`** (NOT sync2hire.com)
- Site is **`generait1.atlassian.net`** (NOT sync2hire)
- Search results scoped to `com.atlassian-mcp-gait` only

**Negative check:**
```
mcpmux_search_tools({ query: "atlassianUserInfo", server_id: "com.atlassian-mcp", detail_level: "name" })
```
**Pass:** S2H Jira clone either inactive in this workspace or zero invokable hits (must not return GAIT data when calling S2H server_id).

---

## §2 — PostHog GAIT (clone isolation)

```
1. mcpmux_search_tools({ query: "projects", server_id: "posthog-personal-gait", detail_level: "description" })
2. mcpmux_get_tool_schema for `projects-get` (use qualified name from search)
3. mcpmux_invoke_tool({ server_id: "posthog-personal-gait", tool: "projects-get", args: {} })
4. mcpmux_invoke_tool({ server_id: "posthog-personal-gait", tool: "insights-list", args: {} })
   — with filter: { "max_rows": 3, "fields": ["name","short_id"] }
```

**Pass criteria:**
- `projects-get` → project id **`433907`**, name **Default project** (NOT When.Band 345911, NOT Sync2Hire 311512)
- Insights include GAIT-specific names (e.g. "Report lifecycle funnel", "Rewrite feature usage", "Template ingest outcomes")
- Filter step returns `{ returned, total, truncated }` envelope with ≤3 rows

**Negative check:**
```
mcpmux_invoke_tool({ server_id: "posthog-work", tool: "projects-get", args: {} })
```
**Pass:** Denied (inactive / not invokable / wrong workspace) — must NOT return Sync2Hire from GAIT workspace without explicit enable + binding.

---

## §3 — Search DX + ACL semantics (Phase A/C)

```
1. mcpmux_list_all_tools({ server_id: "posthog-personal-gait" })
   — report total_installed, total_invokable, and whether rows have invokable: true/false
2. mcpmux_search_tools({ query: "", server_id: "posthog-personal-gait", detail_level: "name", limit: 10 })
3. Compare counts: search total should match total_invokable, NOT total_installed
4. mcpmux_get_tool_schema({
     tools: ["posthog-personal-gait_projects-get", ""]
   })
   — expect missing: [""] for empty string; valid name returns schema
5. (Optional) repeat with a tool NOT in ACL — expect missing entry + message
```

**Pass criteria (per meta-gateway-invoke-retest §3 + §10):**
- `list_all_tools` has `hint` steering to search
- `total_invokable` matches search `total` (may be < `total_installed` when ACL is partial)
- Batch schema returns `missing` array + message for invalid / non-ACL tools (not silent drop)

Repeat briefly for `com.atlassian-mcp-gait` if time permits.

---

## §4 — Fail-closed + actionable errors (Phase A)

```
1. mcpmux_disable_server({ server_id: "posthog-personal-gait", scope: "session" })
2. mcpmux_invoke_tool({ server_id: "posthog-personal-gait", tool: "projects-get", args: {} })
   — paste exact error
3. Follow error hint (enable_server), retry successfully
4. mcpmux_enable_server({ server_id: "posthog-personal-gait", scope: "session" }) to restore
```

**Pass:** Error mentions `mcpmux_enable_server` with server_id; recovery works.

---

## §5 — End-to-end GAIT agent task (realism)

```
Using meta tools only, produce a brief GAIT status brief:

**Jira:** search issues in generAIt project (JQL or search tool — read schema first). Return up to 3 issue keys + summaries.

**PostHog:** from insights-list (filtered), name 3 dashboards/insights that track report workflow or AI rewrite usage.

**Format:** markdown with sections Jira / PostHog / Meta-DX notes (any friction: search empty, schema batch, filter, wrong clone).
```

**Pass:** Completed without guessing tool params; clone data is GAIT-specific throughout.

---

## §6 — Optional: Supabase in GAIT workspace

If `com.supabase-mcp-npx` is enabled via binding:

```
1. mcpmux_invoke_tool({ server_id: "com.supabase-mcp-npx", tool: "list_projects", args: {} })
2. Confirm GAIT projects visible (generait-staging, summarry-app) AND note whether personal projects also appear (unscoped server — document behavior, not a failure)
```

---

## FINAL REPORT (required — paste entire block back)

```
## GAIT Workspace Meta-Gateway Test
Overall: SHIP | SHIP WITH ISSUES | BLOCK
Workspace: generAIt (/Users/joe/Desktop/Repos/Contracts/generAIt)
Date:
Run: 1 | 2 (post-fix)

| Section | Result | Evidence |
|---------|--------|----------|
| §0 Sanity | PASS/FAIL | meta tool count: |
| §1 Jira GAIT | PASS/FAIL | email / site: |
| §2 PostHog GAIT | PASS/FAIL | project id / filter envelope: |
| §3 Search DX | PASS/FAIL | installed vs invokable / missing: |
| §4 Fail-closed | PASS/FAIL | error text: |
| §5 E2E task | PASS/FAIL | |
| §6 Supabase | PASS/FAIL/SKIP | |

## Clone isolation verified?
- [ ] Jira GAIT ≠ S2H
- [ ] PostHog GAIT (433907) ≠ Personal (345911) ≠ S2H (311512)

## Red flags (check any)
[ ] Backend tools in tools/list without Surface
[ ] Wrong clone data when server_id filtered
[ ] list_all_tools invokable count wrong vs search/invoke
[ ] Opaque invoke errors (no enable/invoke hints)
[ ] Schema batch drops invalid entries from missing
[ ] invoke filter not applied (no returned/total/truncated envelope)
[ ] Param guessing without get_tool_schema

## Friction log (verbatim errors / surprises)

## Environment snapshot
- mcpmux_list_servers GAIT rows:
- posthog-personal-gait: installed / invokable from list_all_tools:
- com.atlassian-mcp-gait status:
```

Rules: show exact JSON snippets for §1 email/site, §2 project id + filter envelope, §3 counts + missing array, §4 error message. Do not skip schema reads before invoke.
```

---

## Coverage map

| Planning doc section | Covered in prompt |
| -------------------- | ----------------- |
| `meta-gateway-invoke.md` — search → schema → invoke | §1, §2, §5 |
| Phase B filter | §2 insights filter |
| Phase C ACL + `list_all_tools` DX | §3 |
| `meta-gateway-invoke-qa.md` §0, §2, §7, §10, §11 | §0, §4, clone filter, §3, §5 |
| `meta-gateway-invoke-retest.md` §3, §6, §10 | §3 batch/missing, §2 filter, §3 diagnostic counts |

---

## Optional extensions

### §9 — Surfaced promotion (not in default prompt)

Surface one tool in `bundle:gait` (e.g. `posthog-personal-gait` `projects-get`), reload MCP, then verify:

1. Surfaced tool appears in client `tools/list`
2. Direct one-hop call works without `mcpmux_invoke_tool`
3. Non-surfaced backend on same server still requires invoke

### Shorter smoke (~5 min)

Run §0, §1 steps 3–4, §2 steps 3–4, §3 step 1 only; paste FINAL REPORT with other sections SKIP.

### Resources note

Cursor may show **~124 resources** on mux (PostHog `posthog://skills/...` URIs). Meta-gateway hard cut applies to **tools only** in Phases A–C — resources are still full grant materialization. Can pollute client UI and (depending on host) agent context; **Phase D** tracks `mcpmux_search_resources` / slim `resources/list` (see [`meta-gateway-invoke.md` Phase D](./meta-gateway-invoke.md#phase-d--advanced-optimizations-defer)). Interim: remove unused PostHog skill resources from `bundle:gait`.

---

## Run 2 evidence archive

<details>
<summary>Final report + friction log (2026-05-25)</summary>

**§1 identity:**
```json
{"email":"jsangiorgio@generaitsolutions.com"}
{"url":"https://generait1.atlassian.net","name":"generait1"}
```

**§2 project:**
```json
{"id":433907,"name":"Default project"}
```

**§2 filter (FAIL — same symptom as Run 1):**
- Request: `insights-list` + `filter: { max_rows: 3, fields: ["name","short_id"] }`
- Got: 16 full rows with all fields (id, description, created_by, …)
- Expected: `{ "returned": 3, "total": 16, "truncated": true, "insights": [...] }`

**§3 counts:**
```json
{"total_installed":338,"total_invokable":331,"search_total":331}
{"missing":[""],"message":"1 tool(s) not invokable or unknown with current grants → use mcpmux_search_tools to discover allowed names"}
```

**§4 session disable error:**
```json
{"error":"invoke_failed","message":"server 'posthog-personal-gait' is disabled for this session → mcpmux_enable_server({ \"server_id\": \"posthog-personal-gait\" })"}
```

**§5 Jira issues (JQL `project = GAIT order by updated DESC`, max 3):**
- GAIT-163 — [Wave 2] BE-CITE-READ-1 (Done)
- GAIT-165 — Migrate organization_prompts to template-driven seeding (Done)
- GAIT-160 — BE-PROJ-LIST-1: Server-driven /projects list (Idea)

**§5 PostHog insights (report workflow / AI rewrite):**
- Report lifecycle funnel (`P7EAdk3q`)
- Rewrite feature usage (`9d4ljh6t`)
- Template ingest outcomes (`22sPNhOj`)

**§6 Supabase:** generait-staging, summarry-app + set-times-app* personal projects

**Benign / expected:**
- `posthog-work` inactive → `mcpmux_enable_server` hint
- Unbounded JQL rejected by Atlassian API (requires `project = GAIT …`)

**Environment snapshot:**
- `com.atlassian-mcp-gait` — 37/37 invokable, `enabled_via_binding`
- `posthog-personal-gait` — 338 installed / 331 invokable, `enabled_via_binding`
- S2H / personal clones — `inactive`

</details>

---

## Run 1 evidence archive

<details>
<summary>Friction log + environment snapshot (2026-05-25)</summary>

**Errors (expected / benign):**
- `posthog-work` inactive → `mcpmux_enable_server` hint
- Session disable → same hint pattern

**Failures (fixed in gateway):**
- `insights-list` filter: 16 full rows, no envelope
- `list_all_tools`: `total_invokable: 0`, all rows `invokable: false`
- `get_tool_schema(["…", ""])`: no `missing: [""]`

**GAIT rows (`mcpmux_list_servers`):**
- `com.atlassian-mcp-gait` — 37 tools, `enabled_via_binding`
- `posthog-personal-gait` — 338 tools, `enabled_via_binding`
- S2H / personal PostHog / S2H Jira — `inactive`

**§1 identity:** `jsangiorgio@generaitsolutions.com`, `generait1.atlassian.net`  
**§2 project:** id `433907`, "Default project"  
**§6 Supabase:** generait-staging, summarry-app + set-times-app* personal projects

</details>
