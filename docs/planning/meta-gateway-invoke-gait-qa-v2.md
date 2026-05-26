# GAIT Workspace — Meta-Gateway Invoke Capability Test (v2 / Phase D)

**Last Updated:** May 26, 2026  
**Status:** **Run 2 SHIP** — Issue #4 verified; Phase D gate cleared on `dev` @ **a4a212a**  
**Supersedes for Phase D:** [`meta-gateway-invoke-gait-qa.md`](./meta-gateway-invoke-gait-qa.md) (Phases A–C runs 1–5 remain canonical for tool invoke)  
**Related:** [`meta-gateway-invoke.md`](./meta-gateway-invoke.md), [`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md) (§12–14), [`run-from-source-macos.md`](../run-from-source-macos.md)

**Source of truth for:** GAIT workspace binding QA **after Phase D** — resource/prompt hard cut, 14 meta tools, search/read/fetch disclosure, plus regression on tools invoke path from v1.

---

## What changed in v2 (Phase D)

| Surface | v1 (Runs 1–5) | v2 (Phase D) |
| ------- | ------------- | ------------ |
| Meta tools in client | ~10 `mcpmux_*` | **~14** `mcpmux_*` (+ search/read/fetch for resources & prompts) |
| `tools/list` | Meta + surfaced tools only | Same |
| `resources/list` | Full grant catalog (~124 PostHog skill URIs) | **Surfaced only** (default **0**) |
| `prompts/list` | Full grant catalog | **Surfaced only** (default **0**) |
| Resource discovery | N/A | `mcpmux_search_resources` → `mcpmux_read_resource` |
| Prompt discovery | N/A | `mcpmux_search_prompts` → `mcpmux_fetch_prompt` |
| Invoke errors | Substring hints | **Levenshtein** "did you mean?" |
| Search ranking | Substring + alphabetical | **TF-IDF** when query present |

**Same testing spirit as v1:** clone isolation, fail-closed errors, schema-before-invoke, realistic E2E brief, verbatim evidence in FINAL REPORT.

---

## Current verdict

| Run | Date | Verdict | Notes |
| --- | ---- | ------- | ----- |
| **Run 1** | May 26, 2026 | **BLOCK** | §0–§5 pass; §7 `read_resource` routes to inactive `posthog-personal` parent |
| **Run 2** | May 26, 2026 | **SHIP** | Issue #4 fixed; §7 read returns content; §0–§5 no regressions |

**Target for ship:** Run 2 passes §7 (search → read on clone) with no regressions on §0–§5.

---

## What this test validates

| Area | Sections | Pass signal |
| ---- | -------- | ----------- |
| Meta-only client surface (Phase D) | §0 | **14** `mcpmux_*` tools; **0** resources; **0** prompts (unless surfaced) |
| Tool invoke regression (Phases A–C) | §1–§5 | Same pass criteria as [`meta-gateway-invoke-gait-qa.md`](./meta-gateway-invoke-gait-qa.md) |
| Resource hard cut + search → read | §7 | `search_resources` hits PostHog skills; `read_resource` returns content |
| Prompt hard cut + search → fetch | §8 | `search_prompts` + `fetch_prompt` on bound prompt (if any) |
| Surfaced one-hop (optional) | §9 | Surfaced resource/prompt in client lists; direct read/fetch works |
| Levenshtein invoke hint | §4b | Typo tool name → `did you mean` in error |
| Clone isolation | §1, §2, §7 | GAIT `server_id` filters never return S2H/Personal data |

**Still out of scope / document-only:**

- Supabase unscoped multi-project leak (server design)
- PostHog `projects-get` accepts `{}` without `context` (backend MCP)
- Bundle:gait PostHog skill trim (operator config — optional interim if search fails)

---

## Dev / rebuild (required before any tests)

From repo root on **`dev`**:

```bash
git checkout dev
pnpm dev:restart    # after gateway changes — stop orphans, rebuild, start dev
```

See [`run-from-source-macos.md`](../run-from-source-macos.md) for full detail.

| Command | When |
| ------- | ---- |
| `pnpm dev:restart` | After **any** `crates/mcpmux-gateway` edit (Phase D) |
| `pnpm dev` | Normal UI iteration (predev frees `:1420` / `:45818`) |
| `pnpm dev:stop` | Port conflict — then restart |

**Stale-binary smell:** startup logs `Finished dev profile in 0.20s` with **no** `Compiling mcpmux-gateway` after you edited gateway code → run **`pnpm dev:restart`**.

**Do not** run `./target/debug/mcpmux` alone — skips Vite/Tauri shell.

Wait for log line:

```text
[Gateway] Ready to accept connections
```

Then: **Cursor → MCP → Reload tools** (generAIt workspace, not mcp-mux repo).

---

## Prep (required before any tests)

1. Gateway running via **`pnpm dev:restart`** on `http://localhost:45818/mcp`
2. Open **`/Users/joe/Desktop/Repos/Contracts/generAIt`** in Cursor
3. Cursor → MCP → **Reload tools**; confirm `user-mcpmux` connected
4. GAIT workspace binding: `bundle:core`, `bundle:comms-personal`, `bundle:browser`, `bundle:gait`, `bundle:db-personal`
5. **`com.atlassian-mcp-gait`** → `enabled_via_binding`
6. **`posthog-personal-gait`** → project **433907** → `enabled_via_binding`

**Expected GAIT stack** — unchanged from v1:

| Server ID | Scope |
| --------- | ----- |
| `com.atlassian-mcp-gait` | generAIt Jira |
| `posthog-personal-gait` | PostHog **433907** |
| `com.supabase-mcp-npx` | Unscoped (§6 optional) |

**Must NOT leak** — unchanged from v1:

| Server ID | Wrong data |
| --------- | ---------- |
| `com.atlassian-mcp` | S2H Jira |
| `posthog-personal` | When.Band **345911** |
| `posthog-work` | Sync2Hire **311512** |

**FeatureSet editor (Phase D):**

| Control | Tools | Resources | Prompts |
| ------- | ----- | --------- | ------- |
| **Checkbox** | Invoke ACL | Read ACL | Fetch ACL |
| **Surface** | → `tools/list` | → `resources/list` | → `prompts/list` |

After any Surface change: **Reload MCP tools**.

**Environment:** generAIt folder only — opening **mcp-mux repo** binds `All` (invalid for GAIT isolation).

---

## Agent Prompt (v2)

Copy into a **fresh Cursor agent** (generAIt workspace, prep complete):

```markdown
# GAIT workspace — McpMux meta-gateway v2 (Phase D) capability test

Validate **GAIT workspace binding** on McpMux (`http://localhost:45818/mcp` via `user-mcpmux`).

**Phase D rules:**
- Client lists: ~14 `mcpmux_*` tools; **0 resources** and **0 prompts** unless operator surfaced items
- Backend **tools:** `mcpmux_search_tools` → `mcpmux_get_tool_schema` → `mcpmux_invoke_tool`
- Backend **resources:** `mcpmux_search_resources` → `mcpmux_read_resource`
- Backend **prompts:** `mcpmux_search_prompts` → `mcpmux_fetch_prompt`
- No param guessing; read schemas before invoke/fetch
- Use meta tools only unless §9 tests surfaced one-hop

**Expected GAIT stack:**
- `com.atlassian-mcp-gait` → generAIt Jira
- `posthog-personal-gait` → PostHog project **433907**
- S2H/Personal clones must not leak when `server_id` filtered

---

## §0 — Sanity (Phase D surface)

```
1. mcpmux_list_servers — GAIT Jira + PostHog status
2. Count tools, resources, and prompts in your McpMux client line (e.g. "14 tools, 0 prompts, 0 resources")
3. List all tool names — expect ~14 mcpmux_* only (no backend catalog)
4. Confirm resources count is 0 (NOT ~124) unless user surfaced one
5. Confirm prompts count is 0 unless user surfaced one
```

**Pass:** 14 meta tools; **0 resources**; **0 prompts**; GAIT servers `enabled_via_binding`.

---

## §1 — Jira GAIT (clone isolation) — regression

Same as v1 [`meta-gateway-invoke-gait-qa.md` §1]:

```
1. mcpmux_search_tools({ query: "user info", server_id: "com.atlassian-mcp-gait", detail_level: "description" })
2. mcpmux_get_tool_schema for atlassianUserInfo (qualified name from search)
3. mcpmux_invoke_tool({ server_id: "com.atlassian-mcp-gait", tool: "atlassianUserInfo", args: {} })
4. mcpmux_invoke_tool getAccessibleAtlassianResources
```

**Pass:** `jsangiorgio@generaitsolutions.com` / `generait1.atlassian.net`; S2H search 0 hits.

---

## §2 — PostHog GAIT (clone isolation + filter) — regression

Same as v1 §2 including filter step:

```
1–3. projects-get → project id 433907
4. insights-list with filter: { "max_rows": 3, "fields": ["name","short_id"] }
```

**Pass:** Filter envelope `{ returned, total, truncated }` with ≤3 rows; not S2H/Personal project ids.

---

## §3 — Search DX + ACL (tools) — regression

Same as v1 §3:

```
1. mcpmux_list_all_tools({ server_id: "posthog-personal-gait" })
2. mcpmux_search_tools({ query: "", server_id: "posthog-personal-gait", detail_level: "name", limit: 10 })
3. total_invokable matches search total
4. mcpmux_get_tool_schema({ tools: ["posthog-personal-gait_projects-get", ""] }) → missing: [""]
```

---

## §4 — Fail-closed + recovery — regression

Same as v1 §4 (disable → invoke error with enable hint → recovery).

---

## §4b — Levenshtein invoke hint (Phase D)

```
mcpmux_invoke_tool({ server_id: "posthog-personal-gait", tool: "projects-gt", args: {} })
```

**Pass:** Error contains `did you mean` and suggests `projects-get` (or close match).

---

## §5 — E2E GAIT brief (tools) — regression

Same as v1 §5: Jira issues + PostHog insights brief via search → schema → invoke.

---

## §6 — Supabase (optional)

Same as v1 §6 if server active.

---

## §7 — PostHog resources (Phase D) **NEW**

PostHog exposes many `posthog://skills/...` resources in GAIT binding. v1 showed ~124 in Cursor; v2 should show **0** in client list but full ACL via search.

```
1. mcpmux_search_resources({
     query: "skill",
     server_id: "posthog-personal-gait",
     detail_level: "description",
     limit: 10
   })
2. Pick one URI from results (e.g. posthog://skills/...)
3. mcpmux_read_resource({ uri: "<uri from step 2>" })
4. Report whether Cursor client resources count stayed 0 throughout
```

**Pass criteria:**
- Search returns ≥1 skill URI for GAIT PostHog
- `read_resource` returns content (not ACL denied)
- Search total ≤ grant ACL (no S2H URIs when `server_id` filtered)
- Client resources list remained **0** (unless §9 surfaced one)

**Negative (optional):**
```
mcpmux_search_resources({ query: "skill", server_id: "posthog-work", detail_level: "name" })
```
**Pass:** Zero hits or inactive-server hint — no Sync2Hire skill URIs.

---

## §8 — Prompts disclosure (Phase D) **NEW**

If binding includes fetchable prompts (e.g. Firebase deploy prompts from bundles):

```
1. mcpmux_search_prompts({ query: "", server_id: "<any bound prompt server>", detail_level: "description", limit: 5 })
2. If total > 0: mcpmux_fetch_prompt({ server_id: "...", prompt: "<name from search>", args: {} })
3. Confirm client prompts count is 0 unless surfaced
```

**Pass if prompts exist in ACL:** search → fetch works; client prompts list **0**.

**Skip OK:** No prompt members in bound FeatureSets — note in report.

---

## §9 — Surfaced promotion (optional)

In FeatureSet editor: include + **Surface** one PostHog skill URI and/or one prompt. Reload MCP.

```
1. Confirm surfaced resource appears in client resources/list (count 1)
2. Direct read_resource one-hop on surfaced URI (if client exposes it)
3. Non-surfaced resource on same server still requires mcpmux_read_resource
```

Same pattern for surfaced prompt → `prompts/list` + direct `get_prompt`.

---

## FINAL REPORT (required)

```
## GAIT Workspace Meta-Gateway Test v2 (Phase D)
Overall: SHIP | SHIP WITH ISSUES | BLOCK
Workspace: generAIt
Date:
Run: 1

| Section | Result | Evidence |
|---------|--------|----------|
| §0 Sanity (14/0/0) | PASS/FAIL | tools / resources / prompts counts: |
| §1 Jira GAIT | PASS/FAIL | email / site: |
| §2 PostHog GAIT | PASS/FAIL | project id / filter envelope: |
| §3 Search DX (tools) | PASS/FAIL | installed vs invokable / missing: |
| §4 Fail-closed | PASS/FAIL | error text: |
| §4b Levenshtein | PASS/FAIL | did you mean text: |
| §5 E2E task | PASS/FAIL | |
| §6 Supabase | PASS/FAIL/SKIP | |
| §7 Resources (Phase D) | PASS/FAIL | search total / read snippet: |
| §8 Prompts (Phase D) | PASS/FAIL/SKIP | |
| §9 Surfaced | PASS/FAIL/SKIP | |

## Clone isolation verified?
- [ ] Jira GAIT ≠ S2H
- [ ] PostHog GAIT (433907) ≠ Personal ≠ S2H
- [ ] Resource search scoped to posthog-personal-gait

## Phase D red flags (check any)
[ ] Still ~124 resources in Cursor mux line (hard cut regression)
[ ] Backend tools in tools/list without Surface
[ ] search_resources returns S2H/personal URIs when server_id filtered
[ ] read_resource denied for ACL-visible URI
[ ] No Levenshtein hint on typo invoke
[ ] Tool path regressions vs v1 Runs 1–5

## Friction log

## Environment snapshot
- Gateway: pnpm dev:restart / commit:
- mcpmux meta tool count:
- Cursor mux line (tools / prompts / resources):
```

Rules: paste exact JSON for §2 filter envelope, §7 search/read, §4b error. Do not skip schema reads before invoke.
```

---

## Coverage map

| Spec | v2 section |
| ---- | ---------- |
| `meta-gateway-invoke.md` Phase D hard cut | §0, §7, §8 |
| `meta-gateway-invoke-qa.md` §12–14 | §7, §8, §9 |
| v1 tool invoke + clone isolation | §1–§5 |
| Levenshtein + TF-IDF polish | §4b, §3 search order (informal) |

---

## Shorter smoke (~8 min)

§0 → §7 steps 1–3 → §2 step 4 (filter) → §1 step 3 (email). Paste FINAL REPORT; mark §8–§9 SKIP if N/A.

---

## Relationship to v1 doc

| Doc | Use when |
| --- | -------- |
| [`meta-gateway-invoke-gait-qa.md`](./meta-gateway-invoke-gait-qa.md) | Historical Runs 1–5; Phases A–C ship evidence; Issue #1–#3 tracker |
| **This doc (v2)** | Phase D live QA; resource/prompt disclosure; regression gate before merge |

After v2 Run 2 **SHIP**, update **Current verdict** at top and archive evidence below.

---

## Issue tracker (Phase D)

| # | Symptom (Run 1) | Root cause | Fix | Files | Status |
| - | --------------- | ---------- | --- | ----- | ------ |
| **4** | `mcpmux_read_resource` on GAIT PostHog skill URI → `server 'posthog-personal' is inactive` while search returns `posthog-personal-gait` | Space-wide `find_server_for_resource_uri` picks parent clone when URI is duplicated | Grant-scoped `FeatureService::resolve_resource_server_from_grants` on read paths | `pool/features/facade.rs`, `meta_tools/disclosure.rs`, `mcp/handler.rs` | **Verified** — Run 2 §7 pass |

---

## Run 2 prompt (completed May 26, 2026)

Minimal re-test for Issue #4 — **done**; evidence in Run 2 archive below. Re-run after future §7 regressions:

```
1. mcpmux_search_resources({ query: "skill", server_id: "posthog-personal-gait", limit: 1 })
2. mcpmux_read_resource({ uri: "<uri from step 1>" })
3. Confirm Cursor mux line still 14 / 0 / 0
```

**Pass:** read returns content (not `posthog-personal` inactive error).

---

## Run 1 evidence archive

<details>
<summary>Run 1 — May 26, 2026</summary>

## GAIT Workspace Meta-Gateway Test v2 (Phase D)
Overall: **BLOCK**
Workspace: generAIt
Date: May 26, 2026
Run: 1

| Section | Result | Evidence |
|---------|--------|----------|
| §0 Sanity (14/0/0) | **PASS** | 14 meta tools; Cursor mux line: **14 tools, 0 prompts, 0 resources**; GAIT servers `enabled_via_binding` |
| §1 Jira GAIT | **PASS** | `jsangiorgio@generaitsolutions.com` / `generait1.atlassian.net`; S2H search 0 hits |
| §2 PostHog GAIT | **PASS** | project id **433907**; filter envelope below |
| §3 Search DX (tools) | **PASS** | `list_all_tools` invokable **331** = `search_tools` total **331**; `missing: [""]` |
| §4 Fail-closed | **PASS** | `"server 'posthog-personal-gait' is disabled for this session → mcpmux_enable_server(...)"` |
| §4b Levenshtein | **PASS** | `"tool 'posthog-personal-gait_projects-gt' is not invokable — did you mean gait_projects-get, gait_project-get, ..."` |
| §5 E2E task | **PASS** | Jira GAIT-163…150 + PostHog 5 insights via search → schema → invoke |
| §6 Supabase | **SKIP** | `com.supabase-mcp-npx` inactive in binding |
| §7 Resources (Phase D) | **FAIL** | search total **91** skill URIs; `read_resource` denied (see below) |
| §8 Prompts (Phase D) | **SKIP** | No fetchable prompts in bound FeatureSets (`total: 0`) |
| §9 Surfaced | **SKIP** | No surfaced resource/prompt configured |

## Clone isolation verified?
- [x] Jira GAIT ≠ S2H
- [x] PostHog GAIT (433907) ≠ Personal ≠ S2H
- [x] Resource search scoped to posthog-personal-gait (91 hits; posthog-work → inactive hint)

## Phase D red flags (check any)
- [ ] Still ~124 resources in Cursor mux line (hard cut regression)
- [ ] Backend tools in tools/list without Surface
- [ ] search_resources returns S2H/personal URIs when server_id filtered
- [x] **read_resource denied for ACL-visible URI** — `find_server_for_resource` resolves `posthog://skills/audit/all` to inactive parent `posthog-personal` instead of enabled clone `posthog-personal-gait`
- [ ] No Levenshtein hint on typo invoke
- [ ] Tool path regressions vs v1 Runs 1–5

## §2 filter envelope (verbatim)

```json
{"count":16,"next":null,"previous":null,"_posthogUrl":"https://us.posthog.com/project/433907/insights","results":[{"name":"Rewrite feature usage","short_id":"9d4ljh6t"},{"name":"Section resets (quality signal)","short_id":"AYEN0OCK"},{"name":"Reports created vs completed","short_id":"Sxxd3xCD"}],"returned":3,"total":16,"truncated":true}
```

## §4b error (verbatim)

```json
{"error":"invoke_failed","message":"tool 'posthog-personal-gait_projects-gt' is not invokable — did you mean gait_projects-get, gait_project-get, gait_action-get, gait_alert-get, gait_role-get?"}
```

## §7 search + read (verbatim)

Search (first result):

```json
{"server_id":"posthog-personal-gait","uri":"posthog://skills/audit/all","available":true,"name":"Audit an existing PostHog integration for correctness and best practices","description":"Audit an existing PostHog integration for correctness and best practices"}
```

Search total: **91** (limit 10, cursor `"10"`)

Read attempt:

```json
{"error":"disclosure_denied","message":"server 'posthog-personal' is inactive → mcpmux_enable_server({ \"server_id\": \"posthog-personal\" })"}
```

Client resources count remained **0** throughout.

## §0 meta tool roster (14)

`mcpmux_bind_current_workspace`, `mcpmux_create_feature_set`, `mcpmux_disable_server`, `mcpmux_enable_server`, `mcpmux_fetch_prompt`, `mcpmux_get_tool_schema`, `mcpmux_invoke_tool`, `mcpmux_list_all_tools`, `mcpmux_list_feature_sets`, `mcpmux_list_servers`, `mcpmux_read_resource`, `mcpmux_search_prompts`, `mcpmux_search_resources`, `mcpmux_search_tools`

## Friction log

- `read_resource` URI→server resolution ignores clone binding; ACL search returns `posthog-personal-gait` URIs but read hits inactive parent.
- `projects-get` accepts `{}` without required `context` (known backend quirk, out of scope).

## Environment snapshot

- Gateway: `pnpm dev:restart` / commit: **8314525** on **dev**
- mcpmux meta tool count: **14**
- Cursor mux line (tools / prompts / resources): **14 / 0 / 0**

</details>

## Run 2 evidence archive

<details>
<summary>Run 2 — May 26, 2026</summary>

## GAIT Workspace Meta-Gateway Test v2 (Phase D)
Overall: **SHIP**
Workspace: generAIt
Date: May 26, 2026
Run: 2

| Section | Result | Evidence |
|---------|--------|----------|
| §0 Sanity (14/0/0) | **PASS** | 14 meta tools; Cursor mux line: **14 / 0 / 0**; GAIT servers `enabled_via_binding` |
| §1 Jira GAIT | **PASS** | `jsangiorgio@generaitsolutions.com` / `generait1.atlassian.net`; S2H search 0 hits |
| §2 PostHog GAIT | **PASS** | project id **433907**; filter envelope below |
| §3 Search DX (tools) | **PASS** | invokable **331** = search total **331**; `missing: [""]` |
| §4 Fail-closed | **PASS** | `"server 'posthog-personal-gait' is disabled for this session → mcpmux_enable_server(...)"` |
| §4b Levenshtein | **PASS** | same as Run 1 |
| §5 E2E task | **PASS** | Jira GAIT-163/165/160 + PostHog 3 insights |
| §6 Supabase | **SKIP** | inactive in binding |
| §7 Resources (Phase D) | **PASS** | search total **91**; read returns content (see below) |
| §8 Prompts (Phase D) | **SKIP** | no fetchable prompts in ACL |
| §9 Surfaced | **SKIP** | not configured |

## Clone isolation verified?
- [x] Jira GAIT ≠ S2H
- [x] PostHog GAIT (433907) ≠ Personal ≠ S2H
- [x] Resource search scoped to posthog-personal-gait; posthog-work → inactive hint

## Phase D red flags (check any)
- [ ] Still ~124 resources in Cursor mux line
- [ ] Backend tools in tools/list without Surface
- [ ] search_resources returns S2H/personal URIs when server_id filtered
- [ ] read_resource denied for ACL-visible URI
- [ ] No Levenshtein hint on typo invoke
- [ ] Tool path regressions vs v1 Runs 1–5

## §7 read (verbatim — Issue #4 fix)

```json
{"uri":"posthog://skills/audit/all","contents":[{"uri":"posthog://skills/audit/all","mimeType":"text/plain","text":"https://github.com/PostHog/context-mill/releases/download/v1.13.1/audit.zip"}]}
```

## §2 filter envelope (verbatim)

```json
{"count":16,"next":null,"previous":null,"_posthogUrl":"https://us.posthog.com/project/433907/insights","results":[{"name":"Rewrite feature usage","short_id":"9d4ljh6t"},{"name":"Section resets (quality signal)","short_id":"AYEN0OCK"},{"name":"Reports created vs completed","short_id":"Sxxd3xCD"}],"returned":3,"total":16,"truncated":true}
```

## Environment snapshot

- Gateway: `pnpm dev:restart` / commit: **a4a212a** on **dev**
- mcpmux meta tool count: **14**
- Cursor mux line (tools / prompts / resources): **14 / 0 / 0**

</details>
