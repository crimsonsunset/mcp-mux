//! Tauri commands for workspace-root FeatureSet bindings.
//!
//! Bindings are the middle tier of the FeatureSet resolver (pin > binding >
//! space-active). Paths passed in from the UI are normalized via
//! [`mcpmux_core::normalize_workspace_root`] before storage so lookups from
//! the gateway's session registry hit them consistently.

use mcpmux_core::{normalize_workspace_root, WorkspaceBinding};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{error, info};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceBindingDto {
    pub id: String,
    pub space_id: String,
    pub workspace_root: String,
    pub feature_set_id: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<WorkspaceBinding> for WorkspaceBindingDto {
    fn from(b: WorkspaceBinding) -> Self {
        Self {
            id: b.id.to_string(),
            space_id: b.space_id.to_string(),
            workspace_root: b.workspace_root,
            feature_set_id: b.feature_set_id.to_string(),
            created_at: b.created_at.to_rfc3339(),
            updated_at: b.updated_at.to_rfc3339(),
        }
    }
}

/// List every binding across all Spaces.
#[tauri::command]
pub async fn list_workspace_bindings(
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceBindingDto>, String> {
    state
        .workspace_binding_repository
        .list()
        .await
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(|e| {
            error!("[workspace_binding::list] {e}");
            e.to_string()
        })
}

/// List bindings for a specific Space.
#[tauri::command]
pub async fn list_workspace_bindings_for_space(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceBindingDto>, String> {
    let space_uuid = Uuid::parse_str(&space_id).map_err(|e| e.to_string())?;
    state
        .workspace_binding_repository
        .list_for_space(&space_uuid)
        .await
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(|e| e.to_string())
}

/// Create a new binding. `workspace_root` is normalized before storage.
#[tauri::command]
pub async fn create_workspace_binding(
    space_id: String,
    workspace_root: String,
    feature_set_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceBindingDto, String> {
    let space_uuid = Uuid::parse_str(&space_id).map_err(|e| e.to_string())?;
    let fs_uuid = Uuid::parse_str(&feature_set_id).map_err(|e| e.to_string())?;
    let normalized = normalize_workspace_root(&workspace_root);
    if normalized.is_empty() {
        return Err("workspace_root cannot be empty".into());
    }
    let binding = WorkspaceBinding::new(space_uuid, normalized, fs_uuid);
    state
        .workspace_binding_repository
        .create(&binding)
        .await
        .map_err(|e| e.to_string())?;
    info!(
        space_id = %binding.space_id,
        workspace_root = %binding.workspace_root,
        feature_set_id = %binding.feature_set_id,
        "[workspace_binding] created",
    );
    Ok(binding.into())
}

/// Update an existing binding (e.g., point it at a different FS).
#[tauri::command]
pub async fn update_workspace_binding(
    id: String,
    workspace_root: String,
    feature_set_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceBindingDto, String> {
    let id_uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let fs_uuid = Uuid::parse_str(&feature_set_id).map_err(|e| e.to_string())?;
    let normalized = normalize_workspace_root(&workspace_root);
    if normalized.is_empty() {
        return Err("workspace_root cannot be empty".into());
    }
    let existing = state
        .workspace_binding_repository
        .get(&id_uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("binding not found: {}", id))?;
    let updated = WorkspaceBinding {
        id: existing.id,
        space_id: existing.space_id,
        workspace_root: normalized,
        feature_set_id: fs_uuid,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now(),
    };
    state
        .workspace_binding_repository
        .update(&updated)
        .await
        .map_err(|e| e.to_string())?;
    Ok(updated.into())
}

/// Delete a binding by id.
#[tauri::command]
pub async fn delete_workspace_binding(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let id_uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state
        .workspace_binding_repository
        .delete(&id_uuid)
        .await
        .map_err(|e| e.to_string())
}
