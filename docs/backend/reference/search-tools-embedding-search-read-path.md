# search_tools Embedding Search Read Path

**Last Updated:** May 30, 2026
**Status:** Planned
**Branch:** `docs/feature-set-consent-model`
**Base branch:** `docs/feature-set-consent-model`
**Depends on:** [`search-tools-persistent-embedding-cache.md`](./search-tools-persistent-embedding-cache.md) Phases 1–5 (shipped) + warmer write fix (`b68f672`, `run_spawn_blocking` → `block_in_place`)
**Blocks:** Consent QA [`consent-model-qa-runbook.md`](../../testing/consent-model-qa-runbook.md) Sections O1–O4 (blocked on O0b read-path pass)

---

## Problem

Persistent embedding cache Phases 1–5 shipped the write path (SQLite `tool_embeddings`, on-connect `EmbeddingWarmer`, global `DashMap<content_hash, Vec<f32>>`). O0b QA (May 30, 2026) confirmed the **warmer write path is fixed** after `run_spawn_blocking` was repaired — 27 servers warmed, **945 rows** in `tool_embeddings`, `warmer upserting records` fires, zero panics.

The **search read path still fails**: `mcpmux_search_tools` returns `ranking: "lexical"` even when the store is full.

| Symptom                                      | What O0b logs show                                    | Code interpretation                                                                                                                                                                                                                                                                         |
| -------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Search stuck `lexical` with 945 DB rows      | `[search] cache decision … embedding_store="skipped"` | `rank_with_hybrid` took the early exit: model **not** `Ready` **or** lexical `ranked` set empty before fusion (see `tool_discovery.rs` ~310–317).                                                                                                                                           |
| `store hydrate … hashes_requested=0`         | Looks like "hydrate did nothing"                      | **Misleading, not the root bug.** `hydrate_active_embeddings` only requests hashes **missing from the in-memory** `embedding_store`. After warming, all active `content_hash`es are already in the DashMap → `hashes_requested=0` is **expected** and returns early without hitting SQLite. |
| No `[search] read` / `vectors_present` lines | Hybrid never entered the fusion body                  | Confirms `embedding_store="skipped"` path — not `miss` (which would mean "tried store, zero vectors").                                                                                                                                                                                      |

O0 Run 1 (archived) was **`embedding_store="miss"`** with an empty store. O0b is a **different failure mode**: store populated, hybrid **not attempted** (`skipped`). Fixing O0 does not automatically fix O0b.

---

## Decisions

| #   | Decision              | Choice                                                                                                                                                                                                                                                                                                                                          | Rationale                                                                                                                                                                                                      |
| --- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Scope of this doc     | **Search read path only** — make `search_tools` use warmed vectors and return `ranking: "hybrid"` when the model is `Ready`                                                                                                                                                                                                                     | Warmer write + SQLite persistence are shipped and verified. No changes to fusion weights, consent model, or warmer triggers in this doc.                                                                       |
| 2   | Primary hypothesis    | **Instrument first, then fix the actual skip reason** — do not assume "index lacks content_hash" (O0b filing was imprecise)                                                                                                                                                                                                                     | `hashes_requested=0` with a warm DashMap is correct behavior. The fix target is why `rank_with_hybrid` logs `embedding_store="skipped"` despite a populated store.                                             |
| 3   | Skip-reason telemetry | **Split `embedding_store="skipped"` into explicit reasons** (`model_not_ready`, `empty_ranked`, `no_query`) in `[search] cache decision`                                                                                                                                                                                                        | Today one label hides three branches; QA cannot tell which fired without reading source.                                                                                                                       |
| 4   | Model-ready gate      | **When `tool_embeddings` / in-memory store has vectors for the active binding, search must not permanently stay lexical solely because the model was still `Downloading` at first connect** — call `ensure_init_started()` before search (already exists) and only skip hybrid when state is definitively not ready; log state at decision time | Agents often search seconds after gateway start; warm completes while model is still loading. Lexical fallback is fine **transiently**, but O0b searched after warm with rows present and still got `skipped`. |
| 5   | content_hash parity   | **Warmer and search must use the same hash inputs** — `EmbeddingService::content_hash(feature_name, description)` on `ServerFeature` (warmer) and `entry_content_hash` → same helper on `ToolIndexEntry` (search)                                                                                                                               | Already intended in Phase 2; add an integration assertion that a warmed tool's hash from `ServerFeature` matches the active index entry built from the same row.                                               |
| 6   | Hydrate logging       | **When `hashes_requested=0`, log `store_hits` = count of active tools already in memory** (not all zeros)                                                                                                                                                                                                                                       | Distinguishes "hydrate skipped because warm already filled DashMap" from "hydrate broken".                                                                                                                     |
| 7   | Temporary diag        | **Remove `[embed] diag:` warns** added during O0 investigation once read path passes O0b + integration tests                                                                                                                                                                                                                                    | They were scaffolding for the warmer bug; keep only durable `[embed]` / `[search]` events.                                                                                                                     |

