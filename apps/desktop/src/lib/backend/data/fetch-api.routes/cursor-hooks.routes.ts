import type { RouteHandler } from '../fetch-api.types';

/** Managed Cursor `preToolUse` hook install routes. */
export const cursorHookRoutes: Record<string, RouteHandler> = {
  cursor_hook_status: () => ({ method: 'GET', path: '/api/v1/cursor-hook' }),
  install_cursor_hook: () => ({ method: 'POST', path: '/api/v1/cursor-hook/install' }),
  uninstall_cursor_hook: () => ({ method: 'POST', path: '/api/v1/cursor-hook/uninstall' }),
};
