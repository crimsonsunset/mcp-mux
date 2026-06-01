# search_tools Persistent Embedding Cache

**Last Updated:** May 30, 2026
**Status:** Shipped (Phases 1-5 complete)
**Branch:** `docs/feature-set-consent-model` (continues the search-tools work)
**Base branch:** `docs/feature-set-consent-model`
**Depends on:** [`search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md) Phases 2–3 (shipped) — the `EmbeddingService` and the hybrid fusion path this re-keys
**Supersedes:** Decisions 6, 7 & 8 of [`search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md) — the per-session `(session_id, fingerprint)` embedding cache, the lazy-on-first-query embedding trigger, and the alias-coupled embedding haystack are all replaced here

---

## Problem

Hybrid ranking shipped with the embedding cache keyed on `session_id` and held only in memory. The unit of caching is wrong, and the cost recurs far more than it should:

| Symptom                                                                                                                 | Root cause                                                                                                                                                                                                                                                        |
| ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Every new Cursor chat re-embeds the entire active corpus from scratch (~30 s, all-core CPU spike on a 669-tool binding) | `embedding_cache: DashMap<session_id, (fingerprint, Vec<DocEmbedding>)>` is keyed by **session**. Two chats with identical workspace + feature sets share nothing. `fingerprint` is only a validity tag _within_ a session.                                       |
| App restart pays the full re-embed again                                                                                | The cache lives on `SessionRootsRegistry` in memory only. Nothing is persisted; a relaunch starts cold.                                                                                                                                                           |
| The 30 s embedding pass pegs the request thread                                                                         | `EmbeddingService::embed_documents` runs ONNX inference **synchronously on the async runtime thread** that's handling the `search_tools` call.                                                                                                                    |
| Renaming a server alias would churn embeddings (latent)                                                                 | The embedded haystack is `feature_name + qualified_name + description`, and `qualified_name = {server_alias‖server_id}_{feature_name}`. A user-mutable alias is baked into the embedded text, so a cosmetic rename would invalidate every vector for that server. |

The embedding of a tool is a **pure, deterministic function of its text and the model** — it does not depend on session, workspace, or feature set. Keying by session throws away globally reusable work. Confirmed in code: the embed corpus is built from live `ServerFeature` rows (`entry_search_haystack`), the per-session cache is the only reuse mechanism, and footprint is trivial (~1 MB for the full catalog: tools × 384 dims × 4 B).

---

## Decisions

