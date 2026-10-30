# Mux follow-ups: idle reap, migration test, clippy

**Last Updated:** Oct 30, 2026
**Status:** Implemented, PR open
**Branch:** `fix/35-session-reap-migration-clippy`
**Issues:** [fellowship#35](https://github.com/crimsonsunset/fellowship/issues/35), [#36](https://github.com/crimsonsunset/fellowship/issues/36), [#37](https://github.com/crimsonsunset/fellowship/issues/37)
**Depends on:** [`same-root-supersede-window-gate.md`](./same-root-supersede-window-gate.md) (#27, shipped)

## Flow state

| Field | Value |
|---|---|
| Gate | 6 (CI diagnosed, no bot review) |
| PR | [mcp-mux#20](https://github.com/crimsonsunset/mcp-mux/pull/20) |
| CI | Run on `3c84422e`: rust-check, ts-check, coverage, rust-test Ubuntu and Windows, both report jobs: pass. macOS rust-test and build: cancelled, "The job was not acquired by Runner of type hosted even after multiple attempts" (GitHub arm64 capacity), which also fails the macOS report steps. e2e-web cancelled by its 20 minute cap (123 tests, 1 worker, 2 retries). Linux desktop E2E: 80 of 110 specs pass, 30 fail on stale selectors and session errors. Windows desktop E2E: 0 of 16 specs run, `mcpmux.exe` exits within seconds of launch under msedgedriver. See CI follow-ups below |
| QA | unit and integration tests on the worktree. Live idle-reap not run: :45818 is the installed app, and `pnpm dev` would quit it |
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
| [`tests/rust/tests/database/migrations.rs`](../../tests/rust/tests/database/migrations.rs) | Drop the 043 indexes, then the 039 and 043 columns, in the rollback batch. 041 is `CREATE TABLE IF NOT EXISTS`, so it re-runs |
| [`crates/mcpmux-storage/src/database.rs`](../../crates/mcpmux-storage/src/database.rs) | Expected schema version 42 → 43 |
| `Cargo.lock` | `async-trait` 0.1.89 → 0.1.92 |

## Phases

### Phase 1: Clippy (#37) and the migration test (#36)
- [x] `cargo update -p async-trait` to 0.1.92
- [x] Rollback also drops the 043 indexes before the 039 and 043 columns. SQLite refuses the column drop while `idx_workspace_bindings_git_remote` still references it
- [x] Schema-version assert is 43
- Local CI mirror on 1.97: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings` clean. `cargo test -p tests --test database`: 86 passed, including the older-database test. The 1.99 lint is what CI will prove

**Outcome:** The older-database test reopens cleanly. `database.rs` expects version 43. `Cargo.lock` has `async-trait` 0.1.92.

### Phase 2: Idle reap (#35)
- [x] `touch_last_request` from `attach_process_identity`, so both the request and the initialize response stamp it
- [x] Sweep from `register_session` and `reap_dead_sessions`
- [x] `is_idle_identityless` covers recent sibling, past-ttl sibling, missing stamp, no sibling, and a session that has an identity

**Outcome:** Chief's dead reconnect is eligible for removal two minutes after it stops sending, and a live probe does not take chief with it.

### Phase 3: Reconcile
- [x] This doc matches the diff, including the index drop the plan did not name up front

**Outcome:** Decisions and the file table describe what shipped.

## CI follow-ups found while getting PR 20 green

`rust-check` had been red on every PR, so these jobs never ran far enough to fail. Fixing #37 exposed them.

| Check | Cause | What the PR does |
| ----- | ----- | ---------------- |
| rust-test Windows | `normalize_expands_home_tilde` built its expected path from the raw home dir plus `/`. The normalizer lowercases and uses `\` on Windows | Compare against the normalized value |
| e2e-web | `ubuntu-latest` is 26.04 and Playwright 1.58 cannot install chromium there. Playwright 1.61 adds 26.04 support | `runs-on: ubuntu-24.04`. Remove the pin after a Playwright bump to 1.61 or later |
| e2e-web hang | 123 tests, one worker, two retries. No job timeout, so 6h default | `timeout-minutes: 45`. It did not finish in 20 |
| e2e-desktop Linux | The E2E apt cache restored empty on 26.04, so `WebKitWebDriver` was missing | `ubuntu-24.04` via a matrix `runner` key; `os` stays the gating and artifact key. `wdio:enforceWebDriverClassic: true` because 24.04's WebKitWebDriver predates BiDi |
| e2e-report | wdio writes `<testsuite>` with no name when the driver never starts, and `dorny/test-reporter` crashes on `.name.trim()` | The sanitize step drops those files. `fail-on-empty: false` on the desktop report steps |

### Tried and backed out

Skipping `tauri_plugin_single_instance` under `MCPMUX_E2E_TEST`. WebView2 child processes inherit argv and WebdriverIO's desktop repo patched that plugin for it, so it fit the symptom. On run `3c84422e` the Windows specs still all failed with `Chrome instance exited` and msedgedriver still reported `mcpmux.exe` no longer running. The gate did nothing, so `lib.rs` is back to `main`.

### Not fixed here

- **Windows desktop E2E launch crash.** `mcpmux.exe` exits within 3 to 10 seconds under msedgedriver. The log shows no reason. The next step is uploading the app's own log (`get_logs_dir()`) as an artifact from that job.
- **Linux desktop E2E:** 30 of 110 specs fail on stale selectors and `invalid session id`. This is the existing `TODO(playwright-migration)`. Options: [`@wdio/tauri-service`](https://github.com/webdriverio/desktop-mobile) or [`tauri-plugin-playwright`](https://github.com/srsholmes/tauri-playwright).
- **Ubuntu 26.04 for E2E.** The package is `webkitgtk-webdriver` 2.52 there, and it breaks `setValue()` through `tauri-driver` ([tauri#15871](https://github.com/webdriverio/desktop-mobile/issues/591)).
- **macOS arm64 capacity.** Jobs were cancelled without starting. Re-run, no code change.

## Related

- [`same-root-supersede-window-gate.md`](./same-root-supersede-window-gate.md): the identity rule this must not weaken
- [async-trait 0.1.92](https://github.com/dtolnay/async-trait/releases/tag/0.1.92): the clippy fix
