# search_tools Hybrid Semantic Ranking

**Last Updated:** May 29, 2026
**Status:** Shipped — Phases 1–4 committed on `docs/feature-set-consent-model` (c612af7, ab8cf8e, 91d6942, e7e191e)
**Branch:** `docs/feature-set-consent-model` (stacks on consent-model work)
**Base branch:** `docs/feature-set-consent-model`
**Depends on:** [`search-tools-latency-and-root-race.md`](./search-tools-latency-and-root-race.md) Phase 8 (per-session active index cache) — the embedding cache layers onto the same `(session_id, fingerprint)` keying
**Supersedes:** TF-IDF ranking introduced in [`meta-gateway-invoke.md`](./meta-gateway-invoke.md) Phase D — this doc replaces the `substring-prefilter → TF-IDF rerank` pipeline in `discovery_rank.rs`

---

## Problem

`mcpmux_search_tools` is the agent's primary discovery path — the whole meta-gateway model assumes the LLM searches, reads a schema, then invokes. That makes search relevance the load-bearing wall of the whole design. The current implementation has two distinct failures:

| Symptom | Root cause |
| ------- | ---------- |
| `search_tools("list folder")` → `total: 0` even though `canva_list-folder-items` is active | `filter_and_rank` gates on a **contiguous substring** (`haystack.contains("list folder")`). `list-folder-items` tokenizes with hyphens, so the space-separated phrase never matches. Multi-word queries get *stricter*, not smarter. |
| `search_tools("post a jira comment")` misses `create_issue_comment` | Pipeline is **pure lexical**. Zero token overlap → zero score, even though intent is a perfect match. This is the dominant agent failure mode: agents query by intent, not by tool name. |
| Ranking quality is mediocre even on hits | Hand-rolled TF-IDF in `discovery_rank.rs` has no length normalization; long descriptions distort scores. No field weighting (a name match ranks the same as a buried description match). |

Published evals quantify the gap: lexical (BM25/TF-IDF) tool retrieval lands ~14% top-1 / ~21% top-5, while semantic embedding retrieval hits ~92% top-1 / ~84% top-5 across thousands of tool actions (StackOne, 9,340 actions). The ecosystem has converged on **hybrid lexical + embedding** retrieval for agent tool discovery (MCPFind, meta-mcp-search, ToolCompass, Anthropic Tool Search Tool).

The consumer here is an LLM, not a human typing filename fragments — so the intent gap, not the substring bug, is the real ceiling. But both need fixing.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Overall approach | **Hybrid: lexical recall + embedding rerank, score-fused** | Lexical keeps exact-name/ID precision (`canva_list-folder-items` as a literal); embeddings close the intent gap. Neither alone is sufficient — pure vector blurs exact identifiers, pure lexical misses intent. |
| 2 | Lexical layer | **Token-overlap match (replace contiguous-substring gate), keep TF-IDF for the lexical score** | Smallest change that kills the `"list folder"` → 0 bug. Tokenize query, match on token presence (OR), AND-boost. No new dep for the lexical half. Reuses existing `tokenize` in `discovery_rank.rs`. |
| 3 | Embedding library | **`fastembed-rs`** (ONNX, CPU), default model `bge-small-en-v1.5` (~67 MB) | Mature Rust crate, CPU inference, no API calls — fits the offline-first, loopback-only posture. Quantized small model keeps binary/memory tax bounded. Model is downloaded once on first use, not bundled. |
| 4 | Fallback when model absent | **Lexical-only path is the default; embeddings layer on when the model is present** | First run, air-gapped installs, or download-in-progress must still return results. Search never hard-fails on a missing model. This is the decisive reason hybrid wins over pure-vector. |
| 5 | Score fusion | **Weighted sum, lexical-normalized + cosine, default `0.4 lexical / 0.6 semantic`** | Tunable constant, not learned (we have no labeled eval set yet). Lexical weight keeps exact-name hits from being drowned by semantic noise. Weights live in one constant for easy tuning. |
| 6 | Embedding corpus text | **`feature_name + qualified_name + description`** (same haystack as today) | Keep one source of truth for searchable text. Name tokens give the embedding lexical anchoring; description gives intent. |
| 7 | Embedding cache | **Per-binding embedding cache keyed on `(session_id, fingerprint)`** — same key as the active index cache | Embeddings are computed once per binding, not per query. Slots directly into the Phase 8 `search_cache` from the latency doc; evicted by the same events (`WorkspaceBindingChanged`, session disconnect). |
| 8 | When to embed | **Lazily, on first `search_tools` with a query for a given binding** | Avoids embedding cost on bindings that are never searched. Index build (DB → `ToolIndexEntry`) stays eager; embedding generation is deferred to first query. |
| 9 | Model lifecycle | **Download on first semantic search, cached under app data dir; surface download state, don't block** | No bundling (keeps installer small). While downloading, search runs lexical-only and annotates `ranking: "lexical"` in the payload so the agent/UI knows. |
| 10 | `include_inactive` scans | **Lexical-only for inactive widening; semantic rerank applies to active set only (for now)** | Inactive scans already hit thousands of tools (see latency doc); embedding all of them per query is wasteful. Active set is the hot path for relevance. Revisit if measured as needed. |
| 11 | Eval harness | **Ship a small fixture-based relevance test set** (intent query → expected tool) | We have no labeled data. A ~20-case fixture lets us tune fusion weights and catch regressions. Lives in the integration test crate, not prod. |

