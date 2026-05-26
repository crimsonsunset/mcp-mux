//! Admin Axum router — health, static SPA, API routes.

use axum::{middleware, routing::get, Router};
use mcpmux_core::ApplicationServices;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};
use tracing::warn;

use super::config::AdminConfig;
use super::handlers::{health, read};
use super::middleware::{cf_access_middleware, CfAccessValidator};
use crate::admin::bridge_context::AdminBridgeCtx;

/// Shared state for admin HTTP handlers.
#[derive(Clone)]
pub struct AdminState {
    /// Application services (same instance as Tauri commands).
    pub services: Arc<ApplicationServices>,
    /// Admin server configuration.
    pub config: AdminConfig,
    /// MCP gateway running flag (updated by desktop when gateway starts/stops).
    pub gateway_running: Arc<AtomicBool>,
    /// Directory containing built frontend assets (`index.html`, etc.).
    pub frontend_dist: PathBuf,
    /// CF Access JWT validator when `trust_cf_access` is enabled.
    pub cf_validator: Option<Arc<CfAccessValidator>>,
    /// Shared read bridge context used by REST handlers.
    pub bridge: Arc<AdminBridgeCtx>,
}

/// Build the admin router with health, API stubs, and SPA static fallback.
pub fn build_admin_router(state: AdminState) -> Router {
    let mut router = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/gateway/status", get(read::get_gateway_status))
        .route("/api/v1/gateway/probe-start", get(read::probe_gateway_start))
        .route(
            "/api/v1/gateway/pending-port-conflict",
            get(read::take_pending_port_conflict),
        )
        .route(
            "/api/v1/gateway/port-settings",
            get(read::get_gateway_port_settings),
        )
        .route("/api/v1/gateway/reset-port", get(read::reset_gateway_port))
        .route(
            "/api/v1/gateway/connected-servers",
            get(read::list_connected_servers),
        )
        .route("/api/v1/gateway/pool-stats", get(read::get_pool_stats))
        .route("/api/v1/spaces", get(read::list_spaces))
        .route("/api/v1/spaces/{id}", get(read::get_space))
        .route("/api/v1/spaces/{id}/config", get(read::read_space_config))
        .route("/api/v1/servers/installed", get(read::list_installed_servers))
        .route("/api/v1/registry/discover", get(read::discover_servers))
        .route(
            "/api/v1/registry/definition/{server_id}",
            get(read::get_server_definition),
        )
        .route("/api/v1/registry/ui-config", get(read::get_registry_ui_config))
        .route(
            "/api/v1/registry/home-config",
            get(read::get_registry_home_config),
        )
        .route("/api/v1/registry/offline", get(read::is_registry_offline))
        .route("/api/v1/clients", get(read::list_clients))
        .route("/api/v1/clients/{id}", get(read::get_client))
        .route("/api/v1/feature-sets", get(read::list_feature_sets))
        .route(
            "/api/v1/feature-sets/by-space/{space_id}",
            get(read::list_feature_sets_by_space),
        )
        .route("/api/v1/feature-sets/{id}", get(read::get_feature_set))
        .route(
            "/api/v1/feature-sets/{id}/with-members",
            get(read::get_feature_set_with_members),
        )
        .route("/api/v1/workspaces/bindings", get(read::list_workspace_bindings))
        .route(
            "/api/v1/workspaces/bindings/space/{space_id}",
            get(read::list_workspace_bindings_for_space),
        )
        .route(
            "/api/v1/workspaces/reported-roots",
            get(read::list_reported_workspace_roots),
        )
        .route(
            "/api/v1/workspaces/validate-root",
            get(read::validate_workspace_root),
        )
        .route(
            "/api/v1/workspaces/effective-features",
            get(read::get_workspace_effective_features),
        )
        .route(
            "/api/v1/workspaces/appearances",
            get(read::list_workspace_appearances),
        )
        .route(
            "/api/v1/workspaces/icon-path",
            get(read::resolve_workspace_icon_path),
        )
        .route("/api/v1/session-overrides", get(read::list_session_overrides))
        .route("/api/v1/settings/startup", get(read::get_startup_settings))
        .route(
            "/api/v1/settings/meta-tools-enabled",
            get(read::get_meta_tools_enabled),
        )
        .route(
            "/api/v1/settings/session-overrides-require-approval",
            get(read::get_session_overrides_require_approval),
        )
        .route("/api/v1/app/version", get(read::get_version))
        .route("/api/v1/app/bundle-version", get(read::get_bundle_version))
        .route("/api/v1/app/logs-path", get(read::get_logs_path))
        .route("/api/v1/logs/server/{server_id}", get(read::get_server_logs))
        .route(
            "/api/v1/logs/server/{server_id}/file",
            get(read::get_server_log_file),
        )
        .route("/api/v1/logs/retention-days", get(read::get_log_retention_days))
        .route("/api/v1/oauth/clients", get(read::get_oauth_clients))
        .route(
            "/api/v1/oauth/clients/{client_id}/grants/{space_id}",
            get(read::get_oauth_client_grants),
        )
        .route("/api/v1/oauth/open-url", get(read::open_url))
        .route("/api/v1/meta-tools/grants", get(read::list_meta_tool_grants))
        .route("/api/v1/server-features", get(read::list_server_features))
        .route(
            "/api/v1/server-features/by-server",
            get(read::list_server_features_by_server),
        )
        .route(
            "/api/v1/server-features/by-type",
            get(read::list_server_features_by_type),
        )
        .route("/api/v1/server-features/{id}", get(read::get_server_feature))
        .route(
            "/api/v1/servers/clones/available",
            get(read::is_clone_id_available),
        )
        .route(
            "/api/v1/servers/clones/suggest",
            get(read::suggest_clone_suffix),
        )
        .route(
            "/api/v1/servers/clones/dependents",
            get(read::list_clone_dependents),
        );

    if state.frontend_dist.join("index.html").is_file() {
        let index = state.frontend_dist.join("index.html");
        let static_files =
            ServeDir::new(&state.frontend_dist).not_found_service(ServeFile::new(index));
        router = router.fallback_service(static_files);
    } else {
        warn!(
            "[Admin] frontend dist missing index.html at {:?} — static UI disabled",
            state.frontend_dist
        );
    }

    router
        .layer(middleware::from_fn_with_state(
            state.clone(),
            cf_access_middleware,
        ))
        .with_state(state)
}
