# Same-Root Supersede: Process Identity Gate

**Last Updated:** Oct 30, 2026
**Status:** Implemented, PRs open
**Branch:** `fix/27-supersede-live-session-inactive` (mux), `fix/39-mux-client-instance-header` (karakeep-sync)
**Issue:** [crimsonsunset/fellowship#27](https://github.com/crimsonsunset/fellowship/issues/27) (tracked there, mcp-mux has issues disabled), [karakeep-sync#39](https://github.com/crimsonsunset/karakeep-sync/issues/39)
**Depends on:** [`window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) (`WindowKey`, `session_window`)
**Unblocks:** Probing a live Hermes chief (`hermes mcp test mux`) without breaking its `fellowship-chief` calls

## Flow state

| Field | Value |
|---|---|
| Gate | 5 (shipped) |
| Ticket | fellowship#27 |
| Branch | fix/27-supersede-live-session-inactive · fix/39-mux-client-instance-header |
| Repos | mcp-mux, karakeep-sync |
| Isolation | worktree |
| Worktrees | /Users/joe/Desktop/Repos/Personal/jsg-tech-check-wt/fellowship-27/mcp-mux<br>/Users/joe/Desktop/Repos/Personal/jsg-tech-check-wt/fellowship-27/karakeep-sync |
| Base | mcp-mux origin/main @ ab58ec75 · karakeep-sync origin/main @ 78088df |
| QA mode | lets-qa (backend only), pending deploy |
| Gates | all |
| Updated | 2026-10-30 |

---

## Problem

Oct 29, during the fellowship #21 probe. Chief's Hermes session `0bd81180` (client `mcp_a3c5335a`, header `X-Mcpmux-Workspace: /workspace/hermes-chief`) resolved through Tier 3 ClientGrant to the Gandalf FeatureSet. Three minutes later `hermes mcp test mux`, run in the same container, opened session `60a457c1` with the same client id and the same header. `register_session` logged `Superseded prior same-root sessions on reconnect superseded=1` and called `session_roots.remove("0bd81180")`.

Chief's next `mcpmux_invoke_tool(fellowship-chief, dispatch_status)` got `not_ready / inactive`. DB grants were fine the whole time.

## Root cause

Two layers.

1. **Supersede assumed "same client + same root" means "same process reconnecting".** Two live processes sharing one OAuth client and one root break that.
2. **The obvious process signal, `WindowKey`, is wrong behind the tunnel.** Chief and the probe reach mux through cloudflared, which terminates on loopback. Both sessions logged `window_key=pid:3650`, and pid 3650 is `/opt/homebrew/bin/cloudflared`. karakeep-sync (`mcp_af5debe8`) is on the same key. Every tunneled client looks like one process.

Why eviction misroutes: [`SessionRootsRegistry::remove`](../../crates/mcpmux-gateway/src/services/session_roots.rs) clears `roots_capable`. The header re-pins the root on chief's next request, but nothing re-stamps `roots_capable` (only `on_initialized` does). Hermes is rootless (`Some(false)`) and `/workspace/hermes-chief` has no path binding, so it used to fall through Tier 1b to the Tier 3 grant. With `None` it takes Tier 1b's roots-capable branch in [`feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs): **Unbound**, then `inactive`.

## Field numbers (Oct 29 log)

| Client | Path | `window_key` | Supersedes |
|---|---|---|---|
| `mcp_af5debe8` karakeep-sync | tunnel | `pid:3650` (cloudflared) | 810 |
| `mcp_ed309433` Cursor | local `mcp-remote` | per-window node PIDs | 45 |
| `mcp_a3c5335a` chief | tunnel | `pid:3650` (cloudflared) | 6 |

karakeep opens a session per relay call and peaks at 35 live sessions even with dedup. `reap_dead_sessions` fired 27 times all day, so it can't replace supersede.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Supersede key | `(client_id, root, process identity)`. Both sessions must carry a known, equal identity | Supersede exists for one process reconnecting. Two processes are never that. |
| 2 | Process identity | `X-Mcpmux-Client-Instance` header when sent, else the loopback `WindowKey` | The header is the only per-process signal that survives a tunnel. Values are prefixed (`instance:` / `pid:`) so they can't collide. |
| 3 | Proxied requests | No `WindowKey` when `Cf-Ray`, `Cf-Connecting-Ip` or `X-Forwarded-For` is present | Behind a tunnel the socket PID is the tunnel's. This also stops tunneled clients sharing one window pin, which [`window_identity.rs`](../../crates/mcpmux-gateway/src/services/window_identity.rs) already said should happen ("tunnel / remote clients must stay on per-session pins"). |
| 4 | Unknown identity on either side | No supersede | Fail safe: a missed dedup costs fanout noise, a wrong supersede misroutes credentials. |
| 5 | Header trust | Client-asserted, no auth | Supersede is still scoped to one `client_id`, so a forged value can only evict your own sessions. Values over 128 bytes or with an unexpanded `${` are ignored. |
| 6 | karakeep-sync | Sends `karakeep-sync-<pid>-<start ms>` | It relies on dedup most. PID alone repeats across container restarts (PID 1). |
| 7 | Hermes | No header | Hermes header templating only resolves `${ENV}`, `userHome`, `workspaceFolder` and path vars, and the gateway and `hermes mcp test` share one `.env`, so there's no per-process value to send. Chief gets no identity, so nothing supersedes it, which is the #27 fix. Cost: chief's own reconnects don't dedupe (6/day). Follow-up [fellowship#35](https://github.com/crimsonsunset/fellowship/issues/35). |

Rejected: re-stamping `roots_capable` per request (treats the symptom, chief still drops off `list_changed`), soft supersede (unbounded registry leak), liveness-gated supersede (rmcp doesn't flip `is_transport_closed`), and a `WindowKey` from the proxy header alone (karakeep would lose dedup and pile up).

---

## Scope

**In:**
- Proxy-header detection and `X-Mcpmux-Client-Instance` capture in the OAuth middleware
- `client_instance` map in `SessionRootsRegistry`, cleared on `remove`
- Process-identity gate in `collect_superseded_sessions`, pure helpers + unit tests
- karakeep-sync sends the header
- Follow-up issues: [fellowship#35](https://github.com/crimsonsunset/fellowship/issues/35) (Hermes has no header), [fellowship#36](https://github.com/crimsonsunset/fellowship/issues/36) (migration test already failing on main)

**Out:**

| Item | Reason |
| ---- | ------ |
| Re-stamping `roots_capable` per request | Decision 1; not needed once live sessions aren't evicted |
| Dedup for Hermes reconnects | [fellowship#35](https://github.com/crimsonsunset/fellowship/issues/35) (Decision 7) |
| Migration 39 test failure | Already red on main, [fellowship#36](https://github.com/crimsonsunset/fellowship/issues/36) |

## Deploy order

karakeep-sync first. mux ignores the unknown header today, so it's safe. If mux ships first, karakeep's tunneled sessions lose dedup until karakeep is redeployed.

---

## Architecture

```
request ─► oauth_middleware
             is_proxied   = Cf-Ray | Cf-Connecting-Ip | X-Forwarded-For
             instance     = X-Mcpmux-Client-Instance (trimmed, ≤128, no "${")
             attach_process_identity(sid)
               ├─ instance → session_roots.attach_client_instance
               └─ !is_proxied → attach_window_identity (loopback PID)

on_initialized ─► register_session ─► collect_superseded_sessions
             process_identity(sid) = "instance:<v>" | "pid:<n>" | None
             supersede old iff same client_id, same non-empty root,
                               same Some(process identity)
```

---

## What shipped

| Repo | File | Change |
| ---- | ---- | ------ |
| mcp-mux | [`consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs) | `process_identity`, `is_same_process_reconnect`, gate in `collect_superseded_sessions`, 5 unit tests |
| mcp-mux | [`mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | `is_proxied_request`, `attach_process_identity`, header constants, 1 unit test |
| mcp-mux | [`services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | `client_instance` map, `attach_client_instance` / `client_instance_for`, cleared in `remove` |
| karakeep-sync | `crates/sync/src/mcp_relay.rs` | `client_instance()` + header, 1 unit test |
| karakeep-sync | `README.md` | Header noted next to `X-Mcpmux-Workspace` |

## Phases

### Phase 1: mux process gate ✅
- Local CI mirror: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test -p mcpmux-gateway --lib` (313 pass), `cargo test -p tests` (all pass except the pre-existing migration test, #36). `nextest` isn't installed locally, so `cargo test` stood in.

**Outcome:** Tunneled sessions get no `WindowKey`. Two sessions with the same client and root supersede only when they share an instance header or a direct loopback PID.

### Phase 2: karakeep-sync header ✅
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features --workspace`: green.

**Outcome:** Every relay call from one process carries the same `X-Mcpmux-Client-Instance`.

### Phase 3: QA after deploy (pending)
- Redeploy karakeep-sync, then run the patched gateway.
- Restart chief, run `hermes mcp test mux` in its container, then call `dispatch_status` from chief: succeeds, no `Superseded` line for `mcp_a3c5335a`.
- Gateway log shows `window_key=<none>` for tunneled requests, and `Superseded` lines for `mcp_af5debe8` continue.

**Outcome:** The original repro no longer reproduces, and karakeep's session count stays flat.

---

## Related documentation

- [`window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md): where `WindowKey` comes from
- [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md): header hold-then-pin
- Commit `53d28dff`: original same-root supersede (24-session amplification)
