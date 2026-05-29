# Web Admin Parity Matrix

**Generated:** May 25, 2026 (from `apps/desktop/src` invoke scan + `lib.rs` handler registry)
**Re-scanned:** May 25, 2026 — Phase 5 desktop cleanup sign-off ([pre-web-admin-desktop-cleanup.md](./pre-web-admin-desktop-cleanup.md))
**Phase 8 closure:** May 26, 2026 — all rows resolved (`[x]` or `N/A`)
**Parent plan:** [`docs/guide/gateway.mdx`](../guide/gateway.mdx) (web admin mode)

Living tracker for IPC → HTTP parity. **E2E column:** `[x]` = Playwright admin catalog (`tests/e2e/specs/admin/`) or Rust OAuth integration; `N/A` = desktop-only, deferred, internal, or covered by dual-entry/transport only (no WDIO catalog path).

## Summary

| Metric | Count |
| ------ | ----- |
| Registered Tauri commands | 126 |
| Unique FE `invoke()` commands | 115 |
| Matrix rows (FE + deferred BE) | 125 |
| REST (direct parity) | 104 |
| REST (web variant) | 6 |
| Desktop-only (no HTTP) | 5 |
| Fix mismatch first | 0 |
| Deferred (BE only, no FE invoke) | 10 |

### Desktop cleanup audit (Phase 5)

| Check | Result |
| ----- | ------ |
| FE invokes ⊆ `lib.rs` handlers | ✅ 115/115 — zero mismatches |
| `invoke(` outside `lib/api/**` | ✅ none (except test harness in `main.tsx`) |
| Page/component `listen(` | ✅ hooks + `serverManager.ts` only |
| Dead channel names | ✅ removed (`grants-changed`, `workspace-appearance-changed`, `server-status`) |
| SSE channel registry | ✅ 16 channels documented |

## Regenerate

Re-scan when adding `invoke()` calls:

```bash
rg --no-filename -o "invoke(?:<[^>]*>)?\\(\\s*['\"]([a-z0-9_]+)['\"]" apps/desktop/src | sort -u | wc -l
```

## Known anomalies (fix before bridge)

- ~~**`export_config`** — FE calls `export_config`; Tauri registers `export_config_to_file`~~ — **Fixed** (Phase 1: `configExport.ts`)
- ~~**`list_registry_categories`** — FE invokes but no Tauri handler~~ — **Fixed** (Phase 1: removed from `registry.ts`)
- ~~**`grants-changed`** — hook listened; backend emits `client-grant-changed`~~ — **Fixed** (Phase 4: `useDomainEvents`)
- ~~**`workspace-appearance-changed` / `server-status`** — dead WorkspacesPage listeners~~ — **Fixed** (Phase 4: `useWorkspaceEvents` + `server-status-changed`)
- **Dual Rust emit paths** — EventBus bridge vs direct `app.emit` — **Documented** (Phase 4); SSE fan-in in web-admin Phase 5

## Pilot module (Phase 3)

Start with **`spaces`** — 9 commands, bounded CRUD, template for bridge extraction:

`list_spaces`, `get_space`, `create_space`, `update_space`, `delete_space`, `read_space_config`, `save_space_config`, `remove_server_from_config`, `open_space_config_file` (desktop-only)

## SSE event channels (Phase 5)

**16 channels** — fan in EventBus bridge (`gateway.rs`) **and** direct `app.emit` (`oauth.rs`, `session_overrides.rs`). See [`gateway.rs`](../../apps/desktop/src-tauri/src/commands/gateway.rs) module docs.

