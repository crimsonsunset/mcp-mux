import { apiCall } from './transport';

/** Persisted per-root icon used before a binding exists. */
export interface WorkspaceAppearance {
  workspace_root: string;
  icon: string;
  updated_at: string;
}

export interface WorkspaceAppearanceInput {
  workspace_root: string;
  icon: string;
}

/** List all saved workspace appearances. */
export async function listWorkspaceAppearances(): Promise<WorkspaceAppearance[]> {
  return apiCall('list_workspace_appearances');
}

/** Upsert appearance for a normalized workspace root. */
export async function upsertWorkspaceAppearance(
  input: WorkspaceAppearanceInput
): Promise<WorkspaceAppearance> {
  return apiCall('upsert_workspace_appearance', { input });
}

/** Delete appearance for a workspace root. */
export async function deleteWorkspaceAppearance(workspaceRoot: string): Promise<void> {
  return apiCall('delete_workspace_appearance', { workspaceRoot });
}

/** Copy a source image into app data and return local: ref. */
export async function uploadWorkspaceIcon(sourcePath: string): Promise<string> {
  return apiCall('upload_workspace_icon', { sourcePath });
}

/** Resolve a local:workspace-icons ref to an absolute file path. */
export async function resolveWorkspaceIconPath(iconRef: string): Promise<string | null> {
  return apiCall('resolve_workspace_icon_path', { iconRef });
}
