# Resilience & Routing Leftovers

**Last Updated:** Aug 20, 2026
**Status:** Planning — inventory doc. Nothing here is implemented yet except where noted "shipped."
**Branch:** `root-resolution`
**Depends on:** [`backend-connection-resilience.md`](./backend-connection-resilience.md) Phases 1–3 (shipped), [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md) Phases 1–4 (shipped, `dcc2977`)
**Unblocks:** A single place to look for "what's still parked or open" instead of hunting through three shipped docs' Out/Deferred tables

---

## Why this doc exists

`backend-connection-resilience.md` and `pool-invalidation-and-session-survival.md` both shipped and both carry an Out/Deferred table. Those tables are correct but scattered, and `clone-auth-header-config-editing.md` had drifted — its plan still described Phases 1/2/4 as open when the code had already landed them (desktop side). This doc collects everything still parked or still open in one place, and does not re-litigate anything already shipped. Item 4 (web admin clone parity) shipped in `f024d9e`. Item 3 (`mcpmux-mcp` / `ClientPool`) has since been deleted. Both kept below for history.

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

### 1. Empty `${workspaceFolder}` — **partially shipped (`efabe48`)**

The spawn-path repro this entry asked for is done. An `env-probe` wrapper captured 282 real `mcp-remote` spawns and settled the open questions:

