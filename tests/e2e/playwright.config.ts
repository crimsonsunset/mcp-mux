/**
 * Playwright config for the web admin UI smoke suite (Vite SPA on :1420).
 *
 * The SPA runs in web-admin mode (`VITE_ADMIN_WEB`), so it talks to the real
 * backend over HTTP — Vite proxies `/api` to the AdminServer on :45819. There is
 * no Tauri IPC mock, so these specs need a live backend: the `webServer` below
 * boots one via `admin-e2e-fixture.mjs` (the same fixture the admin parity suite
 * uses). Admin-only specs live in `playwright.admin.config.ts` and are excluded
 * here via `testIgnore`.
 *
 * Locally, a running `pnpm dev` is reused (`reuseExistingServer`); in CI the
 * fixture starts `pnpm dev` (xvfb on Linux) and waits for :45819.
 */

import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { defineConfig, devices } from '@playwright/test';

import {
  adminCfProbeHeaders,
  hasAdminCfProbeAuth,
  loadRepoDotEnv,
} from '../../scripts/cf-access-env.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
loadRepoDotEnv(REPO_ROOT);

// Playwright sets FORCE_COLOR; NO_COLOR (often set by the IDE shell) triggers per-worker Node warnings.
delete process.env.NO_COLOR;

const cfProbeAuth = hasAdminCfProbeAuth();
const extraHTTPHeaders =
  Object.keys(adminCfProbeHeaders()).length > 0 ? adminCfProbeHeaders() : undefined;
const ADMIN_PORT = Number.parseInt(process.env.MCPMUX_ADMIN_PORT ?? '45819', 10);

export default defineConfig({
  testDir: './specs',
  testMatch: '**/*.spec.ts', // Only .spec.ts files (not .wdio.ts)
  // Admin specs need AdminServer on :45819 — use playwright.admin.config.ts + test:e2e:web:admin
  testIgnore: ['**/admin/**'],
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  // One worker: single AdminServer + SQLite; parallel runs race startup sync.
  workers: 1,
  maxFailures: 1,
  expect: { timeout: 15_000 },
  timeout: 60_000,
  reporter: [
    ['html', { outputFolder: './reports/html' }],
    ['junit', { outputFile: './reports/junit.xml' }],
    ['list'],
  ],
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
    extraHTTPHeaders,
    launchOptions: {
      args: ['--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage'],
    },
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
  ],
  // Boot the real backend (admin server :45819 + Vite :1420 via tauri beforeDevCommand).
  // Wait on :45819 so specs don't race startup; with CF trust on locally it returns 401,
  // so probe the port (TCP) instead of an HTTP 2xx.
  webServer: {
    command: 'node scripts/admin-e2e-fixture.mjs',
    ...(cfProbeAuth ? { port: ADMIN_PORT } : { url: `http://127.0.0.1:${ADMIN_PORT}/` }),
    reuseExistingServer: !process.env.CI,
    cwd: '../..',
    timeout: 300_000,
  },
});
