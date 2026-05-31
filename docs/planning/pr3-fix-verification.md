# PR #3 Fix Verification Runbook

Tight regression gate for the post-review fixes on `docs/feature-set-consent-model`.
Run top-to-bottom from `mcp-mux/`. Every step must pass before re-requesting review.

Each section maps to a specific fix so a failure points straight at the change that broke it.

## 0. One-shot gate (fastest)

```bash
cargo clippy --workspace -- -D warnings \
  && cargo nextest run -p mcpmux-gateway -p mcpmux-storage --lib \
  && cargo nextest run -p tests -E 'test(meta_tools) | test(admin_meta_tool_approval) | test(search_relevance_eval)' \
  && pnpm typecheck
```

If that's green, you're done. The sections below isolate each fix when something fails.

## 1. Compile + lint (all fixes)

Catches the mechanical changes: `parking_lot::Mutex` swap, removed `embed_semaphore`,
dropped `OptionalExtension` import, removed `DISABLE_ALL_NOTIFICATIONS`, `pub(crate)` lexical helpers.

```bash
cargo clippy --workspace -- -D warnings   # no unused-import / dead-code regressions
cargo check --workspace
```

Expected: clean. Watch specifically for `unused import` (rusqlite `OptionalExtension`,
tokio `Semaphore`) and `dead_code` (the removed `lexical_score`).

## 2. Embedding service — Major #3 (mutex) + block_in_place note

```bash
cargo nextest run -p mcpmux-gateway --lib embedding
```

Expected pass: `cosine_*`, `content_hash_changes_when_description_changes`,
`embed_query_returns_none_while_model_not_ready`, `ensure_init_started_is_idempotent`.
These exercise the `parking_lot::Mutex` model slot and the state machine.

## 3. Hybrid ranking — Major #2 (O(N) lexical precompute)

```bash
cargo nextest run -p mcpmux-gateway --lib discovery_rank tool_discovery
```

Expected pass: `tf_idf_ranks_closer_match_first`, `and_boost_increases_lexical_score`
(rewritten to use the precomputed helpers), `token_overlap_*`,
`multi_token_ranking_favors_all_tokens_present`, `alias_change_leaves_content_hash_unchanged`,
`description_change_changes_content_hash`.

Full ranking-quality check (injected vectors, no model download):

```bash
cargo nextest run -p tests -E 'test(search_relevance_eval)'
```

Expected: intent→tool fixtures still pass in top-3 (hybrid fusion order unchanged by the refactor).

## 4. Embedding repository — get_many IN-clause batching

```bash
cargo nextest run -p mcpmux-storage --lib embedding
```

Expected pass: `upsert_and_get_round_trip`, `upsert_overwrites_on_primary_key_conflict`,
`get_many_returns_empty_for_missing_hashes`. These confirm the chunked `IN (...)` query
returns the same rows the per-hash loop did, including the empty-input and missing-hash cases.

## 5. Consent + approval flows (unchanged surface, regression guard)

```bash
cargo nextest run -p tests -E 'test(meta_tools) | test(admin_meta_tool_approval)'
```

Expected: bind/approve flows, cross-surface dismiss, rate-limit, and no-desktop deny all pass.

## 6. Frontend typecheck

```bash
pnpm typecheck
```

Expected: clean. (No frontend logic changed in the fix pass, but the gate is cheap.)

## 7. Manual — Major #1 cold-start hybrid (cannot be unit-tested)

The warmer now waits for the model before embedding. Verify the persistent cache actually
populates on a **fresh** profile instead of staying empty until a reconnect.

1. Move the embedding cache + DB aside to simulate first run:
   ```bash
   # macOS app-data dir; adjust if you run a custom data dir
   mv ~/Library/Application\ Support/com.mcpmux.app/embeddings{,.bak} 2>/dev/null || true
   ```
2. Start the gateway (`pnpm dev`), connect at least one server with tools, and wait for the
   model download to finish (watch the log).
3. Grep the gateway log for the warm batch outcome:
   ```bash
   # should show embedded > 0 AFTER the model reaches Ready — not "skipped (model not ready)"
   rg '\[embed\] warm batch (done|skipped)' <gateway-log>
   ```
   - PASS: a `warm batch done embedded=N` (N>0) line appears once the model is `Ready`.
   - FAIL: only `warm batch skipped (model not ready within budget)` with no later `done`.
4. Run a `mcpmux_search_tools` query and confirm `ranking: "hybrid"` (not `"lexical"`) and
   `store_hits > 0` in the `[search]` logs.
5. Restore: `mv ~/Library/Application\ Support/com.mcpmux.app/embeddings{.bak,}`.

Budget note: the warmer polls up to 120s for readiness; on a slow link the first warm may sit
in `Downloading` — that's expected, it embeds once `Ready` lands within the budget.

---

**Done when:** §0 (or §1–6) green + §7 shows `embedded>0` and `ranking: hybrid` on a cold profile.