---

## What this is NOT

- Not re-opening warmer write / `run_spawn_blocking` (fixed in `b68f672` unless read-path work regresses it)
- Not changing hybrid fusion math (`0.4` / `0.6`) or lexical token-overlap behavior
- Not embedding inactive-scan results semantically (unchanged from hybrid doc Decision 10)
- Not orphan prune for `tool_embeddings` (deferred in persistent-cache Phase 5)
- Not a new embedding model or remote API

---

## Architecture

### Intended read path (after persistent cache)

```text
search_tools({ query })
  │
  ├─ build / cache active ToolIndexEntry[]          (per-session index cache)
  ├─ hydrate_active_embeddings(active_index only)
  │     └─ for each content_hash not in DashMap → embedding_repo.get_many → insert
  │        (often no-op when warmer already filled DashMap)
  │
  ├─ rank_with_hybrid on lexical candidates
  │     ├─ if model Ready && ranked non-empty:
  │     │     vectors_present = active tools with hash in DashMap
  │     │     embed query (inline, spawn_blocking)
  │     │     fuse → ranking: "hybrid"
  │     └─ else → ranking: "lexical" (+ explicit skip reason in logs)
  │
  └─ paginate → payload includes ranking field
```

### O0b failure point (observed)

```text
Warmer ✅ → DashMap + SQLite populated (945 rows)
Search:
  hydrate → hashes_requested=0 (all hashes already in DashMap) ✅ expected
  rank_with_hybrid → embedding_store="skipped" ❌ hybrid never ran
  result → ranking="lexical"
```

### Target observability (one `query_id`)

```text
[embed] store hydrate … hashes_requested=0 store_hits=175 store_misses=0
[search] cache decision … embedding_store=hit (or in_memory_warm)
[search] read … vectors_present=175 lexical_only_docs=0
[embed] inline query embed … docs_embedded=1 embed_ms=…
[search] result summary … ranking="hybrid"
```

---

## Observability

| Stage                   | Level   | Target     | Fields                                                                                                            |
| ----------------------- | ------- | ---------- | ----------------------------------------------------------------------------------------------------------------- |
| Hydrate (all in memory) | `debug` | `[embed]`  | `query_id`, `hashes_requested=0`, `store_hits` (= active tools already in DashMap), `store_misses=0`              |
| Hybrid skip             | `debug` | `[search]` | `query_id`, `embedding_store=skipped`, **`skip_reason`** (`model_not_ready` \| `empty_ranked` \| `no_hybrid_ctx`) |
| Hybrid read             | `debug` | `[search]` | `query_id`, `active_tools`, `vectors_present`, `lexical_only_docs`                                                |
| Cache decision          | `debug` | `[search]` | `query_id`, `embedding_store` = `hit` \| `miss` \| `skipped` (+ reason when skipped)                              |
| Result                  | `info`  | `[search]` | `query_id`, `ranking`, `total_ms`                                                                                 |

---

## Files to create / modify