| #   | Decision                  | Choice                                                                                                                                       | Rationale                                                                                                                                                                                                                                               |
| --- | ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Caching unit              | **Per-tool, keyed by `content_hash + model_version`** — global, deduped, scope-free                                                          | The vector is a pure function of the embedded text. A global per-tool key gives strictly more reuse than session/workspace/featureset scoping and dedupes identical tool text across servers/spaces. Replaces the per-session `embedding_cache`.        |
| 2   | Persistence               | **SQLite table `tool_embeddings` in `mcpmux-storage`, unencrypted, survives restart**                                                        | Embeddings carry no secret material, so they sit outside field-level encryption. Surviving restart turns the cold re-embed from "every launch" into "once ever, until text or model changes".                                                           |
| 3   | Skip-if-unchanged         | **Content-hash lookup is the skip mechanism** — hit → skip, miss → embed once + upsert                                                       | This is the "smart enough to know if it needs to re-run" behavior, for free. Text edit → new hash → re-embed only that tool. Model upgrade → `model_version` bumps → corpus re-warms incrementally. Tool removed → orphan row, pruned later.            |
| 4   | Hash function             | **Stable cryptographic hash (SHA-256 via `ring`, already a storage dep) over the embedding text**                                            | Persisted keys must be stable across process restarts and platforms. `std::hash::DefaultHasher` (used by `feature_set_ids_fingerprint`) is explicitly _not_ stable and must not key a persisted store.                                                  |
| 5   | Alias-free embedding text | **Decouple the embedding/hash haystack from the lexical haystack; embedding text = `feature_name + description` (no alias-derived prefix)**  | Makes alias renames free (Decision 1's hash only changes on real semantic change). The lexical haystack keeps `qualified_name` for prefix matching, so lexical precision is unaffected.                                                                 |
| 6   | Population strategy       | **On-connect incremental pre-warm** — driven by existing `ServerFeaturesDiscovered` / connect events, background worker, bounded concurrency | Spreads cost across the natural staggered connect timeline (no boot spike, no inline search spike). Fires _after_ discovery, so it embeds live/correct tool text. Naturally handles new servers and the "fingerprint doesn't capture availability" gap. |
| 7   | Off the hot path          | **All ONNX inference runs in `spawn_blocking`** (warm worker and inline query embed)                                                         | Stops the CPU pass from pegging the async runtime thread serving the request.                                                                                                                                                                           |
| 8   | Corpus breadth            | **Full installed catalog**, not just the active/granted set                                                                                  | A tool's vector is identical wherever it appears; embedding the whole catalog once makes `include_inactive` searches and feature-set switches instant too, at ~1 MB total.                                                                              |
| 9   | Search read path          | **Read vectors from an in-process map hydrated from SQLite; query embedded inline (cheap); missing docs degrade to lexical**                 | Source of truth is the store; an in-memory `DashMap<content_hash, vector>` keeps reads fast. During the connect-warm window, any not-yet-warmed doc simply ranks lexical — the existing graceful-degradation path.                                      |

---

## What this is NOT

- Not a change to the `search → schema → invoke` surface, ranking math (`0.4/0.6` fusion), or the FeatureSet/consent model
- Not a new embedding model or a remote embedding API — same local `bge-small-en-v1.5` ONNX via `fastembed`
- Not a vector DB / ANN index — brute-force cosine over an in-memory `Vec` is still fine at this scale (Decision unchanged from the hybrid doc)
- Not a lazy embed-on-search write-through path — deferred (see Out of scope); the connect-warm + lexical degradation covers the gap window
- Not embedding `input_schema` / params — the embedded text stays name + description (now alias-free)
- Not upstream/registry-precomputed vectors — rejected during brainstorm (live tool text diverges from registry definitions; client embedder still required)

---

## Architecture

### Caching unit — before vs after

```text
BEFORE (hybrid doc, shipped):
  embedding_cache: DashMap<session_id, (fingerprint, Vec<DocEmbedding>)>   in-memory, per session
    → re-embed per session, lost on restart, ONNX on the request thread

AFTER (this doc):
  SQLite  tool_embeddings(content_hash, model_version) -> vector           durable, global, deduped
  memory  DashMap<content_hash, Vec<f32>>                                  hot read mirror, hydrated from SQLite
    → embed once per (text, model), reused across all sessions + restarts
```

### Two haystacks (Decision 5)

```text
lexical haystack  (discovery_rank.rs, unchanged):
    feature_name + qualified_name + description        ← keeps alias/prefix for token matching

embedding haystack (new, alias-free):
    feature_name + description                          ← no {alias}_ prefix
    content_hash = sha256(embedding_haystack)           ← stable, alias-rename-proof
```

### Storage shape

```sql
CREATE TABLE IF NOT EXISTS tool_embeddings (
    content_hash   TEXT    NOT NULL,   -- sha256 hex of the embedding haystack
    model_version  TEXT    NOT NULL,   -- e.g. "bge-small-en-v1.5"
    vector         BLOB    NOT NULL,   -- f32[dims], little-endian
    dims           INTEGER NOT NULL,
    created_at     INTEGER NOT NULL,
    PRIMARY KEY (content_hash, model_version)
);
```

Lives in the existing (field-encryption, not whole-DB-encrypted) SQLite database. Accessed only through a repository trait — no SQLx from gateway/app code, per `AGENTS.md`.

### Component flow

```text
EmbeddingRepository (mcpmux-core trait):
  get_many(&[content_hash], model_version) -> Vec<(content_hash, Vec<f32>)>
  upsert_many(&[(content_hash, model_version, Vec<f32>)])

SqliteEmbeddingRepository (mcpmux-storage): impls the trait over tool_embeddings.

EmbeddingWarmer (gateway, new) — subscribes to ServerFeaturesDiscovered / connect:
  1. build embedding haystack + content_hash for each catalog tool of the server
  2. get_many → diff against store → keep misses only        (skip-if-unchanged)
  3. spawn_blocking embed misses (bounded concurrency)
  4. upsert_many → SQLite, and insert into in-memory map

SearchToolsTool / tool_discovery::search (re-keyed):
  - hydrate in-memory map from repo on first need (by content_hash)
  - per active doc: vector = map.get(content_hash); miss → lexical-only for that doc
  - embed query inline via spawn_blocking (single vector, ~ms)
  - fuse + sort as today; annotate ranking: hybrid | lexical
  - per-session embedding_cache + fingerprint embedding keying removed
```

---

## Observability

Extends the existing `[search]` / `[embed]` targets and `query_id` correlation from the hybrid doc. New events cover the warm path and the store.

| Stage              | Level   | Target     | Fields                                                                    |
| ------------------ | ------- | ---------- | ------------------------------------------------------------------------- |
| Warm enqueue       | `debug` | `[embed]`  | `server_id`, `catalog_tools`, `missing` (after store diff)                |
| Warm batch done    | `info`  | `[embed]`  | `server_id`, `embedded`, `skipped_present`, `embed_ms`, `model_version`   |
| Store hydrate      | `debug` | `[embed]`  | `query_id`, `hashes_requested`, `store_hits`, `store_misses`              |
| Search read        | `debug` | `[search]` | `query_id`, `active_tools`, `vectors_present`, `lexical_only_docs`        |
| Inline query embed | `info`  | `[embed]`  | `query_id`, `docs_embedded = 1`, `embed_ms` (now always `spawn_blocking`) |

Same secret-handling posture: never log raw tool text or query above `debug`.

---

## Files to create / modify

| File                                                                                                                           | Change                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-core/src/repository/mod.rs`](../../crates/mcpmux-core/src/repository/mod.rs)                                   | Add the `EmbeddingRepository` trait (`get_many`, `upsert_many`) + `EmbeddingRecord` type, alongside the existing repository traits                              |
| `crates/mcpmux-storage/src/repositories/embedding_repository.rs`                                                               | **New** — `SqliteEmbeddingRepository`; SHA-256 helper via `ring`; BLOB ↔ `Vec<f32>` codec                                                                       |
| [`crates/mcpmux-storage/src/repositories/mod.rs`](../../crates/mcpmux-storage/src/repositories/mod.rs)                         | Register the new repository module                                                                                                                              |
| `crates/mcpmux-storage/src/migrations/021_tool_embeddings.sql`                                                                 | **New** — `tool_embeddings` table                                                                                                                               |
| [`crates/mcpmux-storage/src/migrations.rs`](../../crates/mcpmux-storage/src/migrations.rs)                                     | Register migration `021`                                                                                                                                        |
| [`crates/mcpmux-gateway/src/services/embedding.rs`](../../crates/mcpmux-gateway/src/services/embedding.rs)                     | Wrap `embed_documents` / `embed_query` ONNX calls in `spawn_blocking`; expose `model_version()`; add `embedding_haystack` + `content_hash` helpers              |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs)           | Split lexical vs embedding haystack; read vectors by `content_hash` from the in-memory map; degrade missing docs to lexical; drop per-session embedding compute |
| `crates/mcpmux-gateway/src/services/embedding_warmer.rs`                                                                       | **New** — background warmer: event-driven, store-diff, `spawn_blocking` embed, bounded concurrency, upsert + map insert                                         |
| [`crates/mcpmux-gateway/src/consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs)             | On `ServerFeaturesDiscovered` / connect, enqueue that server's catalog into the warmer                                                                          |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | Replace `embedding_cache` (per-session) with the global `DashMap<content_hash, Vec<f32>>` + `EmbeddingRepository` handle on `MetaToolContext`                   |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs)       | `SearchToolsTool::call` — remove per-session embedding keying; hydrate from store; inline query embed via `spawn_blocking`                                      |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs)             | Drop the `embedding_cache` field + its eviction sites (search/active-index cache stays)                                                                         |
| `tests/rust/tests/integration/meta_tools.rs`                                                                                   | Cross-session reuse, restart reuse (repo round-trip), alias-rename-is-free, model-version invalidation, warm-then-search                                        |

