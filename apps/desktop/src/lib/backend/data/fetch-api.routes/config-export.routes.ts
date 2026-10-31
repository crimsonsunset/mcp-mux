import type { RouteHandler } from '../fetch-api.types';

/** MCP client config admin routes (paths, existence check, backup). */
export const configExportRoutes: Record<string, RouteHandler> = {
  get_config_paths: () => ({ method: 'GET', path: '/api/v1/config-export/paths' }),
  check_config_exists: (args) => ({
    method: 'POST',
    path: '/api/v1/config-export/check',
    body: { clientType: args.clientType },
  }),
  backup_existing_config: (args) => ({
    method: 'POST',
    path: '/api/v1/config-export/backup',
    body: { clientType: args.clientType },
  }),
};
