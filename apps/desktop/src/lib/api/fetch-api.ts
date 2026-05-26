export interface ApiRoute {
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  path: string;
}

/**
 * Build a query string from optional args, omitting null/undefined values.
 */
function buildQuery(args: Record<string, unknown>): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(args)) {
    if (value === undefined || value === null) {
      continue;
    }
    params.set(key, String(value));
  }
  const query = params.toString();
  return query ? `?${query}` : '';
}

/**
 * Map a Tauri command name and args to an admin REST route.
 */
export function routeFor(command: string, args: Record<string, unknown> = {}): ApiRoute {
  switch (command) {
    case 'get_gateway_status':
      return {
        method: 'GET',
        path: `/api/v1/gateway/status${buildQuery({ spaceId: args.spaceId })}`,
      };
    case 'probe_gateway_start':
      return {
        method: 'GET',
        path: `/api/v1/gateway/probe-start${buildQuery({ port: args.port })}`,
      };
    case 'take_pending_port_conflict':
      return { method: 'GET', path: '/api/v1/gateway/pending-port-conflict' };
    case 'get_gateway_port_settings':
      return { method: 'GET', path: '/api/v1/gateway/port-settings' };
    case 'reset_gateway_port':
      return { method: 'GET', path: '/api/v1/gateway/reset-port' };
    case 'list_connected_servers':
      return { method: 'GET', path: '/api/v1/gateway/connected-servers' };
    case 'get_pool_stats':
      return { method: 'GET', path: '/api/v1/gateway/pool-stats' };
    case 'list_spaces':
      return { method: 'GET', path: '/api/v1/spaces' };
    case 'get_space':
      return { method: 'GET', path: `/api/v1/spaces/${encodeURIComponent(String(args.id))}` };
    case 'read_space_config':
      return {
        method: 'GET',
        path: `/api/v1/spaces/${encodeURIComponent(String(args.spaceId))}/config`,
      };
    case 'list_installed_servers':
      return {
        method: 'GET',
        path: `/api/v1/servers/installed${buildQuery({ spaceId: args.spaceId })}`,
      };
    case 'discover_servers':
      return { method: 'GET', path: '/api/v1/registry/discover' };
    case 'get_server_definition':
      return {
        method: 'GET',
        path: `/api/v1/registry/definition/${encodeURIComponent(String(args.serverId))}`,
      };
    case 'get_registry_ui_config':
      return { method: 'GET', path: '/api/v1/registry/ui-config' };
    case 'get_registry_home_config':
      return { method: 'GET', path: '/api/v1/registry/home-config' };
    case 'is_registry_offline':
      return { method: 'GET', path: '/api/v1/registry/offline' };
    case 'list_clients':
      return { method: 'GET', path: '/api/v1/clients' };
    case 'get_client':
      return { method: 'GET', path: `/api/v1/clients/${encodeURIComponent(String(args.id))}` };
    case 'list_feature_sets':
      return { method: 'GET', path: '/api/v1/feature-sets' };
    case 'list_feature_sets_by_space':
      return {
        method: 'GET',
        path: `/api/v1/feature-sets/by-space/${encodeURIComponent(String(args.spaceId))}`,
      };
    case 'get_feature_set':
      return {
        method: 'GET',
        path: `/api/v1/feature-sets/${encodeURIComponent(String(args.id))}`,
      };
    case 'get_feature_set_with_members':
      return {
        method: 'GET',
        path: `/api/v1/feature-sets/${encodeURIComponent(String(args.id))}/with-members`,
      };
    case 'list_workspace_bindings':
      return { method: 'GET', path: '/api/v1/workspaces/bindings' };
    case 'list_workspace_bindings_for_space':
      return {
        method: 'GET',
        path: `/api/v1/workspaces/bindings/space/${encodeURIComponent(String(args.spaceId))}`,
      };
    case 'list_reported_workspace_roots':
      return { method: 'GET', path: '/api/v1/workspaces/reported-roots' };
    case 'validate_workspace_root':
      return {
        method: 'GET',
        path: `/api/v1/workspaces/validate-root${buildQuery({ path: args.path })}`,
      };
    case 'get_workspace_effective_features':
      return {
        method: 'GET',
        path: `/api/v1/workspaces/effective-features${buildQuery({ workspaceRoot: args.workspaceRoot })}`,
      };
    case 'list_workspace_appearances':
      return { method: 'GET', path: '/api/v1/workspaces/appearances' };
    case 'resolve_workspace_icon_path':
      return {
        method: 'GET',
        path: `/api/v1/workspaces/icon-path${buildQuery({ iconRef: args.iconRef })}`,
      };
    case 'list_session_overrides':
      return {
        method: 'GET',
        path: `/api/v1/session-overrides${buildQuery({ sessionId: args.sessionId })}`,
      };
    case 'get_startup_settings':
      return { method: 'GET', path: '/api/v1/settings/startup' };
    case 'get_meta_tools_enabled':
      return { method: 'GET', path: '/api/v1/settings/meta-tools-enabled' };
    case 'get_session_overrides_require_approval':
      return { method: 'GET', path: '/api/v1/settings/session-overrides-require-approval' };
    case 'get_version':
      return { method: 'GET', path: '/api/v1/app/version' };
    case 'get_bundle_version':
      return { method: 'GET', path: '/api/v1/app/bundle-version' };
    case 'get_logs_path':
      return { method: 'GET', path: '/api/v1/app/logs-path' };
    case 'get_server_logs':
      return {
        method: 'GET',
        path: `/api/v1/logs/server/${encodeURIComponent(String(args.serverId))}${buildQuery({
          limit: args.limit,
          levelFilter: args.levelFilter,
        })}`,
      };
    case 'get_server_log_file':
      return {
        method: 'GET',
        path: `/api/v1/logs/server/${encodeURIComponent(String(args.serverId))}/file`,
      };
    case 'get_log_retention_days':
      return { method: 'GET', path: '/api/v1/logs/retention-days' };
    case 'get_oauth_clients':
      return { method: 'GET', path: '/api/v1/oauth/clients' };
    case 'get_oauth_client_grants':
      return {
        method: 'GET',
        path: `/api/v1/oauth/clients/${encodeURIComponent(String(args.clientId))}/grants/${encodeURIComponent(String(args.spaceId))}`,
      };
    case 'open_url':
      return {
        method: 'GET',
        path: `/api/v1/oauth/open-url${buildQuery({ url: args.url })}`,
      };
    case 'list_meta_tool_grants':
      return { method: 'GET', path: '/api/v1/meta-tools/grants' };
    case 'list_server_features':
      return {
        method: 'GET',
        path: `/api/v1/server-features${buildQuery({
          spaceId: args.spaceId,
          includeUnavailable: args.includeUnavailable,
        })}`,
      };
    case 'list_server_features_by_server':
      return {
        method: 'GET',
        path: `/api/v1/server-features/by-server${buildQuery({
          spaceId: args.spaceId,
          serverId: args.serverId,
          includeUnavailable: args.includeUnavailable,
        })}`,
      };
    case 'list_server_features_by_type':
      return {
        method: 'GET',
        path: `/api/v1/server-features/by-type${buildQuery({
          spaceId: args.spaceId,
          serverId: args.serverId,
          featureType: args.featureType,
          includeUnavailable: args.includeUnavailable,
        })}`,
      };
    case 'get_server_feature':
      return {
        method: 'GET',
        path: `/api/v1/server-features/${encodeURIComponent(String(args.id))}`,
      };
    case 'is_clone_id_available':
      return {
        method: 'GET',
        path: `/api/v1/servers/clones/available${buildQuery({
          spaceId: args.spaceId,
          sourceServerId: args.sourceServerId,
          suffix: args.suffix,
        })}`,
      };
    case 'suggest_clone_suffix':
      return {
        method: 'GET',
        path: `/api/v1/servers/clones/suggest${buildQuery({
          spaceId: args.spaceId,
          sourceServerId: args.sourceServerId,
        })}`,
      };
    case 'list_clone_dependents':
      return {
        method: 'GET',
        path: `/api/v1/servers/clones/dependents${buildQuery({
          spaceId: args.spaceId,
          sourceServerId: args.sourceServerId,
        })}`,
      };
    default:
      throw new Error(`Unknown command: ${command}`);
  }
}

/**
 * Execute an admin REST request for the given command mapping.
 */
export async function fetchApi<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  const { method, path } = routeFor(command, args ?? {});
  const response = await fetch(path, {
    method,
    headers: { Accept: 'application/json' },
    credentials: 'same-origin',
  });

  if (!response.ok) {
    const body = await response.text();
    let message = body || response.statusText;
    try {
      const parsed = JSON.parse(body) as { error?: string };
      if (parsed.error) {
        message = parsed.error;
      }
    } catch {
      // keep raw body
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}
