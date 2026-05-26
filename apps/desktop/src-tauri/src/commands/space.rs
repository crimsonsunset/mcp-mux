//! Space management commands
//!
//! IPC commands for managing spaces (isolated environments). There's no
//! "active space" — gateway routing is decided per reported workspace
//! root via `WorkspaceBinding`, with the `is_default` Space as the
//! built-in fallback. The desktop UI tracks which space the user is
//! viewing in its own Zustand store (frontend-only state).
//!
//! Business logic lives in `mcpmux_gateway::admin::command_bridge::space`;
//! these handlers are thin wrappers adding Tauri/desktop side effects.

use std::sync::Arc;

use mcpmux_core::{ApplicationServices, Space};
use mcpmux_gateway::admin::command_bridge::space::{self, SpaceBridgeCtx};

pub use mcpmux_gateway::admin::command_bridge::space::UpdateSpaceInput;

use tauri::{AppHandle, State};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::commands::gateway::GatewayAppState;
use crate::state::AppState;
use crate::tray;

fn bridge_ctx<'a>(
    app_state: &'a AppState,
    services: &'a ApplicationServices,
) -> SpaceBridgeCtx<'a> {
    SpaceBridgeCtx {
        services,
        spaces_dir: app_state.spaces_dir(),
    }
}

/// Emit a space domain event through the gateway when it is running.
async fn emit_space_domain_event(
    gateway_state: &Arc<RwLock<GatewayAppState>>,
    event: mcpmux_core::DomainEvent,
) {
    let gw_state = gateway_state.read().await;
    if let Some(ref gw) = gw_state.gateway_state {
        let gw = gw.read().await;
        gw.emit_domain_event(event);
    }
}

/// List all spaces.
#[tauri::command]
pub async fn list_spaces(
    state: State<'_, AppState>,
    services: State<'_, Arc<ApplicationServices>>,
) -> Result<Vec<Space>, String> {
    tracing::info!("[list_spaces] Command invoked");

    let spaces = space::list_spaces(&bridge_ctx(&state, &services))
        .await
        .map_err(|e| {
            tracing::error!("[list_spaces] Error: {}", e);
            e.to_string()
        })?;

    tracing::info!("[list_spaces] Returning {} spaces", spaces.len());
    for space in &spaces {
        tracing::info!("[list_spaces] Space: {} ({})", space.name, space.id);
    }

    Ok(spaces)
}

/// Get a space by ID.
#[tauri::command]
pub async fn get_space(
    id: String,
    state: State<'_, AppState>,
    services: State<'_, Arc<ApplicationServices>>,
) -> Result<Option<Space>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    space::get_space(&bridge_ctx(&state, &services), uuid)
        .await
        .map_err(|e| e.to_string())
}

/// Create a new space.
#[tauri::command]
pub async fn create_space(
    name: String,
    icon: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
    services: State<'_, Arc<ApplicationServices>>,
    gateway_state: State<'_, Arc<RwLock<GatewayAppState>>>,
) -> Result<Space, String> {
    let ctx = bridge_ctx(&state, &services);
    let space = space::create_space(&ctx, name.clone(), icon.clone())
        .await
        .map_err(|e| e.to_string())?;

    emit_space_domain_event(
        &gateway_state,
        mcpmux_core::DomainEvent::SpaceCreated {
            space_id: space.id,
            name: space.name.clone(),
            icon: icon.clone(),
        },
    )
    .await;

    if let Err(e) = tray::update_tray_spaces(&app, &state).await {
        warn!("Failed to update tray menu: {}", e);
    }

    info!("[create_space] Space '{}' created successfully", space.name);

    Ok(space)
}

/// Update a space's display metadata.
#[tauri::command]
pub async fn update_space(
    id: String,
    input: UpdateSpaceInput,
    app: AppHandle,
    state: State<'_, AppState>,
    services: State<'_, Arc<ApplicationServices>>,
    gateway_state: State<'_, Arc<RwLock<GatewayAppState>>>,
) -> Result<Space, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let ctx = bridge_ctx(&state, &services);
    let space = space::update_space(&ctx, uuid, input)
        .await
        .map_err(|e| e.to_string())?;

    emit_space_domain_event(
        &gateway_state,
        mcpmux_core::DomainEvent::SpaceUpdated {
            space_id: space.id,
            name: space.name.clone(),
        },
    )
    .await;

    if let Err(e) = tray::update_tray_spaces(&app, &state).await {
        warn!("Failed to update tray menu: {}", e);
    }

    info!("[update_space] Space '{}' updated successfully", space.name);

    Ok(space)
}

/// Delete a space.
#[tauri::command]
pub async fn delete_space(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
    services: State<'_, Arc<ApplicationServices>>,
    gateway_state: State<'_, Arc<RwLock<GatewayAppState>>>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    space::delete_space(&bridge_ctx(&state, &services), uuid)
        .await
        .map_err(|e| e.to_string())?;

    emit_space_domain_event(
        &gateway_state,
        mcpmux_core::DomainEvent::SpaceDeleted { space_id: uuid },
    )
    .await;

    if let Err(e) = tray::update_tray_spaces(&app, &state).await {
        warn!("Failed to update tray menu: {}", e);
    }

    info!("[delete_space] Space '{}' deleted successfully", uuid);

    Ok(())
}

/// Open space configuration file in external editor
#[tauri::command]
pub async fn open_space_config_file(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use std::process::Command;

    let config_path = state.space_config_path(&space_id);

    if !config_path.exists() {
        return Err(format!(
            "Space config file not found: {}",
            config_path.display()
        ));
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", config_path.to_str().unwrap()])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&config_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&config_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    Ok(())
}

/// Read space configuration file
#[tauri::command]
pub async fn read_space_config(
    space_id: String,
    state: State<'_, AppState>,
    services: State<'_, Arc<ApplicationServices>>,
) -> Result<String, String> {
    space::read_space_config(&bridge_ctx(&state, &services), &space_id)
        .await
        .map_err(|e| e.to_string())
}

/// Save space configuration file
#[tauri::command]
pub async fn save_space_config(
    space_id: String,
    content: String,
    state: State<'_, AppState>,
    services: State<'_, Arc<ApplicationServices>>,
) -> Result<(), String> {
    space::save_space_config(&bridge_ctx(&state, &services), &space_id, &content)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a server from the space configuration file
#[tauri::command]
pub async fn remove_server_from_config(
    space_id: String,
    server_id: String,
    state: State<'_, AppState>,
    services: State<'_, Arc<ApplicationServices>>,
) -> Result<bool, String> {
    space::remove_server_from_config(&bridge_ctx(&state, &services), &space_id, &server_id)
        .await
        .map_err(|e| e.to_string())
}

/// Refresh the system tray menu to reflect current spaces
#[tauri::command]
pub async fn refresh_tray_menu(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    tray::update_tray_spaces(&app, &state)
        .await
        .map_err(|e| format!("Failed to update tray menu: {}", e))
}