| File                                                                                                                     | Change                                                                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs)     | Split hybrid skip reasons in `log_cache_decision`; optional `vectors_present` pre-check logging before early return                                     |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | Improve `hydrate_active_embeddings` logging when `hashes_requested=0`; ensure `SearchContext` uses the same `active_index` slice for hydrate and hybrid |
| [`crates/mcpmux-gateway/src/services/embedding.rs`](../../crates/mcpmux-gateway/src/services/embedding.rs)               | Remove temporary `[embed] diag:` warns after verification (or gate behind `debug` assert feature)                                                       |
| [`crates/mcpmux-gateway/src/services/embedding_warmer.rs`](../../crates/mcpmux-gateway/src/services/embedding_warmer.rs) | Remove temporary warmer `diag` warns after verification                                                                                                 |
| `tests/rust/tests/integration/meta_tools.rs`                                                                             | **New:** after warm + populated store, `search_tools` returns `ranking: "hybrid"` and logs show `vectors_present > 0`                                   |
| `tests/rust/tests/integration/search_relevance_eval.rs` (if exists)                                                      | Re-run fixture eval under store-backed path (no regression)                                                                                             |
| [`docs/planning/consent-model-qa-runbook.md`](../../testing/consent-model-qa-runbook.md)                                 | O0b pass criteria update + unblock O1–O4 when read path fixed                                                                                           |

---

## Phasing

### Phase 1 — Diagnose: explicit hybrid skip reasons

**Effort:** ~2 hours

- Add `skip_reason` (or distinct `embedding_store` enum strings) to `log_cache_decision` for the three early-exit branches in `rank_with_hybrid`
- Log `EmbeddingState` at search decision time on the `[search] cache decision` line
- Fix `hydrate_active_embeddings` no-op log: when `hashes_requested=0`, compute and log `store_hits` = active tools already in DashMap (not hardcoded 0)
- Re-run O0b Step 4 once on a rebuilt binary; record which `skip_reason` fired

**Outcome:** A single `search_tools` call produces logs that state **why** hybrid was skipped (model state vs empty ranked vs no context). No more guessing from `embedding_store="skipped"` alone.

#### Phase 1 manual tests

Run **both** captures after a cold `dev:rebuild`:

**Test A — early call (model Downloading)**

Immediately after gateway start (before model download completes), tell the AI client:

> Call `mcpmux_search_tools` with `query: "list files in a folder"`

Expected `[search] cache decision` log:

```
embedding_store=skipped skip_reason=model_not_ready model_state=downloading active_tools=N
```

Expected `[embed] store hydrate` log:

```
hashes_requested=0 store_hits=N store_misses=0   ← N > 0 means DashMap pre-filled by warmer
```

**Test B — after model Ready**

Wait for `[embed] model = bge-small-en-v1.5, state = Ready` in the gateway log (~1–2 min on first run after download). Run the same query again.

If read path is working:

```
[embed] store hydrate    hashes_requested=0 store_hits=N store_misses=0
[search] cache decision  embedding_store=hit active_tools=N
[search] read            vectors_present=N lexical_only_docs=0
[search] result summary  ranking=hybrid
```

If still skipping after model Ready:

```
[search] cache decision  embedding_store=skipped skip_reason=empty_ranked model_state=ready
```

→ means `filter_and_rank` returned empty before hybrid ran — different fix path.

**Decision gate:** the `skip_reason` from Test B (model Ready, store warm) determines the Phase 2 fix direction.

### Phase 2 — Fix the confirmed root cause

**Effort:** ~half day (depends on Phase 1 finding)

**Likely fixes (implement the one Phase 1 proves):**

| If Phase 1 shows                           | Fix direction                                                                                                                                                                                                                                                                     |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `model_not_ready` while store has vectors  | Do not treat "warm DashMap full + model Downloading" as permanent lexical: after `ensure_init_started()`, if store has vectors for active hashes, wait briefly or re-check `Ready` before skip; or allow hybrid when `vectors_present > 0` and only query embed waits for `Ready` |
| `empty_ranked` with `total > 0`            | Bug in `filter_and_rank` vs hybrid input — ensure `ranked` is non-empty when lexical matches exist                                                                                                                                                                                |
| `vectors_present=0` with `hit` store in DB | **content_hash mismatch** — align `ToolIndexEntry` description/name sourcing with warmer's `ServerFeature` rows; add unit test                                                                                                                                                    |
| `no_hybrid_ctx`                            | Wire `SearchContext` when query present (should not happen for normal search)                                                                                                                                                                                                     |