| Channel | Rust source | Desktop hook | SSE test | Playwright |
| ------- | ----------- | ------------ | -------- | ---------- |
| `space-changed` | EventBus bridge | `useDomainEvents` | [x] | N/A |
| `server-changed` | EventBus bridge | `useDomainEvents` | [x] | N/A |
| `server-status-changed` | EventBus bridge | `useDomainEvents` / `useServerManager` | [x] | N/A |
| `server-auth-progress` | EventBus bridge | `useDomainEvents` / `useServerManager` | [x] | N/A |
| `server-features-refreshed` | EventBus bridge | `useDomainEvents` / `useServerManager` | [x] | N/A |
| `feature-set-changed` | EventBus bridge | `useDomainEvents` | [x] | N/A |
| `client-changed` | EventBus bridge | `useDomainEvents` | [x] | N/A |
| `client-grant-changed` | EventBus bridge | `useDomainEvents` (`useClientEvents`) | [x] | N/A |
| `gateway-changed` | EventBus bridge | `useGatewayEvents` | [x] | [x] |
| `mcp-notification` | EventBus bridge | `useDomainEvents` | [x] | N/A |
| `session-roots-changed` | EventBus bridge | `useWorkspaceEvents` | [x] | N/A |
| `workspace-binding-changed` | EventBus bridge | `useWorkspaceEvents` | [x] | N/A |
| `workspace-needs-binding` | EventBus bridge | `useWorkspaceEvents` | [x] | N/A |
| `meta-tool-invoked` | EventBus bridge | `useMetaToolEvents` | [x] | N/A |
| `oauth-client-changed` | Direct emit (`oauth.rs`) | `useOAuthClientEvents` | [x] | N/A |
| `session-overrides-changed` | Direct emit (`session_overrides.rs`) | `useWorkspaceEvents` | [x] | N/A |

**Removed (never emitted):** `grants-changed` (use `client-grant-changed`), `workspace-appearance-changed` (reuse `workspace-binding-changed`), `server-status` (use `server-status-changed`).

## Commands

