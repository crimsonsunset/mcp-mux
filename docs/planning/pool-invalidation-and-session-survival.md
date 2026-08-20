# Pool Invalidation, Stdio Reconnect Hygiene, Session Survival

**Last Updated:** Aug 20, 2026
**Status:** Implemented and verified on `root-resolution` (`dcc2977`). Playbook: [`pool-invalidation-and-session-survival-test.md`](./pool-invalidation-and-session-survival-test.md) — E/F/H PASS, G BLOCKED (admin 401). Inbound 404 body unchanged (mcp-remote / TS SDK do not re-init).
**Branch:** `root-resolution`
**Depends on:** [`backend-connection-resilience.md`](./backend-connection-resilience.md) Phases 1–3 (shipped and verified Aug 20). Reuses `PoolService::reconnect_fresh` + `resolve_auto_connection_context`.
**Unblocks:** Config/definition saves taking effect without a process restart; stdio never mis-routed through OAuth reconnect; inbound MCP surviving a gateway rebuild without a fake "everything is bindable" state

---

## Problem

Outbound `call_tool` retry is done. The Aug 20 verification of [`backend-connection-resilience-test.md`](./backend-connection-resilience-test.md) closed Cases A/B and then surfaced three adjacent holes that the matcher work must **not** keep widening into.

Traced live at planning time (code + the Case B run, not guessed). **Shipped in `dcc2977`:** handler `reconnect_fresh`, stdio OAuth refuse, hold-then-pin. What follows is the pre-fix snapshot:

- [`ServerConfigUpdatedHandler`](../../crates/mcpmux-gateway/src/consumers/server_config_handler.rs) **evicted only**. `handle_config_updated` called `pool_service.remove_instance()` and returned. Next `call_tool` then failed with `format_server_bound_offline_error` ("bound but not connected"). Clone-auth Decision 4 asked for evict **+ reconnect**. After a Configure/definition/`UserSpaceSync` save, the next invoke died until something else reconnected.
- `PoolService::connect_server` returned early when `is_healthy()` was true. Evict-only is why a save could be seen; without a follow-up `reconnect_fresh`, the healthy-reuse lie became a "not connected" lie.
- `reconnect_instance` → `reconnect_after_oauth` warned `"Unexpected STDIO transport for OAuth reconnection, defaulting to HTTP"` and built an HTTP transport. `call_tool` no longer used that path for transport-closed errors, but `read_resource` / `get_prompt` still did on auth errors.
- A Tauri-watch rebuild (or any gateway process death) drops the in-memory `LocalSessionManager`. Inbound `POST /mcp` with a stale `Mcp-Session-Id` is a spec-correct **404**. Middleware already treats 404 as expected session noise (`oauth_middleware.rs`). Clients do not re-`initialize` on their own; the working recovery is Reload MCP. Sessions cannot be persisted across process death in a way Cursor will honor.
- After that reload, a multi-folder Cursor window reports ~6 `roots/list` entries with no pin → resolver `PendingRoots` → every server `bindable`. `X-Mcpmux-Workspace` already shadows roots **once pinned**, but at planning time pin was skipped when the header arrived without `mcp-session-id` (initialize). Empty header still skips. `mcpmux_set_workspace_root` is a session disambiguation tool, not a reconnect tool.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Config/definition save | **Eager `reconnect_fresh` on `ServerConfigUpdated`**, not evict-only | Completes clone-auth Decision 4 with the primitive that already exists. Evict without reconnect leaves enabled servers "bound but not connected" on the next invoke. Highest reuse, same event the handler already listens for |
| 2 | Stdio × OAuth reconnect | **Refuse stdio in `reconnect_after_oauth`** (Failed, no HTTP fallback). Auth retries on `read_resource` / `get_prompt` go through `reconnect_fresh` | The warn-and-default-to-HTTP branch is a landmine, not a fallback. Stdio does not have an OAuth registration URL that is meaningful as a transport |
| 3 | Inbound 404 after gateway restart | **Do not persist Streamable HTTP sessions.** Document Reload MCP as recovery. If `mcp-remote` re-`initialize`s on a specific 404 JSON-RPC shape, return that shape; otherwise stop | Process death kills `LocalSessionManager`. Faking session continuity would lie to the client. 404 warn-spam is already suppressed |
| 4 | Unpinned roots after reload | **If `X-Mcpmux-Workspace` is a non-empty path, trust it.** Buffer the header until `mcp-session-id` exists, pin, invalidate resolve cache. Do not treat a 6-way `roots/list` as a binding | This *is* the `root-resolution` hole the Case B confounder hit. Pin already shadows roots; initialize-without-session-id drops the header on the floor |
| 5 | Matcher / heartbeat / `ClientPool` | **Out.** Matcher stays where `2da2f50` left it. `is_healthy()` heartbeat stays deferred (`backend-connection-resilience.md` Decision 6). Unused `mcpmux-mcp::ClientPool` stays deferred (Decision 7) | Widening the matcher again is how this ticket grows forever. Heartbeat is a first-call hitch fix, not a save/reload fix |
| 6 | Case C / Case D | **Not required for this pass.** Case C is optional HTTP confirmation. Case D died at the grant layer ("did you mean"), never the classifier | Same verification session; neither is a reconnect or session-survival bug |

