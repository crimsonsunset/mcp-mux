# Resilience & Routing Leftovers

**Last Updated:** Aug 20, 2026
**Status:** Planning — inventory doc. Nothing here is implemented yet except where noted "shipped."
**Branch:** `root-resolution`
**Depends on:** [`backend-connection-resilience.md`](./backend-connection-resilience.md) Phases 1–3 (shipped), [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md) Phases 1–4 (shipped, `dcc2977`)
**Unblocks:** A single place to look for "what's still parked or open" instead of hunting through three shipped docs' Out/Deferred tables

---

## Why this doc exists

`backend-connection-resilience.md` and `pool-invalidation-and-session-survival.md` both shipped and both carry an Out/Deferred table. Those tables are correct but scattered, and `clone-auth-header-config-editing.md` had drifted — its plan still described Phases 1/2/4 as open when the code had already landed them (desktop side). This doc collects everything still parked or still open in one place, and does not re-litigate anything already shipped. Item 4 (web admin clone parity) has since shipped (`f024d9e`) — kept below for history since it was the one active, non-frozen item when this doc was written.

---

## Frozen parking (no phases, not being worked)

These were deliberately deferred in prior planning docs. They stay frozen until a real complaint reopens them.

- **Matcher freeze.** [`is_transport_closed_error()` / `normalize_error_tokens()`](../../crates/mcpmux-gateway/src/pool/routing.rs) stay where `2da2f50` left them (lowercase, camelCase split, punctuation → spaces, phrase list, `-32000`). A new error shape logs `trigger=unmatched` and returns the raw error — that is the intended behavior, not a bug. Do not add more substrings without a live repro (`backend-connection-resilience.md` Decision 5).
- **Inbound Streamable HTTP sessions are not persisted.** `LocalSessionManager` (`crates/mcpmux-gateway/src/server/mod.rs`) is in-memory with `keep_alive` at 1800s. Gateway process death drops it; a stale `Mcp-Session-Id` gets a spec-correct 404, already treated as expected noise in [`oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs). [`mcp-remote`](https://www.npmjs.com/package/mcp-remote) and the [TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk/issues/1708) do not re-`initialize` on that 404. Recovery is Reload MCP once; `/health` staying 200 is the liveness signal, not `/mcp` 404s (`pool-invalidation-and-session-survival.md` Decision 3).
- **Clone-auth desktop half — shipped.** `clone_server()` rewrites `definition.source` to `ManualEntry`, seeds `extra_headers`/`input_values` from the parent, and `update_definition()` + the empty-header warning banner + seed-then-Configure flow all exist in code (`crates/mcpmux-core/src/application/server.rs`, `apps/desktop/src/features/servers/ServersPage.tsx`, `CloneAccountModal.tsx`) — reachable through desktop Tauri.
- **Clone-auth web admin half — shipped (`f024d9e`).** See item 4 below; kept for history.

---

## Next work (short-form, not full phase plans)

None of these are being implemented in this pass. Each entry is enough to start from later without re-digging.

### 1. Empty `${workspaceFolder}` / Cursor Agents window

A present-but-empty `X-Mcpmux-Workspace` header still warn-skips the pin in [`oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) (`[SessionRoots] X-Mcpmux-Workspace present but empty — pin skipped`). `set_pinned()` / `remember_pending_workspace()` in [`session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) already normalize and no-op on empty, so the gap is entirely upstream: Cursor's Agents window (or `mcp-remote`'s own `${ENV}` substitution) is not resolving `${workspaceFolder}` before spawn. There is no runtime signal in the gateway that distinguishes an Agents-window session from an editor-window session — only the empty header itself. Next step is a reliable repro of the spawn path (which Cursor surface, whether the literal string `${workspaceFolder}` arrives or gets stripped to empty) before any fix — this is the open question already logged in [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md). Fallback stays the per-repo static header install (`workspace_install.rs`). Do not attempt server-side PID/process-tree inference — already a dead end (TCP transport, no child PID visible to the gateway).

### 2. `is_healthy()` heartbeat / liveness probe

[`ServerInstance::is_healthy()`](../../crates/mcpmux-gateway/src/pool/instance.rs#L397) is `state == Connected && client.is_some()` — a state flag, not a transport check. Three call sites trust it: `PoolService::connect_server()`'s healthy-reuse (`service.rs`), `PoolService::is_connected()`, and `ConnectionService::connect_with_instance()`'s reconnect-skip. The existing 60s loop in [`server_manager.rs`](../../crates/mcpmux-gateway/src/pool/server_manager.rs#L1476) (`start_periodic_refresh`, `REFRESH_INTERVAL`) is a **different state machine** — it walks `ServerManager`'s UI-facing `ConnectionStatus` map, and `refresh_single_server()` (`server_manager.rs#L1297`) is trace-log-only today, touching neither `PoolService` nor any `ServerInstance`. It is also desktop-only (wired from `apps/desktop/src-tauri/src/commands/gateway.rs`), not started in headless/web-admin. A real liveness probe would need to run against `PoolService` instances directly, not piggyback on the UI loop as-is. Keep Decision 10 from `backend-connection-resilience.md`: a transient call failure should stay `record_failure()` (stats-only), not `mark_failed()` — a heartbeat changes detection, not that split.

### 3. Delete unused `ClientPool`

[`crates/mcpmux-mcp/src/client_pool.rs`](../../crates/mcpmux-mcp/src/client_pool.rs) has a 300s `DEFAULT_IDLE_TIMEOUT` (`#L87`) and a `cleanup_idle()` (`#L167`) that is never called from anywhere in the workspace. There are zero `use mcpmux_mcp::ClientPool` references outside the crate itself, and `mcpmux-gateway/Cargo.toml` does not depend on `mcpmux-mcp` at all — the gateway's actual pool is `PoolService` + rmcp directly. Before deleting, confirm whether the entire `mcpmux-mcp` crate (not just `ClientPool`) is orphaned — it is a workspace member with no importers found during this dig, which would make it a bigger deletion than originally scoped.

### 4. Web admin clone parity — **shipped (`f024d9e`)**

`clone_server`, `is_clone_id_available`, `suggest_clone_suffix`, `list_clone_dependents`, and `set_server_display_name` are un-stubbed in [`command_bridge/write.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/write.rs) / [`command_bridge/read.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/read.rs), delegating to the same `ServerAppService` methods desktop Tauri's [`server_clone.rs`](../../apps/desktop/src-tauri/src/commands/server_clone.rs) already used — the web admin frontend (`CloneAccountModal.tsx`, Configure modal) already had the UI wired, so no frontend work was needed. Bonus fix landed in the same commit: `save_server_inputs` was silently dropping `display_name_override` on both runtimes and `update_policy`/`pinned_version` on web admin specifically — now forwards all three. Full spec was Phase 5 in [`clone-auth-header-config-editing.md`](./clone-auth-header-config-editing.md); not tracked in `dev-to-main-port.md`'s Phase 6, which is unrelated desktop cloning UI.

### 5. Leftover verification (playbook cases, not code)

- **Case G (config-save reconnect):** handler code already shipped in `dcc2977` (`ServerConfigUpdatedHandler` → `reconnect_fresh after config update`). The manual run was BLOCKED on admin `:45819` returning 401 (no CF Access probe headers in the test shell). Retrying through the desktop Configure UI save path (which also emits `ServerConfigUpdated`) should unblock this without needing admin credentials at all — do not sqlite-edit the config directly, that produces no event.
- **Case C (HA idle, optional):** 15–20 minute wait against `home-assistant-new` over HTTP. Skipped as non-required; stdio Case B/F already exercises the same `reconnect_fresh` path.
- **Case D (unmatched classifier, optional):** the attempted repro hit the grant-layer "did you mean" (`format_invoke_permission_denied()` in `routing.rs`) before ever reaching the backend `call_tool` classifier. A real Case D run needs a failure shape that passes the grant check and fails inside `call_tool` itself.

---

## Related documentation

- [`backend-connection-resilience.md`](./backend-connection-resilience.md) — shipped `call_tool` retry + bind FK guard; Decisions 5–7 are the matcher/heartbeat/ClientPool deferrals this doc collects
- [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md) — shipped config-save reconnect, stdio OAuth refuse, hold-then-pin; Decision 3 (no session persist) and Decision 5 (matcher/heartbeat/ClientPool out) restated above
- [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — the Agents-window empty-header open question this doc's item 1 points back to
- [`clone-auth-header-config-editing.md`](./clone-auth-header-config-editing.md) — clone seeding/definition-save/warning UI shipped on desktop; Phase 5 there (web admin clone parity, item 4 above) is now shipped too
