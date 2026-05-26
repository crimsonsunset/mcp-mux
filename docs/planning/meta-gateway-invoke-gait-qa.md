# GAIT Workspace — Meta-Gateway Invoke Capability Test

**Last Updated:** May 26, 2026  
**Status:** **SHIP** — Run 5 passed; Issue #2 v3 confirmed live (YAML coalesce + filter envelope on `insights-list`)  
**Branch:** `dev` (Issue #2 v3 in `crates/mcpmux-gateway/services/meta_tools/invoke.rs`)  
**Related:** [`meta-gateway-invoke.md`](./meta-gateway-invoke.md), [`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md), [`meta-gateway-invoke-retest.md`](./meta-gateway-invoke-retest.md)

**Source of truth for:** GAIT workspace binding QA, clone isolation, meta-gateway invoke DX, what passed/failed, what was fixed in code, and what still needs a live re-run.

---

## Current verdict

| Phase | Verdict | Notes |
| ----- | ------- | ----- |
| **Run 1** (2026-05-25, generAIt workspace) | **SHIP WITH ISSUES** | Core binding + clone isolation + E2E pass; §2 filter, §3 ACL reporting failed |
| **Run 2** (2026-05-25, generAIt workspace) | **SHIP WITH ISSUES** | §3 ACL + batch `missing` fixed; §2 filter step still fails (16 full rows, no envelope) |
| **Run 3** (2026-05-26, post-rebuild) | **SHIP WITH ISSUES** | Same §2 filter fail; suspected stale binary (Run 4 ruled that out on fresh build) |
| **Run 4** (2026-05-26, post Issue #2 v2, fresh build) | **SHIP WITH ISSUES** | §2 filter still fails; root cause = PostHog YAML payload, not stale binary |
| **Run 5** (2026-05-26, post Issue #2 v3, fresh build) | **SHIP** | §2 filter **Pass** — `{ returned: 3, total: 16, truncated: true }` + field projection; no regressions |

**Target for ship:** Run 5 passes §2 step 4 → overall **SHIP**. **Achieved.**

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

| Flag | Run 1 | Run 2 | Run 3 | Run 4 | Resolution |
| ---- | ----- | ----- | ----- | ----- | ---------- |
| Backend tools in `tools/list` without Surface | Clear | Clear | Clear | Clear | — |
| Wrong clone data when `server_id` filtered | Clear | Clear | Clear | Clear | — |
| `list_all_tools` invokable reporting broken | **Hit** | Clear | Clear | Clear | Fixed — Issue #1 **confirmed** |
| Opaque invoke errors | Clear | Clear | Clear | Clear | — |
| Schema batch omits empty string from `missing` | **Hit** | Clear | Clear | Clear | Fixed — Issue #3 **confirmed** |
| Param guessing without schema | Clear | Clear | Clear | Clear | — |
| `invoke_tool` filter not applied on `insights-list` | **Hit** | **Hit** | **Hit** | **Hit** | Fixed — Issue #2 **v3 confirmed** (Run 5) |

---

## Run 5 — Sign-off (2026-05-26)

Workspace: `/Users/joe/Desktop/Repos/Contracts/generAIt`  
Context: post Issue #2 v3 (`yaml_serde` YAML coalesce); fresh gateway build + MCP reload confirmed.

| Section | Result | Notes |
| ------- | ------ | ----- |
| §0 Sanity | **Pass** | 10 meta tools; GAIT Jira + PostHog `enabled_via_binding`; S2H/personal clones `inactive` |
| §1 Jira GAIT | **Pass** | `jsangiorgio@generaitsolutions.com` / `generait1.atlassian.net`; S2H search 0 hits |
| §2 PostHog GAIT | **Pass** | Project 433907 correct; filter envelope + ≤3 projected rows |
| §3 Search DX | **Pass** | 338/331; search `""` total=331; `missing: [""]` |
| §4 Fail-closed | **Pass** | disable → enable hint → recovery |
| §5 E2E task | **Pass** | Jira (GAIT-163/165/160) + PostHog brief (filtered insights-list) |
| §6 Supabase | **Skip** | `com.supabase-mcp-npx` inactive in this workspace |
| **Overall** | **SHIP** | Issue #2 v3 confirmed; ready for PR #155 merge |

**Clone isolation verified:**

- [x] Jira GAIT ≠ S2H
- [x] PostHog GAIT (433907) ≠ Personal (345911) ≠ S2H (311512)

**Re-test checklist (Run 5):**

| Step | Result | Evidence |
| ---- | ------ | -------- |
| 1 `list_all_tools` | **Pass** | 331 invokable / 338 installed; hint present |
| 2 `insights-list` + filter (`max_rows` + `fields`) | **Pass** | `{ returned: 3, total: 16, truncated: true, results: [{name, short_id}×3] }` |
| 2b `insights-list` + `max_bytes` only (control) | **Pass** | `{ returned: 314, total: 13795, truncated: true, text: "..." }` |
| 3 batch `""` → `missing` | **Pass** | `"missing": [""]` on `insights-list` batch |
| 4 §1 + §4 | **Pass** | No regressions |

**§2 filter evidence (PASS — v3 fix confirmed):**

Request:
```json
{ "filter": { "max_rows": 3, "fields": ["name", "short_id"] } }
```
Got:
```json
{
  "count": 16,
  "results": [
    { "name": "Rewrite feature usage", "short_id": "9d4ljh6t" },
    { "name": "Section resets (quality signal)", "short_id": "AYEN0OCK" },
    { "name": "Reports created vs completed", "short_id": "Sxxd3xCD" }
  ],
  "returned": 3,
  "total": 16,
  "truncated": true
}
```

---

## Run 4 — Sign-off (2026-05-26)

Workspace: `/Users/joe/Desktop/Repos/Contracts/generAIt`  
Context: post Issue #2 v2 (`coalesce_structured_payload`); **fresh gateway build confirmed** (not stale binary).

| Section | Result | Notes |
| ------- | ------ | ----- |
| §0 Sanity | **Pass** | 10 meta tools; GAIT Jira + PostHog `enabled_via_binding`; S2H/personal clones `inactive` |
| §1 Jira GAIT | **Pass** | `jsangiorgio@generaitsolutions.com` / `generait1.atlassian.net`; S2H search 0 hits |
| §2 PostHog GAIT | **Fail** (filter only) | Project 433907 correct; insights correct; **filter step still failed** |
| §3 Search DX | **Pass** | 338/331; search `""` total=331; `missing: [""]` |
| §4 Fail-closed | **Pass** | disable → enable hint → recovery |
| §5 E2E task | **Pass** | Jira (GAIT-163/165/160) + PostHog brief (unfiltered insights-list) |
| §6 Supabase | **Skip** | `com.supabase-mcp-npx` inactive in this workspace |
| **Overall** | **Ship w/ issues** | Blocked on Issue #2 **v3** |

**Clone isolation verified:**

- [x] Jira GAIT ≠ S2H
- [x] PostHog GAIT (433907) ≠ Personal (345911) ≠ S2H (311512)

**Re-test checklist (Run 4):**

| Step | Result | Evidence |
| ---- | ------ | -------- |
| 1 `list_all_tools` | **Pass** | 331 invokable / 338 installed; hint present |
| 2 `insights-list` + filter (`max_rows` + `fields`) | **Fail** | 16 full rows; no `{ returned, total, truncated }` envelope |
| 2b `insights-list` + `max_bytes` only (control) | **Pass** | Filter pipeline works: `{ returned, total, truncated, text }` |
| 3 batch `""` → `missing` | **Pass** | `"missing": [""]` on `insights-list` batch |
| 4 §1 + §4 | **Pass** | No regressions |

**Run 4 diagnosis (corrected — not stale binary):**

Filter **does** reach the gateway (`max_bytes` truncates on both `projects-get` and `insights-list`). `max_rows` / `fields` fail because live PostHog `insights-list` returns **YAML-style text** in `content[]` (e.g. `count: 16`, `results[16]:`), **not JSON**, and `structuredContent` is absent. v2's `coalesce_structured_payload` only parses JSON from content blocks; when coalesce fails, `shape_content_block` cannot parse the payload and — with only `max_rows`/`fields` (no `max_bytes`) — returns the block **unchanged**.

Integration tests pass because they mock JSON strings in `content[].text`; live PostHog output does not match that fixture shape.

**§2 filter evidence:**

Request:
```json
{ "filter": { "max_rows": 3, "fields": ["name", "short_id"] } }
```
Got: 16 full insight rows (all fields), paginated YAML shape — no envelope.

Control (proves filter pipeline live):
```json
{ "filter": { "max_bytes": 300 } }
→ { "returned": 314, "total": 14848, "truncated": true, "text": "count: 16\n...[truncated]" }
```

**Eng follow-up → Issue #2 v3:** Handle non-JSON (YAML) PostHog content when `max_rows`/`fields` are set — e.g. YAML parse in coalesce/shape path, or fallback row truncation on unparsed text. Add regression test with YAML fixture matching live `insights-list` output.

---

## Run 3 — Sign-off (2026-05-26)

Workspace: `/Users/joe/Desktop/Repos/Contracts/generAIt`  
Context: user rebuilt gateway; live retest of Run 2 blockers.

| Section | Result | Notes |
| ------- | ------ | ----- |
| §2 filter step | **Fail** | Still 16 full rows; no `{ returned, total, truncated }` envelope |
| §3 counts + `missing` | **Pass** | 338/331; `missing: [""]` |
| §1 smoke | **Pass** | GAIT email/site unchanged |
| §4 smoke | **Pass** | disable → enable hint → recovery |

**Re-test checklist (Run 3):**

| Step | Result | Evidence |
| ---- | ------ | -------- |
| 1 `list_all_tools` | **Pass** | 331 invokable |
| 2 `insights-list` + filter | **Fail** | 16 full rows (old binary still serving; `Finished in 0.20s` at startup = no recompile) |
| 3 batch `""` → `missing` | **Pass** | `"missing": [""]` |
| 4 §1 + §4 | **Pass** | No regressions |

**Run 3 diagnosis:** PostHog `insights-list` returns paginated JSON (`{ count, results: [...] }`) in `content[].text` and/or `structuredContent`. v1 filter only mirrored shaped output when `structuredContent` was present; v1 also missed coalescing JSON from content when structured was absent. **Issue #2 v2** refactors `apply_invoke_result_filter` to coalesce payload first, shape once, mirror to both channels.

**Transient error (first filtered call after rebuild):**
```json
{"error":"invoke_failed","message":"MCP call failed: ... HTTP 500 Internal Server Error"}
```
Subsequent calls succeeded but filter still unapplied (stale process).

**Eng follow-up (2026-05-26):** `coalesce_structured_payload` + fields-only projection on nested arrays; tests `posthog_paginated_results_truncates_from_content_json`, `invoke_filter_shapes_posthog_paginated_results_in_content_json`. Run 4 confirmed v2 fix is deployed but **insufficient** for live YAML payloads — see Run 4 diagnosis.

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

### Issue #2 — confirmed fixed (Run 5)

| # | Symptom | Root cause | Fix (v3) | Files | Run 5 |
| - | ------- | ---------- | -------- | ----- | ----- |
| **2** | `insights-list` + `max_rows`/`fields` filter → 16 full rows, no envelope | v2 JSON-only coalesce; live PostHog YAML in `content[]`; JSON substring falsely matched `[16]` in `results[16]:` | YAML parse (`yaml_serde`) before JSON substring; object/array only; `results[N]` key normalize; same `shape_json_value` pipeline | `services/meta_tools/invoke.rs`, workspace `serde_yaml = { package = "yaml_serde" }` | **Pass** |

**Regression tests (pass):** unit `posthog_paginated_results_truncates_from_content_yaml`, `yaml_payload_parses_posthog_insights_list_shape`, `bracketed_array_key_base_normalizes_posthog_results_key`; integration `invoke_filter_shapes_posthog_paginated_results_in_content_yaml`; prior JSON fixtures unchanged.

### Open (non-blocking / follow-up)

| Item | Severity | Owner | Notes |
| ---- | -------- | ----- | ----- |
| ~~Run 5 §2 step 4 live QA~~ | ~~Required before ship~~ | QA | **Done** — filter envelope confirmed (Run 5) |
| Resource list bloat (~124 PostHog skill URIs) | Medium | **Resolved Phase D** | Hard cut + `mcpmux_search_resources`; re-run GAIT §0 expecting 0 resources |
| Supabase hard project isolation | Optional | Config | Needs 4 clones with `--project-ref` or accept unscoped |
| Rename PostHog project 433907 | Cosmetic | PostHog UI | Still "Default project" |
| PR #155 merge + CHANGELOG | Process | Eng | Unblocked — Issue #2 v3 + Run 5 **SHIP** |
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

**Run 5 complete (2026-05-26):** Issue #2 v3 confirmed → Overall **SHIP**. Next: PR #155 merge + CHANGELOG.

---

## Dev / rebuild (required after gateway code changes)

From repo root on **`dev`** ([`fork-integration.md`](./fork-integration.md)):

```bash
git checkout dev
git pull origin dev
pnpm dev:restart    # after gateway changes — stop orphans, rebuild, start dev
# or: pnpm dev      # normal iteration (predev frees ports automatically)
```

**Do not** use `./target/debug/mcpmux` alone — skips Vite and may leave a stale gateway. If startup logs `Finished dev profile in 0.20s` with no `Compiling mcpmux-gateway` after an invoke.rs edit, use **`pnpm dev:restart`**.

After gateway code changes: stop any existing mux process, run `pnpm dev`, wait for `Gateway] Ready to accept connections`, then Cursor → MCP → **Reload tools**.

---

## Prep (required before any tests)

1. Gateway running via **`pnpm dev`** (or production desktop app) on `http://localhost:45818/mcp`
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
Run: 1 | 2 | 3 | 4 | 5 (post-v3)

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

## Run 5 evidence archive

<details>
<summary>Final report + friction log (2026-05-26)</summary>

**§1 identity:**
```json
{"email":"jsangiorgio@generaitsolutions.com"}
{"url":"https://generait1.atlassian.net","name":"generait1"}
```

**§2 project:**
```json
{"id":433907,"name":"Default project"}
```

**§2 filter (PASS — v3 YAML coalesce):**
```json
{
  "count": 16,
  "results": [
    { "name": "Rewrite feature usage", "short_id": "9d4ljh6t" },
    { "name": "Section resets (quality signal)", "short_id": "AYEN0OCK" },
    { "name": "Reports created vs completed", "short_id": "Sxxd3xCD" }
  ],
  "returned": 3,
  "total": 16,
  "truncated": true
}
```

**§2 control (`max_bytes: 300`):**
```json
{"returned":314,"total":13795,"truncated":true,"text":"{\"count\":16,...[truncated]"}
```

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

**§5 PostHog insights (filtered):**
- Rewrite feature usage (`9d4ljh6t`)
- Section resets (quality signal) (`AYEN0OCK`)
- Reports created vs completed (`Sxxd3xCD`)

**§6 Supabase:** SKIP — `com.supabase-mcp-npx` inactive

**Environment snapshot:**
- `com.atlassian-mcp-gait` — 37 tools, `enabled_via_binding`
- `posthog-personal-gait` — 338 installed / 331 invokable, `enabled_via_binding`
- S2H / personal clones — `inactive`

**Friction:** None. Parallel disable/invoke during §3 re-check caused transient race (expected when batching disable with invoke); sequential §4 flow clean.

</details>

---

## Run 4 evidence archive

<details>
<summary>Final report + friction log (2026-05-26)</summary>

**§1 identity:**
```json
{"email":"jsangiorgio@generaitsolutions.com"}
{"url":"https://generait1.atlassian.net","name":"generait1"}
```

**§2 project:**
```json
{"id":433907,"name":"Default project"}
```

**§2 filter (FAIL — YAML payload, not stale binary):**
- Request: `insights-list` + `filter: { max_rows: 3, fields: ["name","short_id"] }`
- Got: 16 full rows with all fields (YAML-shaped content)
- Expected: `{ "returned": 3, "total": 16, "truncated": true, "results": [...] }`
- Control: `filter: { max_bytes: 300 }` → byte envelope applied (filter pipeline confirmed live)

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

**§5 PostHog insights (unfiltered list — filter step blocked):**
- Report lifecycle funnel (`P7EAdk3q`)
- Rewrite feature usage (`9d4ljh6t`)
- Template ingest outcomes (`22sPNhOj`)

**§6 Supabase:** SKIP — `com.supabase-mcp-npx` inactive

**Environment snapshot:**
- `com.atlassian-mcp-gait` — 37 tools, `enabled_via_binding`
- `posthog-personal-gait` — 338 installed / 331 invokable, `enabled_via_binding`
- S2H / personal clones — `inactive`

</details>

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