---

## Phasing

### Phase 1 — Embedding repository + persistence

**Effort:** ~1 day

- Add `EmbeddingRepository` trait to `mcpmux-core`; implement `SqliteEmbeddingRepository` over a new `tool_embeddings` migration
- SHA-256 content hashing via `ring`; `Vec<f32>` ↔ BLOB codec with explicit endianness
- Repo unit tests: upsert + get round-trip, `(content_hash, model_version)` PK upsert semantics, missing-hash returns empty

**Outcome:** Vectors written through the repo survive a process restart and round-trip byte-identical. `get_many` for unknown hashes returns empties; re-upserting the same hash is idempotent.

### Phase 2 — Alias-free embedding text + off-hot-path inference

**Effort:** ~half day

- Introduce the dedicated embedding haystack (`feature_name + description`) distinct from the lexical haystack; compute `content_hash` from it
- Wrap all ONNX inference in `EmbeddingService` in `spawn_blocking`; expose `model_version()`
- Unit tests: changing only a server alias leaves `content_hash` unchanged; changing the description changes it

**Outcome:** Renaming a server's alias does not change any tool's `content_hash` (no spurious re-embed). Embedding inference no longer runs on the async runtime thread — a query no longer pegs the request worker.

### Phase 3 — Global store-backed search read path

