# Backend Connection Resilience + Bind Validation

**Last Updated:** Aug 20, 2026
**Status:** Implemented and verified on `root-resolution` — Phases 1–3. Matcher follow-up: `54d0de2` plus normalized rmcp Display matching (see Verification).
**Branch:** `root-resolution` (off `docs/aug14-gateway-ops-bugs`)
**Depends on:** `aug14-gateway-ops-bugs.md` Decision 3 (inbound session `keep_alive` 300s→1800s) — already on this branch
**Unblocks:** `mcpmux_invoke_tool` surviving a backend connection that died silently between calls, and `mcpmux_bind_current_workspace` failing cleanly instead of a raw SQLite FK error

---

## Problem

`mcpmux_invoke_tool` against a bound backend (`home-assistant-new`) started returning `{"error":"MCP error -32000: Connection closed"}` after a gap with no traffic to that server — no warning beforehand, same `server_id`/`tool`/`args` had succeeded minutes earlier. Re-calling `mcpmux_set_workspace_root` with the *same* `workspace_root` fixed it immediately, but issued a **new** inbound `session_id` for identical input.

Traced live (code reading, not guessed):

- `RoutingService::call_tool`'s error branch only retries when `is_auth_error(&err_str)` matches (`crates/mcpmux-gateway/src/pool/routing.rs:666,816`) — indicators are `"401"`, `"unauthorized"`, etc. A literal `"connection closed"` never matches, so the `else` branch (`routing.rs:776-787`) just returns the raw error with zero retry.
- `ServerInstance::is_healthy()` (`crates/mcpmux-gateway/src/pool/instance.rs:397-399`) is `state == Connected && client.is_some()` — a state flag, not a liveness check. `PoolService::connect_server()` (`crates/mcpmux-gateway/src/pool/service.rs:284-294`) trusts it and reuses the instance without ever touching the transport again. A backend that silently died still reads "healthy" until a real call fails against it.
- `PoolService::reconnect_instance()` (`service.rs:434-459`) unconditionally calls `ConnectionService::reconnect_after_oauth()` (`crates/mcpmux-gateway/src/pool/connection.rs:459`). For a `TransportType::Stdio` instance, that function hits a fallback branch that **warns and builds an `Http` transport anyway** (`connection.rs:499-502`, `"Unexpected STDIO transport for OAuth reconnection, defaulting to HTTP"`) — i.e. today's only reconnect path is not just auth-only, it actively mis-reconnects stdio backends if it's ever invoked for one.
- `InstanceKey::stdio()`/`::http()` (`crates/mcpmux-gateway/src/pool/instance.rs:266,279`) take `command`/`args`/`env`/`headers` as **unused, underscore-prefixed params** — only `space_id` and a debug `description` string survive on the stored key. The original transport config is *not* retained on `ServerInstance` after `connect_server()` returns, so any new reconnect path can't rebuild it from the instance alone — it has to re-resolve from DB, the same way `connect_server()`'s caller does today.
- The one place that already does this correctly is `retry_connection` (`apps/desktop/src-tauri/src/commands/server_manager.rs:516-547`): `pool_service.remove_instance()` then `connect_enabled_server()`, which re-resolves transport config fresh from DB and calls `connect_server()` again — transport-agnostic, unlike `reconnect_after_oauth()`. That path is Tauri-desktop-only today; `RoutingService` (gateway-internal, used by both desktop and headless/web-admin) has no equivalent.
- `mcpmux_set_workspace_root` (`crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs:53-114`) only re-seeds the **inbound** `SessionRootsRegistry` and fires `tools/list_changed` — it never touches `PoolService` or any backend instance. The "fix" working is a side effect of Cursor opening a fresh inbound rmcp session (hence the new `session_id`) — nothing in the gateway explicitly reconnects the dead backend.
- Separately: `mcpmux_bind_current_workspace` (`crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs:175-181`) loads the FeatureSet name via `feature_set_repo.get(&fs_id.to_string())`, and on `Ok(None)` (nonexistent id) falls back to `fs_id.to_string()` as the display name **instead of rejecting** — then proceeds to `binding_repo.create()`. `WorkspaceBindingRepository::create()` (`crates/mcpmux-storage/src/repositories/workspace_binding_repository.rs:236-268`) wraps the parent `INSERT` and `rewrite_fs_for_binding()`'s junction `INSERT`s (`workspace_binding_repository.rs:142-160`) in one transaction against `workspace_binding_feature_sets.feature_set_id → feature_sets(id)` — a nonexistent `feature_set_id` surfaces as a raw SQLite `FOREIGN KEY constraint failed`, not a clean domain error. The transaction wrapping means it fails atomically (no partial binding), but the error is the wrong shape for an agent to act on.
- **At investigation time the pool already had per-instance telemetry and nothing populated it at call time.** `InstanceStats` carries `connected_at`, `consecutive_failures`, `requests_served`, and `last_error`, and `record_success()` / `record_failure()` existed with **zero call sites**. Phase 1 wired them from `call_tool`. `mark_failed()` (flips `state` to `Failed`) stays connect/reconnect-only — a call-time `-32000` on a `Connected` instance used to leave `is_healthy()` true and `consecutive_failures` at 0. That is no longer true for call stats; the state-flag lie (Decision 6) is still deferred.

