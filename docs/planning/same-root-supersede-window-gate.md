# Same-Root Supersede: Window Gate

**Last Updated:** Oct 29, 2026
**Status:** Planned
**Branch:** `fix/27-supersede-live-session-inactive`
**Issue:** [crimsonsunset/fellowship#27](https://github.com/crimsonsunset/fellowship/issues/27) (tracked there, mcp-mux has issues disabled)
**Depends on:** [`window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) (`WindowKey`, `session_window`)
**Unblocks:** Probing a live Hermes chief (`hermes mcp test mux`) without breaking its `fellowship-chief` calls

## Flow state

| Field | Value |
|---|---|
| Gate | 2 (plan ready) |
| Ticket | fellowship#27 |
| Branch | fix/27-supersede-live-session-inactive |
| Repos | mcp-mux |
| Isolation | worktree |
| Worktrees | /Users/joe/Desktop/Repos/Personal/jsg-tech-check-wt/fellowship-27/mcp-mux |
| Base | origin/main @ ab58ec75 |
| QA mode | lets-qa (backend only, no UI) |
| Gates | all |
| Updated | 2026-10-29 |

---

## Problem

Oct 29, during the fellowship #21 probe. Chief's Hermes session `0bd81180` (client `mcp_a3c5335a`, header `X-Mcpmux-Workspace: /workspace/hermes-chief`) resolved through Tier 3 ClientGrant to the Gandalf FeatureSet. Three minutes later `hermes mcp test mux`, run in the same container, opened session `60a457c1` with the same client id and the same header. `register_session` logged `Superseded prior same-root sessions on reconnect superseded=1` and called `session_roots.remove("0bd81180")`.

Chief's next `mcpmux_invoke_tool(fellowship-chief, dispatch_status)` got `not_ready / inactive`. DB grants were fine the whole time.

## Root cause

[`SessionRootsRegistry::remove`](../../crates/mcpmux-gateway/src/services/session_roots.rs) clears everything for the session, including `roots_capable`. The header re-pins the root on chief's next request ([`oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs)), but nothing re-stamps `roots_capable`; that only happens once, in `on_initialized`.

Hermes is rootless (`Some(false)`). `/workspace/hermes-chief` has no path binding, so before the supersede it hit Tier 1b's rootless branch and fell through to the Tier 3 grant. After it, `roots_capable == None` is treated as "probably capable", which takes the `else` branch of Tier 1b in [`feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs): **Unbound**. `invoke_tool` then reports the server as `inactive`.

The real defect is upstream of that: supersede evicted a session that was still alive. It assumes "same `client_id` + same root" means "same client reconnecting", and two processes sharing one OAuth client and one root break that.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Supersede key | `(client_id, root, window_key)`. Both sessions must carry a known, equal `WindowKey` | Supersede exists for `mcp-remote` reconnects, which come from one process. Different processes (Hermes + CLI probe, or a Docker bridge peer with no loopback key) are never the same client reconnecting. Picked over re-stamping `roots_capable` per request (treats the symptom, still kills chief's `list_changed`), soft supersede (unbounded registry leak), and liveness-gated supersede (rmcp doesn't flip `is_transport_closed`, so it brings back the 24-session pileup). |
| 2 | Unknown window on either side | No supersede | Fail safe: a missed dedup costs fanout noise, while a wrong supersede misroutes credentials. |
| 3 | Testability | Pull the match into a pure fn and unit test it | The notifier has no tests and an rmcp `Peer` is heavy to build. The rule is the part that can regress. |
| 4 | Remote-reconnect fanout gap | `ponytail:` comment + follow-up issue | Non-loopback clients lose dedup. Reaper and restarts cover them today. |

---

## Scope

**In:**
- Window-gated match in `collect_superseded_sessions`
- Pure helper + unit tests covering same window (supersede), different window (keep), missing window either side (keep), different root (keep)
- `ponytail:` comment naming the remote-client ceiling
- Follow-up issue on crimsonsunset/fellowship for remote-client dedup

**Out:**

| Item | Reason |
| ---- | ------ |
| Re-stamping `roots_capable` per request | Decision 1; not needed once live sessions aren't evicted |
| Dedup for non-loopback reconnects | Follow-up issue (Decision 4) |
| rmcp transport-close callback | Upstream rmcp gap, separate from this bug |

---

## Architecture

```
on_initialized(new_sid)
  └─ register_session(new_sid, client_id, peer)
       └─ collect_superseded_sessions(new_sid, client_id)
            new_root   = session_roots.get(new_sid)          (unchanged)
            new_window = session_roots.window_key_for(new_sid)   ← new
            for each old_sid with same client_id:
               is_same_window_reconnect(new_root, new_window,
                                        old_root, old_window)   ← new pure fn
```

`window_key` is attached in the OAuth middleware before the request reaches the handler (`attach_window_identity` on the request's `mcp-session-id`, and again on the response header for `initialize`), so it's set by the time `on_initialized` runs for loopback peers.

---

## Files to modify

| File | Change |
| ---- | ------ |
| [`crates/mcpmux-gateway/src/consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs) | Window-gated match, pure helper, unit tests, updated comments |

---

## Phases

### Phase 1: Window-gated supersede (~1h)
- Add `is_same_window_reconnect` helper; call it from `collect_superseded_sessions`
- Update the `register_session` comment block and the `ponytail:` ceiling note
- Unit tests for the helper
- Local CI mirror: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p mcpmux-gateway --lib`

**Outcome:** Two sessions with the same `client_id` and root but different or missing `WindowKey` both stay in the notifier map and keep their `session_roots` entries. Two with the same key still collapse to one.

### Phase 2: Follow-up + reconcile (~15m)
- Open the remote-dedup follow-up issue on crimsonsunset/fellowship, link it here
- Reconcile this doc against what shipped

**Outcome:** The ceiling has a ticket, and this doc matches the diff.

### Phase 3: QA against live chief (Stage 4)
- Run the patched gateway, restart chief, run `hermes mcp test mux` in the chief container, then call `dispatch_status` from chief
- Logs show no `Superseded` line for `mcp_a3c5335a`, and the call succeeds

**Outcome:** The original repro no longer reproduces.

---

## Key files referenced

| File | Notes |
| ---- | ----- |
| [`mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs) | `register_session`, `collect_superseded_sessions`, `reap_dead_sessions` |
| [`session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | `remove` wipes `roots_capable`; `window_key_for` |
| [`feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | Tier 1b `None` vs `Some(false)` split |
| [`oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Per-request header pin, `attach_window_identity` |
| [`window_identity.rs`](../../crates/mcpmux-gateway/src/services/window_identity.rs) | Loopback-only `WindowKey` |

## Related documentation

- [`window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md): where `WindowKey` comes from
- [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md): header hold-then-pin
- Commit `53d28dff`: original same-root supersede (24-session amplification)
