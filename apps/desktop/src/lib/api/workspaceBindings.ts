import { invoke } from '@tauri-apps/api/core';

/**
 * A WorkspaceBinding maps a normalized filesystem path (the workspace root
 * reported by an MCP client via `roots/list`) to a FeatureSet. Bindings are
 * the middle tier of resolver v2: pin > binding > space-active.
 */
export interface WorkspaceBinding {
  id: string;
  space_id: string;
  workspace_root: string;
  feature_set_id: string;
  created_at: string;
  updated_at: string;
}

/** List every binding across all Spaces. */
export async function listWorkspaceBindings(): Promise<WorkspaceBinding[]> {
  return invoke('list_workspace_bindings');
}

/** List bindings for a specific Space. */
export async function listWorkspaceBindingsForSpace(
  spaceId: string
): Promise<WorkspaceBinding[]> {
  return invoke('list_workspace_bindings_for_space', { spaceId });
}

/**
 * Create a new binding. `workspaceRoot` is normalized on the Rust side
 * (Windows drive letter case-folded, `file://` scheme stripped, trailing
 * separator trimmed) so callers can pass whatever the OS or MCP client
 * reports and rely on consistent matching later.
 */
export async function createWorkspaceBinding(
  spaceId: string,
  workspaceRoot: string,
  featureSetId: string
): Promise<WorkspaceBinding> {
  return invoke('create_workspace_binding', {
    spaceId,
    workspaceRoot,
    featureSetId,
  });
}

/** Update an existing binding's path or FeatureSet. */
export async function updateWorkspaceBinding(
  id: string,
  workspaceRoot: string,
  featureSetId: string
): Promise<WorkspaceBinding> {
  return invoke('update_workspace_binding', {
    id,
    workspaceRoot,
    featureSetId,
  });
}

/** Delete a binding by id. */
export async function deleteWorkspaceBinding(id: string): Promise<void> {
  return invoke('delete_workspace_binding', { id });
}
