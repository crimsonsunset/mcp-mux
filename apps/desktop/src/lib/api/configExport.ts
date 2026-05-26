import { invoke } from '@tauri-apps/api/core';

/** Supported MCP client config export targets. */
export type ExportClientType = 'cursor' | 'vscode' | 'claude';

/** Parameters for preview and file export commands. */
export interface ExportConfigRequest {
  client_type: ExportClientType;
  space_id: string;
  mask_credentials?: boolean;
}

/** Preview/export payload returned by the backend. */
export interface ExportConfigResponse {
  content: string;
  default_path: string | null;
  suggested_filename: string;
}

/**
 * Preview generated MCP client config JSON without writing to disk.
 */
export async function previewConfigExport(
  request: ExportConfigRequest
): Promise<ExportConfigResponse> {
  return invoke('preview_config_export', { request });
}

/**
 * Write generated MCP client config JSON to the given file path.
 *
 * @returns Absolute path of the written file.
 */
export async function exportConfigToFile(
  request: ExportConfigRequest,
  path: string
): Promise<string> {
  return invoke('export_config_to_file', { request, path });
}

/**
 * Default config file paths per client type (`cursor`, `vscode`, `claude`).
 */
export async function getConfigPaths(): Promise<Record<string, string | null>> {
  return invoke('get_config_paths');
}

/**
 * Whether a config file already exists at the default path for a client type.
 */
export async function checkConfigExists(clientType: ExportClientType): Promise<boolean> {
  return invoke('check_config_exists', { clientType });
}

/**
 * Copy an existing default config to a `.json.bak` sibling before overwrite.
 *
 * @returns Backup path when a file existed; otherwise `null`.
 */
export async function backupExistingConfig(
  clientType: ExportClientType
): Promise<string | null> {
  return invoke('backup_existing_config', { clientType });
}