- **Not an Agents-window problem.** Editor windows fail to substitute `${workspaceFolder}` 29% of the time, Agents windows 4%, across folder counts from zero to five. Overall failure is ~21%. The old attribution in this doc and in the `oauth_middleware` warn was wrong; both are corrected.
- **The literal reaches `mcp-remote`, which expands it to empty.** Cursor passes `${workspaceFolder}` through unsubstituted; `mcp-remote`'s own `${ENV}` pass finds no such variable and rewrites it to an empty string. That is why the gateway sees present-but-empty rather than a literal.
- **No fallback signal for the active folder exists.** All 22 Cursor/VS Code child env vars were checked. `CURSOR_WORKSPACE_LABEL` is stale (names the window that started the extension host, frequently a folder absent from the set). `VSCODE_PID` (5 distinct across 282 spawns) and `VSCODE_IPC_HOOK` (2 distinct, and they're Cursor version strings) are app-level, not per-window. `cwd` is always `$HOME`. No `.code-workspace` file backs ad-hoc multi-root windows, which also kills any "bind the workspace file" idea.
- **`WORKSPACE_FOLDER_PATHS` is sound but imprecise.** The active folder was a member in 212 of 212 resolved multi-folder spawns. Its _position_ identified the active folder in only 70%. Ordering heuristics are therefore unusable: a 30% misroute rate crosses credential boundaries, which is the failure the product exists to prevent.

**Shipped:** the bridge sends the set as `X-Mcpmux-Workspace-Set`, used strictly as a constraint. One-member sets pin (unambiguous); zero-member sets are a clean no-workspace state; multi-member sets only record candidates. `mcpmux_set_workspace_root` now refuses a root the calling window doesn't have open, closing a self-service grant where any approved client could name any path. Three warns report assumption failures (unexpanded template, pin-absent-from-set, ambiguous set) — see [`cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) section 3.

**Still open:** ~16% of spawns (multi-folder window plus failed substitution) remain genuinely unresolvable and cost one `mcpmux_set_workspace_root` call per session. That is inherent, not a gap in the implementation — no signal exists to close it. Two things could: Cursor making substitution reliable, or exposing the active folder as an env var. Until then the per-repo static header install (`workspace_install.rs`) is the only way to be immune, and it should probably become the recommended path for multi-root users rather than a documented fallback.

Do not attempt server-side PID/process-tree inference — a dead end (TCP transport, no child PID visible to the gateway), and now doubly so given `VSCODE_PID` isn't per-window anyway.

**Known gap in the immune path:** `workspace_install.rs` writes the bearer token into a repo-local config and nothing adds a `.gitignore` entry, so `git add .` in a fresh repo commits a gateway access key. Worth fixing before recommending it more loudly. There is also no web-admin parity for the per-repo installer (desktop-only, three Tauri commands).

### 2. `is_healthy()` heartbeat / liveness probe

[`ServerInstance::is_healthy()`](../../crates/mcpmux-gateway/src/pool/instance.rs#L397) is `state == Connected && client.is_some()` — a state flag, not a transport check. Three call sites trust it: `PoolService::connect_server()`'s healthy-reuse (`service.rs`), `PoolService::is_connected()`, and `ConnectionService::connect_with_instance()`'s reconnect-skip. The existing 60s loop in [`server_manager.rs`](../../crates/mcpmux-gateway/src/pool/server_manager.rs#L1476) (`start_periodic_refresh`, `REFRESH_INTERVAL`) is a **different state machine** — it walks `ServerManager`'s UI-facing `ConnectionStatus` map, and `refresh_single_server()` (`server_manager.rs#L1297`) is trace-log-only today, touching neither `PoolService` nor any `ServerInstance`. It is also desktop-only (wired from `apps/desktop/src-tauri/src/commands/gateway.rs`), not started in headless/web-admin. A real liveness probe would need to run against `PoolService` instances directly, not piggyback on the UI loop as-is. Keep Decision 10 from `backend-connection-resilience.md`: a transient call failure should stay `record_failure()` (stats-only), not `mark_failed()` — a heartbeat changes detection, not that split.

### 3. Delete unused `ClientPool` — **deleted**

The whole `mcpmux-mcp` crate was orphaned (`ClientPool`, transports, and the crate itself had no importers). Removed from the workspace. Gateway pooling stays on `PoolService` + rmcp.

### 4. Web admin clone parity — **shipped (`f024d9e`)**

`clone_server`, `is_clone_id_available`, `suggest_clone_suffix`, `list_clone_dependents`, and `set_server_display_name` are un-stubbed in [`command_bridge/write.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/write.rs) / [`command_bridge/read.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/read.rs), delegating to the same `ServerAppService` methods desktop Tauri's [`server_clone.rs`](../../apps/desktop/src-tauri/src/commands/server_clone.rs) already used — the web admin frontend (`CloneAccountModal.tsx`, Configure modal) already had the UI wired, so no frontend work was needed. Bonus fix landed in the same commit: `save_server_inputs` was silently dropping `display_name_override` on both runtimes and `update_policy`/`pinned_version` on web admin specifically — now forwards all three. Full spec was Phase 5 in [`clone-auth-header-config-editing.md`](./clone-auth-header-config-editing.md); not tracked in `dev-to-main-port.md`'s Phase 6, which is unrelated desktop cloning UI.

### 5. Leftover verification (playbook cases, not code)

- **Case G (config-save reconnect):** handler code already shipped in `dcc2977` (`ServerConfigUpdatedHandler` → `reconnect_fresh after config update`). The manual run was BLOCKED on admin `:45819` returning 401 (no CF Access probe headers in the test shell). Retrying through the desktop Configure UI save path (which also emits `ServerConfigUpdated`) should unblock this without needing admin credentials at all — do not sqlite-edit the config directly, that produces no event.
- **Case C (HA idle, optional):** 15–20 minute wait against `home-assistant-new` over HTTP. Skipped as non-required; stdio Case B/F already exercises the same `reconnect_fresh` path.
- **Case D (unmatched classifier, optional):** the attempted repro hit the grant-layer "did you mean" (`format_invoke_permission_denied()` in `routing.rs`) before ever reaching the backend `call_tool` classifier. A real Case D run needs a failure shape that passes the grant check and fails inside `call_tool` itself.

---

## Related documentation

- [`backend-connection-resilience.md`](./backend-connection-resilience.md) — shipped `call_tool` retry + bind FK guard; Decision 5 matcher freeze and Decision 6 heartbeat stay deferred. Decision 7 (`ClientPool`) is now deleted (item 3)
- [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md) — shipped config-save reconnect, stdio OAuth refuse, hold-then-pin; Decision 3 (no session persist) and Decision 5 matcher/heartbeat stay out. `ClientPool` deletion is done
- [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — the Agents-window empty-header open question this doc's item 1 points back to
- [`clone-auth-header-config-editing.md`](./clone-auth-header-config-editing.md) — clone seeding/definition-save/warning UI shipped on desktop; Phase 5 there (web admin clone parity, item 4 above) is now shipped too
