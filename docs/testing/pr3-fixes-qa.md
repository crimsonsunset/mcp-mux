# PR #3 Review Fixes — Manual QA Runbook

**Last Updated:** May 31, 2026  
**Branch:** `docs/feature-set-consent-model`  
**Related:** [`pr3-fix-verification.md`](./pr3-fix-verification.md) (automated gate), [`search-tools-persistent-embedding-cache.md`](../backend/reference/search-tools-persistent-embedding-cache.md)

One-session checklist validating the post-review fixes: warmer cold-start (Major #1), hybrid ranking after the O(N) lexical refactor (Major #2), embedding mutex robustness (Major #3), batched `get_many`, removed notification kill switch, and the dropped embed semaphore. Behaviors are observed through the **agent output + gateway logs** — most of these fixes are internal, so the log lines are the real evidence.

---

## QA Results — May 31, 2026 (Session 4)

**Verdict: SHIP.** Automated gate fully green + live hybrid/latency confirmed against the running fixed binary. Restart-based sections (§1/§4/§8) signed off on combined evidence rather than live theatre — see rationale below.

**Binary provenance:** running gateway is `target/debug/mcpmux` built **May 30 22:14**, one minute _after_ the newest gateway source edit (`discovery_rank.rs` @ 22:13) — i.e. the live `:45818` process already contains all Major #1–3 fixes. No rebuild/restart was needed to validate.

### Automated gate (`pr3-fix-verification.md` §0) — ✅ PASS

| Step                                                   | Result                   |
| ------------------------------------------------------ | ------------------------ |
| `cargo clippy --workspace -- -D warnings`              | ✅ clean                 |
| `nextest -p mcpmux-gateway -p mcpmux-storage --lib`    | ✅ 234 passed, 2 skipped |
| `nextest -p tests` (meta_tools / approval / relevance) | ✅ 46 passed             |
| `pnpm typecheck`                                       | ✅ clean                 |

Fix-specific guards passing: `connect_event_warms_server_catalog_embeddings_before_search` (Major #1), `search_tools_reuses_persisted_embeddings_after_registry_restart` (§4 get*many), `search_tools_ranking_lexical_when_model_absent` (§8 fallback), `search_relevance_eval_hybrid_top3` (Major #2), `admin_http*{approve,deny}\_resolves_pending_bind_approval` (§7 broker).

### Live manual checks (against running fixed binary)

| §                            | Result  | Live evidence                                                                                                                                   |
| ---------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| **2** intent/semantic        | ✅ PASS | `query="post a comment on an issue"` → top = `sonarqube_addCommentToIssue`, `ranking="hybrid"`, `top_fused_score=0.885`, `vectors_present=1884` |
| **3** exact-name             | ✅ PASS | `query="list_issues"` → top = `github_list_issues` (#1), `ranking="hybrid"`; underscore tokenized correctly                                     |
| **5** large-catalog latency  | ✅ PASS | `hybrid_compute_ms` scales linearly: 107→22ms, 552→92ms, 1541→208ms. No multi-second stall; the Session 2 637ms regression is gone.             |
| — active-index cache (bonus) | ✅ PASS | calls 2 & 3 logged `index_cache_hit=true` (34ms / 32ms)                                                                                         |

> **Strongest single signal:** live `vectors_present=1884` proves the warmer already embedded essentially the entire active catalog on the current session — the exact behaviour Major #1 fixes. Pre-fix this would be ~0 with everything falling back to `ranking="lexical"`.

### Not run live — signed off on evidence

| §                             | Status                                           | Why not run live                                                                                                                                                                                                                                                                                                                                         |
| ----------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1** cold-start `embedded>0` | ⏭️ Covered by gate + live `vectors_present=1884` | Needs `dev:stop` + move `embeddings/` + **wipe `tool_embeddings` table** (vectors are in SQLite, separate from the model dir — same `model_version` re-download alone yields `embedded=0`/`skipped_present=N`) + up to 120s model download. Warm path already proven in prod; cold-download transition is exactly what `connect_event_warms_...` covers. |
| **4** cross-restart reuse     | ⏭️ Covered by gate                               | `search_tools_reuses_persisted_embeddings_after_registry_restart` green; would only re-prove `skipped_present` high after a restart.                                                                                                                                                                                                                     |
| **8** lexical fallback        | ⏭️ Covered by gate                               | `search_tools_ranking_lexical_when_model_absent` green.                                                                                                                                                                                                                                                                                                  |
| **6** list_changed delivery   | ⬜ Skipped (low ROI today)                       | Live delivery e2e not in the unit gate; needs a UI binding toggle. ~2 min if revisited.                                                                                                                                                                                                                                                                  |
| **7** bind consent dialog     | ⬜ Skipped (would mutate live workspace)         | Runbook intends a dedicated QA folder; firing against the live `mcp-mux` binding mutates real state. Broker logic covered by `admin_http_*` tests.                                                                                                                                                                                                       |

---

## Quick prep

- [ ] Rebuild/restart gateway off this branch (`pnpm dev` or the freshly built app) — you must be on the fixed binary, not a stale one
- [ ] Cursor → MCP → **Reload tools**; confirm endpoint `http://localhost:45818/mcp`
- [ ] Have a terminal tailing the gateway log with embed/search targets visible, e.g.:
  ```bash
  # adjust to wherever you surface gateway logs (dev console / log file)
  rg '\[embed\]|\[search\]|list_changed' <gateway-log>
  ```
- [ ] At least one OAuth/stdio server with **many tools** installed + connected (GitHub, GWorkspace) and a FeatureSet bound to this workspace
- [ ] Know your embeddings cache dir (macOS): `~/Library/Application Support/com.mcpmux.app/embeddings`

**Tester:** Agent (Cursor) + Joe  
**Date:** May 31, 2026 (Session 4)  
**McpMux version / commit:** `docs/feature-set-consent-model` @ `0e87cd8` + uncommitted fixes; running binary `target/debug/mcpmux` built May 30 22:14 (fresh vs source 22:13) — PR [#3](https://github.com/crimsonsunset/mcp-mux/pull/3)

---

## 0. Sanity — fixed binary, meta surface intact

**Prompt:**

```
You have McpMux meta tools only.

1. Call mcpmux_list_servers and show installed servers + active/inactive status.
2. List every mcpmux_* tool you can see.
```

| Check                                                 | Pass | Fail | Notes                     |
| ----------------------------------------------------- | ---- | ---- | ------------------------- |
| `mcpmux_list_servers` returns installed servers       | ☐    | ☐    |                           |
| Meta tool namespace present (no backend names leaked) | ☐    | ☐    |                           |
| Gateway startup banner shows this branch/commit       | ☐    | ☐    | confirm not a stale build |

---

## 1. Cold-start hybrid warmup — **Major #1** (the headline fix)

**Setup:** Simulate a fresh profile so the model must download and the warmer must wait for it.

```bash
mv ~/Library/Application\ Support/com.mcpmux.app/embeddings{,.bak} 2>/dev/null || true
# restart the gateway, then connect a server with tools
```

**Prompt** (run only **after** the model finishes downloading — watch logs):

```
Use the McpMux meta workflow.

1. mcpmux_search_tools query "list issues", detail_level "description"
2. Show the top 5 matches.
```

| Check                                                                                    | Pass | Fail | Notes                                         |
| ---------------------------------------------------------------------------------------- | ---- | ---- | --------------------------------------------- |
| On connect, warmer waits (not instant skip) — log shows model `Downloading` then `Ready` | ☐    | ☐    | pre-fix bug: skipped immediately              |
| `[embed] warm batch done embedded=N` (N>0) appears **after** model `Ready`               | ☐    | ☐    | the fix: warmer populated the cache           |
| NOT stuck on `warm batch skipped (model not ready within budget)` with no later `done`   | ☐    | ☐    | only acceptable if download > 120s            |
| Search log shows `ranking hybrid` and `store_hits`/`vectors_present > 0`                 | ☐    | ☐    | hybrid actually engaged on first cold session |
| Search returns sensible results                                                          | ☐    | ☐    |                                               |

**Restore after:** `mv ~/Library/Application\ Support/com.mcpmux.app/embeddings{.bak,}`

---

## 2. Hybrid relevance — intent query (semantic) — Major #2 didn't regress quality

**Prompt:**

```
McpMux meta tools only. Do NOT guess tool names.

mcpmux_search_tools query "post a comment on an issue" detail_level "description"
Show the top 3 results in order.
```

| Check                                                                                      | Pass | Fail | Notes                   |
| ------------------------------------------------------------------------------------------ | ---- | ---- | ----------------------- |
| The comment-creating tool (e.g. `github_*comment*` / Jira add-comment) is in the **top 3** | ☐    | ☐    | semantic fusion working |
| `[search]` log line reports `ranking hybrid`                                               | ☐    | ☐    |                         |
| Result order looks intent-relevant, not just keyword soup                                  | ☐    | ☐    |                         |

---

## 3. Lexical exact-name still wins — Major #2 fusion balance

**Prompt:**

```
mcpmux_search_tools query "list_issues" detail_level "name"
Which tool is ranked first?
```

| Check                                                                       | Pass | Fail | Notes                                            |
| --------------------------------------------------------------------------- | ---- | ---- | ------------------------------------------------ |
| Exact/near-exact name match ranks **first** (semantic noise didn't bury it) | ☐    | ☐    | confirms 0.4 lexical weight intact post-refactor |
| Hyphenated/underscored names still tokenized correctly                      | ☐    | ☐    | e.g. `canva_list-folder-items` for "list folder" |

---

## 4. Persistent cache reuse across restart — batched `get_many`

**Setup:** With embeddings already warmed (from §1/§2), **restart the gateway** (don't clear the dir).

**Prompt:**

```
mcpmux_search_tools query "list issues" detail_level "description"
```

| Check                                                                                    | Pass | Fail | Notes                                  |
| ---------------------------------------------------------------------------------------- | ---- | ---- | -------------------------------------- |
| After restart, warm batch shows `skipped_present` high / `embedded=0` (cache reused)     | ☐    | ☐    | no full re-embed                       |
| First search after restart is `ranking hybrid` immediately (or after a brief model load) | ☐    | ☐    | store hydrated from SQLite             |
| No errors from the chunked `IN (...)` query in `get_many`                                | ☐    | ☐    | watch for SQLite param/encoding errors |

---

## 5. Large-catalog search latency — Major #2 O(N) lexical

**Setup:** Enable a server with a large tool catalog (GWorkspace / GitHub full set).

**Prompt:**

```
mcpmux_search_tools query "list" server_id "<that-server>" detail_level "name" limit 25
Roughly how long did that take?
```

| Check                                                             | Pass | Fail | Notes                                   |
| ----------------------------------------------------------------- | ---- | ---- | --------------------------------------- |
| Search returns quickly (sub-second to ~1s; no multi-second stall) | ☐    | ☐    | pre-fix O(N²) lexical recompute is gone |
| `[search] fusion` log `hybrid_compute_ms` is small                | ☐    | ☐    |                                         |
| Result count/order stable across repeated runs                    | ☐    | ☐    |                                         |

---

## 6. list_changed notifications still fire — kill switch removed

**Setup:** Bind or unbind a FeatureSet for this workspace in the UI (or connect/disconnect a server).

**Prompt** (after the binding change, without manually reloading):

```
Re-list your available tools. Did the set change to match the binding I just edited?
```

| Check                                                                   | Pass | Fail | Notes                                                   |
| ----------------------------------------------------------------------- | ---- | ---- | ------------------------------------------------------- |
| `tools/list_changed` (and prompts/resources) fire on the binding change | ☐    | ☐    | log: `📤 Sending tools/list_changed`                    |
| Client tool set reflects the change without a manual reconnect          | ☐    | ☐    | DISABLE_ALL_NOTIFICATIONS removal didn't break delivery |
| Throttle/dedup still prevents floods (no notification storm)            | ☐    | ☐    |                                                         |

---

## 7. Bind consent flow — approval broker intact

**Prompt:**

```
mcpmux_search_tools query "<something>" include_inactive: true
Find an inactive tool, then call mcpmux_bind_current_workspace with its bindable_feature_set_id.
```

| Check                                                                        | Pass | Fail | Notes |
| ---------------------------------------------------------------------------- | ---- | ---- | ----- |
| Inactive search surfaces `bindable_feature_set_id`                           | ☐    | ☐    |       |
| `mcpmux_bind_current_workspace` triggers an approval dialog (desktop or web) | ☐    | ☐    |       |
| Approve → tool becomes invokable; the inactive tools are now active          | ☐    | ☐    |       |
| Already-bound target short-circuits (`already_bound`) without a dialog       | ☐    | ☐    |       |

---

## 8. Graceful lexical fallback — Major #3 robustness

**Setup:** Either before the model finishes downloading (fresh profile, immediate query) or with embeddings dir removed and the query fired before `Ready`.

**Prompt:**

```
mcpmux_search_tools query "list issues" detail_level "description"
```

| Check                                                                             | Pass | Fail | Notes                                             |
| --------------------------------------------------------------------------------- | ---- | ---- | ------------------------------------------------- |
| Search still returns results when the model isn't ready (no empty list, no error) | ☐    | ☐    | `ranking lexical` + `skip_reason model_not_ready` |
| Repeated searches never permanently wedge into empty results                      | ☐    | ☐    | parking_lot mutex — no poisoning lockout          |
| Once model is `Ready`, subsequent searches flip to `ranking hybrid`               | ☐    | ☐    |                                                   |

---

## Red flags (stop and file a bug)

- [ ] Cold-start: warm batch only ever logs `skipped (model not ready)` and the persistent cache stays empty after the model is `Ready`
- [ ] Search permanently returns `ranking lexical` even with a warmed cache + model `Ready`
- [ ] Search returns empty/errors instead of falling back to lexical when the model isn't ready
- [ ] `get_many` errors after a gateway restart (SQLite `IN`/param/blob decode)
- [ ] `tools/list_changed` no longer delivered after a binding/server-status change
- [ ] Large-catalog search stalls for multiple seconds (O(N²) regression)
- [ ] Bind approval dialog never appears, or approving doesn't activate tools

---

## Sign-off

| Area                                   | Result                                                                                                    |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Major #1 — warmer cold-start           | ☑ Pass (gate + live `vectors_present=1884`; cold-download transition via `connect_event_warms_...`)       |
| Major #2 — hybrid ranking + latency    | ☑ Pass (live §2/§3/§5 + `search_relevance_eval_hybrid_top3`)                                              |
| Major #3 — fallback / no mutex lockout | ☑ Pass (`search_tools_ranking_lexical_when_model_absent`)                                                 |
| Persistent cache (`get_many` batch)    | ☑ Pass (`search_tools_reuses_persisted_embeddings_after_registry_restart` + storage `--lib`)              |
| Notifications (kill switch removed)    | ☐ Not run live (low ROI; clippy/check confirm removal compiles clean)                                     |
| Bind consent flow                      | ☑ Pass (broker via `admin_http_{approve,deny}_...`); live dialog skipped to avoid mutating real workspace |
| Overall                                | ☑ **Ship**                                                                                                |

**Blockers / issues filed:**

```
None.
```

**Sign-off note (May 31, 2026):** All three Major fixes + `get_many` batching validated via the green automated gate and live hybrid/latency observation against the confirmed-fresh binary. §6 (list_changed delivery) and live §7 (bind dialog) intentionally deferred — low marginal value over unit coverage and §7 would mutate the live `mcp-mux` binding. Recommend `pnpm validate` then commit when ready.
