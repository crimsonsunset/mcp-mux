import type { Page, Route } from '@playwright/test';

/**
 * Web E2E runs plain `vite`, which proxies `/api` to an admin server that is
 * not running. The web build calls that REST API for everything (the
 * `fetchApi` transport), so without answers the first-run "Name this device"
 * modal covers the whole UI and every click times out.
 *
 * `mockAdminApi` answers `/api/v1/*` from fixtures so the UI renders against
 * the real app code with no backend. Fixtures mirror the TypeScript shapes in
 * `apps/desktop/src/lib/api/*.ts`; when those change, change these.
 */

export const MOCK_NOW = '2026-01-01T00:00:00Z';

export const MOCK_MACHINE = {
  id: 'machine-e2e',
  name: 'E2E Machine',
  icon: '💻',
  hostname: 'e2e-host',
  created_at: MOCK_NOW,
  updated_at: MOCK_NOW,
};

export const MOCK_SPACE = {
  id: 'space-e2e',
  name: 'Personal',
  icon: '🏠',
  description: null,
  is_default: true,
  sort_order: 0,
  created_at: MOCK_NOW,
  updated_at: MOCK_NOW,
};

/** One OAuth client, shape of `OAuthClient` in `lib/api/gateway.ts`. */
export const MOCK_CLIENT = {
  client_id: 'client-e2e',
  registration_type: 'dynamic',
  client_name: 'E2E Client',
  client_alias: null,
  client_icon: null,
  redirect_uris: [],
  scope: null,
  approved: true,
  logo_uri: null,
  client_uri: null,
  software_id: null,
  software_version: null,
  last_seen: MOCK_NOW,
  created_at: MOCK_NOW,
  reports_roots: true,
  roots_capability_known: true,
  machine_id: MOCK_MACHINE.id,
};

type Body = unknown;
type Handler = Body | ((route: Route, url: URL) => Body);

/** Exact-path fixtures. Anything not listed falls back to `fallbackFor`. */
const BASE_FIXTURES: Record<string, Handler> = {
  '/api/v1/health': { status: 'ok' },
  '/api/v1/csrf-token': { token: 'e2e-csrf' },
  '/api/v1/machines': [MOCK_MACHINE],
  '/api/v1/machines/local': MOCK_MACHINE.id,
  '/api/v1/machines/hostname': MOCK_MACHINE.hostname,
  '/api/v1/spaces': [MOCK_SPACE],
  // Two lists on purpose: the Connections page reads OAuth clients, the
  // dashboard stat counts `list_clients` (`lib/api/clients.ts`).
  '/api/v1/oauth/clients': [MOCK_CLIENT],
  '/api/v1/clients': [
    { id: MOCK_CLIENT.client_id, name: MOCK_CLIENT.client_name, client_type: 'other', last_seen: MOCK_NOW },
  ],
  // `get_server_statuses` is a map of server id to status, not a list.
  '/api/v1/servers/connections': {},
  '/api/v1/gateway/status': {
    running: true,
    url: 'http://localhost:45818',
    active_sessions: 0,
    connected_backends: 0,
  },
  // Settings page. Shapes from `lib/api/settings.ts`; a null here crashes the page.
  '/api/v1/settings/startup': { autoLaunch: false, startMinimized: false, closeToTray: false },
  '/api/v1/settings/server-updates': { defaultUpdatePolicy: 'notify', lastCheckedAt: null },
  '/api/v1/settings/meta-tools-enabled': false,
  '/api/v1/settings/meta-tools-require-approval': true,
  '/api/v1/settings/auto-install-updates': false,
  '/api/v1/settings/update-channel': 'stable',
  '/api/v1/settings/workspace-mapping-prompt': true,
  '/api/v1/gateway/port-settings': { configuredPort: null, defaultPort: 45818, activePort: 45818 },
  '/api/v1/logs/retention-days': 30,
  '/api/v1/app/logs-path': '/tmp/mcpmux-e2e/logs',
  '/api/v1/meta-tools/grants': [],
  '/api/v1/app/version': '0.0.0-e2e',
  '/api/v1/app/bundle-version': '0',
  '/api/v1/app/build-info': {
    git_sha: 'e2e',
    git_branch: 'e2e',
    commit_time: MOCK_NOW,
    build_time: MOCK_NOW,
  },
};

/**
 * Empty-but-valid fallback by shape: collection-like paths get `[]`, everything
 * else `null`. The UI treats both as "nothing here yet".
 */
function fallbackFor(path: string): Body {
  const segments = path.replace('/api/v1/', '').split('/').filter(Boolean);
  const last = segments[segments.length - 1] ?? '';
  // Paths that end in an id (`feature-sets/by-space/<id>`, `bindings/space/<id>`)
  // are still collections, so match on the segment before the id too.
  const idListParents = ['by-space', 'space'];
  if (idListParents.includes(segments[segments.length - 2] ?? '')) {
    return [];
  }
  const collection = [
    'clients',
    'feature-sets',
    'installed',
    'bindings',
    'appearances',
    'reported-roots',
    'grants',
    'categories',
    'discover',
    'builtins',
    'server-features',
    'base-dirs',
    'connected-servers',
  ];
  return collection.includes(last) ? [] : null;
}

export interface AdminApiMockOptions {
  /** Extra or replacement fixtures keyed by exact `/api/v1/...` path (query ignored). */
  overrides?: Record<string, Handler>;
}

/**
 * Install the admin API mock on a page. Call before `page.goto`.
 * Also stubs the SSE stream so the page does not retry it forever.
 */
export async function mockAdminApi(page: Page, options: AdminApiMockOptions = {}): Promise<void> {
  const fixtures = { ...BASE_FIXTURES, ...options.overrides };

  // One handler, not two routes: Playwright runs the most recently registered
  // matching route first, so a separate `/events` route registered before the
  // catch-all would lose to it and get JSON back.
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname.replace(/\/$/, '');

    if (path === '/api/v1/events') {
      await route.fulfill({
        status: 200,
        headers: { 'content-type': 'text/event-stream', 'cache-control': 'no-cache' },
        body: ': e2e\n\n',
      });
      return;
    }

    if (request.method() !== 'GET') {
      await route.fulfill({ status: 200, contentType: 'application/json', body: 'null' });
      return;
    }

    const handler = path in fixtures ? fixtures[path] : fallbackFor(path);
    const body = typeof handler === 'function' ? handler(route, url) : handler;
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(body),
    });
  });
}
