//! Web admin HTTP server startup (loopback :45819 by default).

use crate::state::AppState;
use mcpmux_core::{AppSettingsService, ApplicationServices, ApplicationServicesBuilder, EventBus};
use mcpmux_gateway::{AdminConfig, AdminServer, AdminServerHandle};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tracing::{info, warn};

/// Tracks the running admin server and shared gateway liveness flag.
#[derive(Default)]
pub struct AdminServerState {
    /// Background task handle for graceful shutdown.
    pub handle: Option<AdminServerHandle>,
    /// Updated when the MCP gateway starts or stops (admin `/api/v1/health`).
    pub gateway_running: Arc<AtomicBool>,
}

/// Resolve the built frontend directory for static SPA serving.
pub fn resolve_frontend_dist(app: &AppHandle) -> PathBuf {
    if let Ok(resource) = app.path().resource_dir() {
        let dist = resource.join("dist");
        if dist.join("index.html").is_file() {
            return dist;
        }
    }

    let dev_dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist");
    if dev_dist.join("index.html").is_file() {
        return dev_dist;
    }

    dev_dist
}

/// Build `ApplicationServices` for admin handlers from desktop `AppState`.
fn build_application_services(
    app_state: &AppState,
    event_bus: Arc<EventBus>,
) -> anyhow::Result<Arc<ApplicationServices>> {
    Ok(Arc::new(
        ApplicationServicesBuilder::new()
            .with_event_bus(event_bus)
            .with_space_repo(app_state.space_service.repository())
            .with_installed_server_repo(app_state.installed_server_repository.clone())
            .with_feature_set_repo(app_state.feature_set_repository.clone())
            .with_server_feature_repo(app_state.server_feature_repository_core.clone())
            .with_client_repo(app_state.client_repository.clone())
            .with_credential_repo(app_state.credential_repository.clone())
            .build()?,
    ))
}

/// Start the admin server when `gateway.admin_enabled` is true.
pub async fn start_admin_server_if_enabled(
    app: AppHandle,
    admin_state: Arc<tokio::sync::RwLock<AdminServerState>>,
    event_bus: Arc<EventBus>,
) {
    let app_state: tauri::State<'_, AppState> = app.state();
    let settings = AppSettingsService::new(app_state.settings_repository.clone());
    if !settings.get_admin_enabled().await {
        info!("[Admin] Web admin disabled (gateway.admin_enabled=false)");
        return;
    }

    let port = settings.get_admin_port().await;
    let trust_cf_access = settings.get_admin_trust_cf_access().await;
    let cf_team_domain = settings.get_admin_cf_team_domain().await;

    let config = AdminConfig {
        host: "127.0.0.1".to_string(),
        port,
        trust_cf_access,
        cf_team_domain,
        cf_access_audience: None,
        cf_validator_override: None,
    };

    let gateway_running = {
        let guard = admin_state.read().await;
        guard.gateway_running.clone()
    };

    let services = match build_application_services(&app_state, event_bus) {
        Ok(s) => s,
        Err(e) => {
            warn!("[Admin] Failed to build ApplicationServices: {}", e);
            return;
        }
    };

    let cf_validator = match AdminServer::build_cf_validator(&config).await {
        Ok(v) => v,
        Err(e) => {
            warn!("[Admin] CF Access validator init failed: {}", e);
            return;
        }
    };

    let frontend_dist = resolve_frontend_dist(&app);
    let server = match AdminServer::new(
        config.clone(),
        services,
        gateway_running,
        frontend_dist,
        cf_validator,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!("[Admin] Failed to build admin server: {}", e);
            return;
        }
    };

    let handle = server.spawn();
    info!(
        "[Admin] Started on {}:{} (cf_access={})",
        config.host, config.port, config.trust_cf_access
    );

    let mut guard = admin_state.write().await;
    guard.handle = Some(handle);
}

/// Sync gateway liveness into the admin health endpoint.
pub fn set_gateway_running(admin_state: &AdminServerState, running: bool) {
    admin_state
        .gateway_running
        .store(running, Ordering::Relaxed);
}