---

## Scope

**In:**

- `ServerConfigUpdatedHandler` calls `resolve_auto_connection_context` + `reconnect_fresh` for enabled servers (same helper as admin `retry_connection`)
- `reconnect_after_oauth` refuses `TransportType::Stdio`; remaining auth-retry call sites that can hit stdio use `reconnect_fresh`
- Inbound 404: recovery note in the existing test playbook; optional 404 body that makes `mcp-remote` re-`initialize` (only if a known shape exists — do not invent a new session protocol)
- Header pin: hold a non-empty `X-Mcpmux-Workspace` across initialize → first session-id, then pin and drop out of `PendingRoots`

**Out / deferred:**

| Item | Reason / Deferral |
| ---- | ------------------ |
| Further `is_transport_closed_error` widening | Decision 5 — outbound retry is verified (Case B). New shapes get an `unmatched` log line, not a new substring |
| `ServerInstance::is_healthy()` liveness/heartbeat | `backend-connection-resilience.md` Decision 6 — skip until a first-call hitch is a real complaint |
| Deleting unused `mcpmux-mcp::ClientPool` | Decision 7 there — dead-code consolidation, separate ticket |
| Persisting inbound Streamable HTTP sessions across process death | Decision 3 — Cursor will not honor a resurrected `Mcp-Session-Id` |
| Clone UI / definition-editor / auth-header seeding | Lives in [`clone-auth-header-config-editing.md`](./clone-auth-header-config-editing.md). This doc only finishes Decision 4's pool half |
| Case C (HA idle HTTP) as a required gate | Decision 6 — optional confirmation only |
| Case D classifier work | Decision 6 — grant-layer "did you mean", not reconnect |
| Empty `${workspaceFolder}` / Agents-window header | [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) open question. This pass only handles a *non-empty* header that arrives before `mcp-session-id` |

---

## Architecture

### Config save → reconnect (Decision 1)

```text
update_config / update_definition / UserSpaceSync
        │
        ▼
DomainEvent::ServerConfigUpdated { space_id, server_id }
        │
        ▼
ServerConfigUpdatedHandler          (already registered in server/mod.rs)
  enabled? ──no──► skip
        │ yes
        ▼
  resolve_auto_connection_context    (resolution.rs — same as retry_connection)
        │
        ▼
  pool_service.reconnect_fresh(ctx)  // remove_instance + connect_server
```

Handler needs a `data_dir` (or equivalent) so resolve can read the same transport config `retry_connection` does. Evict-only stays as the fallback if resolve fails (log + leave evicted).

`connect_server`'s `is_healthy()` reuse is unchanged. The event path evicts first, so reuse cannot serve the pre-save instance.

### Stdio refuse (Decision 2)

```text
reconnect_after_oauth
  Http  → existing OAuth URL + token path
  Stdio → ConnectionResult::Failed { "stdio cannot reconnect via OAuth; use reconnect_fresh" }
          (delete the HTTP fallback + warn)

read_resource / get_prompt auth retry
  → reconnect_fresh_from_db (same helper RoutingService already has)
    not reconnect_instance
```

### Inbound 404 (Decision 3)

```text
gateway process dies
  LocalSessionManager gone
  client still sends Mcp-Session-Id
  POST /mcp → 404          (already not a warn)
  recovery: Reload MCP     (new initialize, new session id)
```

Do not write sessions to disk. If investigation finds a 404 JSON-RPC body `mcp-remote` already treats as "re-initialize," return that body. If not, the playbook sentence is the product.

### Header pin after session recreate (Decision 4)

```text
initialize POST
  X-Mcpmux-Workspace=/Users/joe/Desktop/Repos/Personal/mcp-mux
  mcp-session-id: absent     ← shipped: hold, then pin when sid appears
        │
        ▼
  hold pending pin keyed by connection / next session id
        │
mcp-session-id issued
        │
        ▼
  set_pinned(session_id, path)
  invalidate FeatureResolution cache
  resolver: pinned root → WorkspaceBinding
  6-way roots/list is shadowed, not a binding
```