**Effort:** ~1 day

- Replace the per-session `embedding_cache` with a global `DashMap<content_hash, Vec<f32>>` on `MetaToolContext`, hydrated from `EmbeddingRepository`
- `tool_discovery::search` reads vectors by `content_hash`; docs absent from the store rank lexical-only; the query is embedded inline (single vector)
- Remove the per-session embedding keying from `SearchToolsTool::call` and the `embedding_cache` field from `SessionRootsRegistry`
- Integration tests: a second session with the same binding does **zero** re-embedding (store hit); cold-start after restart reuses persisted vectors

**Outcome:** Embeddings are computed at most once per `(text, model)`. A new chat, a feature-set switch, or an app restart reuses existing vectors with no 30 s spike. With an empty store, search still returns results labeled `ranking: lexical`.

### Phase 4 — On-connect incremental warmer

**Effort:** ~1 day

- New `EmbeddingWarmer` subscribed to `ServerFeaturesDiscovered` / connect events; per server it diffs the catalog against the store and embeds only misses, `spawn_blocking`, with bounded concurrency
- Embeds the **full catalog** for each server as it connects (Decision 8), not just the active set
- Integration test: connect a server → its catalog is embedded in the background → a subsequent `search_tools` (active or `include_inactive`) finds vectors already present (store hit, no inline corpus embed)

**Outcome:** After a server connects, its tools are embedded off the hot path before the user typically searches. The first real search is warm; the all-core inline spike is gone. New servers self-warm on connect; unchanged servers skip entirely.

### Phase 5 — Observability, pruning & reconciliation

**Effort:** ~half day

- Add the warm/hydrate `[embed]` / `[search]` events from the Observability table
- Optional orphan prune (**deferred — not implemented**, see scope note in the Reconciliation section): drop `tool_embeddings` rows whose `content_hash` is unreferenced by any current catalog tool (age- or count-bounded)
- **Reconcile this planning doc** via [`update-planning-md`](../../.cursor/commands/update-planning-md.md) — fill the Reconciliation section with shipped commits, planned-vs-shipped deltas, validation results, and outstanding manual QA. This step is non-optional; the plan is not complete until the doc reflects what was actually built.

**Outcome:** A single `query_id` can be followed from store hydrate → read → fusion, warm passes are observable, and the planning doc matches the shipped code. (Orphan pruning was deferred, so the store can still accumulate stale rows after description edits / model bumps — bounded growth is a follow-up.)

---

## Pre-PR validation

| Step                  | Command                                                                                                     | Purpose                                                            |
| --------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Full validate         | `pnpm validate`                                                                                             | fmt, clippy, check, eslint, typecheck                              |
| Rust tests            | `pnpm test:rust`                                                                                            | unit + integration incl. new persistence/warm tests                |
| Cross-platform check  | clippy on the `ring` SHA-256 + `spawn_blocking` path for Win/macOS/Linux                                    | storage + ONNX both touch native code — verify on the CI matrix    |
| Smoke — cross-session | Two Cursor chats, same workspace: second chat's first hybrid `search_tools` shows store hits, no 30 s spike | Per-session re-embed regression                                    |
| Smoke — restart       | Quit + relaunch, run the same query: vectors load from SQLite, `ranking: hybrid` immediately                | Persistence regression                                             |
| Smoke — alias rename  | Rename a server alias, re-run search: no re-embed in `[embed]` logs                                         | Alias-free hash (Decision 5)                                       |
| Smoke — warm          | Connect a fresh server, wait, then search it: store hit, no inline corpus embed                             | On-connect warmer (Phase 4)                                        |
| Trace one query       | `RUST_LOG=mcpmux_gateway=debug` → grep `query_id`                                                           | Confirm hydrate → read → fusion path; raw text never above `debug` |

---

## Out of scope

| Item                                                   | Reason                                                                                                                                                     |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lazy embed-on-search write-through fallback (Option 4) | Connect-warm + lexical degradation covers the gap window; add only if real-world misses prove material. File standalone if needed                          |
| Upstream/registry-precomputed embeddings               | Rejected in brainstorm — live tool text diverges from registry definitions; a client embedder is still required, so it adds a pipeline for partial benefit |
| Vector DB / ANN index                                  | Brute-force cosine over hundreds–low-thousands of tools is fast enough; the store is a vector cache, not a search index                                    |
| Embedding `input_schema` / params                      | Separate relevance enhancement; embedded text stays name + description                                                                                     |
| Whole-DB encryption for `tool_embeddings`              | Embeddings carry no secret material; consistent with the field-level (not whole-DB) encryption posture                                                     |
| Per-agent usage-frequency boost (MFU)                  | Carried over from the hybrid doc's out-of-scope list; needs usage tracking, file standalone                                                                |

