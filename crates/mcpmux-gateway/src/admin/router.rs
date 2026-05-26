//! Admin Axum router — route mount point for Phase 2+ handlers.

use axum::Router;
use mcpmux_core::ApplicationServices;
use std::sync::Arc;

use super::AdminConfig;

/// Shared state for admin HTTP handlers.
#[derive(Clone)]
pub struct AdminState {
    /// Application services (same instance as Tauri commands).
    pub services: Arc<ApplicationServices>,
    /// Admin server configuration.
    pub config: AdminConfig,
}

/// Build the admin router. Phase 2 mounts health, static SPA, and API routes here.
pub fn build_admin_router(services: Arc<ApplicationServices>, config: AdminConfig) -> Router {
    let state = AdminState { services, config };

    Router::new().with_state(state)
}