---

## What this is NOT

- Not a change to the `search → schema → invoke` meta surface, the FeatureSet/consent model, or invoke authorization
- Not a remote/API embedding service — local ONNX only, no network calls at query time
- Not a vector database (Qdrant/HNSW) — corpus is hundreds of tools per binding; brute-force cosine over an in-memory `Vec` is fine. Revisit only if a binding exceeds low-thousands of *active* tools
- Not re-ranking `include_inactive` discovery results semantically (Decision 10)
- Not a learned fusion model or fine-tuned embedder — fixed weights + off-the-shelf model (Decision 5)
- Not replacing the `strsim` Levenshtein "did you mean" path on invoke — that stays as-is

---

## Architecture

### Pipeline (active-set query)

```text
search_tools({ query })
  │
  ├─ build/fetch ToolIndexEntry[]            (existing, cached per binding)
  │
  ├─ LEXICAL: token-overlap filter + TF-IDF score   (discovery_rank.rs)
  │     → candidates with lexical_score
  │
  ├─ SEMANTIC (if model present):
  │     ├─ fetch/compute per-binding doc embeddings  (cached, fingerprint-keyed)
  │     ├─ embed query
  │     └─ cosine(query, doc) → semantic_score
  │
  ├─ FUSE: 0.4 * norm(lexical) + 0.6 * semantic       (skipped if model absent)
  │
  └─ sort by fused score → paginate → annotate ranking: "hybrid" | "lexical"
```

### Lexical layer change (`discovery_rank.rs`)

```text
BEFORE (filter_and_rank):
  keep entry IF haystack.to_lowercase().contains(query_lower)   ← contiguous gate

AFTER:
  query_tokens = tokenize(query)
  keep entry IF any(token ∈ doc_tokens)                          ← token-overlap (OR)
  lexical_score = tf_idf_score(...) + and_boost(all tokens present)
```

### Embedding cache shape

```text
MetaToolContext (extends Phase 8 search_cache):
  search_cache:     DashMap<session_id, (FsFingerprint, ToolIndex)>          (existing)
  embedding_cache:  DashMap<session_id, (FsFingerprint, Vec<DocEmbedding>)>  (new)

DocEmbedding = { qualified_name: String, vector: Vec<f32> }

Eviction: same sites as search_cache —
  WorkspaceBindingChanged → evict (session)
  session disconnect       → evict (session)
```

### Model wrapper

```text
EmbeddingService (new, gateway services layer):
  - lazy TextEmbedding init (fastembed), guarded by OnceCell / async lock
  - state: NotDownloaded | Downloading | Ready | Failed
  - embed_documents(&[String]) -> Vec<Vec<f32>>
  - embed_query(&str) -> Vec<f32>
  - cosine(a, b) -> f32   (or use a small helper)
  Search callers check state(); Ready → hybrid, else → lexical-only.
```

---

## Observability

The whole point of hybrid ranking is that a query takes several hops (lexical filter → embedding lookup → fusion → pagination), and when a result looks wrong it must be obvious *which hop* produced it. Every stage emits one structured `tracing` event with a shared correlation field so a single query can be followed end to end.

