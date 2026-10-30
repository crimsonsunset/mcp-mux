# Mux follow-ups: idle reap, migration test, clippy

**Last Updated:** Oct 30, 2026
**Status:** Planned
**Branch:** `fix/35-session-reap-migration-clippy`
**Issues:** [fellowship#35](https://github.com/crimsonsunset/fellowship/issues/35), [#36](https://github.com/crimsonsunset/fellowship/issues/36), [#37](https://github.com/crimsonsunset/fellowship/issues/37)
**Depends on:** [`same-root-supersede-window-gate.md`](./same-root-supersede-window-gate.md) (#27, shipped)

## Flow state

| Field | Value |
|---|---|
| Gate | 2 (plan ready) |
| Ticket | fellowship#35 #36 #37 |
| Branch | fix/35-session-reap-migration-clippy |
| Repos | mcp-mux |
| Isolation | worktree |
| Worktrees | /Users/joe/Desktop/Repos/Personal/jsg-tech-check-wt/fellowship-35/mcp-mux |
| Base | origin/main @ 53d25174 |
| QA mode | lets-qa (backend only, no UI) |
| Gates | all |
| Updated | 2026-10-30 |

---

## Problem

Three leftovers from #27, one branch.

**#35.** Hermes still cannot send `X-Mcpmux-Client-Instance` (header templating is `${ENV}` plus a few context vars, and the gateway and `hermes mcp test` share one `.env`). Tunneled sessions with no process identity are never superseded. Chief's own reconnects (~6/day) leave the old session in the notifier map. `reap_dead_sessions` only drops peers whose transport reports closed, and rmcp often never flips that. Two live identity-less processes (chief plus the probe) must both stay while both are sending requests. Immediate supersede is the bug #27 fixed.

**#36.** `test_pending_migrations_apply_to_an_existing_older_database` fully migrates, deletes `schema_migrations` rows `>= 36`, and drops only the 036–038 objects. Reopen replays 039, which `ADD COLUMN client_icon`s a column that is still there. 041 and 043 have the same shape and would fail next. Production DBs that already recorded v39 never re-run that SQL.

**#37.** CI `cargo clippy --workspace -- -D warnings` on stable 1.99 fails with `clippy::double_must_use`, emitted by `async-trait` 0.1.89. Local 1.97 is clean, so this never showed up here. `rust-test` is skipped behind `rust-check`, which is why #36's failure and a stale `assert_eq!(version, 42)` in `database.rs` (migrations now go to 43) are not in CI yet.

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | #35 eviction | Drop an identity-less session only when another session of the same client and root exists and this one's last request is older than 2 minutes | A successor plus silence is a reconnect. Two sessions both touched inside the window (chief and `hermes mcp test`) both stay. A lone idle chief is kept. |
| 2 | #35 clock | `SessionRootsRegistry` stamps `last_request` from the OAuth middleware, which already sees `mcp-session-id` on every request. Cleared in `remove` | The notifier is not on `ServiceContainer`. No new type. |
| 3 | #35 trigger | Same check in `register_session` and in `reap_dead_sessions` | Register catches a successor hours later. Fanout catches one that was still fresh at register time and then went quiet. No timer. |
| 4 | #36 | Fix the test's rollback, do not edit migration 039 | Shipped DBs are fine. The ledger and the file disagree only because the test undoes half the schema. |
| 5 | #37 | `cargo update -p async-trait` to 0.1.92 | Upstream 0.1.92 release notes: resolve `double_must_use` in generated code. The workspace constraint is already `0.1`. An allow would have to be copied onto every crate. Pinning CI to 1.97 hides it from anyone on 1.99. |
| 6 | Stale version assert | Bump `assert_eq!(version, 42)` to the current max (43) in the same PR | Otherwise `rust-test` goes red the moment #37 unblocks it. |

## Scope

**In:**
- `last_request` map, touch from middleware, idle sweep of identity-less same-client-and-root sessions, unit tests for the pure "is this one stale" helper
- Rollback batch in the migration integration test extended through 043
- `async-trait` lockfile bump
- Schema-version assert bumped to 43

**Out:**

| Item | Reason |
| ---- | ------ |
| Forking Hermes or teaching it a per-process header | No local Hermes repo. #35 is the mux-side ceiling from #27 |
| Making 039/043 idempotent | Not a production bug. Test-only drift |
| Pinning the CI toolchain | Decision 5 |

## Architecture

```
request ─► oauth middleware
             touch_last_request(session_id)     // Instant::now, every request

register_session / reap_dead_sessions
             for each other session, same client_id + same non-empty root:
               both lack process identity
               AND a sibling session exists
               AND last_request older than 2 min (missing stamp = not stale)
             → sessions.remove + session_roots.remove
```

Identity-bearing sessions are unchanged: same-process reconnects still supersede immediately (#27).

## Files to modify

| File | Change |
| ---- | ------ |
| [`services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | `last_request` map, `touch_last_request`, clear in `remove` |
| [`mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Call `touch_last_request` next to `attach_process_identity` |
| [`consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs) | Idle sweep helper + call sites, tests |
| [`tests/rust/tests/database/migrations.rs`](../../tests/rust/tests/database/migrations.rs) | Drop 039 column, 041 table, 043 columns in the rollback batch |
| [`crates/mcpmux-storage/src/database.rs`](../../crates/mcpmux-storage/src/database.rs) | Expected schema version 42 → 43 |
| `Cargo.lock` | `async-trait` 0.1.89 → 0.1.92 |

## Phases

### Phase 1: Clippy (#37) and the migration test (#36)
- `cargo update -p async-trait`
- Extend the rollback batch. 040 and 042 are data statements and can re-run
- Bump the schema-version assert
- Local CI mirror: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test -p mcpmux-storage --lib`, `cargo test -p tests --test database`. Clippy on 1.97 will not reproduce the 1.99 lint. The lockfile bump is the proof we can run here. CI on the PR is the 1.99 proof

**Outcome:** The older-database test reopens cleanly. `database.rs` expects version 43. `Cargo.lock` has `async-trait` 0.1.92.

### Phase 2: Idle reap (#35)
- Stamp, sweep, tests: a recent sibling is kept, a stale sibling with a live peer is dropped, a lone session is kept, an identity-bearing session is not touched by this sweep
- Re-run `cargo test -p mcpmux-gateway --lib` and `cargo clippy --workspace -- -D warnings`

**Outcome:** Chief's dead reconnect is eligible for removal two minutes after it stops sending, and a live probe does not take chief with it.

### Phase 3: Reconcile
- Update this doc to match the diff

**Outcome:** Decisions and the file table describe what shipped.

## Related

- [`same-root-supersede-window-gate.md`](./same-root-supersede-window-gate.md): the identity rule this must not weaken
- [async-trait 0.1.92](https://github.com/dtolnay/async-trait/releases/tag/0.1.92): the clippy fix
