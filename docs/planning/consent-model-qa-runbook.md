# Consent-Model PR — Manual QA Runbook

**Last Updated:** May 29, 2026
**Branch:** `docs/feature-set-consent-model`
**Related:** [`feature-set-consent-model.md`](./feature-set-consent-model.md) · [`search-tools-latency-and-root-race.md`](./search-tools-latency-and-root-race.md) · [`search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md)

Full checklist for validating Phases 1–8 of the consent-model PR plus hybrid search ranking (Phases 1–4 of the semantic-ranking doc): discovery of inactive tools, bind layering, removed ephemeral path, human-only authoring, web approval, latency/cache fixes (root-race, inactive scan, active index cache), and hybrid lexical + embedding search. Sections A–G map to consent Phases 1–5; Sections H–J map to latency Phases 6–8; Sections K–N map to hybrid-ranking Phases 1–4.

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
| First call returns `total > 0` with active tools | ☐ | ☐ | root-race fixed |
| No-match query returns hint (not silent 0 / binding-missing) | ☐ | ☐ | |
| `scope: "active_only"` in both responses | ☐ | ☐ | |

Record: full JSON for both calls.

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
| Wide `include_inactive` scan completes < 2 s | ☐ | ☐ | Phase 7 fixed 84 s hang |
| `total` reflects large inactive set | ☐ | ☐ | |
| Hint present when `total > 50` and no `server_id` | ☐ | ☐ | |
| `server_id`-filtered call returns scoped results | ☐ | ☐ | |

Record: timing for call 1, total count, hint text.

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
| Calls 2–5 return consistent results (same active index) | ☐ | ☐ | |
| Different query on call 6 still returns active tools | ☐ | ☐ | cache key is index not query |

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
| Post-bind search returns tools from newly bound bundle | ☐ | ☐ | eviction on WorkspaceBindingChanged |
| Prior bundle tools still present (layering intact) | ☐ | ☐ | |

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
| Post-reconnect search returns correct active tools | ☐ | ☐ | cache evicted on disconnect |
| No stale data from previous session | ☐ | ☐ | |

Record: pre/post-rebind results for J2, post-reconnect result for J3.

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
| H Root-race | | |
| I Inactive scan perf | | |
| J Cache (hit/evict/disconnect) | | |
| K Lexical token-overlap | | |
| L Embedding lifecycle | | |
| M Hybrid fusion + cache | | |
| N Intent relevance | | |

List any regressions. Flag BLOCKED if gateway unreachable or no inactive bundle available.

---

## Sign-off

| Phase | Result |
| ----- | ------ |
| Phase 1 — discovery inactive opt-in | ☐ Pass ☐ Fail |
| Phase 2 — bind layering | ☐ Pass ☐ Fail |
| Phase 3 — ephemeral path removed | ☐ Pass ☐ Fail |
| Phase 4 — human-only authoring | ☐ Pass ☐ Fail |
| Phase 5 — web approval | ☐ Pass ☐ Fail |
| Phase 6 — root-race fix | ☐ Pass ☐ Fail |
| Phase 7 — inactive scan perf | ☐ Pass ☐ Fail |
| Phase 8 — active index cache | ☐ Pass ☐ Fail |
| Hybrid 1 — lexical token-overlap | ☐ Pass ☐ Fail |
| Hybrid 2 — embedding lifecycle | ☐ Pass ☐ Fail |
| Hybrid 3 — hybrid fusion + cache | ☐ Pass ☐ Fail |
| Hybrid 4 — intent relevance | ☐ Pass ☐ Fail |
| Overall | ☐ Ship ☐ Block |