### Conventions

- Structured `tracing` with field syntax (`key = value`, `%var`) and a `[component]` prefix — matches existing gateway logs (e.g. `[meta_tools]`, `[FeatureSetResolver]`).
- New target prefix: **`[search]`** for ranking/fusion, **`[embed]`** for the embedding service.
- **Correlation key:** every event in one `search_tools` call carries `query_id` (short random id minted at the top of `SearchToolsTool::call`) plus `session_id` and `fingerprint`. Grep one `query_id` → see the full path.
- **Never log the raw query at `info`.** Query text can contain user intent/PII — log it only at `debug`, and log a `query_len` + token count at `info`. Follows the secret-handling posture in `AGENTS.md`.
- Hot-path per-entry scoring is `trace` only; per-query summaries are `debug`/`info`.

### What each stage logs

| Stage | Level | Target | Fields |
| ----- | ----- | ------ | ------ |
| Call entry | `info` | `[search]` | `query_id`, `session_id`, `fingerprint`, `query_len`, `detail_level`, `limit`, `include_inactive` |
| Cache decision | `debug` | `[search]` | `query_id`, `index_cache = hit\|miss`, `embedding_cache = hit\|miss\|skipped`, `active_tools` |
| Embedding state | `info` | `[embed]` | `query_id`, `model_state = ready\|downloading\|absent\|failed`, `docs_embedded`, `embed_ms` (on compute), `cached` (on reuse) |
| Lexical pass | `debug` | `[search]` | `query_id`, `tokens`, `candidates_after_filter`, `and_boost_hits` |
| Fusion | `debug` | `[search]` | `query_id`, `ranking = hybrid\|lexical`, `lexical_weight`, `semantic_weight` |
| Per-entry score | `trace` | `[search]` | `query_id`, `qualified_name`, `lexical_score`, `semantic_score`, `fused_score` |
| Result summary | `info` | `[search]` | `query_id`, `ranking`, `total`, `returned`, `top_qualified_name`, `top_fused_score`, `total_ms` |
| Model lifecycle | `info` | `[embed]` | `model`, `state` transitions (`Downloading → Ready`/`Failed`), `download_ms`, `error` (on fail) |

### Surfaced to the agent

The result payload already gains `ranking: "hybrid" | "lexical"` (Decision 9). That's the one observability signal the *agent* sees — so a model that's still downloading is visible in-band, not just in logs. Internal scores stay in logs, not the payload.

---

## Files to create / modify