| Command | TS source | Rust module | HTTP | Planned route | Bridge fn | R/W | Web scope | Phase | Bridge | Dual | Transport | E2E |
| ------- | --------- | ----------- | ---- | ------------- | --------- | --- | --------- | ----- | ------ | ---- | --------- | --- |
| `add_feature_set_member` | `lib/api/featureSets.ts` | `feature_set` | POST | `POST /api/v1/feature-sets` → `add_feature_set_member` | `command_bridge::feature_set::add_feature_set_member` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `add_feature_to_set` | `— (deleted Phase 3; batch APIs only)` | `feature_members` | POST | `POST /api/v1/feature-sets/members` → `add_feature_to_set` | `command_bridge::feature_members::add_feature_to_set` | W | Deferred | — | — | — | — | — |
| `add_to_cursor` | `lib/api/clientInstall.ts` | `client_install` | — | `— /api/v1/client-install` → `add_to_cursor` | `command_bridge::client_install::add_to_cursor` | — | Desktop-only | — | N/A | N/A | N/A | N/A |
| `add_to_vscode` | `lib/api/clientInstall.ts` | `client_install` | — | `— /api/v1/client-install` → `add_to_vscode` | `command_bridge::client_install::add_to_vscode` | — | Desktop-only | — | N/A | N/A | N/A | N/A |
| `approve_oauth_consent` | `lib/api/oauth.ts` | `oauth` | POST | `POST /api/v1/oauth` → `approve_oauth_consent` | `command_bridge::oauth::approve_oauth_consent` | W | REST | P7 | [x] | [x] | [x] | [x] |
| `cancel_auth_v2` | `lib/api/serverManager.ts` | `server_manager` | POST | `POST /api/v1/servers/connections` → `cancel_auth_v2` | `command_bridge::server_manager::cancel_auth_v2` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `clear_server_logs` | `lib/api/logs.ts` | `logs` | DELETE | `DELETE /api/v1/logs` → `clear_server_logs` | `command_bridge::logs::clear_server_logs` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `clear_session_overrides` | `lib/api/sessionOverrides.ts` | `session_overrides` | DELETE | `DELETE /api/v1/session-overrides` → `clear_session_overrides` | `command_bridge::session_overrides::clear_session_overrides` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `clone_server` | `lib/api/serverClone.ts` | `server_clone` | POST | `POST /api/v1/servers/clones` → `clone_server` | `command_bridge::server_clone::clone_server` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `connect_all_enabled_servers` | `lib/api/gateway.ts` | `gateway` | POST | `POST /api/v1/gateway` → `connect_all_enabled_servers` | `command_bridge::gateway::connect_all_enabled_servers` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `connect_server` | `— (E2E/internal; FE wrapper removed Phase 1)` | `gateway` | POST | `POST /api/v1/gateway` → `connect_server` | `command_bridge::gateway::connect_server` | W | Deferred | — | — | — | — | — |
| `create_client` | `lib/api/clients.ts` | `client` | POST | `POST /api/v1/clients` → `create_client` | `command_bridge::client::create_client` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `create_feature_set` | `lib/api/featureSets.ts` | `feature_set` | POST | `POST /api/v1/feature-sets` → `create_feature_set` | `command_bridge::feature_set::create_feature_set` | W | REST | P6 | [x] | [x] | [x] | [x] |
| `create_space` | `lib/api/spaces.ts` | `space` | POST | `POST /api/v1/spaces` → `create_space` | `command_bridge::space::create_space` | W | REST | P6 | [x] | [x] | [x] | [x] |
| `create_workspace_binding` | `lib/api/workspaceBindings.ts` | `workspace_binding` | POST | `POST /api/v1/workspaces/bindings` → `create_workspace_binding` | `command_bridge::workspace_binding::create_workspace_binding` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `delete_client` | `lib/api/clients.ts` | `client` | DELETE | `DELETE /api/v1/clients` → `delete_client` | `command_bridge::client::delete_client` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `delete_feature_set` | `lib/api/featureSets.ts` | `feature_set` | DELETE | `DELETE /api/v1/feature-sets` → `delete_feature_set` | `command_bridge::feature_set::delete_feature_set` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `delete_oauth_client` | `lib/api/oauth.ts` | `oauth` | DELETE | `DELETE /api/v1/oauth` → `delete_oauth_client` | `command_bridge::oauth::delete_oauth_client` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `delete_space` | `lib/api/spaces.ts` | `space` | DELETE | `DELETE /api/v1/spaces` → `delete_space` | `command_bridge::space::delete_space` | W | REST | P6 | [x] | [x] | [x] | [x] |
| `delete_workspace_appearance` | `lib/api/workspaceAppearances.ts` | `workspace_appearance` | DELETE | `DELETE /api/v1/workspaces/appearances` → `delete_workspace_appearance` | `command_bridge::workspace_appearance::delete_workspace_appearance` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `delete_workspace_binding` | `lib/api/workspaceBindings.ts` | `workspace_binding` | DELETE | `DELETE /api/v1/workspaces/bindings` → `delete_workspace_binding` | `command_bridge::workspace_binding::delete_workspace_binding` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `disable_server_v2` | `lib/api/serverManager.ts` | `server_manager` | POST | `POST /api/v1/servers/connections` → `disable_server_v2` | `command_bridge::server_manager::disable_server_v2` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `disconnect_server` | `lib/api/gateway.ts` | `gateway` | POST | `POST /api/v1/gateway` → `disconnect_server` | `command_bridge::gateway::disconnect_server` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `disconnect_server_v2` | `— (BE only; pause-without-logout — use gateway.disconnectServer)` | `server_manager` | POST | `POST /api/v1/servers/connections` → `disconnect_server_v2` | `command_bridge::server_manager::disconnect_server_v2` | W | Deferred | — | — | — | — | — |
| `discover_servers` | `lib/api/registry.ts` | `server_discovery` | GET | `GET /api/v1/registry` → `discover_servers` | `command_bridge::server_discovery::discover_servers` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `enable_server_v2` | `lib/api/serverManager.ts` | `server_manager` | POST | `POST /api/v1/servers/connections` → `enable_server_v2` | `command_bridge::server_manager::enable_server_v2` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `export_config_to_file` | `lib/api/configExport.ts` | `config_export` | POST | `POST /api/v1/config-export` → `export_config_to_file` | `command_bridge::config_export::export_config_to_file` | W | Deferred | — | — | — | — | — |
| `flush_pending_deep_link` | `lib/api/oauth.ts` | `oauth` | — | `— /api/v1/oauth` → `flush_pending_deep_link` | `command_bridge::oauth::flush_pending_deep_link` | — | Desktop-only | — | N/A | N/A | N/A | N/A |
| `get_bundle_version` | `lib/api/app.ts` | `app` | GET | `GET /api/v1/app` → `get_bundle_version` | `command_bridge::app::get_bundle_version` | R | REST (web variant) | P4 | [x] | [x] | [x] | N/A |
| `get_client` | `lib/api/clients.ts` | `client` | GET | `GET /api/v1/clients` → `get_client` | `command_bridge::client::get_client` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_feature_set` | `lib/api/featureSets.ts` | `feature_set` | GET | `GET /api/v1/feature-sets` → `get_feature_set` | `command_bridge::feature_set::get_feature_set` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_feature_set_members` | `— (deleted Phase 3; batch APIs only)` | `feature_members` | GET | `GET /api/v1/feature-sets/members` → `get_feature_set_members` | `command_bridge::feature_members::get_feature_set_members` | R | Deferred | — | — | — | — | — |
| `get_feature_set_with_members` | `lib/api/featureSets.ts` | `feature_set` | GET | `GET /api/v1/feature-sets` → `get_feature_set_with_members` | `command_bridge::feature_set::get_feature_set_with_members` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_gateway_port_settings` | `lib/api/settings.ts` | `gateway` | GET | `GET /api/v1/gateway` → `get_gateway_port_settings` | `command_bridge::gateway::get_gateway_port_settings` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_gateway_status` | `lib/api/gateway.ts` | `gateway` | GET | `GET /api/v1/gateway` → `get_gateway_status` | `command_bridge::gateway::get_gateway_status` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_log_retention_days` | `lib/api/logs.ts` | `logs` | GET | `GET /api/v1/logs` → `get_log_retention_days` | `command_bridge::logs::get_log_retention_days` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_logs_path` | `lib/api/settings.ts` | `app` | GET | `GET /api/v1/app` → `get_logs_path` | `command_bridge::app::get_logs_path` | R | REST (web variant) | P4 | [x] | [x] | [x] | N/A |
| `get_meta_tools_enabled` | `lib/api/metaTools.ts` | `settings` | GET | `GET /api/v1/settings` → `get_meta_tools_enabled` | `command_bridge::settings::get_meta_tools_enabled` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_oauth_client_grants` | `lib/api/oauth.ts` | `oauth` | GET | `GET /api/v1/oauth` → `get_oauth_client_grants` | `command_bridge::oauth::get_oauth_client_grants` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_oauth_clients` | `lib/api/oauth.ts` | `oauth` | GET | `GET /api/v1/oauth` → `get_oauth_clients` | `command_bridge::oauth::get_oauth_clients` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_pending_consent` | `lib/api/oauth.ts` | `oauth` | GET | `GET /api/v1/oauth` → `get_pending_consent` | `command_bridge::oauth::get_pending_consent` | R | REST | P7 | [x] | [x] | [x] | [x] |
| `get_pool_stats` | `lib/api/gateway.ts` | `gateway` | GET | `GET /api/v1/gateway` → `get_pool_stats` | `command_bridge::gateway::get_pool_stats` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_registry_home_config` | `lib/api/registry.ts` | `server_discovery` | GET | `GET /api/v1/registry` → `get_registry_home_config` | `command_bridge::server_discovery::get_registry_home_config` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_registry_ui_config` | `lib/api/registry.ts` | `server_discovery` | GET | `GET /api/v1/registry` → `get_registry_ui_config` | `command_bridge::server_discovery::get_registry_ui_config` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_server_definition` | `lib/api/registry.ts` | `server_discovery` | GET | `GET /api/v1/registry` → `get_server_definition` | `command_bridge::server_discovery::get_server_definition` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_server_feature` | `lib/api/serverFeatures.ts` | `server_feature` | GET | `GET /api/v1/server-features` → `get_server_feature` | `command_bridge::server_feature::get_server_feature` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_server_log_file` | `lib/api/logs.ts` | `logs` | GET | `GET /api/v1/logs` → `get_server_log_file` | `command_bridge::logs::get_server_log_file` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_server_logs` | `lib/api/logs.ts` | `logs` | GET | `GET /api/v1/logs` → `get_server_logs` | `command_bridge::logs::get_server_logs` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_session_overrides_require_approval` | `lib/api/sessionOverrides.ts` | `settings` | GET | `GET /api/v1/settings` → `get_session_overrides_require_approval` | `command_bridge::settings::get_session_overrides_require_approval` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_space` | `lib/api/spaces.ts` | `space` | GET | `GET /api/v1/spaces` → `get_space` | `command_bridge::space::get_space` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_startup_settings` | `lib/api/settings.ts` | `settings` | GET | `GET /api/v1/settings` → `get_startup_settings` | `command_bridge::settings::get_startup_settings` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `get_version` | `lib/api/app.ts` | `app` | GET | `GET /api/v1/app` → `get_version` | `command_bridge::app::get_version` | R | REST (web variant) | P4 | [x] | [x] | [x] | N/A |
| `get_workspace_effective_features` | `lib/api/workspaceBindings.ts` | `workspace_binding` | GET | `GET /api/v1/workspaces/bindings` → `get_workspace_effective_features` | `command_bridge::workspace_binding::get_workspace_effective_features` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `grant_oauth_client_feature_set` | `lib/api/oauth.ts` | `oauth` | POST | `POST /api/v1/oauth` → `grant_oauth_client_feature_set` | `command_bridge::oauth::grant_oauth_client_feature_set` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `init_preset_clients` | `lib/api/clients.ts` | `client` | POST | `POST /api/v1/clients` → `init_preset_clients` | `command_bridge::client::init_preset_clients` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `install_server` | `lib/api/registry.ts` | `server` | POST | `POST /api/v1/servers` → `install_server` | `command_bridge::server::install_server` | W | REST | P6 | [x] | [x] | [x] | [x] |
| `is_clone_id_available` | `lib/api/serverClone.ts` | `server_clone` | GET | `GET /api/v1/servers/clones` → `is_clone_id_available` | `command_bridge::server_clone::is_clone_id_available` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `is_registry_offline` | `lib/api/registry.ts` | `server_discovery` | GET | `GET /api/v1/registry` → `is_registry_offline` | `command_bridge::server_discovery::is_registry_offline` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_clients` | `lib/api/clients.ts` | `client` | GET | `GET /api/v1/clients` → `list_clients` | `command_bridge::client::list_clients` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_clone_dependents` | `lib/api/serverClone.ts` | `server_clone` | GET | `GET /api/v1/servers/clones` → `list_clone_dependents` | `command_bridge::server_clone::list_clone_dependents` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_connected_servers` | `lib/api/gateway.ts` | `gateway` | GET | `GET /api/v1/gateway` → `list_connected_servers` | `command_bridge::gateway::list_connected_servers` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_feature_sets` | `lib/api/featureSets.ts` | `feature_set` | GET | `GET /api/v1/feature-sets` → `list_feature_sets` | `command_bridge::feature_set::list_feature_sets` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_feature_sets_by_space` | `lib/api/featureSets.ts` | `feature_set` | GET | `GET /api/v1/feature-sets` → `list_feature_sets_by_space` | `command_bridge::feature_set::list_feature_sets_by_space` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_installed_servers` | `lib/api/registry.ts` | `server` | GET | `GET /api/v1/servers` → `list_installed_servers` | `command_bridge::server::list_installed_servers` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_meta_tool_grants` | `lib/api/metaTools.ts` | `meta_tool_approval` | GET | `GET /api/v1/meta-tools` → `list_meta_tool_grants` | `command_bridge::meta_tool_approval::list_meta_tool_grants` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_registry_categories` | `— (removed Phase 1)` | `server_discovery` | — | — | — | — | Removed | — | N/A | N/A | N/A | N/A |
| `list_reported_workspace_roots` | `lib/api/workspaceBindings.ts` | `workspace_binding` | GET | `GET /api/v1/workspaces/bindings` → `list_reported_workspace_roots` | `command_bridge::workspace_binding::list_reported_workspace_roots` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_server_features` | `lib/api/serverFeatures.ts` | `server_feature` | GET | `GET /api/v1/server-features` → `list_server_features` | `command_bridge::server_feature::list_server_features` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_server_features_by_server` | `lib/api/serverFeatures.ts` | `server_feature` | GET | `GET /api/v1/server-features` → `list_server_features_by_server` | `command_bridge::server_feature::list_server_features_by_server` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_server_features_by_type` | `lib/api/serverFeatures.ts` | `server_feature` | GET | `GET /api/v1/server-features` → `list_server_features_by_type` | `command_bridge::server_feature::list_server_features_by_type` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_session_overrides` | `lib/api/sessionOverrides.ts` | `session_overrides` | GET | `GET /api/v1/session-overrides` → `list_session_overrides` | `command_bridge::session_overrides::list_session_overrides` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_spaces` | `lib/api/spaces.ts` | `space` | GET | `GET /api/v1/spaces` → `list_spaces` | `command_bridge::space::list_spaces` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_workspace_appearances` | `lib/api/workspaceAppearances.ts` | `workspace_appearance` | GET | `GET /api/v1/workspaces/appearances` → `list_workspace_appearances` | `command_bridge::workspace_appearance::list_workspace_appearances` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_workspace_bindings` | `lib/api/workspaceBindings.ts` | `workspace_binding` | GET | `GET /api/v1/workspaces/bindings` → `list_workspace_bindings` | `command_bridge::workspace_binding::list_workspace_bindings` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `list_workspace_bindings_for_space` | `lib/api/workspaceBindings.ts` | `workspace_binding` | GET | `GET /api/v1/workspaces/bindings` → `list_workspace_bindings_for_space` | `command_bridge::workspace_binding::list_workspace_bindings_for_space` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `logout_server` | `lib/api/serverManager.ts` | `server_manager` | POST | `POST /api/v1/servers/connections` → `logout_server` | `command_bridge::server_manager::logout_server` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `open_logs_folder` | `lib/api/settings.ts` | `app` | — | `— /api/v1/app` → `open_logs_folder` | `command_bridge::app::open_logs_folder` | — | Desktop-only | — | N/A | N/A | N/A | N/A |
| `open_space_config_file` | `lib/api/spaces.ts` | `space` | — | `— /api/v1/spaces` → `open_space_config_file` | `command_bridge::space::open_space_config_file` | — | Desktop-only | — | N/A | N/A | N/A | N/A |
| `open_url` | `lib/api/gateway.ts` | `oauth` | GET | `GET /api/v1/oauth` → `open_url` | `command_bridge::oauth::open_url` | R | REST (web variant) | P4 | [x] | [x] | [x] | N/A |
| `probe_gateway_start` | `lib/api/gateway.ts` | `gateway` | GET | `GET /api/v1/gateway` → `probe_gateway_start` | `command_bridge::gateway::probe_gateway_start` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `read_space_config` | `lib/api/spaces.ts` | `space` | GET | `GET /api/v1/spaces` → `read_space_config` | `command_bridge::space::read_space_config` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `refresh_oauth_tokens_on_startup` | `lib/api/gateway.ts` | `gateway` | POST | `POST /api/v1/gateway` → `refresh_oauth_tokens_on_startup` | `command_bridge::gateway::refresh_oauth_tokens_on_startup` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `refresh_registry` | `lib/api/registry.ts` | `server_discovery` | POST | `POST /api/v1/registry` → `refresh_registry` | `command_bridge::server_discovery::refresh_registry` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `remove_feature_from_set` | `— (deleted Phase 3; batch APIs only)` | `feature_members` | DELETE | `DELETE /api/v1/feature-sets/members` → `remove_feature_from_set` | `command_bridge::feature_members::remove_feature_from_set` | W | Deferred | — | — | — | — | — |
| `remove_feature_set_member` | `lib/api/featureSets.ts` | `feature_set` | DELETE | `DELETE /api/v1/feature-sets` → `remove_feature_set_member` | `command_bridge::feature_set::remove_feature_set_member` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `remove_server_from_config` | `lib/api/spaces.ts` | `space` | DELETE | `DELETE /api/v1/spaces` → `remove_server_from_config` | `command_bridge::space::remove_server_from_config` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `reset_gateway_port` | `lib/api/settings.ts` | `gateway` | GET | `GET /api/v1/gateway` → `reset_gateway_port` | `command_bridge::gateway::reset_gateway_port` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `resolve_workspace_icon_path` | `lib/api/workspaceAppearances.ts` | `workspace_appearance` | GET | `GET /api/v1/workspaces/appearances` → `resolve_workspace_icon_path` | `command_bridge::workspace_appearance::resolve_workspace_icon_path` | R | REST (web variant) | P4 | [x] | [x] | [x] | N/A |
| `respond_to_meta_tool_approval` | `lib/api/metaTools.ts` | `meta_tool_approval` | POST | `POST /api/v1/meta-tools` → `respond_to_meta_tool_approval` | `command_bridge::meta_tool_approval::respond_to_meta_tool_approval` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `restart_gateway` | `lib/api/gateway.ts` | `gateway` | POST | `POST /api/v1/gateway` → `restart_gateway` | `command_bridge::gateway::restart_gateway` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `retry_connection` | `lib/api/serverManager.ts` | `server_manager` | POST | `POST /api/v1/servers/connections` → `retry_connection` | `command_bridge::server_manager::retry_connection` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `revoke_meta_tool_grant` | `lib/api/metaTools.ts` | `meta_tool_approval` | DELETE | `DELETE /api/v1/meta-tools` → `revoke_meta_tool_grant` | `command_bridge::meta_tool_approval::revoke_meta_tool_grant` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `revoke_oauth_client_feature_set` | `lib/api/oauth.ts` | `oauth` | DELETE | `DELETE /api/v1/oauth` → `revoke_oauth_client_feature_set` | `command_bridge::oauth::revoke_oauth_client_feature_set` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `save_server_inputs` | `lib/api/registry.ts` | `server` | PUT | `PUT /api/v1/servers` → `save_server_inputs` | `command_bridge::server::save_server_inputs` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `save_space_config` | `lib/api/spaces.ts` | `space` | PUT | `PUT /api/v1/spaces` → `save_space_config` | `command_bridge::space::save_space_config` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `set_feature_set_members` | `lib/api/featureSets.ts` | `feature_set` | PUT | `PUT /api/v1/feature-sets` → `set_feature_set_members` | `command_bridge::feature_set::set_feature_set_members` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `set_gateway_port` | `lib/api/settings.ts` | `gateway` | PUT | `PUT /api/v1/gateway` → `set_gateway_port` | `command_bridge::gateway::set_gateway_port` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `set_log_retention_days` | `lib/api/logs.ts` | `logs` | PUT | `PUT /api/v1/logs` → `set_log_retention_days` | `command_bridge::logs::set_log_retention_days` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `set_meta_tools_enabled` | `lib/api/metaTools.ts` | `settings` | PUT | `PUT /api/v1/settings` → `set_meta_tools_enabled` | `command_bridge::settings::set_meta_tools_enabled` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `set_server_display_name` | `lib/api/registry.ts` | `server` | PUT | `PUT /api/v1/servers` → `set_server_display_name` | `command_bridge::server::set_server_display_name` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `set_server_enabled` | `— (dead export in registry.ts; UI uses enable_server_v2)` | `server` | PUT | `PUT /api/v1/servers` → `set_server_enabled` | `command_bridge::server::set_server_enabled` | W | Deferred | — | — | — | — | — |
| `set_server_oauth_connected` | `lib/api/registry.ts` | `server` | PUT | `PUT /api/v1/servers` → `set_server_oauth_connected` | `command_bridge::server::set_server_oauth_connected` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `set_session_overrides_require_approval` | `lib/api/sessionOverrides.ts` | `settings` | PUT | `PUT /api/v1/settings` → `set_session_overrides_require_approval` | `command_bridge::settings::set_session_overrides_require_approval` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `start_auth_v2` | `lib/api/serverManager.ts` | `server_manager` | POST | `POST /api/v1/servers/connections` → `start_auth_v2` | `command_bridge::server_manager::start_auth_v2` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `start_gateway` | `lib/api/gateway.ts` | `gateway` | POST | `POST /api/v1/gateway` → `start_gateway` | `command_bridge::gateway::start_gateway` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `stop_gateway` | `lib/api/gateway.ts` | `gateway` | POST | `POST /api/v1/gateway` → `stop_gateway` | `command_bridge::gateway::stop_gateway` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `suggest_clone_suffix` | `lib/api/serverClone.ts` | `server_clone` | GET | `GET /api/v1/servers/clones` → `suggest_clone_suffix` | `command_bridge::server_clone::suggest_clone_suffix` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `take_pending_port_conflict` | `lib/api/gateway.ts` | `gateway` | GET | `GET /api/v1/gateway` → `take_pending_port_conflict` | `command_bridge::gateway::take_pending_port_conflict` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `uninstall_server` | `lib/api/registry.ts` | `server` | DELETE | `DELETE /api/v1/servers` → `uninstall_server` | `command_bridge::server::uninstall_server` | W | REST | P6 | [x] | [x] | [x] | [x] |
| `update_feature_set` | `lib/api/featureSets.ts` | `feature_set` | PUT | `PUT /api/v1/feature-sets` → `update_feature_set` | `command_bridge::feature_set::update_feature_set` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `update_oauth_client` | `lib/api/oauth.ts` | `oauth` | PUT | `PUT /api/v1/oauth` → `update_oauth_client` | `command_bridge::oauth::update_oauth_client` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `update_space` | `lib/api/spaces.ts` | `space` | PUT | `PUT /api/v1/spaces` → `update_space` | `command_bridge::space::update_space` | W | REST | P6 | [x] | [x] | [x] | [x] |
| `update_startup_settings` | `lib/api/settings.ts` | `settings` | PUT | `PUT /api/v1/settings` → `update_startup_settings` | `command_bridge::settings::update_startup_settings` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `update_workspace_binding` | `lib/api/workspaceBindings.ts` | `workspace_binding` | PUT | `PUT /api/v1/workspaces/bindings` → `update_workspace_binding` | `command_bridge::workspace_binding::update_workspace_binding` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `upload_workspace_icon` | `lib/api/workspaceAppearances.ts` | `workspace_appearance` | POST | `POST /api/v1/workspaces/appearances` → `upload_workspace_icon` | `command_bridge::workspace_appearance::upload_workspace_icon` | W | REST (web variant) | P6 | [x] | [x] | [x] | N/A |
| `upsert_workspace_appearance` | `lib/api/workspaceAppearances.ts` | `workspace_appearance` | PUT | `PUT /api/v1/workspaces/appearances` → `upsert_workspace_appearance` | `command_bridge::workspace_appearance::upsert_workspace_appearance` | W | REST | P6 | [x] | [x] | [x] | N/A |
| `validate_workspace_root` | `lib/api/workspaceBindings.ts` | `workspace_binding` | GET | `GET /api/v1/workspaces/bindings` → `validate_workspace_root` | `command_bridge::workspace_binding::validate_workspace_root` | R | REST | P4 | [x] | [x] | [x] | N/A |
| `backup_existing_config` | `lib/api/configExport.ts` | `config_export` | POST | `POST /api/v1/config-export` → `backup_existing_config` | `command_bridge::config_export::backup_existing_config` | W | Deferred (no UI) | — | — | — | — | — |
| `check_config_exists` | `lib/api/configExport.ts` | `config_export` | POST | `POST /api/v1/config-export` → `check_config_exists` | `command_bridge::config_export::check_config_exists` | W | Deferred (no UI) | — | — | — | — | — |
| `generate_gateway_config` | `— (BE only; no FE consumer)` | `gateway` | POST | `POST /api/v1/gateway` → `generate_gateway_config` | `command_bridge::gateway::generate_gateway_config` | W | Deferred | — | — | — | — | — |
| `get_config_paths` | `lib/api/configExport.ts` | `config_export` | GET | `GET /api/v1/config-export` → `get_config_paths` | `command_bridge::config_export::get_config_paths` | R | Deferred (no UI) | — | — | — | — | — |
| `get_server_statuses` | `— (BE only; events drive UI status)` | `server_manager` | GET | `GET /api/v1/servers/connections` → `get_server_statuses` | `command_bridge::server_manager::get_server_statuses` | R | Deferred | — | — | — | — | — |
| `preview_config_export` | `lib/api/configExport.ts` | `config_export` | GET | `GET /api/v1/config-export` → `preview_config_export` | `command_bridge::config_export::preview_config_export` | R | Deferred (no UI) | — | — | — | — | — |
| `search_servers` | `— (BE only; registry uses client-side filter)` | `server_discovery` | GET | `GET /api/v1/registry` → `search_servers` | `command_bridge::server_discovery::search_servers` | R | Deferred | — | — | — | — | — |
| `seed_server_features` | `— (BE only; test/setup)` | `server_feature` | GET | `GET /api/v1/server-features` → `seed_server_features` | `command_bridge::server_feature::seed_server_features` | R | Deferred | — | — | — | — | — |
| `approve_oauth_client` | `— (E2E test mode only)` | `oauth` | POST | `POST /api/v1/oauth` → `approve_oauth_client` | `command_bridge::oauth::approve_oauth_client` | W | Desktop/E2E-only | — | N/A | N/A | N/A | N/A |
| `refresh_tray_menu` | `— (internal; tray subsystem)` | `space` | — | — | — | — | Internal | — | N/A | N/A | N/A | N/A |
