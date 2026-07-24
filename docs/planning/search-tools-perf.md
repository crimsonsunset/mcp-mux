# `mcpmux_search_tools` Performance

**Last Updated:** Jul 24, 2026
**Status:** Phase 0 implemented — awaiting cold/warm baselines in Notes (reassess before Phase 1+)
**Branch:** `dev-rebased` (or current working branch)
**Depends on:** Existing hybrid search + session index cache (already on mainline)
**Unblocks:** Data-backed decision on whether quick wins / indexing / hybrid changes are worth doing

---

## Problem

Origin: dig-and-ask session (Jul 24, 2026). Agent-facing `mcpmux_search_tools` feels **really slow**. We do not know which phase dominates — cold index/embeddings vs every-call work vs `include_inactive` widen vs ONNX query embed — because the current timing breakdown is incomplete and usage of the expensive flags is not easy to mine from logs.

Verified in code during dig (not assumed):

- Orchestrator + timing live in [`search_tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs). It already logs `[search] call entry` and `[search] timing breakdown` with `resolve_ms`, `active_index_ms`, `index_clone_ms`, `inactive_widen_ms`, `hydrate_ms`, `rank_ms`, `post_ms`.
- **`build_server_readiness_map` + `build_installed_server_meta_maps` run before timed index work and are not in `accounted_ms`** — they show up only as `unaccounted_ms`.
- Rank is one outer bucket; inner lexical timing exists in [`tool_discovery_search.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery_search.rs) but is not plumbed to the breakdown. Query-embed / semantic fusion costs are not split out.
- `include_inactive` is already on call-entry logs, but there is no clear `scope: "all"` source flag, widen size, or zero-result post-path markers — so we cannot tell how often agents pay for the expensive widen/catalog paths.
- Session cache ([`session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs)) skips index rebuild on hit but still clones the full `Vec` index and still runs enrichment / hydrate / rank every call.
- Hybrid 0.4/0.6 fusion is the product default; code comments claim hybrid beats lexical on an intent fixture. Do **not** rip hybrid without measured cost + relevance discussion.

**Net: optimize nothing until Phase 0 logs answer where time goes and how callers use widen/scope.**

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Surface | **`mcpmux_search_tools` only** (gateway meta-tool) | User confirmed (1a). Discover / My Servers / FeatureSetPanel client-side search are different stacks — out of this doc. |
| 2 | Cold vs warm | **Unknown — measure** | User (2c): need concrete ground from logs before claiming first-call vs every-call. |
| 3 | Hybrid vs lexical | **Keep hybrid as default for now; revisit only after Phase 0 numbers** | User wants tradeoff discussion with data, not a vibes flip. Relevance comments in `tool_discovery_search.rs` argue against casual lexical-only. |
| 4 | Ambition | **Phased: measure → quick wins → indexing if still slow** | User (4c). Later phases are parked in this doc until Phase 0 baselines exist. |
| 5 | `include_inactive` / `scope=all` | **Unknown — add logging to find out** | User (5c). Already partially logged; Phase 0 makes usage mineable. |
| 6 | Active work now | **Phase 0 only** | Explicit user call (Jul 24): ship measurement, then reassess. Phases 1 / 1.5 / 2 are **do not do yet**. |
| 7 | Metrics backend | **Structured tracing only** — no PostHog / SaaS latency pipeline | Enough to grep cold vs warm; keeps Phase 0 small. |

---

## Scope

**In (Phase 0 — do now):**

- Close timing blind spots so `accounted_ms ≈ total_ms` for the real work
- Plumb rank sub-timings (lexical / query-embed / semantic)
- Make `include_inactive` / `scope=all` / zero-result widen paths easy to mine from info-level logs (no query text at info)
- Capture cold vs warm baselines on a fat Space and record them in this doc's Notes
- Reassess in a follow-up session before touching optimization phases

**Out:**

| Item | Reason / Deferral |
| ---- | ----------------- |
| Phase 1 quick wins (dedupe DB loads, `Arc` index cache, warmer batching, slim feature projection) | **Do not do yet** — parked until Phase 0 baselines; see Phases below |
| Phase 1.5 hybrid tradeoff memo + any ranking weight / lexical-only flag changes | **Do not do yet** — needs Phase 0 `rank_embed_query_ms` / `embedding_state` data |
| Phase 2 FTS / ANN / new search index | **Do not do yet** — only if warm path still bad after Phase 1 (itself not started) |
| Discover / Registry / ServersPage / FeatureSetPanel UI search | Different stack (client-side filter over preloaded catalogs); not this ticket |
| New metrics SaaS / PostHog for search latency | Decision 7 — tracing is enough for Phase 0 |
| Changing fusion weights or synonym maps | Product/relevance change; only after Phase 1.5 discussion |

---

## Architecture

### Call path (unchanged behavior; measurement only in Phase 0)

```text
mcpmux_search_tools
  → caller_resolution                         (resolve_ms)
  → readiness map + installed meta            (TODAY: unaccounted — Phase 0 times these)
  → active index (session cache or build)     (active_index_ms, index_cache_hit)
  → optional clone for mutate/widen           (index_clone_ms)
  → include_inactive widen                    (inactive_widen_ms)
  → embedding hydrate                         (hydrate_ms)
  → lexical + optional hybrid rank            (rank_ms — Phase 0 splits sub-buckets)
  → zero-result post paths                    (post_ms + flags)
  → [search] timing breakdown                 (query_id correlation)
```

### Phase 0 log fields (add / promote to info on timing or call-entry)

| Field | Why |
| ----- | --- |
| `readiness_ms`, `installed_meta_ms` | Close the enrichment blind spot currently in `unaccounted_ms` |
| `index_cache_hit` | Distinguish cold index build vs warm cache on the timing line (today debug-only) |
| `rank_lexical_ms`, `rank_embed_query_ms`, `rank_semantic_ms` | Split hybrid cost; zeros when lexical-only fallback |
| `include_inactive`, `scope_all`, `server_id_set` | Mine widen usage without reading query text |
| `inactive_tool_count`, `inactive_widen_ms` | Size + cost of inactive path |
| `zero_result_inactive_preview`, `zero_result_catalog_scan` | Catch expensive post-search paths |
| `embedding_state`, `hydrated_missing_count` | Cold model / missing vectors vs warm hybrid |

No query text at info level (keep existing debug-only query log).

### How to measure (after Phase 0 ships)

1. Use a Space with multiple connected servers / a large tool surface.
2. Cold: new MCP session (or reconnect) → first `mcpmux_search_tools`.
3. Warm: repeat search in the same session.
4. Optionally one call with `include_inactive: true` and one with `scope: "all"` to see widen cost.
5. Grep gateway logs for `[search] call entry` and `[search] timing breakdown` by `query_id`.
6. Paste cold/warm numbers into **Notes** below; then decide whether to unlock Phase 1.

---

## Files to create / modify

### Phase 0 (active)

| Area | File | Action |
| ---- | ---- | ------ |
| Gateway | [`crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs) | Time enrichment; promote `index_cache_hit` + usage flags onto timing/summary logs; fold new ms into `accounted_ms` |
| Gateway | [`crates/mcpmux-gateway/src/services/tool_discovery_search.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery_search.rs) | Return / plumb `rank_lexical_ms`, `rank_embed_query_ms`, `rank_semantic_ms` (and embedding state if available at rank time) |
| Gateway | [`crates/mcpmux-gateway/src/services/meta_tools/search_tools_index.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools_index.rs) | Surface `hydrated_missing_count` (or equivalent) if hydrate already knows it |
| Planning | [`docs/planning/search-tools-perf.md`](./search-tools-perf.md) | This doc — update Notes with baselines after Phase 0 ships |

### Later phases (do not touch yet — inventory only)

| Phase | Likely files | Notes |
| ----- | ------------ | ----- |
| 1 | `meta_tool_common.rs`, `search_tools.rs`, `session_roots.rs`, `tool_discovery_index.rs`, `embedding_warmer.rs`, maybe `server_feature_repository.rs` | Dedupe loads, `Arc` cache, warmer batch, optional slim projection |
| 1.5 | this planning doc only (memo) | Data-backed hybrid discussion |
| 2 | storage migrations + discovery rank / index path | FTS candidate filter if still slow after Phase 1 |

---

## Phases

### Phase 0 — Measurement — **DO NOW** (~half day)

- Time `build_server_readiness_map` and `build_installed_server_meta_maps`; include in `accounted_ms`
- Plumb rank sub-timings from `tool_discovery_search` onto `[search] timing breakdown`
- Promote / add usage fields: `index_cache_hit`, `include_inactive`, `scope_all`, `server_id_set`, `inactive_tool_count`, zero-result path booleans, `embedding_state`, `hydrated_missing_count`
- Keep query text off info-level logs
- Reproduce cold + warm searches; record numbers in Notes

**Outcome:** For any `mcpmux_search_tools` call, logs answer (1) where wall time went phase-by-phase with little unaccounted remainder, (2) whether the call was cache-hit and hybrid-ready, (3) whether inactive/scope widen or zero-result catalog scan ran. No ranking or cache behavior changes. Ready to reassess Phase 1 with concrete ground.

---

### Phase 1 — Quick wins — **DO NOT DO YET** (parked)

*Unlock only after Phase 0 baselines are written in Notes and we agree the hot path is enrichment / clone / hydrate / warmer — not "something else."*

- Dedupe `resolve_feature_sets` / `list_for_space` between readiness and installed-meta
- Store `Arc<ToolIndex>` in session `search_cache`; avoid full `Vec` clone every query (copy-on-write only when inactive widen mutates)
- Batch embedding warmer (today: one doc at a time)
- Optional slim feature projection for index build (only if Phase 0 shows feature load dominating cold path)

**Outcome (when eventually done):** Same relevance; lower `total_ms` on warm and/or cold paths per the Phase 0 comparison method. Before/after numbers recorded in Notes.

---

### Phase 1.5 — Hybrid tradeoff memo — **DO NOT DO YET** (parked)

*Unlock only after Phase 0 has `rank_embed_query_ms` / `rank_semantic_ms` / `embedding_state` on real traffic.*

- Short Decisions addendum in this doc: cost when warm vs cold fallback
- Default unless data forces otherwise: keep 0.4/0.6 hybrid; optimize embed path if embed dominates; do not add a user-facing lexical-only flag in the first optimization pass

**Outcome (when eventually done):** Written decision on hybrid keep vs change, grounded in measured ms — not vibes.

---

### Phase 2 — Deeper indexing — **DO NOT DO YET** (parked)

*Unlock only if Phase 1 ships and warm `total_ms` is still dominated by rank over a large `merged_index` (or cold feature load that projection could not fix).*

- Lexical: SQLite FTS5 (or equivalent) over name/description/server_id → candidate set, then existing TF-IDF/fusion
- Semantic: keep linear cosine on candidates in the first indexing PR (no ANN unless logs prove need)
- Invalidate with existing search-cache eviction + feature refresh / warmer hooks

**Outcome (when eventually done):** Candidate retrieval no longer full linear scan of the space catalog; warm p50 in an acceptable band relative to Phase 0 baseline (exact bar set from those numbers).

---

## Key files referenced

| File | Notes |
| ---- | ----- |
| [`crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools.rs) | Orchestrator; existing timing breakdown; enrichment currently unaccounted |
| [`crates/mcpmux-gateway/src/services/meta_tools/search_tools_index.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/search_tools_index.rs) | Active index build, cache write, embedding hydration |
| [`crates/mcpmux-gateway/src/services/tool_discovery_search.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery_search.rs) | Hybrid fusion 0.4/0.6; inner lexical timing to plumb |
| [`crates/mcpmux-gateway/src/services/discovery_rank.rs`](../../crates/mcpmux-gateway/src/services/discovery_rank.rs) | TF-IDF lexical ranking |
| [`crates/mcpmux-gateway/src/services/tool_discovery_index.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery_index.rs) | Full-space feature load → index |
| [`crates/mcpmux-gateway/src/services/meta_tools/meta_tool_common.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/meta_tool_common.rs) | `build_server_readiness_map` / installed meta — duplicate DB loads (Phase 1, not now) |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | Per-session `search_cache` |
| [`crates/mcpmux-gateway/src/services/embedding.rs`](../../crates/mcpmux-gateway/src/services/embedding.rs) | ONNX query/doc embed; `Ready` fallback |
| [`crates/mcpmux-gateway/src/services/embedding_warmer.rs`](../../crates/mcpmux-gateway/src/services/embedding_warmer.rs) | Sequential warm loop (Phase 1, not now) |
| [`crates/mcpmux-storage/src/repositories/server_feature_repository.rs`](../../crates/mcpmux-storage/src/repositories/server_feature_repository.rs) | `list_for_space` including `raw_json` |

---

## Related documentation

- dig-and-ask session (Jul 24, 2026) — backend search path + frontend search UX recon; this ticket is backend-only
- [`docs/planning/dev-to-main-port.md`](./dev-to-main-port.md) — historical note that tool embeddings + semantic search were ported as a fork feature
- [`docs/planning/dev-rebased-post-port-completion.md`](./dev-rebased-post-port-completion.md) — synonyms / inactive preview relevance work (not latency)
- [`docs/guide/tool-optimization.mdx`](../guide/tool-optimization.mdx) — agent-facing search → invoke flow (behavior docs, not perf)

---

## Notes

### Phase 0 baselines (fill after instrumentation ships)

| Scenario | `query_id` | `total_ms` | Dominant phase(s) | `index_cache_hit` | `include_inactive` / `scope_all` | `embedding_state` |
| -------- | ---------- | ---------- | ----------------- | ----------------- | -------------------------------- | ----------------- |
| Cold — first search in session | | | | | | |
| Warm — repeat same session | | | | | | |
| Widen — `include_inactive: true` | | | | | | |
| Widen — `scope: "all"` | | | | | | |

### Reassess gate

Do **not** start Phase 1 / 1.5 / 2 until:

1. Phase 0 code is shipped
2. The table above has real numbers from a fat Space
3. A short follow-up decides which parked phase (if any) is unlocked
