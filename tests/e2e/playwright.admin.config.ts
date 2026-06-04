/**
 * Playwright config for web admin parity E2E (real AdminServer on :45819).
 *
 * Prereqs: McpMux running with web admin enabled, `pnpm build:web:admin` (serves
 * `apps/desktop/dist`), optional `MCPMUX_ADMIN_CF_JWT` when CF Access trust is on,
 * optional `MCPMUX_CF_ACCESS_CLIENT_ID` + `MCPMUX_CF_ACCESS_CLIENT_SECRET` for
 * service-token headers (tunnel smoke or admin origin fallback when env is set),
 * optional `MCPMUX_ADMIN_TEST=1` for SSE/oauth publish helpers.
 *
 * Negative CF Access spec (`security-negative.spec.ts`): set
 * `MCPMUX_ADMIN_CF_TRUST_ENABLED=1` with trust on and omit `MCPMUX_ADMIN_CF_JWT`.
 *
 * Note: `/api/v1/health` requires a valid CF Access JWT (or matching service-token
 * headers when `MCPMUX_CF_ACCESS_*` env vars are set on the admin process) when trust
 * is enabled — Cloudflare Tunnel origin health probes cannot authenticate to the admin server.
 *
 * CI/local: `webServer` runs `scripts/admin-e2e-fixture.mjs`. With CF credentials in `.env`,
 * Playwright waits on port :45819 (not HTTP 200) because trust-on returns 401 without headers.
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

const cfProbeAuth = hasAdminCfProbeAuth();
const extraHTTPHeaders =
  Object.keys(adminCfProbeHeaders()).length > 0 ? adminCfProbeHeaders() : undefined;

export default defineConfig({
  testDir: './specs/admin',
  testMatch: '**/*.spec.ts',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  maxFailures: 1,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  // Single worker: one AdminServer + SQLite; parallel runs race startup sync.
  workers: 1,
  reporter: [
    ['html', { outputFolder: './reports/admin-html' }],
    ['junit', { outputFile: './reports/admin-junit.xml' }],
    ['list'],
  ],
  use: {
    baseURL: process.env.MCPMUX_ADMIN_BASE_URL ?? 'http://localhost:45819',
    trace: 'on-first-retry',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
    extraHTTPHeaders,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: 'node scripts/admin-e2e-fixture.mjs',
    ...(cfProbeAuth
      ? { port: Number.parseInt(process.env.MCPMUX_ADMIN_PORT ?? '45819', 10) }
      : { url: 'http://127.0.0.1:45819/' }),
    reuseExistingServer: !process.env.CI,
    cwd: '../..',
    timeout: 300_000,
  },
});
