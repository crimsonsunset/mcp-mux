//! Admin Axum router — health, static SPA, API routes.

use axum::{middleware, routing::get, Router};
use mcpmux_core::ApplicationServices;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};
use tracing::warn;

use super::config::AdminConfig;
use super::handlers::health;
use super::middleware::{cf_access_middleware, CfAccessValidator};

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
}

/// Build the admin router with health, API stubs, and SPA static fallback.
pub fn build_admin_router(state: AdminState) -> Router {
    let mut router = Router::new().route("/api/v1/health", get(health));

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