This is the third distinct surface of the same reconnect gap in this project's history — the [Aug 13–14 investigation](cbeb19ed-ab32-408b-9dac-37ab0ac011d7) hit "inactive" mid-call, [a later session](b9424e65-59ac-4642-bb05-c09d36eaf04c) built a habit of re-pinning without ever root-causing it, and this session's `-32000` is the same gap with a third error shape. [PR #221](https://github.com/mcpmux/mcp-mux/pull/221) (`docs/aug14-gateway-ops-bugs.md`, this branch's parent) fixed the adjacent inbound-session keep-alive and closed **without merging** on Aug 20 — its changes stay on this branch by choice (see Decision 2) but none of them touch the outbound pool.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Backend reconnect trigger | **Widen `call_tool` retry to transport-closed errors**, not just `is_auth_error()`. Matcher is case/punctuation/camelCase-normalized against rmcp Display strings (`Transport closed`, `connection closed`, `transport channel closed`, `unexpected end of stream`, `session expired`, `broken pipe`, …) plus MCP `-32000` | Directly closes the gap — a non-auth transport failure used to get zero retry. Literal `"connection closed"` / `-32000` was not enough: live stdio kill stringified as `MCP call failed: Transport closed` |
| 2 | Reconnect mechanism for the widened path | **New transport-agnostic reconnect** (`remove_instance()` + re-resolve config + `connect_server()`), not `reconnect_instance()`/`reconnect_after_oauth()` | `reconnect_after_oauth()` is OAuth-token-based and actively mis-reconnects stdio backends (`connection.rs:499-502`). The correct pattern already exists in desktop's `retry_connection` — this pulls it into gateway-internal code so both desktop and headless/web-admin get it, not just Tauri callers |
| 3 | `bind_current_workspace` FK bug | **Reject with a clean `InvalidArgument`/`NotFound` error when `feature_set_repo.get()` returns `None`**, before touching `binding_repo.create()` | Same investigation surfaced this as a distinct, isolated bug — small fix, same root-cause-tracing session, no reason to defer |
| 4 | PR #221 disposition | **Leave closed. Keep its commits on this branch (`root-resolution`) as-is** — no revert, no re-open | User call: the branch already carries the keep-alive/log-noise fixes locally; re-opening #221 isn't required to build on top of it, and nothing in this plan depends on it landing on `main` first |
| 5 | `aug14-gateway-ops-bugs.md` disposition | **Mark stale with a pointer to this doc**, following the `PR #8 review doc` precedent for flagging superseded planning docs | That doc's Phase 4 close-out never happened (PR closed instead of merged); readers need to know the connection-reliability thread continues here |
| 6 | `ServerInstance::is_healthy()` liveness probe | **Deferred, not in this pass** | Real fix for the underlying "state flag lies" problem, but bigger scope (background interval, touches the already-half-built `server_manager.rs` 60s refresh stub) and not required to close the reported bug — Decision 1/2 already recovers from a dead connection reactively, just not proactively |
| 7 | Unused `mcpmux-mcp::ClientPool` | **Deferred, not in this pass** | Has its own working 300s idle-cleanup (`client_pool.rs:87,167`) but isn't wired into the gateway's actual call path (`PoolService` is used instead) and its 300s timeout doesn't match this bug's ~15-20 min window anyway. Tech-debt consolidation, separate ticket |
| 8 | Inbound-vs-outbound diagnostic tracing | **Not a separate phase — folded into Phase 1's logging** | Enough circumstantial evidence already points at outbound pool staleness (auth-only retry, state-only `is_healthy()`, OAuth-biased reconnect) to skip a dedicated diagnose-first phase. Phase 1's new reconnect path logs which trigger fired, which gives future occurrences the same signal for free |
| 9 | Logging scope for Phase 1 | **Not just a "which trigger fired" line — wire up the already-dead `record_success()`/`record_failure()` on every `call_tool()` outcome, and add a catch-all `warn!` when a failure matches *neither* classifier** | User call: this fix is likely not the whole story, so the failure path needs to leave real breadcrumbs (instance age, consecutive failures, calls served since connect, raw error) instead of a single happy-path log line — otherwise a 4th occurrence with a novel error shape disappears silently again, same as this one did until traced by hand |
| 10 | `record_failure()` vs `mark_failed()` on a call-time failure | **Call `record_failure()` (stats-only), not `mark_failed()` (also flips `state` to `Failed`)** | `mark_failed()` changing `state` on every transient call error would make `is_healthy()` (Decision 6, deferred) start lying the *other* direction — reporting a live-but-blipped instance as failed and forcing a reconnect on the next call even when the retry in Decision 1/2 already recovered it. `record_failure()` gives the same visibility (`consecutive_failures`, `last_error`) without that side effect; state transitions stay connect/reconnect-only, unchanged from today |

---

## Scope

**In:**

- Widen `RoutingService::call_tool`'s retry trigger to include transport-closed errors, alongside the existing auth-error check
- New gateway-internal reconnect helper on `PoolService` (or a new method alongside `reconnect_instance`) that evicts + re-resolves + calls `connect_server()` fresh — transport-agnostic, mirrors `retry_connection`'s desktop pattern
- Wire the existing-but-unused `ServerInstance::record_success()`/`record_failure()` into every `call_tool()` outcome, and log a structured line (trigger matched, instance age, consecutive failures, requests served, raw error) on every failure — including a catch-all `warn!` when neither classifier matches (Decisions 9/10)
- `bind_current_workspace`: reject with `MetaToolError::InvalidArgument` (or a new `NotFound`-shaped variant if one exists) when the target `feature_set_id` doesn't resolve, before any DB write
- Mark `docs/planning/aug14-gateway-ops-bugs.md` stale with a pointer to this doc

**Out / deferred:**

| Item | Reason / Deferral |
| ---- | ------------------ |
| `ServerInstance::is_healthy()` liveness/heartbeat probe | Decision 6 — real fix for proactive detection, bigger scope, not required to close this bug. Fast-follow candidate once Decision 1/2 ships |
| Wiring or deleting `mcpmux-mcp::ClientPool` | Decision 7 — dead-code consolidation, unrelated timeout window, separate ticket |
| Reviving/re-opening PR #221 | Decision 4 — its commits already live on this branch; re-opening isn't required |
| Dedicated inbound-vs-outbound tracing phase | Decision 8 — folded into Phase 1's log lines instead of a standalone diagnostic pass |

---

## Architecture

### Widened retry trigger (Decision 1)

```rust
// crates/mcpmux-gateway/src/pool/routing.rs, call_tool() error branch (~L657-788)

// Before: only auth-shaped errors retry
let is_auth = Self::is_auth_error(&err_str);
if is_auth { /* reconnect_instance() + retry */ } else { /* return e as-is */ }

// After: auth OR transport-closed errors retry, routed to the new reconnect path
let is_auth = Self::is_auth_error(&err_str);
let is_transport_closed = Self::is_transport_closed_error(&err_str);

// Decision 9/10: always record + log the outcome, retry or not — this is
// the breadcrumb trail for whatever error shape shows up next time.
instance.record_failure(&err_str);
let stats = instance.stats.read(); // pub field, RwLock<InstanceStats> — consecutive_failures, requests_served, connected_at
tracing::warn!(
    server_id = %server_id,
    space_id = %space_id,
    tool = %tool_name,
    error = %err_str,
    trigger = if is_auth { "auth" } else if is_transport_closed { "transport_closed" } else { "unmatched" },
    instance_age_secs = stats.connected_at.map(|t| t.elapsed().as_secs()),
    consecutive_failures = stats.consecutive_failures,
    requests_served = stats.requests_served,
    "backend call_tool failed"
);

if is_auth || is_transport_closed {
    let reconnect_result = if is_auth {
        self.pool_service.reconnect_instance(space_id, &server_id).await // unchanged OAuth path
    } else {
        self.pool_service.reconnect_fresh(space_id, &server_id).await // new, Decision 2
    };
    tracing::info!(server_id = %server_id, ok = reconnect_result.is_ok(), "reconnect attempted after call_tool failure");
    // existing retry-once logic below, unchanged
} else {
    // Decision 9: neither classifier matched — this is the "4th error shape"
    // case. Log loud (already done above via the warn!) and return as-is;
    // do NOT silently swallow an unrecognized failure shape.
}
```

On success, `RoutingService::call_tool` calls `instance.record_success()` so `requests_served` reflects real traffic (Decision 9 — wired in Phase 1).

`is_transport_closed_error()` normalizes the Display string (lowercase, camelCase split, punctuation → spaces) then matches rmcp 1.5 phrases plus `-32000`. Confirmed live Aug 20 against stdio `wakatime`: `MCP call failed: Transport closed` (`rmcp::service::ServiceError::TransportClosed`). HTTP idle historically was `MCP error -32000: Connection closed`. Auth is classified first, so `401 … connection closed` still takes the OAuth path.

### Transport-agnostic reconnect (Decision 2)

```rust
// resolution.rs: resolve_auto_connection_context(repo, state_dir, space_id, server_id)
// service.rs: PoolService::reconnect_fresh(&self, ctx: &ConnectionContext)

// PoolService stays thin (no repo). RoutingService and LiveGatewayWriteRuntime
// hold InstalledServerRepository + state_dir, call resolve then reconnect_fresh.
// reconnect_fresh = remove_instance() + connect_server(ctx). Logs ok/duration_ms.
```

**Pre-flight result:** `build_transport_config()` was already gateway-internal. `PoolService` has no repo/`state_dir`, so `reconnect_fresh` takes a caller-built `ConnectionContext`. Shared resolve helper lives in `resolution.rs`; `RoutingService` gained those two deps; admin `retry_connection` now uses the same helper.

### `bind_current_workspace` FK guard (Decision 3)

```rust
// crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs (~L175-181)

// Before: falls back to the raw id string and proceeds regardless
let fs_name = call.ctx.feature_set_repo.get(&fs_id.to_string()).await?
    .map(|fs| fs.name)
    .unwrap_or_else(|| fs_id.to_string());

// After: reject before any binding_repo write
let fs_name = match call.ctx.feature_set_repo.get(&fs_id.to_string()).await? {
    Some(fs) => fs.name,
    None => {
        return Err(MetaToolError::InvalidArgument(format!(
            "feature_set_id '{fs_id}' does not exist — call mcpmux_list_feature_sets \
             to obtain a valid id"
        )));
    }
};
```

---

## Files to Modify

| File | Change |
| ---- | ------ |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | New `is_transport_closed_error()` alongside `is_auth_error()` (~L816); widen `call_tool`'s error branch (~L657-788) to route auth errors through the existing `reconnect_instance()` path and transport-closed errors through the new `reconnect_fresh()`; add the structured failure/reconnect log lines (Decisions 8/9) and `record_success()`/`record_failure()` calls |
| [`crates/mcpmux-gateway/src/pool/service.rs`](../../crates/mcpmux-gateway/src/pool/service.rs) | New `reconnect_fresh(&self, ctx: &ConnectionContext)`: `remove_instance()` + `connect_server()`, logs `ok`/`duration_ms` |
| [`crates/mcpmux-gateway/src/pool/instance.rs`](../../crates/mcpmux-gateway/src/pool/instance.rs) | No signature changes — `record_success()`/`record_failure()` now called from `routing.rs` (Decision 9) |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | New `resolve_auto_connection_context()` wrapping `build_transport_config()` + `ConnectionContext::auto` |
| [`crates/mcpmux-gateway/src/admin/write_runtime.rs`](../../crates/mcpmux-gateway/src/admin/write_runtime.rs) | `retry_connection` now uses the shared resolve + `reconnect_fresh` helper |
| [`crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs) | `feature_set_repo.get()` result (~L175-181): reject with `InvalidArgument` on `None` instead of falling back to the raw id string; log a `warn!` on rejection |
| [`docs/planning/aug14-gateway-ops-bugs.md`](./aug14-gateway-ops-bugs.md) | Header `Status` line: mark stale, point to this doc (Decision 5) |

---

## Phases

### Phase 1 — Widen backend reconnect to non-auth transport errors, with real telemetry (~1.5 days)

- `instance.rs`: wire `record_success()` into `call_tool()`'s happy path and `record_failure()` into every failure path (Decision 9) — both methods exist today and are called from nowhere; this alone makes `PoolService`'s existing stats accessors tell the truth about call-time outcomes, not just connect-time ones
- `routing.rs`: `is_transport_closed_error()` + widened `call_tool` error branch (Decision 1)
- `routing.rs`: on **every** failure (retried or not), one structured `tracing::warn!` with `server_id`, `space_id`, `tool`, raw `error`, `trigger` (`"auth"` / `"transport_closed"` / `"unmatched"`), `instance_age_secs` (from `stats.connected_at`), `consecutive_failures`, `requests_served` (Decisions 8/9) — the `"unmatched"` case is the important one: it's the catch-all that keeps a *future* novel error shape from disappearing silently the way `-32000` did until traced by hand this session
- `service.rs`: `reconnect_fresh()` — evict + re-resolve + `connect_server()`, confirming and reusing (or extracting) `connect_enabled_server`'s existing re-resolve chain — logs `ok`/`duration_ms` on completion (Decision 2)
- `routing.rs`: `tracing::info!` after any reconnect attempt (either path) with `server_id` and `ok` — makes reconnect attempts independently searchable in logs from the failure that triggered them
- Confirm `record_failure()` (stats-only) is used here, not `mark_failed()` (also flips `state` to `Failed`) — Decision 10, avoids a transient blip making `is_healthy()` lie in the *opposite* direction
- Unit test: simulate a `-32000`-shaped error from `execute_call`, assert `reconnect_fresh()` is invoked (not `reconnect_instance()`), the retry succeeds against a healthy re-resolved instance, and `record_failure()`/`consecutive_failures` reflect the failure
- Unit test: simulate an error matching neither classifier, assert the `"unmatched"` log path fires and the raw error still returns to the caller (no regression on today's behavior for truly-unknown errors — Phase 1 adds visibility, not swallowed errors)
- `cargo nextest run -p mcpmux-gateway` targeted on `pool::routing` / `pool::service` / `pool::instance`

**Outcome:** A backend connection that dies silently between calls (auth or not, stdio or HTTP) gets one automatic reconnect-and-retry instead of surfacing `-32000`/`connection closed` straight to the agent. `mcpmux_set_workspace_root` is no longer required as a side-door fix. Equally important given this is the third occurrence of this gap: if the widened trigger *doesn't* fully close it, the next occurrence has `consecutive_failures`, `instance_age_secs`, `requests_served`, and an explicit `"unmatched"` log line to start from — not another blind trace.

### Phase 2 — `bind_current_workspace` FK guard (~1 hour)

- `bind_workspace.rs`: reject on `feature_set_repo.get() == None` before any `binding_repo.create()`/`update()` call
- `bind_workspace.rs`: `tracing::warn!` on rejection with the requested `feature_set_id` and `space_id` — a client repeatedly passing a stale/nonexistent id (e.g. from cached UI state) is itself a signal worth seeing in logs, not just handling gracefully
- Unit/integration test: calling `mcpmux_bind_current_workspace` with a nonexistent `feature_set_id` returns a clean `InvalidArgument` message, not a raw SQLite FK error, and no partial binding row is created
- `cargo nextest run -p mcpmux-gateway` targeted on `meta_tools::bind_workspace`

**Outcome:** An agent passing a stale/nonexistent `feature_set_id` gets an actionable error pointing at `mcpmux_list_feature_sets`, not a raw `FOREIGN KEY constraint failed`.

### Phase 3 — Doc close-out

- Update `docs/planning/aug14-gateway-ops-bugs.md`'s header: `Status` → note PR #221 closed without merging Aug 20, 2026, pointer to this doc for the connection-reliability follow-on (Decision 5)
- Reconcile this doc's own header/decisions if Phase 1's pre-flight (re-resolve chain) revealed anything different from what's written above
- Run `pnpm validate` (fmt + clippy + check + eslint + typecheck)

**Outcome:** Both planning docs accurately reflect what shipped vs. what's deferred (liveness probe, `ClientPool` cleanup). `pnpm validate` clean.

---

## Validation

```bash
pnpm test:rust:unit   # pool::routing, pool::service, meta_tools::bind_workspace targeted tests
pnpm lint             # cargo clippy --workspace -- -D warnings
pnpm validate         # full gate before calling this done
```

Manual:

- Reproduce the original bug shape: leave `home-assistant-new` idle long enough to go stale (or force-kill its backend process if stdio, or block its port if HTTP), then call `mcpmux_invoke_tool` twice in a row — first call should trigger one automatic reconnect+retry and succeed, not surface `-32000` to the caller
- Grep the gateway log for that reproduction and confirm the new structured line appears with a real `instance_age_secs`/`consecutive_failures`/`requests_served`, plus the follow-up `"reconnect attempted after call_tool failure"` line with `ok = true`
- Force an error string that matches neither `is_auth_error()` nor `is_transport_closed_error()` (e.g. temporarily rename the check or use a malformed request) and confirm the `trigger = "unmatched"` line fires and the raw error still surfaces to the caller unchanged — this is the regression guard for "don't accidentally swallow a real unknown error just because we added logging"
- Call `mcpmux_bind_current_workspace` with a made-up UUID for `feature_set_id` — confirm a clean `InvalidArgument` message, not a raw SQLite error, a `warn!` line in the log, and no orphaned row via `sqlite3 mcpmux.db "select * from workspace_bindings"`

### Verification (Aug 20, 2026)

Playbook: [`backend-connection-resilience-test.md`](./backend-connection-resilience-test.md). Commits: `c09e569` (retry + FK guard), `54d0de2` (`Transport closed` substring). Matcher since then also normalizes rmcp Display variants.

| Case | Result |
| ---- | ------ |
| A bind FK | **PASS** — `invalid_argument` + `mcpmux_list_feature_sets`; counts unchanged |
| B stdio kill (`wakatime`) | **PASS** after `54d0de2`. First matcher miss: `MCP call failed: Transport closed` logged `trigger=unmatched`. After the substring (and now normalize): `trigger=transport_closed`, `reconnect_fresh completed ok=true`, invoke succeeded |
| C HA idle | **SKIPPED** (15–20 min wait) |
| D unmatched | **SKIPPED** — grant-layer "did you mean", never reached the classifier |

Tauri-watch rebuild mid-test wiped Streamable HTTP sessions (`POST /mcp` → 404). Reload MCP once, then pin this repo if `list_servers` reports unpinned roots. Do not use `set_workspace_root` as the reconnect itself.

---

## Key Files Referenced

| File | Notes |
| ---- | ----- |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | `call_tool()` L298; error branch L657-788; `is_auth_error()` L816-824 — Phase 1 target |
| [`crates/mcpmux-gateway/src/pool/service.rs`](../../crates/mcpmux-gateway/src/pool/service.rs) | `connect_server()` L267-341 (health check L284, non-healthy reconnect L297-301); `remove_instance()` L344; `reconnect_instance()` L434-459 (OAuth-only, unchanged) — Phase 1 target |
| [`crates/mcpmux-gateway/src/pool/instance.rs`](../../crates/mcpmux-gateway/src/pool/instance.rs) | `is_healthy()` still state-only (Decision 6, deferred); `record_success()`/`record_failure()` wired from `routing.rs` (Decision 9); `mark_failed()` stays connect/reconnect-only (Decision 10) |
| [`crates/mcpmux-gateway/src/pool/connection.rs`](../../crates/mcpmux-gateway/src/pool/connection.rs) | `reconnect_after_oauth()` L459; stdio-to-HTTP fallback warn L499-502 — root of why `reconnect_instance()` is unsafe for the widened trigger |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | `resolve_auto_connection_context()` + `build_transport_config()` — shared re-resolve for `reconnect_fresh` |
| [`apps/desktop/src-tauri/src/commands/server_manager.rs`](../../apps/desktop/src-tauri/src/commands/server_manager.rs) | `retry_connection()` L516-547 — existing evict-then-reconnect pattern `reconnect_fresh()` generalizes into gateway-internal code |
| [`crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs) | `feature_set_repo.get()` fallback L175-181 — Phase 2 target |
| [`crates/mcpmux-storage/src/repositories/workspace_binding_repository.rs`](../../crates/mcpmux-storage/src/repositories/workspace_binding_repository.rs) | `create()` L236-268; `rewrite_fs_for_binding()` L142-160 — FK constraint site, not modified (guard moves upstream instead) |
| [`crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs) | `SetWorkspaceRootTool::call()` L53-114 — confirmed this only touches `SessionRootsRegistry`, never `PoolService`; explains why it "fixed" the bug only as a side effect |
| [`crates/mcpmux-mcp/src/client_pool.rs`](../../crates/mcpmux-mcp/src/client_pool.rs) | `DEFAULT_IDLE_TIMEOUT` L87 (300s), `cleanup_idle()` L167 — unused by the gateway, Decision 7 defers this |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | `session_config.keep_alive = 1800s` (~L454) — inbound-side fix already on this branch via PR #221's commits, unrelated to the outbound gap this doc fixes |
| [PR #221](https://github.com/mcpmux/mcp-mux/pull/221) | Closed without merging Aug 20, 2026; this branch (`root-resolution`) carries its commits regardless (Decision 4) |
| [Aug 13–14 gateway ops investigation](cbeb19ed-ab32-408b-9dac-37ab0ac011d7) | First surface of this reconnect gap ("inactive" mid-call), never root-caused there |
| [Repeated set_workspace_root re-pin session](b9424e65-59ac-4642-bb05-c09d36eaf04c) | Second surface, same workaround habit, no RCA |
| [This session's dig-and-ask investigation](198c4918-6b75-46cc-84c7-ea94b9753236) | Third surface (`-32000`); root-caused the OAuth-bias in `reconnect_instance()` and the FK gap in `bind_workspace.rs` |

---

## Related Documentation

- [`docs/planning/backend-connection-resilience-test.md`](./backend-connection-resilience-test.md) — manual playbook + Aug 20 results (A/B pass; C skipped; D never reached classifier)
- [`docs/planning/aug14-gateway-ops-bugs.md`](./aug14-gateway-ops-bugs.md) — this branch's parent investigation (inbound session keep-alive, log noise); marked stale by this doc (Decision 5)
- [`docs/planning/clone-auth-header-config-editing.md`](./clone-auth-header-config-editing.md) — separate "Connection closed" bug (stale pool reuse after a config edit), same `PoolService::connect_server()` healthy-reuse code path this doc's Decision 1/2 also touches
- [`docs/planning/deny-by-default-bindable-callers.md`](./deny-by-default-bindable-callers.md) — established rmcp's inbound keepalive as expected client-hang behavior (Jun 29), informed PR #221's Decision 3
- [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — `set_workspace_root`/`SessionRootsRegistry` design this doc confirms is inbound-only, not a backend reconnect mechanism
