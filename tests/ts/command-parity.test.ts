/**
 * Every command the UI sends must exist on the backend it can reach.
 *
 * Desktop: `apiCall`/`invoke` → a `#[tauri::command]` listed in `generate_handler!`.
 * Web admin: `apiCall` → a `COMMAND_ROUTES` entry → a route in the admin Axum router.
 *
 * Reads source text on purpose. A merge that drops a command from `lib.rs` or the
 * admin router compiles fine and only fails when that button is clicked.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { registeredCommands, routeFor } from '../../apps/desktop/src/lib/api/fetch-api';

const REPO = path.resolve(__dirname, '../..');
const UI_SRC = path.join(REPO, 'apps/desktop/src');
const TAURI_LIB = path.join(REPO, 'apps/desktop/src-tauri/src/lib.rs');
const ADMIN_ROUTER = path.join(REPO, 'crates/mcpmux-gateway/src/admin/router.rs');

/** Commands the UI only calls under `isTauri()`; no admin route is expected. */
const DESKTOP_ONLY = new Set(['open_url']);

/**
 * Known web-admin gaps. Remove an entry once its admin route lands; the
 * "stale allowlist" test fails if an entry here gains a route.
 */
const ADMIN_GAPS = new Set([
  'create_client_api_key',
  'dismiss_workspace_binding_prompt',
  'is_workspace_binding_prompt_dismissed',
  'list_client_api_keys',
  'register_api_key_client',
  'revoke_client_api_key',
]);

/** Commands chosen at runtime (`apiCall(command, …)`), listed explicitly. */
const DYNAMIC_COMMANDS = ['approve_oauth_consent', 'reject_oauth_consent'];

/**
 * Recursively list `.ts`/`.tsx` files under a directory.
 *
 * @param dir - Directory to walk.
 * @returns Absolute file paths.
 */
function listSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const full = path.join(dir, name);
    if (statSync(full).isDirectory()) return listSourceFiles(full);
    return /\.tsx?$/.test(name) ? [full] : [];
  });
}

/**
 * Collect literal command names passed to `apiCall(...)` or `invoke(...)`.
 *
 * @param fn - Call name to match.
 * @returns Sorted unique command names.
 */
function uiCommands(fn: 'apiCall' | 'invoke'): string[] {
  const pattern = new RegExp(`\\b${fn}(?:<[^(]*>)?\\(\\s*['"]([a-z0-9_]+)['"]`, 'g');
  const names = listSourceFiles(UI_SRC).flatMap((file) =>
    [...readFileSync(file, 'utf8').matchAll(pattern)].map((m) => m[1])
  );
  return [...new Set(names)].sort();
}

/**
 * Commands registered with Tauri's `generate_handler!` in `lib.rs`.
 *
 * @returns Set of bare command names (module path stripped).
 */
function tauriCommands(): Set<string> {
  const source = readFileSync(TAURI_LIB, 'utf8');
  const block = source.match(/generate_handler!\[([\s\S]*?)\]\)/);
  if (!block) throw new Error('generate_handler! block not found in lib.rs');
  const names = block[1]
    .split('\n')
    .map((line) =>
      line
        .replace(/\/\/.*$/, '')
        .trim()
        .replace(/,$/, '')
    )
    .filter(Boolean)
    .map((entry) => entry.split('::').pop() as string);
  return new Set(names);
}

/**
 * `METHOD path` pairs mounted on the admin router, with `{param}` segments normalized.
 *
 * @returns Set like `GET /api/v1/spaces/{}`.
 */
function adminRouterRoutes(): Set<string> {
  const source = readFileSync(ADMIN_ROUTER, 'utf8');
  const routes = new Set<string>();
  for (const chunk of source.split('.route(').slice(1)) {
    const routePath = chunk.match(/^\s*"([^"]+)"/)?.[1];
    if (!routePath) continue;
    for (const [, method] of chunk.matchAll(/\b(get|post|put|delete|patch)\(/g)) {
      routes.add(`${method.toUpperCase()} ${routePath.replace(/\{[^}]+\}/g, '{}')}`);
    }
  }
  return routes;
}

/**
 * Normalize a concrete admin path (from `routeFor`) to router form.
 *
 * @param method - HTTP method.
 * @param routePath - Path with real ids and an optional query string.
 * @param placeholders - Argument values substituted into the path.
 * @returns `METHOD path` with placeholders replaced by `{}`.
 */
function normalizeRoute(method: string, routePath: string, placeholders: string[]): string {
  let normalized = routePath.split('?')[0];
  for (const value of placeholders) {
    normalized = normalized.split(encodeURIComponent(value)).join('{}');
  }
  return `${method} ${normalized}`;
}

const PLACEHOLDER = 'PARITY_ARG';
const placeholderArgs = new Proxy({} as Record<string, unknown>, {
  get: (_target, key) => (typeof key === 'string' ? PLACEHOLDER : undefined),
  has: () => true,
});

const apiCallCommands = [...new Set([...uiCommands('apiCall'), ...DYNAMIC_COMMANDS])].sort();
const invokeCommands = uiCommands('invoke');
const tauri = tauriCommands();

describe('desktop command parity', () => {
  it('finds commands to check (guards against a broken scan)', () => {
    expect(apiCallCommands.length).toBeGreaterThan(100);
    expect(tauri.size).toBeGreaterThan(100);
  });

  it.each([...apiCallCommands, ...invokeCommands])('%s is a registered Tauri command', (cmd) => {
    expect(tauri.has(cmd)).toBe(true);
  });
});

describe('web admin command parity', () => {
  const routed = new Set(registeredCommands);
  const router = adminRouterRoutes();
  const adminCommands = apiCallCommands.filter(
    (cmd) => !DESKTOP_ONLY.has(cmd) && !ADMIN_GAPS.has(cmd)
  );

  it('parses the admin router (guards against a broken scan)', () => {
    expect(router.size).toBeGreaterThan(100);
  });

  it.each(adminCommands)('%s has an admin route mapping', (cmd) => {
    expect(routed.has(cmd)).toBe(true);
  });

  it.each(adminCommands.filter((cmd) => routed.has(cmd)))(
    '%s maps to a route the admin router mounts',
    (cmd) => {
      const { method, path: routePath } = routeFor(cmd, placeholderArgs);
      expect(router).toContain(normalizeRoute(method, routePath, [PLACEHOLDER]));
    }
  );

  it('has no stale admin-gap allowlist entries', () => {
    expect([...ADMIN_GAPS].filter((cmd) => routed.has(cmd))).toEqual([]);
  });
});