Empty header stays the Agents-window problem (out). `set_workspace_root` stays the manual escape hatch, never a reconnect stand-in.

---

## Files to Modify (shipped in `dcc2977`)

| File | Change |
| ---- | ------ |
| [`crates/mcpmux-gateway/src/consumers/server_config_handler.rs`](../../crates/mcpmux-gateway/src/consumers/server_config_handler.rs) | After enabled check: resolve + `reconnect_fresh`. Keep evict-only if resolve fails. Structured log: `ok` / `duration_ms` / server_id |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | Pass `data_dir` (or resolve deps) into `ServerConfigUpdatedHandler::new` |
| [`crates/mcpmux-gateway/src/pool/connection.rs`](../../crates/mcpmux-gateway/src/pool/connection.rs) | `reconnect_after_oauth`: Stdio → Failed. Delete HTTP fallback + warn |
| [`crates/mcpmux-gateway/src/pool/service.rs`](../../crates/mcpmux-gateway/src/pool/service.rs) | `read_resource` / `get_prompt` auth retry: `reconnect_fresh` (needs a resolve at this layer, or a small helper next to `reconnect_fresh_from_db`) |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | Only if the resolve helper is lifted out of `RoutingService` for pool-level reuse. No matcher edits |
| [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Buffer non-empty `X-Mcpmux-Workspace` when `mcp-session-id` is missing; pin once the session id appears. Optional: 404 body for Decision 3 |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | Pending-pin API if the hold does not belong in middleware alone |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | Confirm pin invalidates cached `PendingRoots`; fix if a 6-root probe can stick after `set_pinned` |
| [`docs/planning/backend-connection-resilience-test.md`](./backend-connection-resilience-test.md) | Reload-MCP recovery + "do not use `set_workspace_root` as reconnect" already there; add config-save + header-pin cases |
| [`docs/planning/clone-auth-header-config-editing.md`](./clone-auth-header-config-editing.md) | Decision 4 evict **and** reconnect shipped here |

---

## Phases

### Phase 1 — Config save reconnects — **shipped** (`dcc2977`)

- `ServerConfigUpdatedHandler`: add resolve deps; call `reconnect_fresh` for enabled servers
- Log `reconnect_fresh` outcome on this path (same `ok` / `duration_ms` fields as the call-tool path)
- Unit test: handler with an enabled server evicts the stale instance and `connect_server` sees the post-save transport (e.g. changed `extra_headers` / command). Disabled server: no reconnect
- Confirm `UserSpaceSync` already emits `ServerConfigUpdated` (it does as of the current branch). No second event

**Outcome:** Save a header or command on an enabled server, invoke a tool on that server without Retry Connection in the UI. The call succeeds against the new config. Log shows `ServerConfigHandler` + `reconnect_fresh completed ok=true`. Killing the process is not required.

### Phase 2 — Stdio cannot go through OAuth reconnect — **shipped** (`dcc2977`)

- `reconnect_after_oauth`: Stdio → `ConnectionResult::Failed` with a clear error. Delete the HTTP fallback
- `read_resource` / `get_prompt`: auth-error retry uses `reconnect_fresh`, not `reconnect_instance`
- Unit test: `reconnect_after_oauth` on a stdio instance does not build `ResolvedTransport::Http` and does not call `TransportFactory` with a URL
- Unit test: `read_resource` auth failure on a stdio-shaped instance goes through `reconnect_fresh`

**Outcome:** Grep the crate: the string `Unexpected STDIO transport for OAuth reconnection` is gone. A stdio auth-shaped error cannot spawn an HTTP client to the OAuth registration URL.

### Phase 3 — Inbound 404 after process death — **shipped** (playbook only; 404 body unchanged)

- Confirm current 404 body vs what [`mcp-remote`](https://www.npmjs.com/package/mcp-remote) does on `404` / missing session (read its reconnect path; do not guess)
- If a known re-`initialize` shape exists, return it from the MCP nest. If not, leave the 404 as-is
- Playbook: one paragraph — gateway rebuild → 404 is expected → Reload MCP once → do not treat 404 as "gateway is down" (`/health` is the liveness check)
- No session persistence. No new keep-alive number (1800s already on this branch)

**Outcome:** After `pnpm dev` rebuilds the gateway, Cursor chats that 404 recover with one MCP reload (or automatically if the 404 body change works). `/health` stays 200 the whole time. Logs do not warn-spam 404.

### Phase 4 — Trust a non-empty workspace header — **shipped** (`dcc2977`)

- Middleware (or `SessionRootsRegistry`): if `X-Mcpmux-Workspace` is a non-empty path and `mcp-session-id` is missing, hold it; pin on the first request that has both
- After `set_pinned`, resolver must not stay on cached `PendingRoots` for that session
- Unit test: initialize with header, no session id → later POST with session id + same header → `get()` returns the pinned path, not a 6-root ambiguous set
- Unit test: empty header still skips pin (Agents-window behavior unchanged)
- Playbook: after Reload MCP on a multi-folder window that *sends* a real header, `mcpmux_list_servers` is bound, not 6-way `bindable`. `set_workspace_root` is not part of the success path

**Outcome:** Reload MCP after a gateway restart, with `X-Mcpmux-Workspace` set to this repo. Invoke works without `mcpmux_set_workspace_root`. A 6-folder `roots/list` is ignored while that header is set.

---

## Validation

```bash
cargo nextest run -p mcpmux-gateway --lib
# targeted: consumers::server_config_handler, pool::connection, pool::service, session_roots, feature_set_resolver
pnpm lint
```

Manual (same caveats as the parent playbook: mux only via `user-mcpmux`, do not `pkill -f mcpmux`, log at `~/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.YYYY-MM-DD.log`):

- **Config save:** change an enabled server's header or stdio arg, invoke immediately, confirm `reconnect_fresh` on the handler path and the new value is used
- **Stdio refuse:** no HTTP fallback string in logs; `wakatime` still reconnects via the existing transport-closed path (regression guard for Case B)
- **Rebuild 404:** trigger a Rust rebuild, confirm `/health` 200, chats 404, Reload MCP recovers
- **Header pin:** multi-folder Cursor window with a real `X-Mcpmux-Workspace`, Reload MCP, `list_servers` is not all-`bindable`

---

## Key Files Referenced

| File | Notes |
| ---- | ----- |
| [`crates/mcpmux-gateway/src/consumers/server_config_handler.rs`](../../crates/mcpmux-gateway/src/consumers/server_config_handler.rs) | Enabled servers: resolve + `reconnect_fresh`. Evict-only only if resolve fails |
| [`crates/mcpmux-gateway/src/pool/service.rs`](../../crates/mcpmux-gateway/src/pool/service.rs) | `reconnect_fresh` / `reconnect_fresh_from_db`; `read_resource`/`get_prompt` auth retry uses the latter |
| [`crates/mcpmux-gateway/src/pool/connection.rs`](../../crates/mcpmux-gateway/src/pool/connection.rs) | Stdio → `Failed` (`stdio cannot reconnect via OAuth`). HTTP fallback deleted |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | `resolve_auto_connection_context` — shared by handler, routing, admin `retry_connection` |
| [`crates/mcpmux-gateway/src/admin/write_runtime.rs`](../../crates/mcpmux-gateway/src/admin/write_runtime.rs) | `retry_connection` uses `reconnect_fresh` |
| [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Non-empty header without sid: hold, then pin. Empty header still skipped. 404 is expected noise |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | `remember_pending_workspace` / `apply_pending_workspace`; `set_pinned` clears last resolution |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | Multi-root + no pin → `PendingRoots` (~L573) |
| [`crates/mcpmux-gateway/src/pool/routing.rs`](../../crates/mcpmux-gateway/src/pool/routing.rs) | `reconnect_fresh_from_db`; missing instance → bound-offline error. Matcher frozen |
| [`docs/planning/backend-connection-resilience.md`](./backend-connection-resilience.md) | Parent. Decisions 6/7 stay deferred. Verification table Aug 20 |

---

## Related Documentation

- [`docs/planning/backend-connection-resilience.md`](./backend-connection-resilience.md) — outbound retry + bind FK. This doc is the follow-on after verification
- [`docs/planning/pool-invalidation-and-session-survival-test.md`](./pool-invalidation-and-session-survival-test.md) — Aug 20 live results (E/F/H PASS, G BLOCKED)
- [`docs/planning/backend-connection-resilience-test.md`](./backend-connection-resilience-test.md) — Case A/B results; reload/pin confounders that became Decisions 3/4 here
- [`docs/planning/clone-auth-header-config-editing.md`](./clone-auth-header-config-editing.md) — Decision 4 pool half shipped here. Clone UI/header work stays there
- [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — empty `${workspaceFolder}` / Agents window (out of this pass)
- [`docs/planning/resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) — inventory of everything still parked or open after this doc shipped, including Case G/C/D verification and the Agents-window hole
- [`docs/planning/aug14-gateway-ops-bugs.md`](./aug14-gateway-ops-bugs.md) — inbound keep-alive 1800s, 404-as-noise. Stale; commits live on this branch
- [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) — per-repo static header fallback when the templated header is empty
