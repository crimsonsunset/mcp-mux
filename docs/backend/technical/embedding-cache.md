# Embedding Cache

**Synthesizes:** [`search-tools-persistent-embedding-cache.md`](../reference/search-tools-persistent-embedding-cache.md)

**Last Updated:** Jun 1, 2026

**Status:** Shipped (Phases 1–5 on `docs/feature-set-consent-model`)

---

## Purpose

The embedding cache decouples the cost of ONNX vector inference from search-time and from session lifetime. A tool's embedding is a pure function of its text and the model — it does not depend on session, workspace, or feature set. The cache computes each embedding at most once, persists it across restarts, and makes it available globally to all sessions.

**Before:** Per-session `DashMap<session_id, (fingerprint, Vec<DocEmbedding>)>`. Every new Cursor chat re-embedded the full active corpus (~30 s, all-core CPU spike for a 669-tool binding). App restart paid the full cost again.

**After:** Global `SQLite tool_embeddings` table + in-process `DashMap<content_hash, Vec<f32>>` hot mirror. Each tool is embedded once per `(text, model_version)`; the result survives restarts and is reused across all sessions.

---

## Caching Unit

```
Key:   (content_hash, model_version)
Value: f32[384]  (bge-small-en-v1.5, CPU ONNX via fastembed-rs)

content_hash = SHA-256( feature_name + description )
```

The embedding haystack is **alias-free**: `feature_name + description` only, no `{server_alias}_{feature_name}` prefix. This makes alias renames a no-op for the embedding cache — only a semantic change (description or name edit) generates a new hash and triggers re-embedding.

The lexical haystack (`feature_name + qualified_name + description`) retains the alias prefix for token-match precision — the two haystacks serve different purposes.

---

## Storage Schema

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

Lives in the existing SQLite database (field-level encrypted, not whole-DB encrypted). Embeddings carry no secret material, so they sit outside field-level encryption. Accessed only via the `EmbeddingRepository` trait — no SQLx from gateway or app code.

---

## Component Flow

```
EmbeddingRepository (mcpmux-core trait):
  get_many(&[content_hash], model_version) -> Vec<(content_hash, Vec<f32>)>
  upsert_many(&[(content_hash, model_version, Vec<f32>)])

SqliteEmbeddingRepository (mcpmux-storage):
  impls the trait over tool_embeddings

EmbeddingWarmer (mcpmux-gateway) — subscribes to ServerFeaturesDiscovered / connect:
  1. build alias-free haystack + content_hash for each catalog tool of the server
  2. get_many → diff against store → keep misses only          (skip-if-unchanged)
  3. spawn_blocking embed misses via EmbeddingService          (bounded concurrency)
  4. upsert_many → SQLite; insert into in-memory DashMap

SearchToolsTool::call (read path):
  1. build active ToolIndexEntry[] (per-session index cache)
  2. hydrate_active_embeddings: hashes not in DashMap → get_many → insert
     (usually no-op when warmer already filled DashMap)
  3. rank_with_hybrid: read vectors by content_hash from DashMap
     → embed query inline via spawn_blocking (~ms)
     → fuse 0.4 lexical + 0.6 semantic → ranking: "hybrid"
     → docs missing from DashMap degrade to lexical-only (graceful)
```

---

## EmbeddingService

`crates/mcpmux-gateway/src/services/embedding.rs`

Wraps `fastembed-rs` (CPU ONNX, `bge-small-en-v1.5`, ~67 MB, downloaded on first use to the app data dir). State machine:

```
NotDownloaded → Downloading → Ready
                           ↘ Failed
```

While not `Ready`, search runs lexical-only and annotates `ranking: "lexical"`. All ONNX inference runs in `spawn_blocking` — inference never pegs the async runtime thread.

The model is downloaded once and cached in the OS app data directory. It is not bundled in the installer.

---

## On-Connect Incremental Warmer

`EmbeddingWarmer` subscribes to `ServerFeaturesDiscovered` and `Connected` events. For each server that connects:

1. Builds the alias-free haystack and computes `content_hash` for every tool in its catalog.
2. Calls `get_many` → only hashes missing from the store need embedding.
3. Embeds misses via `spawn_blocking` with bounded concurrency (no boot spike).
4. Upserts to SQLite and inserts into the global `DashMap`.

The warmer covers the **full installed catalog**, not just the active/granted set. This makes `include_inactive` searches instant (vectors already present) and makes feature-set switches free.

---

## Search Read Path

After warming, `hydrate_active_embeddings` in `SearchToolsTool::call` checks whether each active tool's `content_hash` is in the `DashMap`. When the warmer has already run, `hashes_requested=0` and `store_hits=N` is the expected log output — it is not a sign of failure.

If the model is still `Downloading` when search runs, vectors in the DashMap are still usable for the doc side of cosine similarity; only the query embed needs the model. The search path checks model state and can fall back to lexical when truly necessary.

---

## Observability

`query_id` correlation (minted at `SearchToolsTool::call`) threads through all warm and read events:

| Stage | Level | Target | Key fields |
| ----- | ----- | ------ | ---------- |
| Warm enqueue | `debug` | `[embed]` | `server_id`, `catalog_tools`, `missing` |
| Warm batch done | `info` | `[embed]` | `server_id`, `embedded`, `skipped_present`, `embed_ms`, `model_version` |
| Store hydrate | `debug` | `[embed]` | `query_id`, `hashes_requested`, `store_hits`, `store_misses` |
| Hybrid skip | `debug` | `[search]` | `query_id`, `embedding_store=skipped`, `skip_reason` |
| Search read | `debug` | `[search]` | `query_id`, `active_tools`, `vectors_present`, `lexical_only_docs` |
| Inline query embed | `info` | `[embed]` | `query_id`, `docs_embedded=1`, `embed_ms` |

Raw tool text and query strings are logged at `debug` only, never `info`.

---

## Key Source Locations

| Path | Role |
| ---- | ---- |
| `crates/mcpmux-core/src/repository/mod.rs` | `EmbeddingRepository` trait, `EmbeddingRecord` type |
| `crates/mcpmux-storage/src/repositories/embedding_repository.rs` | `SqliteEmbeddingRepository`, SHA-256 via `ring`, BLOB codec |
| `crates/mcpmux-storage/src/migrations/021_tool_embeddings.sql` | Schema migration |
| `crates/mcpmux-gateway/src/services/embedding.rs` | `EmbeddingService`, model state machine, `spawn_blocking` wrappers |
| `crates/mcpmux-gateway/src/services/embedding_warmer.rs` | `EmbeddingWarmer`, event-driven warm, bounded concurrency |
| `crates/mcpmux-gateway/src/services/tool_discovery.rs` | `entry_content_hash`, `rank_with_hybrid`, read-path hydration |
| `crates/mcpmux-gateway/src/services/meta_tools/tools.rs` | `hydrate_active_embeddings`, `SearchToolsTool::call` |
| `crates/mcpmux-gateway/src/consumers/mcp_notifier.rs` | `ServerFeaturesDiscovered` → warmer enqueue trigger |