**Outcome:** On a gateway with completed warm + model `Ready`, `mcpmux_search_tools({ "query": "list folder" })` returns `ranking: "hybrid"` and `[search] read` shows `vectors_present > 0`.

### Phase 3 — Hydrate telemetry + cleanup

**Effort:** ~2 hours

- When `missing_hashes` is empty, log `store_hits` = number of active tools whose hash is already in `embedding_store`
- Remove temporary `[embed] diag:` instrumentation from `embedding.rs` and `embedding_warmer.rs`
- Update consent QA runbook O0b verdict text (clarify `hashes_requested=0` vs read-path bug)

**Outcome:** Logs read cleanly for QA; no diag noise in production paths. `store hydrate` with `hashes_requested=0` reads as success when warmer pre-filled memory.

### Phase 4 — Integration tests + QA unblock

**Effort:** ~half day

- Integration test: seed or warm vectors for github fixture → `search_tools` → `ranking == "hybrid"` (extend `connect_event_warms_server_catalog_embeddings_before_search` or add store-backed variant without test vector stubs for docs)
- Re-run consent QA **O0b** → expect FIX VERIFIED; then **O1–O4** in order
- Reconcile this doc (status → Shipped, reconciliation section)

**Outcome:** CI guards read path; O1–O4 unblocked in runbook; persistent-cache feature end-to-end complete (warm → persist → search hybrid).

---

## Pre-PR validation

| Step       | Command / action                                                       | Pass criteria                                                     |
| ---------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Rust tests | `cargo test -p tests connect_event_warms` + new hybrid-after-warm test | green                                                             |
| Lint       | `pnpm lint` (gateway crate)                                            | no new warnings                                                   |
| O0b manual | Cold `dev:rebuild` → wait for warm → one `search_tools`                | `ranking: hybrid`, `vectors_present > 0`, `hashes_requested=0` OK |
| O1 manual  | Second session, same query                                             | fast, `store_hits > 0`, hybrid                                    |
| O2 manual  | Restart → search                                                       | hybrid without re-warm spike                                      |
| Trace      | `grep query_id` on one search                                          | hydrate → read → inline query → fusion → summary                  |

---

## Out of scope

| Item                                                               | Reason                                                                        |
| ------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| Warmer duplicate enqueue (`Connected` + `ServerFeaturesRefreshed`) | Noisy but correct; dedupe optimization separate                               |
| Inline embed-on-search write-through for cache misses              | Persistent-cache doc deferred; warm + hydrate sufficient once read path works |
| Learned fusion weights                                             | No labeled eval pipeline                                                      |

---

## Key files referenced

| File                                                                                                                     | Notes                                                     |
| ------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs)     | `rank_with_hybrid`, `SearchContext`, `entry_content_hash` |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | `hydrate_active_embeddings`, `SearchToolsTool::call`      |
| [`crates/mcpmux-gateway/src/services/embedding_warmer.rs`](../../crates/mcpmux-gateway/src/services/embedding_warmer.rs) | Write path (verified)                                     |
| [`crates/mcpmux-gateway/src/services/embedding.rs`](../../crates/mcpmux-gateway/src/services/embedding.rs)               | `run_spawn_blocking` / model state machine                |
| [`docs/planning/consent-model-qa-runbook.md`](../../testing/consent-model-qa-runbook.md)                                 | O0 Run 1 + O0b findings                                   |
| [`docs/planning/search-tools-persistent-embedding-cache.md`](./search-tools-persistent-embedding-cache.md)               | Parent feature (write path)                               |

---

## Related documentation

- [`search-tools-persistent-embedding-cache.md`](./search-tools-persistent-embedding-cache.md) — parent plan (Phases 1–5 shipped)
- [`search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md) — fusion pipeline and lexical layer
- [`consent-model-qa-runbook.md`](../../testing/consent-model-qa-runbook.md) — Section O0b gates O1–O4
- [`search-tools-latency-and-root-race.md`](./search-tools-latency-and-root-race.md) — per-session active index cache (unchanged)

---

## Reconciliation

_To be filled when this doc ships — planned vs shipped, validation results, link to commit(s)._
