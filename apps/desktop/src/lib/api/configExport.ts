/** @deprecated Prefer `@/lib/backend` — shim during facade migration. */
import { apiCall } from './transport';

/** Supported MCP client config export targets. */
export type ExportClientType = 'cursor' | 'vscode' | 'claude';

/**
 * Default config file paths per client type (`cursor`, `vscode`, `claude`).
 */
export async function getConfigPaths(): Promise<Record<string, string | null>> {
  return apiCall('get_config_paths');
}

/**
 * Whether a config file already exists at the default path for a client type.
 */
export async function checkConfigExists(clientType: ExportClientType): Promise<boolean> {
  return apiCall('check_config_exists', { clientType });
}

/**
 * Copy an existing default config to a `.json.bak` sibling before overwrite.
 *
 * @returns Backup path when a file existed; otherwise `null`.
 */
export async function backupExistingConfig(
  clientType: ExportClientType
): Promise<string | null> {
  return apiCall('backup_existing_config', { clientType });
}
