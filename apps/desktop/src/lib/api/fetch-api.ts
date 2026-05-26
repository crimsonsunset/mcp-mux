export interface ApiRoute {
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  path: string;
  body?: Record<string, unknown>;
}

let cachedCsrfToken: string | null = null;

/**
 * Fetch and cache the CSRF token for mutating admin requests.
 */
async function ensureCsrfToken(): Promise<string> {
  if (cachedCsrfToken) {
    return cachedCsrfToken;
  }
  const response = await fetch('/api/v1/csrf-token', {
    method: 'GET',
    headers: { Accept: 'application/json' },
    credentials: 'same-origin',
  });
  if (!response.ok) {
    throw new Error('Failed to fetch CSRF token');
  }
  const body = (await response.json()) as { token?: string };
  if (!body.token) {
    throw new Error('CSRF token missing from response');
  }
  cachedCsrfToken = body.token;
  return cachedCsrfToken;
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
    case 'create_space':
      return { method: 'POST', path: '/api/v1/spaces', body: { name: args.name, icon: args.icon } };
    case 'update_space':
      return {
        method: 'PUT',
        path: `/api/v1/spaces/${encodeURIComponent(String(args.id))}`,
        body: args.input as Record<string, unknown>,
      };
    case 'delete_space':
      return { method: 'DELETE', path: `/api/v1/spaces/${encodeURIComponent(String(args.id))}` };
    case 'save_space_config':
      return {
        method: 'PUT',
        path: `/api/v1/spaces/${encodeURIComponent(String(args.spaceId))}/config`,
        body: { content: args.content },
      };
    case 'remove_server_from_config':
      return {
        method: 'DELETE',
        path: `/api/v1/spaces/${encodeURIComponent(String(args.spaceId))}/config/servers/${encodeURIComponent(String(args.serverId))}`,
      };
    case 'start_gateway':
      return {
        method: 'POST',
        path: '/api/v1/gateway/start',
        body: { port: args.port, allowDynamicFallback: args.allowDynamicFallback },
      };
    case 'stop_gateway':
      return { method: 'POST', path: '/api/v1/gateway/stop' };
    case 'restart_gateway':
      return {
        method: 'POST',
        path: '/api/v1/gateway/restart',
        body: { port: args.port, allowDynamicFallback: args.allowDynamicFallback },
      };
    case 'disconnect_server':
      return {
        method: 'POST',
        path: '/api/v1/gateway/disconnect',
        body: { serverId: args.serverId, spaceId: args.spaceId, logout: args.logout },
      };
    case 'connect_all_enabled_servers':
      return { method: 'POST', path: '/api/v1/gateway/connect-all' };
    case 'refresh_oauth_tokens_on_startup':
      return { method: 'POST', path: '/api/v1/gateway/refresh-oauth-tokens' };
    case 'set_gateway_port':
      return { method: 'PUT', path: '/api/v1/gateway/port', body: { port: args.port } };
    case 'install_server':
      return {
        method: 'POST',
        path: '/api/v1/servers/install',
        body: { id: args.id, space_id: args.spaceId },
      };
    case 'uninstall_server':
      return {
        method: 'DELETE',
        path: `/api/v1/servers/${encodeURIComponent(String(args.id))}`,
        body: { space_id: args.spaceId },
      };
    case 'save_server_inputs':
      return {
        method: 'PUT',
        path: `/api/v1/servers/${encodeURIComponent(String(args.id))}/inputs`,
        body: {
          input_values: args.inputValues,
          space_id: args.spaceId,
          env_overrides: args.envOverrides,
          args_append: args.argsAppend,
          extra_headers: args.extraHeaders,
          display_name_override: args.displayNameOverride,
        },
      };
    case 'set_server_display_name':
      return {
        method: 'PUT',
        path: `/api/v1/servers/${encodeURIComponent(String(args.id))}/display-name`,
        body: { space_id: args.spaceId, display_name: args.displayName },
      };
    case 'set_server_oauth_connected':
      return {
        method: 'PUT',
        path: `/api/v1/servers/${encodeURIComponent(String(args.id))}/oauth-connected`,
        body: { space_id: args.spaceId, connected: args.connected },
      };
    case 'enable_server_v2':
      return {
        method: 'POST',
        path: '/api/v1/servers/connections/enable',
        body: { space_id: args.spaceId, server_id: args.serverId },
      };
    case 'disable_server_v2':
      return {
        method: 'POST',
        path: '/api/v1/servers/connections/disable',
        body: { space_id: args.spaceId, server_id: args.serverId },
      };
    case 'start_auth_v2':
      return {
        method: 'POST',
        path: '/api/v1/servers/connections/start-auth',
        body: { space_id: args.spaceId, server_id: args.serverId },
      };
    case 'cancel_auth_v2':
      return {
        method: 'POST',
        path: '/api/v1/servers/connections/cancel-auth',
        body: { space_id: args.spaceId, server_id: args.serverId },
      };
    case 'retry_connection':
      return {
        method: 'POST',
        path: '/api/v1/servers/connections/retry',
        body: { space_id: args.spaceId, server_id: args.serverId },
      };
    case 'logout_server':
      return {
        method: 'POST',
        path: '/api/v1/servers/connections/logout',
        body: { space_id: args.spaceId, server_id: args.serverId },
      };
    case 'clone_server':
      return {
        method: 'POST',
        path: '/api/v1/servers/clones',
        body: {
          space_id: args.spaceId,
          source_server_id: args.sourceServerId,
          suffix: args.suffix,
          alias: args.alias,
          display_name: args.displayName,
        },
      };
    case 'create_feature_set':
      return { method: 'POST', path: '/api/v1/feature-sets', body: args.input as Record<string, unknown> };
    case 'update_feature_set':
      return {
        method: 'PUT',
        path: `/api/v1/feature-sets/${encodeURIComponent(String(args.id))}`,
        body: args.input as Record<string, unknown>,
      };
    case 'delete_feature_set':
      return { method: 'DELETE', path: `/api/v1/feature-sets/${encodeURIComponent(String(args.id))}` };
    case 'add_feature_set_member':
      return {
        method: 'POST',
        path: `/api/v1/feature-sets/${encodeURIComponent(String(args.featureSetId))}/members`,
        body: args.input as Record<string, unknown>,
      };
    case 'remove_feature_set_member':
      return {
        method: 'DELETE',
        path: `/api/v1/feature-sets/${encodeURIComponent(String(args.featureSetId))}/members/${encodeURIComponent(String(args.memberId))}`,
      };
    case 'set_feature_set_members':
      return {
        method: 'PUT',
        path: `/api/v1/feature-sets/${encodeURIComponent(String(args.featureSetId))}/members`,
        body: { members: args.members },
      };
    case 'create_client':
      return { method: 'POST', path: '/api/v1/clients', body: args.input as Record<string, unknown> };
    case 'delete_client':
      return { method: 'DELETE', path: `/api/v1/clients/${encodeURIComponent(String(args.id))}` };
    case 'init_preset_clients':
      return { method: 'POST', path: '/api/v1/clients/init-presets' };
    case 'create_workspace_binding':
      return { method: 'POST', path: '/api/v1/workspaces/bindings', body: args.input as Record<string, unknown> };
    case 'update_workspace_binding':
      return {
        method: 'PUT',
        path: `/api/v1/workspaces/bindings/${encodeURIComponent(String(args.id))}`,
        body: args.input as Record<string, unknown>,
      };
    case 'delete_workspace_binding':
      return {
        method: 'DELETE',
        path: `/api/v1/workspaces/bindings/${encodeURIComponent(String(args.id))}`,
      };
    case 'upsert_workspace_appearance':
      return {
        method: 'PUT',
        path: '/api/v1/workspaces/appearances',
        body: args.input as Record<string, unknown>,
      };
    case 'delete_workspace_appearance':
      return {
        method: 'DELETE',
        path: '/api/v1/workspaces/appearances',
        body: { workspace_root: args.workspaceRoot },
      };
    case 'upload_workspace_icon':
      return {
        method: 'POST',
        path: '/api/v1/workspaces/appearances',
        body: { source_path: args.sourcePath },
      };
    case 'update_startup_settings':
      return { method: 'PUT', path: '/api/v1/settings/startup', body: args.settings as Record<string, unknown> };
    case 'set_meta_tools_enabled':
      return {
        method: 'PUT',
        path: '/api/v1/settings/meta-tools-enabled',
        body: { enabled: args.enabled },
      };
    case 'set_session_overrides_require_approval':
      return {
        method: 'PUT',
        path: '/api/v1/settings/session-overrides-require-approval',
        body: { requireApproval: args.requireApproval },
      };
    case 'clear_session_overrides':
      return {
        method: 'POST',
        path: '/api/v1/session-overrides/clear',
        body: { session_id: args.sessionId },
      };
    case 'clear_server_logs':
      return {
        method: 'DELETE',
        path: `/api/v1/logs/server/${encodeURIComponent(String(args.serverId))}`,
      };
    case 'set_log_retention_days':
      return { method: 'PUT', path: '/api/v1/logs/retention-days', body: { days: args.days } };
    case 'refresh_registry':
      return { method: 'POST', path: '/api/v1/registry/refresh' };
    case 'respond_to_meta_tool_approval':
      return {
        method: 'POST',
        path: '/api/v1/meta-tools/approval',
        body: {
          request_id: args.requestId,
          client_id: args.clientId,
          tool_name: args.toolName,
          decision: args.decision,
        },
      };
    case 'revoke_meta_tool_grant':
      return {
        method: 'POST',
        path: '/api/v1/meta-tools/grants/revoke',
        body: { client_id: args.clientId, tool_name: args.toolName },
      };
    case 'update_oauth_client':
      return {
        method: 'PUT',
        path: `/api/v1/oauth/clients/${encodeURIComponent(String(args.clientId))}`,
        body: { client_alias: (args.settings as { client_alias?: string } | undefined)?.client_alias },
      };
    case 'delete_oauth_client':
      return {
        method: 'DELETE',
        path: `/api/v1/oauth/clients/${encodeURIComponent(String(args.clientId))}`,
      };
    case 'grant_oauth_client_feature_set':
      return {
        method: 'POST',
        path: `/api/v1/oauth/clients/${encodeURIComponent(String(args.clientId))}/grants`,
        body: { space_id: args.spaceId, feature_set_id: args.featureSetId },
      };
    case 'revoke_oauth_client_feature_set':
      return {
        method: 'POST',
        path: `/api/v1/oauth/clients/${encodeURIComponent(String(args.clientId))}/grants/revoke`,
        body: { space_id: args.spaceId, feature_set_id: args.featureSetId },
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
  const { method, path, body } = routeFor(command, args ?? {});
  const headers: Record<string, string> = { Accept: 'application/json' };
  const init: RequestInit = {
    method,
    headers,
    credentials: 'same-origin',
  };

  if (method !== 'GET') {
    headers['Content-Type'] = 'application/json';
    headers['X-CSRF-Token'] = await ensureCsrfToken();
    if (body !== undefined) {
      init.body = JSON.stringify(body);
    } else if (method === 'POST' || method === 'PUT' || method === 'DELETE') {
      init.body = '{}';
    }
  }

  const response = await fetch(path, init);

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