---

## Key files referenced

| File                                                                                                                     | Notes                                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/services/embedding.rs`](../../crates/mcpmux-gateway/src/services/embedding.rs)               | `EmbeddingService` — ONNX wrapper, state machine; inference currently on the calling async thread (~line 230)          |
| [`crates/mcpmux-gateway/src/services/tool_discovery.rs`](../../crates/mcpmux-gateway/src/services/tool_discovery.rs)     | `entry_search_haystack` (~line 285), `rank_with_hybrid`, the per-session embedding compute being re-keyed (~line 308)  |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | `SearchToolsTool::call`, `feature_set_ids_fingerprint`, hybrid `SearchContext` wiring (~line 601)                      |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs)       | Owns `search_cache` + `embedding_cache` today; the embedding half moves out                                            |
| [`crates/mcpmux-core/src/domain/server_feature.rs`](../../crates/mcpmux-core/src/domain/server_feature.rs)               | `qualified_name()` / `prefix()` — shows `server_alias` is the mutable bit excluded from the embedding hash (~line 184) |
| [`crates/mcpmux-gateway/src/consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs)       | Existing event consumer (`ServerFeaturesDiscovered`, `WorkspaceBindingChanged`) — the warmer's trigger source          |

---

## Related work

- [`search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md) — introduced `EmbeddingService` + hybrid fusion; this doc re-keys its embedding cache and supersedes its Decisions 6–8
- [`search-tools-latency-and-root-race.md`](./search-tools-latency-and-root-race.md) — the per-session `search_cache` (active index) stays as-is; only the embedding cache moves to a persistent global store
- [`consent-model-qa-runbook.md`](../../testing/consent-model-qa-runbook.md) — **Section O** (persistent embedding cache) holds the agent-facing QA: cross-session reuse, restart persistence, alias-rename-is-free, on-connect warm, model-version invalidation. Marked Planned until this work ships

---

## Reconciliation

### Shipped commits (Phases 1-5)

1. `2e05cad` — `feat(search-tools): Phase 1 — Embedding repository + persistence`
2. `de4f616` — `feat(search-tools): Phase 2 — Alias-free embedding text + spawn_blocking`
3. `2736717` — `feat(search-tools): Phase 3 — Global store-backed search read path`
4. `e07d6de` — `feat(search-tools): Phase 4 — On-connect incremental warmer`
5. _(this commit)_ — `feat(search-tools): Phase 5 — Observability, pruning & reconciliation`

### Planned vs shipped deltas

- **Shipped as planned:** Added warm/hydrate/search observability with `query_id` correlation and `[embed]` / `[search]` targets:
  - Warm enqueue (`debug`, `[embed]`): `server_id`, `catalog_tools`, `missing`
  - Warm batch done (`info`, `[embed]`): `server_id`, `embedded`, `skipped_present`, `embed_ms`, `model_version`
  - Store hydrate (`debug`, `[embed]`): `query_id`, `hashes_requested`, `store_hits`, `store_misses`
  - Search read (`debug`, `[search]`): `query_id`, `active_tools`, `vectors_present`, `lexical_only_docs`
  - Inline query embed (`info`, `[embed]`): `query_id`, `docs_embedded=1`, `embed_ms`
- **Scope decision:** Optional orphan prune was intentionally deferred in this phase to avoid introducing wider repository/scheduler lifecycle behavior beyond the observability + reconciliation objective.
- **Secret posture:** No new logging of raw tool text or query above `debug`; info-level logs remain aggregate/metadata-only.

### Validation results

- `cargo clippy --workspace -- -D warnings` ✅ passed clean
- `cargo test -p mcpmux-gateway -p mcpmux-storage` ✅ passed (`mcpmux-gateway`: 193 passed, 1 ignored; `mcpmux-storage`: 41 passed, 1 ignored)

### Outstanding manual QA

- Run the existing smoke checks from this plan's **Pre-PR validation** table in a live app session:
  - Cross-session store-hit/no-spike check
  - Restart persistence check
  - Alias-rename no-reembed check
  - On-connect warm check
  - `query_id` trace walk (`store hydrate -> search read -> fusion`)
