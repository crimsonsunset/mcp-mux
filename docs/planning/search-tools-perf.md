# `mcpmux_search_tools` Performance

**Last Updated:** Jul 24, 2026
**Status:** Closed — warm path fixed (Phase 0 + 1). Widen/cold follow-ups parked (see Phase 2).
**Branch:** `dev-rebased`
**Shipped:** `102bb92` (Phase 0 instrumentation), `cf36934` (Phase 1 quick wins)
**Depends on:** Hybrid search + session index cache (pre-existing)
**Unblocks:** Optional widen/cold work only if those paths become the user-visible complaint

---

## Problem

Origin: dig-and-ask session (Jul 24, 2026). Agent-facing `mcpmux_search_tools` felt really slow. Phase 0 instrumentation showed the warm path was dominated by readiness enrichment that re-ran `resolve_feature_sets` (full-space `server_features` load including `raw_json`) on every call — even when the session index cache hit.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Surface | **`mcpmux_search_tools` only** | User (1a). UI catalog search is a different stack. |
| 2 | Cold vs warm | **Warm was the pain; cold is secondary** | Phase 0: warm ~346ms almost all readiness; cold ~900ms index build. |
| 3 | Hybrid vs lexical | **Keep 0.4/0.6 hybrid** | Phase 1.5: `rank_embed_query_ms` ~5–6ms when ready; not a latency lever. Lexical-only would buy almost nothing and lose intent ranking. |
| 4 | Ambition | **Measure → quick wins → index only if still slow** | User (4c). Warm path satisfied after Phase 1; FTS/ANN not justified yet. |
| 5 | `include_inactive` | **Still expensive (~700ms widen); not fixed in this pass** | Logged; ~998 inactive tools on All. Parked as next optional ticket. |
| 6 | Readiness source | **Binding servers from active index** — no `resolve_feature_sets` on search | Eliminated the warm killer (Phase 1). |
| 7 | Metrics | **Structured tracing only** | `[search] timing breakdown` + `query_id` is enough. |
| 8 | Close criteria | **Warm active-only ≪ 100ms on fat Space** | Achieved: **33ms** (`fe800d29`). |

---

## Scope

**In (shipped):**

- Phase 0 timing/usage instrumentation
- Phase 1: enrichment without FeatureSet re-resolve; `Arc<ToolIndex>` cache; batched embedding warmer
- Phase 1.5 hybrid keep decision (this doc)

**Out / deferred:**

| Item | Reason / Deferral |
| ---- | ----------------- |
| Inactive widen optimization (`list_inactive_discovery_tools`) | Still ~690ms; open a follow-up only if agents routinely use `include_inactive` / `scope=all` and feel it |
| Cold index build / slim `list_for_space` projection | Cold ~600ms; acceptable once-per-session; revisit if cold starts dominate UX |
| SQLite FTS5 / ANN | Warm path already fine; trigger was “warm still slow after Phase 1” — not met |
| Lexical-only flag / fusion weight retune | Not a latency issue (Decision 3) |
| Discover / ServersPage / FeatureSetPanel UI search | Different stack |
| Metrics SaaS | Decision 7 |

---

## Architecture (after Phase 1)

```text
mcpmux_search_tools
  → caller_resolution                         (resolve_ms)
  → active index (session Arc cache or build) (active_index_ms, index_cache_hit)
  → include_inactive widen (copy-on-write)    (inactive_widen_ms, index_clone_ms)
  → enrichment from index server ids          (readiness_ms — installed list + pool only)
  → embedding hydrate                         (hydrate_ms)
  → lexical + optional hybrid rank            (rank_*_ms)
  → zero-result post paths                    (post_ms + flags)
  → [search] timing breakdown
```

---

## Files modified

| Commit | Files |
| ------ | ----- |
| Phase 0 `102bb92` | `search_tools.rs`, `search_tools_index.rs`, `tool_discovery_search.rs`, `tool_discovery_types.rs`, `embedding.rs`, this planning doc |
| Phase 1 `cf36934` | `search_tools.rs`, `search_tools_index.rs`, `meta_tool_common.rs` (`build_search_server_enrichment`), `session_roots.rs` / `registry.rs` (`Arc<ToolIndex>`), `embedding_warmer.rs`, this planning doc |

---

## Phases

### Phase 0 — Measurement — **DONE** (`102bb92`)

Instrumented enrichment, rank sub-timings, usage flags. Baselines in Notes.

### Phase 1 — Quick wins — **DONE** (`cf36934`)

- `build_search_server_enrichment` — no FeatureSet resolve on search path
- `Arc<ToolIndex>` session cache; clone only on inactive widen
- Warmer batches via one `embed_documents` call

**Outcome:** Warm HogQL **346ms → 33ms**; readiness **315ms → 2ms**.

### Phase 1.5 — Hybrid tradeoff — **DONE** (memo only)

| When | `rank_embed_query_ms` | `rank_semantic_ms` | Verdict |
| ---- | -------------------- | ------------------ | ------- |
| Model downloading | 0 (lexical fallback) | 0 | Fine |
| Model ready (warm) | ~5–6 | ~5–80 depending on index size | Keep hybrid |

No code change. No lexical-only flag.

### Phase 2 — Deeper indexing / widen — **PARKED**

**Do not start** unless one of these is true:

1. Warm active-only regresses or feels slow again on a fat Space, or
2. Real traffic shows frequent `include_inactive` / `scope=all` and ~700ms+ is unacceptable, or
3. Cold `active_index_ms` (~600ms) becomes a product complaint

**If unlocked, preferred order:** (a) speed up inactive discovery / cache widen results per session+fingerprint, (b) slim feature projection for cold index build, (c) FTS only if rank over large `merged_index` dominates after (a)/(b).

---

## How to re-measure

```text
@mux run the search-tools-perf matrix … (four mcpmux_search_tools calls)
```

```bash
LOG="$HOME/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log"
rg '\[search\] (call entry|result summary|timing breakdown)' "$LOG" | tail -24
```

---

## Notes

### Phase 0 baselines (session `73ad8447…`)

| Scenario | `query_id` | `total_ms` | Dominant phase(s) |
| -------- | ---------- | ---------- | ----------------- |
| Cold HogQL | `008242c4` | 900 | index 574 + readiness 304 |
| Warm HogQL | `4e0d0e5a` | 346 | **readiness 315** |
| Widen HogQL | `59e0ef81` | 1112 | widen 696 + readiness 308 |
| Widen Jira/Confluence | `f17a555c` | 1319 | widen 716 + readiness 358 |

### Phase 1 after (session `6f919fc5…`)

| Scenario | `query_id` | `total_ms` (was) | `readiness_ms` (was) |
| -------- | ---------- | ---------------- | -------------------- |
| Cold HogQL | `18e2e725` | 631 (900) | **1** (304) |
| Warm HogQL | `fe800d29` | **33** (346) | **2** (315) |
| Widen HogQL | `4733b05b` | 790 (1112) | **1** (308) |
| Widen Jira/Confluence | `1e5711b4` | 927 (1319) | **1** (358) |

### Close-out (Jul 24, 2026)

Original complaint (slow search) addressed for the common warm active-only path. Follow-up ticket material: inactive widen (~690ms) and optional cold index. Instrumentation stays — keep grepping `[search] timing breakdown` if latency regressions show up.
