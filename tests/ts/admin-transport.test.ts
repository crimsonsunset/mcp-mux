import { describe, it, expect } from 'vitest';

import { routeFor } from '../../apps/desktop/src/lib/api/fetch-api';

const SPACE_ID = '11111111-1111-1111-1111-111111111111';
const CLIENT_ID = '22222222-2222-2222-2222-222222222222';
const FEATURE_SET_ID = '33333333-3333-3333-3333-333333333333';
const SERVER_ID = 'demo-server';
const FEATURE_ID = '44444444-4444-4444-4444-444444444444';

/** P4 read commands — one vitest row per parity matrix entry. */
const P4_READ_ROUTES: Array<{
  command: string;
  args?: Record<string, unknown>;
  method: 'GET';
  path: string;
}> = [
  {
    command: 'get_gateway_status',
    args: { spaceId: SPACE_ID },
    method: 'GET',
    path: `/api/v1/gateway/status?spaceId=${SPACE_ID}`,
  },
  {
    command: 'probe_gateway_start',
    args: { port: 45818 },
    method: 'GET',
    path: '/api/v1/gateway/probe-start?port=45818',
  },
  { command: 'take_pending_port_conflict', method: 'GET', path: '/api/v1/gateway/pending-port-conflict' },
  { command: 'get_gateway_port_settings', method: 'GET', path: '/api/v1/gateway/port-settings' },
  { command: 'reset_gateway_port', method: 'GET', path: '/api/v1/gateway/reset-port' },
  { command: 'list_connected_servers', method: 'GET', path: '/api/v1/gateway/connected-servers' },
  { command: 'get_pool_stats', method: 'GET', path: '/api/v1/gateway/pool-stats' },
  { command: 'list_spaces', method: 'GET', path: '/api/v1/spaces' },
  { command: 'get_space', args: { id: SPACE_ID }, method: 'GET', path: `/api/v1/spaces/${SPACE_ID}` },
  {
    command: 'read_space_config',
    args: { spaceId: SPACE_ID },
    method: 'GET',
    path: `/api/v1/spaces/${SPACE_ID}/config`,
  },
  {
    command: 'list_installed_servers',
    args: { spaceId: SPACE_ID },
    method: 'GET',
    path: `/api/v1/servers/installed?spaceId=${SPACE_ID}`,
  },
  { command: 'discover_servers', method: 'GET', path: '/api/v1/registry/discover' },
  {
    command: 'get_server_definition',
    args: { serverId: SERVER_ID },
    method: 'GET',
    path: `/api/v1/registry/definition/${SERVER_ID}`,
  },
  { command: 'get_registry_ui_config', method: 'GET', path: '/api/v1/registry/ui-config' },
  { command: 'get_registry_home_config', method: 'GET', path: '/api/v1/registry/home-config' },
  { command: 'is_registry_offline', method: 'GET', path: '/api/v1/registry/offline' },
  { command: 'list_clients', method: 'GET', path: '/api/v1/clients' },
  { command: 'get_client', args: { id: CLIENT_ID }, method: 'GET', path: `/api/v1/clients/${CLIENT_ID}` },
  { command: 'list_feature_sets', method: 'GET', path: '/api/v1/feature-sets' },
  {
    command: 'list_feature_sets_by_space',
    args: { spaceId: SPACE_ID },
    method: 'GET',
    path: `/api/v1/feature-sets/by-space/${SPACE_ID}`,
  },
  {
    command: 'get_feature_set',
    args: { id: FEATURE_SET_ID },
    method: 'GET',
    path: `/api/v1/feature-sets/${FEATURE_SET_ID}`,
  },
  {
    command: 'get_feature_set_with_members',
    args: { id: FEATURE_SET_ID },
    method: 'GET',
    path: `/api/v1/feature-sets/${FEATURE_SET_ID}/with-members`,
  },
  { command: 'list_workspace_bindings', method: 'GET', path: '/api/v1/workspaces/bindings' },
  {
    command: 'list_workspace_bindings_for_space',
    args: { spaceId: SPACE_ID },
    method: 'GET',
    path: `/api/v1/workspaces/bindings/space/${SPACE_ID}`,
  },
  { command: 'list_reported_workspace_roots', method: 'GET', path: '/api/v1/workspaces/reported-roots' },
  {
    command: 'validate_workspace_root',
    args: { path: '/tmp/workspace' },
    method: 'GET',
    path: '/api/v1/workspaces/validate-root?path=%2Ftmp%2Fworkspace',
  },
  {
    command: 'get_workspace_effective_features',
    args: { workspaceRoot: '/tmp/workspace' },
    method: 'GET',
    path: '/api/v1/workspaces/effective-features?workspaceRoot=%2Ftmp%2Fworkspace',
  },
  { command: 'list_workspace_appearances', method: 'GET', path: '/api/v1/workspaces/appearances' },
  {
    command: 'resolve_workspace_icon_path',
    args: { iconRef: 'local:workspace-icons/demo.png' },
    method: 'GET',
    path: '/api/v1/workspaces/icon-path?iconRef=local%3Aworkspace-icons%2Fdemo.png',
  },
  { command: 'list_session_overrides', method: 'GET', path: '/api/v1/session-overrides' },
  {
    command: 'list_session_overrides',
    args: { sessionId: 'sess-1' },
    method: 'GET',
    path: '/api/v1/session-overrides?sessionId=sess-1',
  },
  { command: 'get_startup_settings', method: 'GET', path: '/api/v1/settings/startup' },
  { command: 'get_meta_tools_enabled', method: 'GET', path: '/api/v1/settings/meta-tools-enabled' },
  {
    command: 'get_session_overrides_require_approval',
    method: 'GET',
    path: '/api/v1/settings/session-overrides-require-approval',
  },
  { command: 'get_version', method: 'GET', path: '/api/v1/app/version' },
  { command: 'get_bundle_version', method: 'GET', path: '/api/v1/app/bundle-version' },
  { command: 'get_logs_path', method: 'GET', path: '/api/v1/app/logs-path' },
  {
    command: 'get_server_logs',
    args: { serverId: SERVER_ID, limit: 50, levelFilter: 'error' },
    method: 'GET',
    path: `/api/v1/logs/server/${SERVER_ID}?limit=50&levelFilter=error`,
  },
  {
    command: 'get_server_log_file',
    args: { serverId: SERVER_ID },
    method: 'GET',
    path: `/api/v1/logs/server/${SERVER_ID}/file`,
  },
  { command: 'get_log_retention_days', method: 'GET', path: '/api/v1/logs/retention-days' },
  { command: 'get_oauth_clients', method: 'GET', path: '/api/v1/oauth/clients' },
  {
    command: 'get_oauth_client_grants',
    args: { clientId: CLIENT_ID, spaceId: SPACE_ID },
    method: 'GET',
    path: `/api/v1/oauth/clients/${CLIENT_ID}/grants/${SPACE_ID}`,
  },
  {
    command: 'open_url',
    args: { url: 'https://example.com/path' },
    method: 'GET',
    path: '/api/v1/oauth/open-url?url=https%3A%2F%2Fexample.com%2Fpath',
  },
  { command: 'list_meta_tool_grants', method: 'GET', path: '/api/v1/meta-tools/grants' },
  {
    command: 'list_server_features',
    args: { spaceId: SPACE_ID },
    method: 'GET',
    path: `/api/v1/server-features?spaceId=${SPACE_ID}`,
  },
  {
    command: 'list_server_features_by_server',
    args: { spaceId: SPACE_ID, serverId: SERVER_ID },
    method: 'GET',
    path: `/api/v1/server-features/by-server?spaceId=${SPACE_ID}&serverId=${SERVER_ID}`,
  },
  {
    command: 'list_server_features_by_type',
    args: { spaceId: SPACE_ID, serverId: SERVER_ID, featureType: 'tool' },
    method: 'GET',
    path: `/api/v1/server-features/by-type?spaceId=${SPACE_ID}&serverId=${SERVER_ID}&featureType=tool`,
  },
  {
    command: 'get_server_feature',
    args: { id: FEATURE_ID },
    method: 'GET',
    path: `/api/v1/server-features/${FEATURE_ID}`,
  },
  {
    command: 'is_clone_id_available',
    args: { spaceId: SPACE_ID, sourceServerId: SERVER_ID, suffix: 'work' },
    method: 'GET',
    path: `/api/v1/servers/clones/available?spaceId=${SPACE_ID}&sourceServerId=${SERVER_ID}&suffix=work`,
  },
  {
    command: 'suggest_clone_suffix',
    args: { spaceId: SPACE_ID, sourceServerId: SERVER_ID },
    method: 'GET',
    path: `/api/v1/servers/clones/suggest?spaceId=${SPACE_ID}&sourceServerId=${SERVER_ID}`,
  },
  {
    command: 'list_clone_dependents',
    args: { spaceId: SPACE_ID, sourceServerId: SERVER_ID },
    method: 'GET',
    path: `/api/v1/servers/clones/dependents?spaceId=${SPACE_ID}&sourceServerId=${SERVER_ID}`,
  },
];

describe('admin transport mapping', () => {
  it.each(P4_READ_ROUTES)('maps $command', ({ command, args, method, path }) => {
    expect(routeFor(command, args)).toEqual({ method, path });
  });

  it('rejects unknown commands', () => {
    expect(() => routeFor('start_gateway')).toThrow('Unknown command: start_gateway');
  });
});
