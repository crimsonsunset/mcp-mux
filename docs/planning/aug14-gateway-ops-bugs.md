# Aug 14 Gateway Ops Bugs

**Last Updated:** Aug 14, 2026
**Status:** STALE — Phases 1-3 shipped on this branch, but [PR #221](https://github.com/mcpmux/mcp-mux/pull/221) closed without merging on Aug 20, 2026. Phase 4 close-out below never happened on `main`. Commits stay on this branch's lineage (now `root-resolution`) by choice — see [`backend-connection-resilience.md`](./backend-connection-resilience.md) Decision 4. That doc is the active thread for the connection-reliability work that grew out of this investigation.
**Branch:** `docs/aug14-gateway-ops-bugs` (off `dev-rebased`)
**Depends on:** `cursor-workspace-routing-bridge.md` (empty-header / Agents Window), `search-tools-perf.md` (`resolve_feature_sets` hot path), `7ac5dc1` (filesystem multi-root disambiguation)
**Unblocks:** Quieter logs, a process that stays up, fewer false "gateway is dead" signals, and a clean connect set on boot

Origin: live log review after swapping in the resolver + log-level work (`mcpmux.2026-08-14.log`, ~933 MB from the morning `debug` flood; new-binary slice after 16:45 local is ~6k lines).

---

## Already shipped this session (uncommitted on this branch)

| Item | What landed |
| ---- | ----------- |
| Log default | `init_tracing` no longer pins every mcpmux crate to `debug`. Default is `info`. `RUST_LOG` / `.env` still overrides. |
| `resolve_feature_sets` cache | Per-`(space_id, sorted feature_set_ids)` result cache on `FeatureResolutionService`, invalidated via `DomainEvent::affects_mcp_capabilities()`. |
| Per-feature debug collapse | `[FeatureResolution] Feature X filtered out` gone; summary line has `filtered_out=`. |
| Empty-header warn | Present-but-empty `X-Mcpmux-Workspace` now `warn`s in `oauth_middleware`. Manual + planning docs updated. |

These stay on the branch. They are not the dig targets.

---

## Bug catalog

Severity is "how much it hurts today," not "how hard to fix."

### B1. Empty `${workspaceFolder}` header

**Severity:** High (wrong or stuck routing)
**Status:** Root-caused and bounded (`efabe48`); residual ~16% is inherent
**Symptom:** Cursor sends `X-Mcpmux-Workspace` present but empty. `set_pinned` no-ops. Session falls through to multi-folder `roots/list` → `PendingRoots` or the wrong space.
**Evidence:** 477 empty-header warns in ~40 min of new-binary uptime. Session `896e45f3…` fires it on every `tools/list` / `prompts/list` / `resources/list`. A later 282-spawn `env-probe` put the substitution failure at 21% overall.
**Cause (confirmed):** Cursor leaves `${workspaceFolder}` unresolved; `mcp-remote` treats `${…}` as an env var and substitutes empty.
**Correction:** this was filed as an Agents-window bug. It isn't. Editor windows fail at 29%, Agents windows at 4%. The `oauth_middleware` warn that blamed the Agents window has been reworded.
**Related:** [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) resolved question (Aug 20) and [`resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) item 1. Fully immune path is the per-repo static header in [`cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md).
**Decision:** still no auto-disambiguation — a first-entry heuristic on `WORKSPACE_FOLDER_PATHS` would misroute 30% of the time. The window's folder set is now carried as a constraint instead, which bounds `set_workspace_root` rather than guessing.

### B2. Empty-header warn is per-request, not per-session

**Severity:** Medium (will re-bloat the log)
**Status:** Won't-fix this pass — Decision 1 keeps the per-request warn for its telemetry value
**Symptom:** Same empty header logged on every MCP POST. 477 lines in 40 minutes from a handful of sessions.
**Question:** rate-limit / first-pin-only / drop to debug after first warn?

### B3. Unexplained SIGTERM (~10 min after `dev:admin`)

**Severity:** High (gateway vanishes)
**Status:** Shipped attribution logging (Phase 1) — root cause still open, watch next occurrence
**Symptom:** Process does not panic. Log ends with `[Signal] SIGTERM — requesting exit` then a 2s graceful-shutdown timeout. No `.ips` crash report. Window-close would have logged `[Window] Close requested`, not SIGTERM.
**Times today:** 16:38 (our `dev:stop`), 16:58 (~13 min after start), 17:15 (~9 min after start).
**Suspects:** dock Quit on the McpMux app name, `tauri dev` recycling the child, Cursor-managed terminal SIGTERM on the process group, `osascript tell application "McpMux" to quit` from `dev-stop.mjs` colliding with the named Tauri window.

### B4. Graceful shutdown times out at 2s

**Severity:** Low (follow-on of B3)
**Status:** Deferred — gated on B3's root cause (Decision 2), no change this pass
**Symptom:** `[Gateway] Graceful shutdown timed out after 2s — aborting task (listener socket may briefly linger in kernel)`. Happens on every SIGTERM. May leave `:45818` / `:45819` in `TIME_WAIT` and make the next start flaky.

### B5. rmcp 5-minute keep-alive kills idle sessions

**Severity:** Medium (looks like a crash; is not)
**Status:** Shipped (Phase 2) — `keep_alive` raised to 1800s
**Symptom:** `worker quit with fatal: keep alive timeout after 300000ms` → client `GET /mcp` → 404. 17 hits since 16:45. Gateway stays up. Already noted as expected RMCP behavior in [`deny-by-default-bindable-callers.md`](./deny-by-default-bindable-callers.md) (Jun 29).
**Question:** raise the timeout, treat 404-after-keepalive as a reconnect, or leave it and stop logging it as ERROR?

### B6. `GET /mcp` 400 before `initialize`

**Severity:** Low (noise)
**Status:** Shipped (Phase 2) — warn narrowed to skip `(GET, 400)` and any `404`
**Symptom:** `mcp-remote` opens the SSE GET before it has `mcp-session-id`. Pin skipped, 400, then POST `initialize` succeeds. Same pattern for a leftover `mcp-remote-fallback-test` client (`0.0.0`) that reconnects on every restart.

### B7. `typesense` handshake fails every boot

**Severity:** Medium (one backend always down)
**Status:** Shipped (Phase 3) — user config args fixed; stderr surfacing added generically for the next time this happens
**Symptom:** `uv` stdio: `MCP handshake failed: connection closed: initialize response.` Then `[Startup] ✗ Failed to connect …/typesense`. Repeats every `dev:admin`.

### B8. OAuth-skipped backends on every boot

**Severity:** Low (expected if tokens missing)
**Status:** Open — confirm, don't "fix" if intentional
**Symptom:** `jambase` (`invalid_token` / missing Authorization) and `taylorwilsdon.google-workspace-mcp-uvx-gait` skipped as "needs OAuth."

### B9. Some backends `-32601 Method not found` on `resources/list`

**Severity:** Low
**Status:** Shipped (Phase 3) — downgraded to `debug!` when the error is `-32601 Method not found`
**Symptom:** `[FeatureDiscovery] Failed to list resources: Mcp error: -32601: Method not found`. Discovery continues. Confirm which servers and whether we should skip the call when the capability is absent.

### B10. Admin `:45819` missing `dist/index.html`

**Severity:** Low (dev-only)
**Status:** Shipped (Phase 3) — `dev-admin.mjs` waits for the first `vite build` write before starting the gateway
**Symptom:** `[Admin] frontend dist missing index.html … serving build hint page`. Vite watch on `:1420` is up; the production-parity SPA for `:45819` / CF tunnel is not.

### B11. macOS Contacts permission denied

**Severity:** Low
**Status:** Shipped (Phase 3) — `ensure_contacts_registered()` skips under `cfg!(debug_assertions)`
**Symptom:** `[Permissions] Contacts request failed: Access Denied` on every launch.

### Parked (not digging)

| Item | Why parked |
| ---- | ---------- |
| Genuine multi-root `PendingRoots` (14 since 16:45) | Decision already made: never auto-disambiguate. Client/onboarding problem. |
| Phantom-root FS filter | Shipped in `7ac5dc1`. Zero `disambiguated` events today (no phantoms in the new-binary window). |
| App-log size rotation | Morning 933 MB was the old `debug` default. Info default is in. Size-cap is a later hardening item, not a dig. |

---

## Dig grouping

Hard cap 4 parallel digs. Catalog items map as:

| Dig | Bugs | Domain |
| --- | ---- | ------ |
| A. Empty workspace header | B1, B2 | Cursor bridge / `mcp-remote` / `oauth_middleware` |
| B. Process death | B3, B4 | Tauri shell / signals / `dev-stop` / `tauri dev` |
| C. Session lifecycle noise | B5, B6 | rmcp streamable HTTP / inbound sessions |
| D. Startup + connect hygiene | B7, B8, B9, B10, B11 | Pool connect / discovery / admin SPA / macOS perms |

---

## Decisions

Locked after four parallel digs + `AskQuestion` + a `propose-opts-brainstorm` on session keep-alive (Aug 14).

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | B1/B2 empty-header warn | **Keep per-request `warn!`, no change** | Telemetry value while B1 is still open outweighs 477 lines/40min. Revisit once a client-side fix for the header itself lands. |
| 2 | B3/B4 SIGTERM | **Attribute the sender before touching the shutdown timeout or `dev-stop`** | No in-repo timer explains the ~9–13 min gap. Changing the 2s timeout or `dev-stop`'s AppleScript quit blind risks masking the real cause instead of fixing it. |
| 3 | B5 session keep-alive | **Raise `LocalSessionManager.session_config.keep_alive` from rmcp's 300s default to 30 min** | 300s is a multi-tenant-server default fighting a single local agent host that legitimately idles mid-thread. 30 min survives normal "user went to lunch"; a session that's still dead after that is a real signal worth the `error!`. |
| 4 | B6 GET `/mcp` 400/404 | **Downgrade the two spec-correct shapes (GET with no `Mcp-Session-Id`, GET/POST to an unknown session) out of the `← MCP error` warn** | Both are rmcp spec behavior (`mcp-remote` opening SSE before init; reconnect after a session died). Not gateway bugs — logging them as warnings caused the "gateway keeps dying" false alarm. |
| 5 | B7 typesense | **Fix the user's spawn args *and* surface child stderr on handshake failure** | Root cause is `uv run mcp run main.py` (missing `mcp` binary in that venv) instead of `uv run python main.py`. Also: the generic `MCP handshake failed: connection closed` message hid an OS-level `No such file or directory` that would have made this obvious immediately. |
| 6 | B8 OAuth skips | **Park — expected, no change** | jambase has no stored token; `taylorwilsdon.google-workspace-mcp-uvx-gait` is gated behind explicit OAuth approval by design (deny-by-default). Neither is a bug. |
| 7 | B9 `-32601` on `resources/list` | **Downgrade to `debug!`, do not retry or special-case per server** | The Atlassian family (`com.atlassian-mcp`, `-mesh`, `-gait`) advertises `resources: {}` at initialize but returns Method not found. Discovery already tolerates it; only the log level was wrong. |
| 8 | B10 admin dist race | **`dev-admin.mjs` waits for the first `vite build` to finish before declaring the stack ready** | `admin/router.rs`'s dist check is one-shot at router-build time; the race is purely in dev orchestration timing, not gateway code. |
| 9 | B11 Contacts prompt | **Skip `requestAccess` when running under `tauri dev`** | TCC never persists a decision for the unsigned dev binary, so every dev launch re-prompts and fails. Production behavior (signed `.app`) is unchanged. |
| 10 | Leftover `mcp-remote-fallback-test` client | **Corrected during implementation — not a stale artifact, no action** | No `inbound_clients` row exists for it (confirmed via SQLite query). Traced to `mcp-remote`'s own `dist/chunk-*.js`: `connectToRemoteServer()` spins up a disposable `Client({ name: "mcp-remote-fallback-test", version: "0.0.0" })` to test the HTTP transport on *every* real connect/reconnect before servicing the actual bridge, then tears it down. It's mcp-remote-side connection-test noise inherent to the `http-first` strategy, not a leftover client we can delete — same class of expected noise as B6, now covered by Decision 4's narrowed warn. |

---

## Scope

**In (this pass):**

- SIGTERM attribution logging (Phase 1) — no shutdown-timeout or `dev-stop` behavior change
- Session keep-alive raise + expected-noise downgrade (Phase 2)
- typesense arg fix + stderr surfacing on stdio handshake failure (Phase 3)
- `-32601` resources/list log downgrade (Phase 3)
- Admin dist startup race fix in `dev-admin.mjs` (Phase 3)
- Contacts prompt skip under `tauri dev` (Phase 3)
- ~~Delete leftover `mcp-remote-fallback-test` inbound client~~ (Phase 3) — corrected during implementation, see Decision 10

**Out / deferred:**

| Item | Reason / Deferral |
| ---- | ------------------ |
| Empty `${workspaceFolder}` header client-side fix (B1) | Decision already made in the parent session: no client workaround, no new UI this pass. Telemetry (warn + docs) already shipped. |
| Shutdown-timeout change (2s → longer) / `dev-stop` AppleScript rework (B4) | Blocked on Decision 2 — fix the timeout only after we know who sends the SIGTERM, otherwise we're tuning a number that may not even be the real problem. |
| jambase / gait OAuth auto-connect on cached tokens (B8) | Decision 6 — approval gate is intentional deny-by-default behavior, not a bug. |
| Genuine multi-root `PendingRoots` auto-disambiguation | Decided in the parent session: never auto-disambiguate real multi-root. Client/onboarding problem. |
| App-log size rotation | Info-level default (already shipped) removed the actual 933 MB driver. A size cap is a separate hardening ticket, not part of this catalog. |

---

## Architecture notes

### Session keep-alive (Decision 3)

`server/mod.rs` builds `LocalSessionManager::default()` (rmcp's `SessionConfig::keep_alive` defaults to `Some(Duration::from_secs(300))` — `rmcp-1.5.0/.../session/local.rs`). The worker loop races `event_rx.recv()` against a `sleep(keep_alive)`; on timeout it returns `WorkerQuitReason::fatal(KeepAliveTimeout)`, which rmcp's own `Worker::spawn` logs at `error!` before the session is closed:

```text
LocalSessionManager::default()
  → session_config.keep_alive = Some(300s)   [rmcp default]
  → LocalSessionWorker::run() sleep(keep_alive) races event_rx
  → timeout → WorkerQuitReason::Fatal(KeepAliveTimeout) → rmcp error! log → session closed
  → next client request (GET or POST) → LocalSessionManager: session not found → 404
```

`session_config` is a public field (`pub session_config: SessionConfig`), so the fix is a value change on the manager we already construct — no fork, no new dependency. rmcp's own `error!` line is left alone (Decision 3's rationale: raising the bound makes real timeouts rare enough that the `error!` stays a useful signal instead of routine noise).

### `← MCP error` warn (Decision 4)

`oauth_middleware.rs`'s trailing block warns on any 4xx/5xx regardless of shape:

```text
if status.is_server_error() || status.is_client_error() { warn!(... "← MCP error") }
```

Two of those shapes are spec-correct per rmcp's `tower.rs` (`get_without_session_id_header_returns_400`, `get_without_valid_session_returns_404`), not gateway problems. The fix narrows the warn condition by (method, status) instead of touching rmcp.

---

## Files to Modify

| File | Change |
| ---- | ------ |
| [`apps/desktop/src-tauri/src/lib.rs`](../../apps/desktop/src-tauri/src/lib.rs) | SIGTERM/SIGINT handler (~L917-918): log `std::process::id()` alongside the existing `[Signal] SIGTERM` line for cross-referencing against script logs |
| [`scripts/dev-admin.mjs`](../../scripts/dev-admin.mjs) | Signal forwarding loop (~L192-197): log the signal + timestamp before forwarding to `child`/`spaWatchChild`. Also: block `waitThenOpenBrowser()` / readiness until the first `vite build` write completes (Decision/B10), instead of racing admin startup against it |
| [`scripts/dev-stop.mjs`](../../scripts/dev-stop.mjs) | `killByPort` (~L67-69) and the `osascript` quit (~L79-86): log every kill/quit attempt (target PID, signal) unconditionally, not just on `status === 0` |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | Replace `LocalSessionManager::default()` (~L454) with a manager whose `session_config.keep_alive = Some(Duration::from_secs(1800))` |
| [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Capture `let method = request.method().clone();` before `next.run(request)`; narrow the `← MCP error` warn (~L326-335) to skip `(GET, 400)` and `(GET \| POST, 404)` |
| [`crates/mcpmux-gateway/src/pool/features/discovery.rs`](../../crates/mcpmux-gateway/src/pool/features/discovery.rs) | `resources/list` failure log (~L116-131): downgrade `-32601 Method not found` from `warn!`/error path to `debug!` |
| [`crates/mcpmux-gateway/src/pool/transport/stdio.rs`](../../crates/mcpmux-gateway/src/pool/transport/stdio.rs) | Handshake failure path: include captured child stderr in the `MCP handshake failed` message instead of the generic `connection closed: initialize response` |
| User config: `~/Library/Application Support/com.mcpmux.desktop/spaces/00000000-0000-0000-0000-000000000001.json` | typesense server args: `["--directory", ".../typesense-mcp-server", "run", "mcp", "run", "main.py"]` → `["--directory", ".../typesense-mcp-server", "run", "python", "main.py"]` |
| [`apps/desktop/src-tauri/src/macos_permissions.rs`](../../apps/desktop/src-tauri/src/macos_permissions.rs) | `ensure_contacts_registered()` (~L30): early-return when `cfg!(debug_assertions)` (or a `TAURI_ENV_DEBUG`-style check) instead of always requesting |
| ~~SQLite `inbound_clients` table~~ | Not touched — no row exists; see Decision 10 correction |

---

## Phases

### Phase 1 — SIGTERM sender attribution (no behavior change)

- `lib.rs`: append `pid = std::process::id()` to the existing `[Signal] SIGTERM — requesting exit` / `SIGINT` log lines
- `dev-admin.mjs`: log before forwarding `SIGINT`/`SIGTERM` to `child` and `spaWatchChild` (currently silent — `child.kill(signal)` / `spaWatchChild.kill(signal)` give no trace today)
- `dev-stop.mjs`: log every port-PID kill attempt and every `osascript` quit attempt, success or not (currently only logs on `result.status === 0`)

**Outcome:** Next time `[Signal] SIGTERM` appears in the app log, cross-referencing the same timestamp against the `dev-admin`/`dev-stop` terminal output (or their absence) tells us whether our own tooling sent it or something external did. No shutdown-timeout or quit-path behavior changes yet — that's gated on what this reveals.

### Phase 2 — Session lifecycle: raise keep-alive, quiet expected noise (B5, B6)

- `server/mod.rs`: build `LocalSessionManager` with `session_config.keep_alive = Some(Duration::from_secs(1800))` instead of the rmcp 300s default
- `oauth_middleware.rs`: capture the HTTP method before consuming the request; skip the `← MCP error` warn for `(GET, 400)` (no `Mcp-Session-Id`) and `(GET | POST, 404)` (unknown session) — everything else still warns

**Outcome:** An idle agent thread survives up to 30 minutes without losing its session/pin. `mcp-remote`'s pre-init GET and post-timeout reconnect no longer show up as `← MCP error`. A session that still dies after 30 minutes keeps its `error!` from rmcp — now a meaningful signal instead of routine noise.

### Phase 3 — Startup + connect hygiene (B7, B9, B10, B11, leftover client)

- Fix the typesense spawn args in the user's space config (`uv run mcp run main.py` → `uv run python main.py`)
- `stdio.rs`: include captured child stderr in the handshake-failure log so a bad spawn command fails loudly next time, for any server
- `discovery.rs`: downgrade `-32601 Method not found` on `resources/list` to `debug!` (discovery already tolerates it; only the level was wrong)
- `dev-admin.mjs`: don't declare the dev stack ready (open-browser / health-probe success) until the first `vite build --watch` write lands, closing the race with `admin/router.rs`'s one-shot `dist/index.html` check
- `macos_permissions.rs`: skip `ensure_contacts_registered()` under `tauri dev` / debug builds
- ~~Delete the `mcp-remote-fallback-test` row from `inbound_clients`~~ — no such row exists; traced to `mcp-remote`'s own per-connection transport test client (Decision 10), not our code

**Outcome:** A clean `pnpm dev:admin` boot: typesense connects, no Contacts prompt, admin `:45819` serves the real SPA on first load, resources/list mismatches don't warn.

### Phase 4 — Close-out

- Reconcile this doc: fill in Status per bug (Shipped/Won't-fix/Deferred), move Phase 1's SIGTERM finding into the catalog once known
- Run `pnpm validate` (fmt + clippy + check + eslint + typecheck)
- If Phase 1 identifies the SIGTERM sender, file the follow-up (timeout/dev-stop change) as its own small item rather than folding it in here

**Outcome:** Doc reflects what shipped vs. what's still open (SIGTERM root cause may carry to a follow-up). `pnpm validate` clean.

---

## Validation

```bash
pnpm test:rust        # gateway session-lifecycle behavior (Phase 2)
pnpm lint             # ESLint (dev-admin.mjs/dev-stop.mjs) + cargo clippy --workspace -- -D warnings
pnpm validate          # full gate before calling this done
```

Manual, per phase:

- Phase 1: run `pnpm dev:admin`, let it sit past the next observed SIGTERM window, read both the app log and the `dev-admin`/`dev-stop` terminal output for the new attribution lines
- Phase 2: open an agent thread, let it idle >5 min but <30 min, confirm the session/pin survives; grep the log for `← MCP error` and confirm GET 400/404 no longer appear
- Phase 3: `pnpm dev:stop && pnpm dev:admin`, confirm typesense connects, no `[Permissions] Contacts request failed`, admin `:45819` loads the real SPA immediately, and `mcp-remote-fallback-test` is gone from the client list

---

## Key Files Referenced

| File | Notes |
| ---- | ----- |
| [`apps/desktop/src-tauri/src/lib.rs`](../../apps/desktop/src-tauri/src/lib.rs) | `init_tracing` default (~L83-91, already shipped), SIGTERM/SIGINT handler (~L896-949), close-to-tray, `ensure_contacts_registered()` call site (~L280) |
| [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Empty-header warn (~L261-269, shipped), pin skip, `→ MCP` entry log, `← MCP error` exit log (~L326-335) — Phase 2 target |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | `set_pinned` no-ops on empty |
| [`crates/mcpmux-gateway/src/pool/features/resolution.rs`](../../crates/mcpmux-gateway/src/pool/features/resolution.rs) | Resolution cache + collapsed debug (already shipped) |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | `StreamableHttpServerConfig` + `LocalSessionManager::default()` (~L446-455) — Phase 2 target |
| [`crates/mcpmux-gateway/src/pool/features/discovery.rs`](../../crates/mcpmux-gateway/src/pool/features/discovery.rs) | `resources/list` capability gate + failure log (~L65-131) — Phase 3 target |
| [`crates/mcpmux-gateway/src/pool/transport/stdio.rs`](../../crates/mcpmux-gateway/src/pool/transport/stdio.rs) | stdio spawn + handshake-failure message — Phase 3 target |
| [`crates/mcpmux-gateway/src/admin/router.rs`](../../crates/mcpmux-gateway/src/admin/router.rs) | One-shot `dist/index.html` check (~L448-458) — root of B10 race, fixed on the dev-orchestration side instead |
| [`apps/desktop/src-tauri/src/macos_permissions.rs`](../../apps/desktop/src-tauri/src/macos_permissions.rs) | `ensure_contacts_registered()` (~L30-69) — Phase 3 target |
| [`apps/desktop/src-tauri/src/commands/gateway.rs`](../../apps/desktop/src-tauri/src/commands/gateway.rs) | `shutdown_gateway_handle` 2s timeout (~L115-133) — not touched this pass (Decision 2) |
| [`scripts/dev-admin.mjs`](../../scripts/dev-admin.mjs) | Signal forwarding (~L192-197) — Phase 1 target; readiness/dist-wait — Phase 3 target |
| [`scripts/dev-stop.mjs`](../../scripts/dev-stop.mjs) | `osascript` quit + SIGTERM on port PIDs (~L67-86) — Phase 1 target (logging only) |
| [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) | Agents Window spike + open question (B1, out of scope this pass) |
| [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) | User-facing empty-header fallback (B1, out of scope this pass) |
| [`docs/planning/deny-by-default-bindable-callers.md`](./deny-by-default-bindable-callers.md) | Prior note establishing rmcp 300s keepalive as expected (Jun 29) — informed Decision 3 |
| `rmcp-1.5.0/src/transport/streamable_http_server/session/local.rs` | `SessionConfig::keep_alive` default + worker timeout loop (vendored crate, not modified — config value change only) |
| `rmcp-1.5.0/src/transport/streamable_http_server/tower.rs` | Spec-correct GET 400 (`get_without_session_id_header_returns_400`) / 404 (`get_without_valid_session_returns_404`) — informed Decision 4 |

---

## Related Documentation

- [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — B1 (empty `${workspaceFolder}` header), Agents Window repro, Aug 14 open question
- [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) — user-facing fallback for B1
- [`docs/planning/search-tools-perf.md`](./search-tools-perf.md) — `resolve_feature_sets` hot path this session's cache work builds on
- [`docs/planning/deny-by-default-bindable-callers.md`](./deny-by-default-bindable-callers.md) — Jun 29 note establishing the 300s rmcp keepalive as expected client-hang behavior, not a probe bug