| File | Change |
| ---- | ------ |
| `crates/mcpmux-gateway/Cargo.toml` | Add `fastembed` dependency (default features, CPU/ONNX) |
| [`crates/mcpmux-gateway/src/services/discovery_rank.rs`](../../crates/mcpmux-gateway/src/services/discovery_rank.rs) | Replace contiguous-substring gate in `filter_and_rank` with token-overlap match; expose `lexical_score` so the fuser can read it; keep `tf_idf_score` as the lexical scorer |
| `crates/mcpmux-gateway/src/services/embedding.rs` | **New** — `EmbeddingService`: lazy model init, download-state machine, `embed_documents`, `embed_query`, cosine helper; emit `[embed]` lifecycle/state events |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs) | `search` gains an optional embedding path: compute/fetch doc embeddings, fuse lexical + semantic, sort by fused score; annotate `ranking` in result; emit `[search]` lexical/fusion/per-entry events |
| [`crates/mcpmux-gateway/src/services/mod.rs`](../../crates/mcpmux-gateway/src/services/mod.rs) | Export `EmbeddingService` |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | Add `embedding_cache` + `EmbeddingService` handle to `MetaToolContext`; extend cache-eviction helper to clear embeddings too |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | `SearchToolsTool::call` wires the embedding cache + service into `search`; add `ranking` field to payload |
| [`crates/mcpmux-gateway/src/consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs) | On `WorkspaceBindingChanged`, evict embedding cache entry alongside the active index entry |
| `tests/rust/tests/integration/meta_tools.rs` | Hybrid ranking tests + relevance fixture set (intent query → expected tool); lexical-only fallback when model absent |
| `crates/mcpmux-gateway/src/services/discovery_rank.rs` (tests) | Unit tests for token-overlap match + AND-boost + fusion math |

---

## Phasing

### Phase 1 — Lexical fix (token-overlap)

**Effort:** ~2 hours

- Replace the contiguous-substring gate in `filter_and_rank` with token-overlap (OR match), add an AND-boost when all query tokens are present
- Keep TF-IDF as the lexical scorer; expose the score for later fusion
- Mint `query_id` at `SearchToolsTool::call` entry; add the **call-entry**, **lexical-pass**, and **result-summary** events from the Observability table (lexical-only path — `ranking = lexical`)
- Unit tests: `"list folder"` matches `canva_list-folder-items`; `"xyznotreal"` still returns zero; multi-token ranking favors all-tokens-present docs

**Outcome:** `search_tools("list folder")` returns `canva_list-folder-items` instead of `total: 0`. The screenshot bug is dead. No new dependency, no behavior change for single-token queries that already worked.

### Phase 2 — Embedding service + model lifecycle

**Effort:** ~1 day

- Add `fastembed` dep; implement `EmbeddingService` with lazy init and a `NotDownloaded | Downloading | Ready | Failed` state machine
- Model downloads on first request to app data dir; never blocks search (state is checked, not awaited indefinitely)
- `embed_documents`, `embed_query`, cosine helper with unit coverage on known vectors
- Add the **model-lifecycle** and **embedding-state** events from the Observability table (`[embed]` target — state transitions, `download_ms`, `embed_ms`, failures)

**Outcome:** `EmbeddingService::state()` reports `Ready` after first model fetch; embedding a query and a matching doc yields higher cosine than a non-matching doc. Search still runs (lexical-only) while state is not `Ready`.

### Phase 3 — Hybrid fusion in search + embedding cache

**Effort:** ~1 day

- `ToolDiscoveryService::search` computes/fetches per-binding doc embeddings (cached, fingerprint-keyed), embeds the query, fuses `0.4 lexical / 0.6 semantic`, sorts by fused score
- Wire `embedding_cache` into `MetaToolContext`; evict on `WorkspaceBindingChanged` + session disconnect (same sites as `search_cache`)
- Payload annotates `ranking: "hybrid" | "lexical"`
- Add the **cache-decision**, **fusion**, and **per-entry score** (`trace`) events; upgrade the result-summary event to carry `ranking`, `top_fused_score`, `total_ms`
- Integration tests: cache hit on second query (no re-embed), eviction on binding change, fallback to `"lexical"` when model absent

**Outcome:** `search_tools("post a comment")` surfaces `*_create_issue_comment` in the top results on a binding where the active set contains it. Repeated queries in a session reuse cached embeddings. With the model removed/unavailable, search degrades cleanly to lexical and labels itself `"lexical"`.

### Phase 4 — Relevance eval + weight tuning

**Effort:** ~half day

- Add a fixture relevance set (~20 intent-query → expected-tool cases drawn from real bundles: Jira, GitHub, Canva, PostHog)
- Assert expected tool appears in top-3 for hybrid; record lexical-only baseline for comparison
- Tune the `0.4 / 0.6` fusion constant against the fixture; lock the chosen value with a comment explaining the tradeoff

**Outcome:** A repeatable relevance test guards against ranking regressions, and the fusion weight is chosen from data rather than guessed. CI fails if a known intent query drops its expected tool out of the top-3.

---

## Pre-PR validation

| Step | Command | Purpose |
| ---- | ------- | ------- |
| Full validate | `pnpm validate` | fmt, clippy, check, eslint, typecheck |
| Rust tests | `pnpm test:rust` | unit + integration incl. relevance fixtures |
| Cross-platform check | clippy on the ONNX/`fastembed` path for Win/macOS/Linux | `fastembed` pulls native ONNX runtime — verify the build resolves on the CI matrix before pushing |
| Manual smoke — intent | `search_tools("post a jira comment")` on a Jira binding returns the comment-creation tool in top results | Semantic relevance regression |
| Manual smoke — exact | `search_tools("canva_list-folder-items")` still ranks the literal tool first | Lexical-precision regression |
| Manual smoke — offline | Remove cached model → `search_tools` still returns results labeled `ranking: "lexical"` | Fallback path |
| Trace one query | `RUST_LOG=mcpmux_gateway=debug` (or `trace` for per-entry) → run a query, grep the logged `query_id` | Confirm every hop (entry → cache → embed → lexical → fusion → summary) is followable end to end and the raw query text never appears above `debug` |

---

## Out of scope

| Item | Reason |
| ---- | ------ |
| Semantic rerank of `include_inactive` results | Decision 10 — inactive scans hit thousands of tools; embedding per query is wasteful. Revisit if measured |
| Vector DB / HNSW / FAISS index | Brute-force cosine over hundreds of active tools is fast enough; no index infra needed at this scale |
| Fine-tuned / learned fusion or reranker | No labeled data yet; fixed weights + off-the-shelf model first. Revisit after the eval fixture grows |
| Usage-frequency boost (MFU-style) | Promising (MCPFind uses 15% MFU weight) but needs per-agent usage tracking — separate enhancement, file standalone |
| Bundling the embedding model in the installer | Keeps installer small; download-on-first-use is acceptable with the lexical fallback |
| Latency/cache/root-race work | Covered by [`search-tools-latency-and-root-race.md`](./search-tools-latency-and-root-race.md) — this doc layers onto its Phase 8 cache |

---

## Key files referenced

| File | Notes |
| ---- | ----- |
| [`crates/mcpmux-gateway/src/services/discovery_rank.rs`](../../crates/mcpmux-gateway/src/services/discovery_rank.rs) | `tokenize`, `tf_idf_score`, `filter_and_rank` — the contiguous-substring gate (~line 77) and TF-IDF scorer being reworked |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs) | `ToolDiscoveryService::search`, `ToolIndexEntry`, haystack construction (~line 170) |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | `SearchToolsTool::call`, active-index build + per-session cache wiring |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | `MetaToolContext`, `search_cache` (fingerprint-keyed) — embedding cache slots in here |
| [`crates/mcpmux-gateway/Cargo.toml`](../../crates/mcpmux-gateway/Cargo.toml) | Current deps (`strsim` present; no embedding/full-text lib yet) |

---

## Related work

- [`search-tools-latency-and-root-race.md`](./search-tools-latency-and-root-race.md) — Phase 8 per-session active index cache; this doc reuses its `(session_id, fingerprint)` keying and eviction sites for the embedding cache
- [`meta-gateway-invoke.md`](./meta-gateway-invoke.md) — original `search_tools` design; TF-IDF ranking introduced in Phase D (superseded here)
- [`feature-set-consent-model.md`](./feature-set-consent-model.md) — defines the invokable active-set that bounds the semantic-rerank corpus

---

## Reconciliation

**Shipped May 29, 2026** on commits `c612af7` (Phase 1), `ab8cf8e` (Phase 2), `91d6942` (Phase 3), `e7e191e` (Phase 4).

| Phase | Planned | Shipped | Deviation |
| ----- | ------- | ------- | --------- |
| 1 | Token-overlap + AND-boost; `[search]` tracing; `ranking: lexical` | Same | `filter_and_rank_traced` added to preserve 5-arg API for prompt/resource discovery callers |
| 2 | `fastembed` + `EmbeddingService` state machine | Same | BGESmallENV15 (non-quantized); background thread init; `#[ignore]` download test |
| 3 | Hybrid fusion + `embedding_cache` | Same | `data_dir` wired through registry/service_container (outside strict file list); semantic rerank active set only; cache-hit test pre-seeds embeddings for CI |
| 4 | ~20-case relevance fixture + weight tuning | Same | `EmbeddingService::install_test_vectors` test-utils stub; 0.4/0.6 retained (20/20 on fixture) |

**Post-ship fix (uncommitted):** `partial_feature_set_binding_limits_search_and_invoke` query updated `"issue"` → `"issues"` — token-overlap no longer matches `list_issues` via substring.

**Validation:** `pnpm validate` + `pnpm test:rust` green — 766 passed, 2 skipped (May 29, 2026).

**Manual QA still required:** intent smoke (`post a jira comment`), exact-name smoke (`canva_list-folder-items`), offline fallback (`ranking: lexical` without model), trace one query via `RUST_LOG=mcpmux_gateway=debug`.
