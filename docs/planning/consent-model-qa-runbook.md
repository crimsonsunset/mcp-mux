# Consent-Model PR — Manual QA Runbook

**Last Updated:** May 30, 2026
**Branch:** `docs/feature-set-consent-model`
**Related:** [`feature-set-consent-model.md`](./feature-set-consent-model.md) · [`search-tools-latency-and-root-race.md`](./search-tools-latency-and-root-race.md) · [`search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md) · [`search-tools-persistent-embedding-cache.md`](./search-tools-persistent-embedding-cache.md)

Full checklist for validating Phases 1–8 of the consent-model PR plus hybrid search ranking (Phases 1–4 of the semantic-ranking doc): discovery of inactive tools, bind layering, removed ephemeral path, human-only authoring, web approval, latency/cache fixes (root-race, inactive scan, active index cache), and hybrid lexical + embedding search. Sections A–G map to consent Phases 1–5; Sections H–J map to latency Phases 6–8; Sections K–N map to hybrid-ranking Phases 1–4; **Section O maps to persistent-embedding-cache Phases 1–5 (Shipped, fe122d8) — run O0b fix verification now (O0 Run 1 archived).**

---

## Testing runway setup (do this once)

### 1. Create a dedicated QA workspace folder

```bash
mkdir -p ~/Desktop/QA/consent-model-qa
```

Open **this folder** in Cursor for all tests below.

**Why not use an existing workspace:**

| Folder | Problem |
| ------ | ------- |
| `mcp-mux/`, `jsg-tech-check/`, `katelaub.com/`, `set-times-app/` | Bound to `All` — nothing is inactive |
| `priv/` | Includes `All` in its binding stack — same problem |
| `.cursor` folder | Bound to `All` |

**Usable existing alternatives** (skip creating a new folder):

| Folder | Current binding | Good inactive targets |
| ------ | --------------- | --------------------- |
| `~/Desktop/Repos/Contracts/MESH` | `bundle:browser` only (52 tools) | `bundle:design`, `bundle:devops-personal` |
| `~/Desktop/Repos/Sync2Hire/sync2hire-platform` | `bundle:s2h` only (878 tools) | `bundle:design`, `bundle:devops-personal`, `bundle:browser` |

### 2. Configure the QA Space in McpMux

In McpMux → **Workspaces**, bind `~/Desktop/QA/consent-model-qa` to:

| Slot | Bundle | UUID | Members | Why |
| ---- | ------ | ---- | ------- | --- |
| **Active (bound)** | `bundle:core` | `15109e39-151e-419c-8281-4db528907e1e` | 63 | Lightweight baseline |
| **First inactive target** | `bundle:design` | `4397fd99-3d6a-41a9-ad07-38cc1b38569c` | 36 | Smallest — fast inactive scan, use for Sections B/C |
| **Second inactive target** | `bundle:devops-personal` | `9034e26f-5430-464c-9599-11e74f7df322` | 29 | For Phase 5 web-approval (needs a distinct UUID) |

> **Do not use `bundle:observability-personal` (494 tools) or `bundle:s2h` (878 tools) for routine `include_inactive` tests** — Phase 7 fixed the hang, but wide scans are still slow on huge bundles. Use `bundle:design` / `bundle:devops-personal` for day-to-day inactive discovery; reserve observability for Section I perf smoke only.
>
> **`bundle:gait` is NOT empty** — 488 members as of May 29 2026. Fine for invoke testing once bound, but too heavy for wide `include_inactive` scans.

### 3. Bundle inventory (verified from DB, May 29 2026)

| Bundle | UUID | Members | Currently bound to |
| ------ | ---- | ------- | ------------------ |
| `bundle:browser` | `382d3067-e608-4183-bf65-894bcc915a6f` | 52 | `MESH`, `generAIt` |
| `bundle:comms-personal` | `1f19d0ff-8073-40bf-b168-67a8db9a5896` | 134 | `generAIt`, `priv` |
| `bundle:core` | `15109e39-151e-419c-8281-4db528907e1e` | 63 | `generAIt`, `priv` |
| `bundle:db-personal` | `7f281c16-dc4e-4897-bf11-13c2a15aabe3` | 78 | `generAIt` |
| `bundle:design` | `4397fd99-3d6a-41a9-ad07-38cc1b38569c` | 36 | **nowhere** ← ideal inactive target |
| `bundle:devops-personal` | `9034e26f-5430-464c-9599-11e74f7df322` | 29 | **nowhere** ← ideal inactive target |
| `bundle:gait` | `51d2ee64-f439-4223-ac50-42b8d2277978` | 488 | `generAIt` |
| `bundle:observability-personal` | `9deb355f-94e7-4d92-9d56-f46ca83e9d1c` | 494 | **nowhere** — too heavy for `include_inactive` without `server_id` |
| `bundle:s2h` | `10ae1e44-2c76-467b-8ddb-b7f04b575c30` | 878 | `sync2hire-platform`, `priv` |
| `All` (built-in) | `fs_default_00000000-0000-0000-0000-000000000001` | 2348 | most personal folders |

### 4. Confirm setup before starting

After creating the QA workspace binding, verify from the QA folder:

```
mcpmux_list_feature_sets({})
```

Expect `bundle:core` as `active`, `bundle:design` and `bundle:devops-personal` as `inactive`.

### 5. Dev environment checklist

- [ ] `pnpm dev:admin` running on `docs/feature-set-consent-model` branch
- [ ] Cursor → MCP → **Reload tools** (descriptor folder must reflect dev binary)
- [ ] Confirm endpoint: `http://localhost:45818/mcp`
- [ ] QA workspace folder open in the Cursor window running the agent
- [ ] Web admin open in browser: `http://127.0.0.1:1420` (for Phase 5 / Section F)
- [ ] **Do not approve bind dialogs** unless the step says to
- [ ] First hybrid-ranking query (Section L) may trigger a ~67 MB embedding model download — allow network once, or SKIP L/M/N if air-gapped

---

## Agent preamble (paste before any test section)

```text
McpMux consent-model QA — setup

- Gateway: http://localhost:45818/mcp via user-mcpmux (reload MCP tools first)
- Branch under test: docs/feature-set-consent-model (dev build via pnpm dev:admin)
- Workspace: ~/Desktop/QA/consent-model-qa — bundle:core active, bundle:design + bundle:devops-personal inactive
- Do NOT approve bind dialogs unless the test step says to
- Report exact tool names, JSON payloads, and error messages verbatim
- For search_tools: always report the ranking field (lexical | hybrid) when present
- Format: PASS / FAIL / SKIP / BLOCKED per step with one-line evidence
```

---

## A. Meta-tool surface (Phase 4 — removed tools absent)

**Prompt:**

```text
List every mcpmux_* tool you can see. Count them.
Report which of these are present or absent:
  mcpmux_bind_current_workspace
  mcpmux_search_tools
  mcpmux_list_feature_sets
  mcpmux_list_servers
  mcpmux_invoke_tool
  mcpmux_get_tool_schema
  mcpmux_search_resources
  mcpmux_read_resource
  mcpmux_search_prompts
  mcpmux_fetch_prompt
  mcpmux_enable_server
  mcpmux_disable_server
  mcpmux_create_feature_set
  mcpmux_list_all_tools
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| ~11 `mcpmux_*` tools total | ✅ | | Exactly 11 |
| `mcpmux_bind_current_workspace` present | ✅ | | |
| `mcpmux_enable_server` **absent** | ✅ | | Phase 3 removal |
| `mcpmux_disable_server` **absent** | ✅ | | Phase 3 removal |
| `mcpmux_create_feature_set` **absent** | ✅ | | Phase 4 removal |
| `mcpmux_list_all_tools` **absent** | ✅ | | Phase 4 removal |
| No backend catalog tools in `tools/list` | ✅ | | lean surface |

Record: 11 tools — `mcpmux_search_prompts`, `mcpmux_invoke_tool`, `mcpmux_fetch_prompt`, `mcpmux_get_tool_schema`, `mcpmux_search_tools`, `mcpmux_diagnose_server`, `mcpmux_list_servers`, `mcpmux_bind_current_workspace`, `mcpmux_list_feature_sets`, `mcpmux_read_resource`, `mcpmux_search_resources`. Note: `mcpmux_diagnose_server` present but not in runbook checklist — not a removed tool, no concern.

---

## B. Phase 1 — Discovery (active default, inactive opt-in)

**Setup:** `bundle:design` (36 tools) is inactive. Do not bind it yet.

> **Query isolation:** Use `"canva"` or `"figma"` — not `"design"`. The word "design" semantically matches active Notion tools in `bundle:core`, which prevents `total: 0` and suppresses the hint. Canva/Figma tools only exist in the inactive design bundle.

**Prompt:**

```text
Search for tools from the design bundle using:

1. mcpmux_search_tools({ "query": "canva", "detail_level": "description" })
   — expect scope: active_only, total: 0, with a hint about include_inactive or list_feature_sets

2. mcpmux_search_tools({ "query": "canva", "include_inactive": true, "detail_level": "description", "limit": 10 })
   — expect inactive rows with a bindable_feature_set_id field

3. mcpmux_list_feature_sets({})
   — expect bundle:core as active, bundle:design + bundle:devops-personal as inactive with UUIDs

4. mcpmux_list_servers({})
   — expect inactive servers include bindable_feature_set_ids array

Paste the JSON for each call.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Default search returns `total: 0` for inactive-only query (`"canva"`) | ✅ | | `total: 0`, `scope: active_only` |
| Default search response includes a hint mentioning `include_inactive` | ✅ | | "Retry with `include_inactive: true` to discover bindable capability, or call `mcpmux_list_feature_sets` then `mcpmux_bind_current_workspace`" |
| `include_inactive: true` returns rows with `bindable_feature_set_id` | ✅ | | 30 canva tools, all `status: inactive`, all `bindable_feature_set_id: 4397fd99-…` |
| `list_feature_sets` shows `status: inactive` for unbound bundles | ✅ | | `bundle:core` active; `bundle:design`, `bundle:devops-personal` inactive |
| `list_servers` shows `bindable_feature_set_ids` on inactive servers | ✅ | | `canva`, `chrome-devtools`, `glips.figma-context-npx`, `mantine` etc. all carry the array |
| No backend tools appear in `tools/list` (count unchanged from Section A) | ✅ | | Still 11 mcpmux_* tools |

Record: all 4 calls re-run on sha 16d5fff — full pass.

---

## C. Phase 2 — Bind layering (needs human approval)

**Setup:** Use `bundle:design` UUID `4397fd99-3d6a-41a9-ad07-38cc1b38569c`. Note current binding FS count (should be 1: `bundle:core`).

**Prompt:**

```text
1. Call mcpmux_bind_current_workspace({ "feature_set_id": "4397fd99-3d6a-41a9-ad07-38cc1b38569c" })
   STOP and tell me when an approval dialog appears — do not proceed until I say approve.

2. After I approve: confirm the response. Note whether feature sets were replaced or appended
   (expect appended — bundle:core should still be in the binding alongside bundle:design).

3. Call mcpmux_bind_current_workspace({ "feature_set_id": "4397fd99-3d6a-41a9-ad07-38cc1b38569c" }) again.
   Expect: already_bound: true (no duplicate entry).

4. Now call mcpmux_search_tools({ "query": "design" }) WITHOUT include_inactive.
   Expect: previously inactive design tools now match as active/invokable.

Paste the JSON for each call.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Bind triggers approval dialog (Tauri and/or browser) | ✅ | | Dialog appeared, approved with "Allow once" |
| After approval: response confirms success | ✅ | | `ok: true`, `already_bound: false` |
| Prior binding FS IDs still present (append, not replace) | ✅ | | `feature_set_ids: [bundle:core, bundle:design]` — both present |
| Second bind same UUID → `already_bound: true` | ✅ | | No dialog — returned `already_bound: true` immediately; dedup check fires pre-approval |
| Default search now finds the previously inactive tools | ✅ | | 30 canva tools, `scope: active_only`, `available: true` |

Record: first bind `feature_set_ids: ["15109e39-…core", "4397fd99-…design"]`, FS count 1→2. Second bind short-circuited correctly with `already_bound: true`, no consent prompt raised.

---

## D. Phase 3 — Ephemeral path removed

**Prompt:**

```text
1. Try to call mcpmux_enable_server — it should not exist. If your client lets you attempt it, report the exact error.

2. Pick a tool from a server that is still inactive in this space (use a DIFFERENT server than what you bound in Section C).
   Call it directly — not via mcpmux_invoke_tool.
   Expect: an error mentioning mcpmux_bind_current_workspace, NOT mcpmux_enable_server.

Paste the exact error strings verbatim.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `mcpmux_enable_server` does not exist / call fails | ✅ | | `Tool not found` — not in surface |
| Direct call on inactive tool errors with `bind_feature_set` hint | ✅ | | `"server 'wakatime' is inactive → mcpmux_bind_current_workspace with a FeatureSet that includes this server"` |
| Error message points to `mcpmux_bind_current_workspace`, not `enable_server` | ✅ | | `enable_server` not mentioned anywhere in the error |

Record: step 2 tested via `mcpmux_invoke_tool` on `wakatime` (inactive, no bound feature set) — error verbatim: `"server 'wakatime' is inactive → mcpmux_bind_current_workspace with a FeatureSet that includes this server"`

---

## E. Phase 4 — Human-only authoring

**Prompt:**

```text
1. Confirm mcpmux_create_feature_set is absent from your tool list (from Section A).

2. Run mcpmux_search_tools({ "query": "<a tool you know is installed>", "include_inactive": true })
   for a query where the tool exists but NO FeatureSet covers it (if such a server exists in this Space).
   Expect: a hint asking the user to create a bundle in McpMux UI (Workspaces → Feature Sets), then bind.

If no uncovered tool exists in this Space, SKIP with reason.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `mcpmux_create_feature_set` absent | ✅ | | Confirmed in Section A |
| Uncovered-tool hint points to McpMux UI | ✅ | | `"Matching tools exist in this Space but no FeatureSet contains them. Ask the user to create a bundle in the McpMux desktop or web UI (Workspaces → Feature Sets), then mcpmux_bind_current_workspace with the new feature_set_id."` |

Record: query `"cloudflare"` — server installed but no FeatureSet covers it. Hint correctly directs to UI bundle creation, not agent-side tool.

---

## F. Phase 5 — Web approval (human step)

**Setup:**
- Browser open at `http://127.0.0.1:1420` (McpMux web admin HMR)
- Have `bundle:devops-personal` UUID `9034e26f-5430-464c-9599-11e74f7df322` ready (not yet bound)
- Tauri window visible but DO NOT approve in it — approve in browser only

**Prompt:**

```text
Call mcpmux_bind_current_workspace({ "feature_set_id": "9034e26f-5430-464c-9599-11e74f7df322" }).
STOP immediately and do not proceed — wait for me to confirm where the dialog appears.
```

After dialog appears, report location (Tauri / browser / both / neither), then approve in browser only.

**Prompt (after browser approval):**

```text
Confirm:
1. List feature sets — is the newly bound FS now active?
2. Search for tools from that bundle without include_inactive — do they appear?
```

**Prompt (deny test):**

```text
Call mcpmux_bind_current_workspace({ "feature_set_id": "9034e26f-5430-464c-9599-11e74f7df322" }) again.
Wait — I will deny in the browser.
Confirm: binding unchanged after deny, already_bound still false (or appropriate state).
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Approval dialog appears in browser (SSE render) | ✅ | | Appeared in both Tauri and browser |
| Approve in browser → binding written | ✅ | | `bundle:devops-personal` active; `feature_set_ids` has all 3 bundles |
| No double-dialog sync issue | ✅ | | Approving in browser auto-dismissed Tauri dialog (post-fix) |
| Deny in browser → binding unchanged | ✅ | | `bundle:browser` not written; Tauri auto-dismissed on deny too |

Record: post-fix retest — approve and deny both correctly sync across Tauri and browser. Deny test used `bundle:browser` (fresh unbound bundle) to avoid `already_bound` short-circuit.

---

## G. Invoke path still works after bind

**Prompt:**

```text
From the bundle you bound in Section C:

1. mcpmux_search_tools({ "query": "<a read-only tool in that bundle>", "detail_level": "description" })
2. mcpmux_get_tool_schema({ "tools": ["<tool_name>"] })
3. mcpmux_invoke_tool with safe read-only args from the schema

Expect: invoke succeeds or fails for an auth/server reason — NOT a bind or inactive reason.
Paste the invoke result summary.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Search finds tools in newly bound bundle | ✅ | | 30 canva tools, `scope: active_only` |
| Schema loaded before invoke | ✅ | | `canva_list-folder-items` schema retrieved cleanly |
| Invoke result is not a bind/inactive error | ✅ | | Full successful response — 26 items returned from Canva root folder |

Record: `canva_list-folder-items` with `folder_id: "root"` — full data response, no auth or bind errors.

---

## G2. Hybrid search ranking (post-ship QA)

**Context:** Phases 1–4 of [`search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md) shipped on this branch. Replaces contiguous-substring gate with token-overlap + semantic rerank (`fastembed`, BGESmallENV15). Run after Section G; requires model to be downloaded (first call may be lexical-only).

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Token-overlap fix: `"list folder"` returns results | ✅ | | `canva_list-folder-items` ranks #1, `total: 31`, `ranking: lexical` |
| `ranking` field present in response | ✅ | | Both `"lexical"` and `"hybrid"` observed correctly |
| Exact-name precision: `"canva_list-folder-items"` ranks #1 | ✅ | | `total: 51`, `ranking: hybrid` |
| Wide `include_inactive` no hang | ✅ | | `total: 57`, `ranking: hybrid`, fast — download-blocking bug fixed |
| Intent: `"post a comment on a jira issue"` → Jira comment tool in top 3 | ✅ | | `atlassian_addCommentToJiraIssue` ranks #3, `ranking: hybrid`, `server_id: com.atlassian-mcp` |

Record: all smokes re-run post fix — clean pass. Model download no longer blocks inactive scan; lexical fallback works correctly during download.

---

## H. Phase 6 — Root-race fix

**Setup:** QA workspace with `bundle:core` active. **Fresh session required** — new Cursor chat or MCP disconnect/reconnect. Do not call `tools/list` or any other `mcpmux_*` tool first.

**Prompt:**

```text
This must be the FIRST tool call in this session — do not call tools/list or list_feature_sets first.

1. mcpmux_search_tools({ "query": "core" })
   Expect: scope: "active_only", total > 0, tools from bundle:core returned.

2. mcpmux_search_tools({ "query": "zznotreal" })
   Expect: total: 0, but the hint should mention include_inactive or list_feature_sets —
   NOT a PendingRoots/empty-binding message.

Paste both responses verbatim.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| First call returns `total > 0` with active tools | ✅ | | Test query `"core"` has no tool-name/description matches → `total: 0`, but root-race fix confirmed via `search_tools("canva")` → 30 active tools on first effective call to this gateway instance (no prior `tools/list` warmup). Fix is working. |
| No-match query returns hint (not silent 0 / binding-missing) | ✅ | | `"zznotreal"` returns `"No active tools matched. Retry with include_inactive: true…"` — correct Phase 6 hint, NOT a PendingRoots/empty-binding message |
| `scope: "active_only"` in both responses | ✅ | | Present in all calls |

Record: `search_tools("core")` → `{total: 0, scope: "active_only", ranking: "lexical", hint: "No active tools matched. Retry with include_inactive: true to discover bindable capability, or call mcpmux_list_feature_sets then mcpmux_bind_current_workspace with a feature_set_id."}`. `search_tools("zznotreal")` → same shape. `search_tools("canva")` (3rd call, first effective match) → `{total: 30, scope: "active_only", ranking: "lexical"}` — no lag, roots already probed. Note: runbook test query `"core"` should be changed to `"canva"` or similar for a more reliable first-call check.

---

## I. Phase 7 — Inactive scan perf

**Setup:** Temporarily add `bundle:observability-personal` (`9deb355f-94e7-4d92-9d56-f46ca83e9d1c`, 494 tools) as an inactive bundle in the QA space. **Do not bind it** — it should remain inactive so it shows up in the inactive scan.

> This is the bundle the runbook previously warned against using. Phase 7 fixed the hang, so it's now the right tool for the perf smoke test.

**Prompt:**

```text
Time the following calls (note wall-clock or "felt fast/slow"):

1. mcpmux_search_tools({ "include_inactive": true, "limit": 100 })
   Expect: completes in < 2 s, scope: "active_and_inactive", large total.
   If total > 50 and no server_id filter: expect hint "Narrow with `server_id` for faster results."

2. mcpmux_list_servers({})
   — get the server_id for one of the observability servers

3. mcpmux_search_tools({ "include_inactive": true, "server_id": "<observability-server-id>", "limit": 50 })
   Expect: fast, scoped result, no hint or smaller set.

Paste responses and note timing for call 1.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Wide `include_inactive` scan completes < 2 s | ✅ | | Returned instantly (`total: 1804`) — no hang |
| `total` reflects large inactive set | ✅ | | `total: 1804`, `scope: active_and_inactive` |
| Hint present when `total > 50` and no `server_id` | ✅ | | `"Narrow with \`server_id\` for faster results."` |
| `server_id`-filtered call returns scoped results | ✅ | | `server_id: "posthog-personal"` → `total: 337`, no hint, fast |

Record: call 1 felt instant (sub-second). `total: 1804`, hint: `"Narrow with \`server_id\` for faster results."`. `bundle:observability-personal` (`9deb355f-…`) already present in this Space as inactive — no manual setup required. Filtered call: `server_id: "posthog-personal"` → `{total: 337, scope: "active_and_inactive", ranking: "lexical"}`.

After this section, **remove `bundle:observability-personal`** from the QA space binding if you added it only for this test.

---

## J. Phase 8 — Per-session active index cache

**Setup:** QA workspace with `bundle:core` active (back to normal after Section I cleanup). Warm session (already called `search_tools` at least once this session is fine).

### J1 — Cache hit (repeat calls)

**Prompt:**

```text
Call mcpmux_search_tools({ "query": "core" }) five times in a row with identical args.
Note whether calls 2–5 feel noticeably faster than call 1.
Then try a different query: mcpmux_search_tools({ "query": "file" }).
Expect calls 2–5 to be fast (cached active index); different query still uses cache.

Paste all six responses and note any latency difference.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Calls 2–5 return consistent results (same active index) | ✅ | | Identical top-5, `total: 30`, `ranking: lexical` across all 4 repeat calls |
| Different query on call 6 still returns active tools | ✅ | | `search_tools("file")` → `{total: 2, scope: "active_only"}` — cache key is index not query |

Note: used query `"canva"` (not `"core"` — core has no tool matches). All calls felt instant, no latency difference visible between call 1 and calls 2–5.

### J2 — Cache eviction on rebind

**Prompt:**

```text
1. mcpmux_search_tools({ "query": "core" }) — warm the cache.
2. mcpmux_bind_current_workspace({ "feature_set_id": "4397fd99-3d6a-41a9-ad07-38cc1b38569c" })
   (bundle:design — approve when prompted)
3. mcpmux_search_tools({ "query": "design" }) — WITHOUT include_inactive.
   Expect: design tools now appear as active (cache was evicted and rebuilt with new binding).

Paste responses for steps 1 and 3.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Post-bind search returns tools from newly bound bundle | ✅ | | `search_tools("browser")` → `{total: 25, scope: "active_only"}` — Playwright/browser tools immediately active post-bind |
| Prior bundle tools still present (layering intact) | ✅ | | Canva tools still returned by prior searches; `feature_set_ids` shows all 4 bundles |

Note: used `bundle:browser` (`382d3067-…`) as bind target — `bundle:design` was already bound from earlier in the session. Bind returned `{ok: true, already_bound: false, feature_set_ids: [core, design, devops-personal, browser]}`. Approved via dialog.

### J3 — Cache eviction on disconnect

**Prompt:**

```text
1. mcpmux_search_tools({ "query": "core" }) — warm the cache.
2. I will now disable and re-enable McpMux in Cursor MCP settings (simulates session disconnect).
   Tell me when you're ready and I'll do it, then reconnect.
3. mcpmux_search_tools({ "query": "core" }) — after reconnect.
   Expect: works correctly; first call after reconnect may be slightly slower (cold cache).

Paste the post-reconnect response.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Post-reconnect search returns correct active tools | ✅ | | `search_tools("canva")` → `{total: 30, scope: "active_only", ranking: "lexical"}` — identical to pre-disconnect |
| No stale data from previous session | ✅ | | Same top-5 order, no phantom tools, correct binding reflected |

Record: pre/post-rebind (J2) — pre: `total: 30` (canva, active_only); post-bind: `total: 25` (browser tools, active_only, scope unchanged). J3 post-reconnect: `{total: 30, scope: "active_only", ranking: "lexical"}` — clean. `tool_embeddings` remained at **0 rows** throughout all of H/I/J — warmer running but nothing persisting to SQLite (O0 bug already manifesting pre-cold-restart).

---

## K. Hybrid Phase 1 — Lexical token-overlap

**Setup:** Complete Section C first (`bundle:design` bound — Canva tools active). This tests the fix for multi-word queries against hyphenated tool names.

**Prompt:**

```text
1. mcpmux_search_tools({ "query": "list folder", "detail_level": "description" })
   Expect: total > 0, canva_list-folder-items (or similar Canva list/folder tool) in results.
   Before the fix this returned total: 0 because "list folder" did not match list-folder-items as a contiguous substring.

2. mcpmux_search_tools({ "query": "zznotrealxyz" })
   Expect: total: 0, scope: active_only, ranking field present (see below).

Paste both responses verbatim. Note the ranking field value on each.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `"list folder"` returns Canva folder/list tools | ☐ | ☐ | token-overlap fix |
| Zero-match query still returns `total: 0` | ☐ | ☐ | |
| Payload includes `ranking` (`"lexical"` or `"hybrid"`) | ☐ | ☐ | new field |

Record: top `qualified_name` from call 1, `ranking` value on both calls.

---

## L. Hybrid Phase 2 — Embedding model lifecycle

**Setup:** Fresh dev gateway (`pnpm dev:admin`). First hybrid query may download BGE-small (~67 MB) to app data under `{data_dir}/embeddings`. Watch gateway logs for `[embed]` state transitions.

**Prompt:**

```text
1. mcpmux_search_tools({ "query": "folder" })
   Note: ranking field on first call — may be "lexical" if model still downloading.

2. Wait ~30 s if needed, then repeat:
   mcpmux_search_tools({ "query": "folder" })
   Expect: ranking may become "hybrid" once model is Ready.

3. Report whether call 1 felt slower (cold) vs call 2 (warm index; embedding cache may still be cold on first hybrid query).

Paste both responses. Note ranking values and any download delay.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Search never hard-fails while model downloads | ☐ | ☐ | lexical fallback |
| First call returns results (`total > 0` or valid zero with hint) | ☐ | ☐ | |
| `ranking: "lexical"` acceptable while model not Ready | ☐ | ☐ | |
| Second call works after download window | ☐ | ☐ | may show `ranking: "hybrid"` |

**Optional (air-gapped / no download):** Rename or move `{data_dir}/embeddings` aside, restart gateway, confirm search still returns results with `ranking: "lexical"`. Restore folder after.

Record: ranking on call 1 vs call 2, download observed (yes/no), gateway `[embed]` log snippet if available.

---

## M. Hybrid Phase 3 — Hybrid fusion + embedding cache

**Setup:** Model Ready from Section L (`ranking: "hybrid"` observed at least once). QA workspace with `bundle:core` + `bundle:design` bound.

**Prompt:**

```text
1. mcpmux_search_tools({ "query": "list folder" })
   Expect: ranking: "hybrid" (if model Ready), Canva tools in top results.

2. Call the same query five times with identical args.
   Expect: consistent results; calls 2–5 should not regress ranking or drop tools.

3. mcpmux_search_tools({ "query": "canva_list-folder-items" })
   Expect: literal tool name ranks first or near-first (lexical precision preserved in fusion).

Paste responses for calls 1, 2, 5, and 3.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `ranking: "hybrid"` when model Ready | ☐ | ☐ | |
| Repeat queries return consistent tool set | ☐ | ☐ | embedding + index cache |
| Exact qualified_name query ranks target tool highly | ☐ | ☐ | lexical weight in fusion |

Record: `ranking`, top 3 `qualified_name` values from call 3.

---

## N. Hybrid Phase 4 — Intent relevance smoke

**Setup:** Requires a workspace with Jira/Atlassian tools in the **active** binding. Options:

| Workspace | Binding | Use for |
| --------- | ------- | ------- |
| `generAIt` (or folder with `bundle:gait`) | includes Atlassian | intent query below |
| QA folder only | core + design | SKIP intent step; run exact-name step only |

**Prompt (Atlassian binding):**

```text
1. mcpmux_search_tools({ "query": "post a jira comment", "detail_level": "description", "limit": 10 })
   Expect: a comment/issue-creation Jira tool in top 3 (e.g. create_issue_comment or similar).
   ranking: "hybrid" if model Ready.

2. mcpmux_search_tools({ "query": "canva_list-folder-items", "limit": 5 })
   Expect: canva_list-folder-items is #1 (exact lexical precision).

Paste both responses with top 5 qualified_name list.
```

**Prompt (QA folder only — SKIP step 1):**

```text
mcpmux_search_tools({ "query": "canva_list-folder-items", "limit": 5 })
Expect: canva_list-folder-items ranks first among Canva tools.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Intent query surfaces semantically related tool in top 3 | ☐ | ☐ | SKIP if no Jira binding |
| Exact tool name ranks first | ☐ | ☐ | |
| `include_inactive: true` results NOT semantically reranked | ☐ | ☐ | optional: confirm inactive rows lack hybrid boost vs active |

**Optional trace (developer):** Run gateway with `RUST_LOG=mcpmux_gateway=debug`, one `search_tools` call, grep logs for `query_id` — confirm entry → cache → embed → lexical → fusion → summary chain. Raw query text must not appear above `debug`.

Record: top 3 from intent query (or SKIP reason), exact-name rank, `ranking` values.

---

## O. Persistent embedding cache (Shipped — [`search-tools-persistent-embedding-cache.md`](./search-tools-persistent-embedding-cache.md))

**Status:** Phases 1–5 shipped on this branch (fe122d8+). **Run [O0b](#o0b--fix-verification-re-run-run-this-now)** now (fix verification after `run_spawn_blocking` change). O0 Run 1 is archived (bug confirmed May 30). O0b gates O1–O4.

**Why this needs the gateway logs:** unlike hybrid ranking, the win here is *no recomputation* — and the tool payload looks identical whether vectors were embedded fresh or loaded from the store. The in-band signals are **latency** (the ~30 s cold embed should disappear) and **`ranking`**; the authoritative signal is the `[embed]` / `[search]` log targets + the SQLite row count. Run the gateway with `RUST_LOG=mcpmux_gateway=debug` and watch:

- macOS log path: `~/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.YYYY-MM-DD.log`
- DB path: `~/Library/Application Support/com.mcpmux.desktop/mcpmux.db` (table `tool_embeddings`)

**Shipped log vocabulary (use these exact strings — older drafts of this section said `cached = false`, which the shipped code does not emit):**

| Log line | Meaning |
| -------- | ------- |
| `[embed] model = … state = Ready` | ONNX model loaded; embedding is possible. Until this, every embed returns nothing and ranking is `lexical` (benign). |
| `[embed] warm enqueue … catalog_tools=N missing=M` | Warmer fired for a server; `M` tools are absent from the store and *should* get embedded. |
| `[embed] warm batch done … embedded=X skipped_present=Y embed_ms=…` | Warmer finished. **`embedded` is the money field** — it must be `> 0` when `missing > 0` and model is `Ready`. |
| `[embed] warmer upserting records` | Vectors are being written to SQLite. Should appear whenever `embedded > 0`. |
| `[embed] store hydrate … store_hits=… store_misses=…` | Search loaded vectors from the store into memory. |
| `[search] cache decision … embedding_store=hit\|miss` | Whether the active corpus had vectors at search time. |
| `[search] read … vectors_present=… lexical_only_docs=…` | Per-search vector coverage. |
| `[search] result summary … ranking=hybrid\|lexical total_ms=…` | Final ranking + latency. |
| `[embed] spawn_blocking panicked` + `panic = "…"` | ONNX/runtime panic inside `spawn_blocking` (logged after May 30 fix; was previously swallowed). |
| `[embed] spawn_blocking cancelled` / `join failed` | Task cancelled or join error (not a panic). |
| `[embed] diag: …` | Temporary warn scaffolding on inner failure paths (mutex / empty slot / `fastembed embed() failed`). |

> **Note on `ranking`:** during the connect-warm window, tools not yet in the store rank lexical-only. A `ranking: "lexical"` result on a Ready model is acceptable *transiently* — it must flip to `"hybrid"` once the warmer finishes for that binding. A Ready model that **stays** `lexical` after the warmer reported done is the bug O0 hunts.

### O0 — Warmer diagnostic (Run 1 — archived May 30, 2026)

> **Historical only.** Run 1 confirmed the bug. **Active work is [O0b](#o0b--fix-verification-re-run-run-this-now)** after the `run_spawn_blocking` fix lands in the dev binary.

**Goal:** Reproduce and capture the suspected failure: the warmer logs `warm enqueue … missing=M` (M large) but then `warm batch done … embedded=0`, no `warmer upserting records` line ever appears, and `tool_embeddings` stays empty — so every search falls back to `lexical` even with a Ready model. This phase is pure evidence-gathering; it has no "make it pretty" expectation.

**Setup:**

1. Human: fully restart the gateway so warming runs from cold — `pnpm dev:stop` → `pnpm dev:admin` on `docs/feature-set-consent-model`. Reload MCP tools in Cursor afterward.
2. Human (baseline, before any search): record starting row count and that the table exists —
   ```bash
   sqlite3 ~/Library/Application\ Support/com.mcpmux.desktop/mcpmux.db \
     "SELECT COUNT(*) AS rows, COALESCE(model_version,'-') FROM tool_embeddings GROUP BY model_version;"
   ```
   Expect either no rows (cold) or a known count. Note it — this is the before number.
3. Set `LOG="$HOME/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log"` for the greps below.

**Step 1 — Did the model ever become Ready?**

```bash
grep '\[embed\] model' "$LOG" | tail -5
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| A `state = Ready` line exists after the last restart | ✅ | | `2026-05-30T22:20:29.273438Z INFO ... [embed] model = bge-small-en-v1.5, state = Ready, download_ms = 132` |

**Step 2 — Did the warmer run, and what did it embed?**

```bash
grep '\[embed\] warm' "$LOG" | tail -60
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `warm enqueue` lines appear with `missing > 0` | ✅ | | Every server fires enqueue with `missing = catalog_tools` (entire corpus missing from store) |
| At least one `warm batch done` shows `embedded > 0` | | ❌ | **BUG** — every server: `embedded=0 skipped_present=0` despite `model_state=Ready` and `missing > 0` |
| `[embed] warmer upserting records` appears at least once | | ❌ | **0 occurrences** in entire log — nothing ever written to SQLite |
| Each server's `warm enqueue` fires at most ~2× | ✅ | | Each server enqueues exactly twice (~1 s apart) — both `Connected` and `ServerFeaturesRefreshed` trigger |

Record verbatim — representative `warm enqueue` + `warm batch done` pairs:

```
2026-05-30T22:20:56.452606Z DEBUG [embed] warm enqueue server_id="firebase-dev" catalog_tools=52 missing=52
2026-05-30T22:20:56.479127Z  INFO [embed] warm batch done server_id="firebase-dev" embedded=0 skipped_present=0 missing=52 embed_ms=5 model_version="bge-small-en-v1.5" model_state=Ready

2026-05-30T22:20:58.814427Z DEBUG [embed] warm enqueue server_id="openrouter" catalog_tools=4 missing=4
2026-05-30T22:20:58.836827Z  INFO [embed] warm batch done server_id="openrouter" embedded=0 skipped_present=0 missing=4 embed_ms=0 model_version="bge-small-en-v1.5" model_state=Ready

2026-05-30T22:21:03.755340Z DEBUG [embed] warm enqueue server_id="posthog-work" catalog_tools=351 missing=351
2026-05-30T22:21:03.809422Z  INFO [embed] warm batch done server_id="posthog-work" embedded=0 skipped_present=0 missing=351 embed_ms=32 model_version="bge-small-en-v1.5" model_state=Ready

2026-05-30T22:21:06.221258Z DEBUG [embed] warm enqueue server_id="posthog-personal" catalog_tools=351 missing=351
2026-05-30T22:21:06.275627Z  INFO [embed] warm batch done server_id="posthog-personal" embedded=0 skipped_present=0 missing=351 embed_ms=32 model_version="bge-small-en-v1.5" model_state=Ready

2026-05-30T22:22:16.950237Z DEBUG [embed] warm enqueue server_id="com.atlassian-mcp" catalog_tools=37 missing=37
2026-05-30T22:22:16.975090Z  INFO [embed] warm batch done server_id="com.atlassian-mcp" embedded=0 skipped_present=0 missing=37 embed_ms=3 model_version="bge-small-en-v1.5" model_state=Ready

2026-05-30T22:22:24.672050Z DEBUG [embed] warm enqueue server_id="github" catalog_tools=41 missing=41
2026-05-30T22:22:24.697540Z  INFO [embed] warm batch done server_id="github" embedded=0 skipped_present=0 missing=41 embed_ms=4 model_version="bge-small-en-v1.5" model_state=Ready

2026-05-30T22:22:27.196102Z DEBUG [embed] warm enqueue server_id="community.playwright-npx" catalog_tools=23 missing=23
2026-05-30T22:22:27.220258Z  INFO [embed] warm batch done server_id="community.playwright-npx" embedded=0 skipped_present=0 missing=23 embed_ms=2 model_version="bge-small-en-v1.5" model_state=Ready
```

Note: `embed_ms > 0` for all large servers (32 ms for 351-tool posthog servers, 4–5 ms for 40–52 tool servers) — warmer is spending time but producing 0 embeddings. Root cause unknown.

**Step 3 — Did anything land in SQLite?**

```bash
sqlite3 ~/Library/Application\ Support/com.mcpmux.desktop/mcpmux.db \
  "SELECT COUNT(*) FROM tool_embeddings;"
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Row count increased vs the baseline from Setup #2 | | ❌ | **0 rows** before restart, **0 rows** after full warm cycle — persistence never happened |

**Step 4 — Walk one search end-to-end.**

Prompt:

```text
mcpmux_search_tools({ "query": "list folder" })
Report ranking, total, top qualified_name, and felt latency.
```

Then:

```bash
grep '\[search\]\|\[embed\] store hydrate\|\[embed\] inline query' "$LOG" | tail -25
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `[search] result summary` shows `ranking=hybrid` (model Ready) | | ❌ | `ranking="lexical"` — stuck permanently; model is Ready but store is empty |
| `[search] cache decision … embedding_store=hit` | | ❌ | `embedding_store="miss"` — no vectors for active corpus |
| `store hydrate … store_hits > 0` | | ❌ | `store_hits=0 store_misses=175` — store completely empty |

Step 4 log verbatim:

```
[embed] store hydrate query_id="22b960b1" hashes_requested=175 store_hits=0 store_misses=175
[search] read query_id="22b960b1" active_tools=175 vectors_present=0 lexical_only_docs=175
[search] cache decision query_id="22b960b1" index_cache="miss" embedding_store="miss" active_tools=175
[search] result summary query_id=22b960b1 ranking="lexical" total=37 returned=20 top_qualified_name="canva_list-folder-items" total_ms=988
```

Tool result: `{ranking: "lexical", total: 37, top: "canva_list-folder-items"}` — lexical is working but no hybrid ranking.

**Verdict: 🐛 BUG CONFIRMED** (May 30, 2026)

All conditions met:
- ✅ Model `state = Ready` (bge-small-en-v1.5, download_ms=132)
- ✅ `warm enqueue missing > 0` (every server, repeatedly)
- ❌ ALL `warm batch done embedded=0` (every server, no exceptions)
- ❌ `warmer upserting records` — 0 occurrences in entire log
- ❌ `tool_embeddings` — 0 rows before and after full warm cycle
- ❌ Search stuck `ranking=lexical` with `embedding_store=miss`, `vectors_present=0`

**Filed as:** *"On-connect warmer enqueues but never embeds/persists — `embedded=0` despite `missing>0` on a Ready model; `tool_embeddings` empty; search degraded to lexical permanently."*

**O1–O4: BLOCKED on O0.** All four sections share this root cause — the store never populates so cross-session reuse (O1), restart persistence (O2), alias-rename free (O3), and on-connect warm (O4) cannot be validated.

- **If model never Ready** (Step 1 fail): file *"Embedding model never reaches Ready ([embed] state stuck Downloading/Failed)"* with the Step 1 lines; O1–O5 are SKIP (blocked on model).
- **WORKS** if `embedded>0` + `warmer upserting` + row count climbs + search `hybrid`: O0 passes, continue to O1–O4 as normal regression checks.

Record: before/after `tool_embeddings` counts, model state line, the warm log block, the search summary + hydrate lines, and the verdict.

#### Diagnostic instrumentation findings (root cause narrowed — May 30, 2026)

Before this cold-restart run, temporary `[embed] diag:` warn-level instrumentation was added to the three silently-swallowed failure points in the embed path (`crates/mcpmux-gateway/src/services/embedding.rs` + `embedding_warmer.rs`). Grep them with:

```bash
grep '\[embed\] diag:' "$LOG"
```

Observed chain (fires once per missing tool, e.g. `firebase-local`, `model_state=Ready`):

```
WARN [embed] diag: embed_with_spawn_blocking returned None (spawn_blocking join or inner failure)
WARN [embed] diag: state=Ready but inference produced no vectors docs_embedded=1 embed_ms=0
WARN [embed] diag: warmer embed_documents returned None — skipping tool server_id="firebase-local" model_state=Ready
```

**The discriminating signal is what did NOT fire.** The closure inside `embed_with_spawn_blocking` logs every error branch it can hit:

- `[embed] diag: model mutex poisoned` — **never fired** ⇒ lock acquired fine
- `[embed] diag: model slot empty despite Ready state` — **never fired** ⇒ model `Some(_)` present
- `[embed] diag: fastembed embed() failed` — **never fired** ⇒ `embed()` did not return `Err`

Yet the **outer** `embed_with_spawn_blocking returned None` always fires. The only way to reach the outer `None` without any inner branch logging is if the closure **never returned at all** — i.e. it **panicked**, and the panic was caught as a `JoinError` by `tokio::task::spawn_blocking` and then dropped by `.ok().flatten()` in `run_spawn_blocking` (`embedding.rs` ~line 342).

**Root-cause hypothesis:** a panic inside the `spawn_blocking` task (most likely the `fastembed` `embedding.embed()` ONNX call panicking rather than returning `Err`, or a nested-runtime issue in `run_spawn_blocking`'s `std::thread::spawn` → `handle.block_on(spawn_blocking(...))` shape), swallowed by `.ok()`. This matches the symptom: `embed_ms` is non-zero (32 ms for 351 tools) because the loop *iterates* all docs, but each `embed_documents` returns `None` instantly so `records` stays empty → `embedded=0`, no `upserting`, empty SQLite.

**Fix landed (uncommitted, `embedding.rs`):** `run_spawn_blocking` no longer uses `std::thread::spawn` → `block_on` → `.ok().flatten()`. It now uses `tokio::task::block_in_place` + explicit `JoinError` logging (`[embed] spawn_blocking panicked` with `panic = "…"`). Re-run via **O0b** on a **rebuilt** dev binary — `pnpm dev` alone is not enough.

### O0b — Fix verification re-run (RUN THIS NOW)

**Goal:** Confirm the fix on a cold gateway, or capture the panic message / remaining failure mode if still broken.

**Prerequisites (human — do before the agent runs greps or `search_tools`):**

1. In `mcp-mux` repo on `docs/feature-set-consent-model`: **`pnpm dev:stop` → `pnpm dev:rebuild` → `pnpm dev:admin`** (Rust changed; rebuild is mandatory).
2. Reload MCP tools in Cursor.
3. Do **not** call `search_tools` until Step 4 — let the connect-warm window finish first (~30–90 s after gateway up while servers connect).

**Agent preamble (paste for O0b only):**

```text
McpMux O0b — embedding warmer fix verification

- Read: docs/planning/consent-model-qa-runbook.md Section O0b (this run supersedes archived O0 Run 1)
- Gateway must be on a binary rebuilt AFTER the run_spawn_blocking fix (human did dev:rebuild)
- Workspace: ~/Desktop/QA/consent-model-qa — bundle:core active
- Do NOT call search_tools until Step 4 says to
- You may run shell greps and sqlite3 against local paths below
- Format: PASS / FAIL per check; paste verbatim log lines
```

**Setup (agent or human):**

```bash
LOG="$HOME/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log"
sqlite3 ~/Library/Application\ Support/com.mcpmux.desktop/mcpmux.db \
  "SELECT COUNT(*) FROM tool_embeddings;"
```

Record **before** count (expect 0 on cold start).

**Step 1 — Model Ready after this restart**

```bash
grep '\[embed\] model' "$LOG" | tail -3
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `state = Ready` after latest restart timestamp | ✅ | | `2026-05-30T22:40:55.461613Z ... state = Ready, download_ms = 130` (restart at 22:40) |

**Step 2 — Warmer + failure mode (primary)**

Wait until server connect storm settles (~60 s after gateway up), then:

```bash
grep '\[embed\] warm batch done\|\[embed\] warmer upserting\|\[embed\] spawn_blocking' "$LOG" | tail -40
grep '\[embed\] diag:' "$LOG" | tail -15
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| At least one `warm batch done` with **`embedded > 0`** | ✅ | | 27 servers, all with `embedded > 0` (e.g. posthog-personal-gait=351, firebase-prod=52, canva=30, github=41, community.playwright-npx=23, com.apify-mcp-http=11…) |
| At least one **`warmer upserting records`** | ✅ | | 27 occurrences — one per server, embedded counts match warm batch done values |
| **No** flood of `spawn_blocking panicked` (or paste panic text if present) | ✅ | | 0 `spawn_blocking panicked` lines in this session. Pre-fix diags at 22:22 are from archived O0 Run 1. Only benign: one `model_state=Downloading` skip for cloudflare at gateway start. |
| If still `embedded=0`: `fastembed embed() failed` or other `[embed] diag:` explains why | N/A | | Warmer write path is fixed — `embedded > 0` on all servers |

Representative warm lines (O0b session, 22:40–22:43):
```
[embed] warmer upserting records server_id="posthog-personal-gait" embedded=351
[embed] warm batch done server_id="posthog-personal-gait" embedded=351 skipped_present=0 embed_ms=4975
[embed] warmer upserting records server_id="canva" embedded=30
[embed] warm batch done server_id="canva" embedded=30 skipped_present=0 embed_ms=612
[embed] warmer upserting records server_id="github" embedded=41
[embed] warm batch done server_id="github" embedded=41 skipped_present=0 embed_ms=578
[embed] warmer upserting records server_id="com.apify-mcp-http" embedded=11
[embed] warm batch done server_id="com.apify-mcp-http" embedded=11 skipped_present=0 embed_ms=924
```
Second-trigger runs (each server fires twice): show `embedded=0 skipped_present=N model_state=NotDownloaded` — correct, tools already in store.

**Step 3 — SQLite row count**

```bash
sqlite3 ~/Library/Application\ Support/com.mcpmux.desktop/mcpmux.db \
  "SELECT COUNT(*) FROM tool_embeddings;"
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Row count **> 0** vs Step 0 baseline | ✅ | | Before: 0 rows → After: **945 rows** (`bge-small-en-v1.5`). Sum of all 27 server embeds = 945 exactly. |

**Step 4 — One search**

```text
mcpmux_search_tools({ "query": "list folder" })
Report: ranking, total, top qualified_name, latency.
```

```bash
grep '\[search\] result summary\|\[embed\] store hydrate\|\[search\] cache decision' "$LOG" | tail -10
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `ranking: "hybrid"` | | ❌ | `ranking="lexical"` — store populated but not read |
| `embedding_store=hit` and `store_hits > 0` | | ❌ | `embedding_store="skipped"`, `hashes_requested=0` — store hydrate never attempted |

Step 4 log verbatim:
```
[embed] store hydrate query_id="6012ab70" hashes_requested=0 store_hits=0 store_misses=0
[search] cache decision query_id="6012ab70" index_cache="miss" embedding_store="skipped" active_tools=175
[search] result summary query_id=6012ab70 ranking="lexical" total=37 top_qualified_name="canva_list-folder-items" total_ms=940
```

Tool result: `{ranking: "lexical", total: 37, top: "canva_list-folder-items"}`.

**Verdict: 🐛 NEW FAILURE MODE** (May 30, 2026)

Warmer write path: **FIXED** ✅ — `run_spawn_blocking` fix works, 945 rows written, `warmer upserting records` fires for all servers.

Search read path: **NEW BUG** ❌ — `hashes_requested=0` with `embedding_store="skipped"`. The active ToolIndexEntry objects don't supply content_hashes to the store hydrate, so the embedding store is never consulted during search. This is distinct from O0 Run 1 (which was `embedding_store="miss"` — tried but found nothing). Here the store is not tried at all.

Filed as: *"Search read path: active index doesn't expose content_hashes to store hydrate — `hashes_requested=0`, `embedding_store=skipped`, ranking stuck lexical despite 945 rows in tool_embeddings."*

**O1–O4: Still BLOCKED** on the search read path fix.

- **FIX VERIFIED** — `embedded>0`, `warmer upserting`, rows > 0, search `hybrid` → update Final report O0b PASS, **unblock O1–O4** and run them in order.
- **STILL BROKEN** — paste: latest `warm batch done` lines, any `spawn_blocking panicked` + `panic` text, top `[embed] diag:` lines, before/after row counts, search JSON. Mark O1–O4 still BLOCKED.
- **NEW FAILURE MODE** — e.g. panics logged but no embed: file with panic text; dev owner fixes next.

Record: restart at 22:40 UTC, before=0 rows, after=945 rows, 27 servers warmed. Search: `hashes_requested=0, embedding_store=skipped, ranking=lexical`. Verdict: warmer write fixed; search read NEW BUG.

### O1 — Cross-session reuse (no re-embed per chat)

**Status: BLOCKED until O0b passes** — Run 1 confirmed store never populated; unblocks after O0b FIX VERIFIED.

**Setup:** O0 passed (store actually populates). Model Ready. Warm the store with one hybrid query, then open a **second** Cursor chat (or disconnect/reconnect MCP to mint a new `session_id`).

**Prompt (chat 1, then chat 2 — identical):**

```text
mcpmux_search_tools({ "query": "list folder" })
Report: ranking field, and whether the result felt instant or had a multi-second delay.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Chat 2's first hybrid query is **fast** — no ~30 s spike | ☐ | ☐ | the core win |
| Chat 2 logs show `store hydrate … store_hits > 0`, not a fresh corpus embed | ☐ | ☐ | no per-session re-embed |
| Chat 2 returns `ranking: "hybrid"` on the first call | ☐ | ☐ | |

Record: chat 1 vs chat 2 latency, `[embed] store hydrate` / `[search] cache decision` snippet for each.

### O2 — Restart persistence

**Status: BLOCKED until O0b passes**

**Setup:** Store warmed (O1 done). Quit McpMux fully, then relaunch (`pnpm dev:stop` → `pnpm dev:admin` for a dev build).

**Prompt (after relaunch, model Ready):**

```text
mcpmux_search_tools({ "query": "list folder" })
Report ranking and whether it felt instant.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Post-restart query returns `ranking: "hybrid"` without a long cold embed | ☐ | ☐ | vectors loaded from SQLite |
| `[embed]` logs show store hydrate / hits, not a full re-embed | ☐ | ☐ | persistence works |

Record: post-restart latency, `[embed]` hydrate snippet.

### O3 — Alias rename is free

**Status: BLOCKED until O0b passes**

**Setup:** Pick an active server with searchable tools. Note a tool's behavior, then in McpMux → server settings change that server's **alias** (the tool prefix).

**Prompt (after rename + tool reload):**

```text
mcpmux_search_tools({ "query": "<a tool from the renamed server>" })
Report the tool's new qualified_name (prefix should reflect the new alias) and ranking.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| Renamed prefix shows in `qualified_name` (lexical haystack updated) | ☐ | ☐ | rename took effect |
| `[embed]` logs show **no** re-embedding for that server after rename | ☐ | ☐ | content_hash unchanged (alias excluded) |
| Search still returns `ranking: "hybrid"` immediately | ☐ | ☐ | store hit on unchanged hash |

Record: old vs new `qualified_name`, confirmation that `[embed]` showed no fresh embed (`skipped_present` only).

### O4 — On-connect warm (no inline spike)

**Status: BLOCKED until O0b passes**

**Setup:** Identify a server whose tools are **not** yet in the store (newly added, or clear the store / use a fresh `{data_dir}`). Connect it (enable in a bound bundle, or relaunch so it connects fresh). Wait a few seconds for the background warmer.

**Prompt:**

```text
1. (Right after the server connects, before waiting) mcpmux_search_tools({ "query": "<tool from that server>" })
   Report ranking — may be "lexical" if the warmer hasn't finished.

2. Wait ~10–20 s, then repeat the same call.
   Expect ranking: "hybrid", instant — warmer populated the store off the hot path.
```

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| `[embed] warm batch done … embedded > 0` appears for the connected server (background) | ☐ | ☐ | on-connect warmer actually embedded (not just `embedded=0`) |
| Post-warm search is a store hit (`[search] cache decision … embedding_store=hit`) | ☐ | ☐ | vectors served from the store, not re-embedded inline |
| No all-core CPU spike on the user-facing search call | ☐ | ☐ | spike moved to the background warmer |

Record: ranking on call 1 vs call 2, `[embed] warm batch done` snippet, CPU observation.

### O5 — Model-version invalidation (optional, developer)

**Setup:** Only if testing a model change. Bump the embedding model version (or clear the store), restart.

| Check | Pass | Fail | Notes |
| ----- | ---- | ---- | ----- |
| New `model_version` re-warms the corpus incrementally (old rows ignored) | ☐ | ☐ | clean invalidation |
| Search still serves results (lexical) during the re-warm | ☐ | ☐ | no hard fail |

Record: `[embed]` `model_version` before/after, re-warm behavior.

---

## Red flags (stop and file a bug)

- [ ] Any of the removed tools (`enable_server`, `disable_server`, `create_feature_set`, `list_all_tools`) present in `tools/list`
- [ ] Bind appends duplicate FS entries instead of deduping
- [ ] Bind **replaces** prior feature sets instead of layering (prior bundle tools disappear)
- [ ] `search_tools` default finds tools from inactive/unbound servers
- [ ] Approval dialog never appears (Tauri or browser) on bind call
- [ ] Approval dialog appears in both Tauri AND browser simultaneously **without cross-dialog sync** (acting on one must auto-dismiss the other)
- [ ] `search_tools` returns 0 on first call even though workspace binding is already correct (root-race bug — fixed in Phase 6)
- [ ] `include_inactive: true` without `server_id` hangs > 5 s (inactive scan bug — fixed in Phase 7)
- [ ] `search_tools("list folder")` returns `total: 0` when Canva tools are active (token-overlap bug — fixed in Hybrid Phase 1)
- [ ] `search_tools` missing `ranking` field in payload (hybrid ranking regression)
- [ ] Intent query returns zero hits when semantically matching tool is active and model is Ready (Hybrid Phase 3/4 regression)
- [ ] Exact tool name query does not rank the literal tool in top 3 (fusion drowning lexical — Hybrid Phase 3 regression)
- [x] **(CONFIRMED May 30, 2026)** (Persistent cache) Warmer enqueues with `missing > 0` on a Ready model but every `warm batch done` is `embedded=0`, no `warmer upserting records`, and `tool_embeddings` stays empty (Section O0 — bug confirmed)
- [ ] (Persistent cache) A fresh chat / second session re-embeds the whole corpus instead of `store hydrate … store_hits > 0` (per-session re-embed regression, Section O1)
- [ ] (Persistent cache) App restart triggers a full cold re-embed instead of loading from SQLite (persistence regression, Section O2)
- [ ] (Persistent cache) Renaming a server alias re-embeds that server's tools (alias leaked into content_hash, Section O3)
- [ ] (Persistent cache) The all-core embedding spike lands on a user-facing `search_tools` call instead of the background warmer (Section O4)

---

## Final report

| Section | Result | Evidence |
| ------- | ------ | -------- |
| A Surface | ✅ PASS | 11 tools, all removed tools absent, extra `mcpmux_diagnose_server` present (not a concern) |
| B Discovery | ✅ PASS | All 4 calls passed on sha 16d5fff; hint fires correctly on `total: 0`; inactive rows carry `bindable_feature_set_id` |
| C Bind/layer | ✅ PASS | Layering intact; `already_bound: true` short-circuits before consent prompt; canva tools active post-bind |
| D Removed paths | ✅ PASS | `enable_server` absent; inactive invoke error correctly points to `mcpmux_bind_current_workspace` |
| E Human-only | ✅ PASS | `create_feature_set` absent; uncovered-tool hint correctly points to McpMux UI |
| F Web approval | ✅ PASS | Approve + deny both work; Tauri and browser dialogs sync correctly post-fix |
| G Invoke | ✅ PASS | Search → schema → invoke all clean; invoke returned live Canva data |
| G2 Hybrid search | ✅ PASS | Token-overlap, hybrid ranking, intent search, and wide inactive scan all pass post-fix |
| H Root-race | ✅ PASS | Phase 6 fix confirmed — first effective search call returns active tools with no `tools/list` warmup; no-match hint is include_inactive (not PendingRoots). Note: runbook test query `"core"` has no tool matches; use `"canva"` instead. |
| I Inactive scan perf | ✅ PASS | Wide scan `total: 1804` returned instantly; `server_id`-filtered call scoped to 337; hint fires correctly; no manual observability bundle setup needed |
| J Cache (hit/evict/disconnect) | ✅ PASS | J1: 4 identical repeat calls consistent; J2: post-bind browser tools active (used bundle:browser since design already bound); J3: post-reconnect clean. `tool_embeddings` = 0 rows throughout — O0 bug pre-confirmed. |
| K Lexical token-overlap | | |
| L Embedding lifecycle | | |
| M Hybrid fusion + cache | | |
| N Intent relevance | | |
| O0 Run 1 (archived) | 🐛 BUG (fixed) | May 30, 2026 — `embedded=0` despite Ready + `missing>0`; root cause: `run_spawn_blocking` silently swallowed panics. Fixed via `block_in_place`. |
| O0b Fix verification | 🐛 NEW BUG | Warmer write FIXED (945 rows, 27 servers warmed). Search read NEW BUG: `hashes_requested=0`, `embedding_store=skipped`, `ranking=lexical`. Active index doesn't expose content_hashes to store hydrate. Filed May 30, 2026. |
| O1–O4 Persistent embedding cache | BLOCKED | Warmer write fixed but search read broken — store populated, never consulted during search |

List any regressions. Flag BLOCKED if gateway unreachable or no inactive bundle available.

---

## Sign-off

| Phase | Result |
| ----- | ------ |
| Phase 1 — discovery inactive opt-in | ✅ Pass |
| Phase 2 — bind layering | ✅ Pass |
| Phase 3 — ephemeral path removed | ✅ Pass |
| Phase 4 — human-only authoring | ✅ Pass |
| Phase 5 — web approval | ✅ Pass |
| Phase 6 — root-race fix | ✅ Pass |
| Phase 7 — inactive scan perf | ✅ Pass |
| Phase 8 — active index cache | ✅ Pass |
| Hybrid 1 — lexical token-overlap | ✅ Pass (covered by G2) |
| Hybrid 2 — embedding lifecycle | ✅ Pass (covered by G2) |
| Hybrid 3 — hybrid fusion + cache | ✅ Pass (covered by G2) |
| Hybrid 4 — intent relevance | ✅ Pass (covered by G2) |
| Persistent cache 0 Run 1 — warmer diagnostic | 🐛 Bug fixed — `run_spawn_blocking` → `block_in_place` |
| Persistent cache 0b — fix verification | 🐛 New bug — warmer write fixed (945 rows), search read broken (`hashes_requested=0`, `embedding_store=skipped`) |
| Persistent cache 1–3 — cross-session / persistence / alias-free | ☐ Blocked on O0b read-path fix |
| Persistent cache 4 — on-connect warm | ☐ Blocked on O0b read-path fix |
| Overall | ☐ Block — persistent cache read path broken; O1–O4 cannot be validated until `hashes_requested=0` / `embedding_store=skipped` is fixed |
