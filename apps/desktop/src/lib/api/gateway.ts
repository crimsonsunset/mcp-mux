/** @deprecated Prefer `@/lib/backend` — shim during facade migration. */
import { apiCall } from './transport';

/**
 * Gateway lifecycle and pool disconnect API.
 *
 * Server enable/disable and OAuth flows live in `serverManager.ts`
 * (`enable_server_v2`, `disable_server_v2`, `logout_server`, etc.).
 *
 * | Tauri command | Wrapper | When to use |
 * | ------------- | ------- | ----------- |
 * | `disconnect_server` | `disconnectServer` | Tear down gateway pool connection; `logout: true` also clears stored OAuth tokens. `logout: false` (default) pauses while preserving credentials — ServersPage Disconnect button and uninstall cleanup. |
 *
 * Prefer `serverManager` for enable/disable toggles; use this module only for
 * gateway-scoped disconnect and gateway start/stop.
 */

/**
 * Gateway status.
 */
export interface GatewayStatus {
  running: boolean;
  url: string | null;
  active_sessions: number;
  connected_backends: number;
}

/**
 * Get gateway status.
 */
export async function getGatewayStatus(spaceId?: string): Promise<GatewayStatus> {
  return apiCall('get_gateway_status', { spaceId });
}

/**
 * Probe result for a proposed gateway start.
 *
 * `source` tells the UI which tier the preferred port came from so it can
 * phrase the prompt correctly ("your configured port" vs "the default port").
 */
export interface GatewayStartProbe {
  preferredPort: number;
  preferredAvailable: boolean;
  source: 'override' | 'configured' | 'default';
}

/**
 * Ask the backend whether the gateway can start on its preferred port.
 * Does not start anything — used by the UI to decide whether to prompt.
 */
export async function probeGatewayStart(port?: number): Promise<GatewayStartProbe> {
  return apiCall('probe_gateway_start', { port });
}

/**
 * Auto-start port conflict raised during app launch. When non-null, the UI
 * must prompt the user before the gateway will bind.
 */
export interface PendingPortConflict {
  preferredPort: number;
  source: 'configured' | 'default';
}

/**
 * Atomically read AND clear the deferred auto-start port conflict.
 *
 * "Take" semantics — only the first caller gets the conflict; subsequent
 * calls return null. Prevents duplicate prompts under React StrictMode's
 * double-mount.
 */
export async function takePendingPortConflict(): Promise<PendingPortConflict | null> {
  return apiCall('take_pending_port_conflict');
}

/**
 * Error marker the backend returns when the preferred port is busy and
 * `allowDynamicFallback` is false. Shape: `PORT_IN_USE:<port>:<source>`.
 */
export interface PortInUseError {
  kind: 'PortInUse';
  port: number;
  source: 'override' | 'configured' | 'default';
}

/** Parse the `PORT_IN_USE:<port>:<source>` sentinel the backend emits. */
export function parsePortInUseError(err: unknown): PortInUseError | null {
  const msg = err instanceof Error ? err.message : typeof err === 'string' ? err : '';
  const match = /^PORT_IN_USE:(\d+):(override|configured|default)$/.exec(msg);
  if (!match) return null;
  return {
    kind: 'PortInUse',
    port: Number(match[1]),
    source: match[2] as PortInUseError['source'],
  };
}

/**
 * Start the gateway server. Strict by default — pass `allowDynamicFallback`
 * to let the gateway pick a dynamic port when the preferred one is taken.
 */
export async function startGateway(opts?: {
  port?: number;
  allowDynamicFallback?: boolean;
}): Promise<string> {
  return apiCall('start_gateway', {
    port: opts?.port,
    allowDynamicFallback: opts?.allowDynamicFallback,
  });
}

/**
 * Stop the gateway server.
 */
export async function stopGateway(): Promise<void> {
  return apiCall('stop_gateway');
}

/**
 * Restart the gateway server. Same semantics as `startGateway`.
 */
export async function restartGateway(opts?: {
  port?: number;
  allowDynamicFallback?: boolean;
}): Promise<string> {
  return apiCall('restart_gateway', {
    port: opts?.port,
    allowDynamicFallback: opts?.allowDynamicFallback,
  });
}

/**
 * Backend server status.
 */
export interface BackendStatus {
  id: string;
  name: string;
  status: string;
  tools_count: number;
}

/**
 * Disconnect a server from the gateway pool.
 *
 * @param serverId - The server ID to disconnect
 * @param spaceId - The space ID (required for proper space isolation)
 * @param logout - When true, also clear stored OAuth tokens (credential logout)
 */
export async function disconnectServer(serverId: string, spaceId: string, logout?: boolean): Promise<void> {
  return apiCall('disconnect_server', { serverId, spaceId, logout });
}

/**
 * List all connected backend servers.
 */
export async function listConnectedServers(): Promise<BackendStatus[]> {
  return apiCall('list_connected_servers');
}

/**
 * Result of bulk server connection.
 */
export interface BulkConnectResult {
  connected: number;
  reused: number;
  failed: number;
  oauth_required: number;
  errors: string[];
}

/**
 * Connect all enabled servers from all spaces.
 * This is typically called on gateway startup.
 */
export async function connectAllEnabledServers(): Promise<BulkConnectResult> {
  return apiCall('connect_all_enabled_servers');
}

/**
 * Pool statistics.
 */
export interface PoolStats {
  total_instances: number;
  connected_instances: number;
  total_space_server_mappings: number;
}

/**
 * Get server pool statistics.
 */
export async function getPoolStats(): Promise<PoolStats> {
  return apiCall('get_pool_stats');
}

/**
 * Result of OAuth token refresh operation.
 */
export interface RefreshResult {
  servers_checked: number;
  tokens_refreshed: number;
  refresh_failed: number;
}

let refreshOAuthOnStartupPromise: Promise<RefreshResult> | null = null;

/**
 * Refresh OAuth tokens on startup for all installed HTTP servers.
 * This should be called during app initialization before connecting to servers.
 */
export async function refreshOAuthTokensOnStartup(): Promise<RefreshResult> {
  if (!refreshOAuthOnStartupPromise) {
    refreshOAuthOnStartupPromise = apiCall<RefreshResult>('refresh_oauth_tokens_on_startup');
  }
  return refreshOAuthOnStartupPromise;
}

// OAuth client CRUD and grants live in `oauth.ts`. Re-export for existing imports.
export type {
  OAuthClient,
  RegistrationType,
  UpdateClientRequest,
} from './oauth';
export {
  deleteOAuthClient,
  getOAuthClientGrants,
  grantOAuthClientFeatureSet,
  listOAuthClients,
  revokeOAuthClientFeatureSet,
  updateOAuthClient,
} from './oauth';
