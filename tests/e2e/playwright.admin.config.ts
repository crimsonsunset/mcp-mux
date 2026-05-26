/**
 * Playwright config for web admin parity E2E (real AdminServer on :45819).
 *
 * Prereqs: McpMux running with web admin enabled, `pnpm build:web:admin` (serves
 * `apps/desktop/dist`), optional `MCPMUX_ADMIN_CF_JWT` when CF Access trust is on,
 * optional `MCPMUX_ADMIN_TEST=1` for SSE/oauth publish helpers.
 *
 * Negative CF Access spec (`security-negative.spec.ts`): set
 * `MCPMUX_ADMIN_CF_TRUST_ENABLED=1` with trust on and omit `MCPMUX_ADMIN_CF_JWT`.
 *
 * Note: `/api/v1/health` requires a valid CF Access JWT when trust is enabled —
 * Cloudflare Tunnel origin health probes cannot authenticate to the admin server.
 *
 * CI: `test:e2e:web:admin` is wired in package.json; full Linux CI job deferred until
 * an AdminServer fixture starts automatically in the workflow.
 */

import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './specs/admin',
  testMatch: '**/*.spec.ts',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [
    ['html', { outputFolder: './reports/admin-html' }],
    ['junit', { outputFile: './reports/admin-junit.xml' }],
    ['list'],
  ],
  use: {
    baseURL: 'http://localhost:45819',
    trace: 'on-first-retry',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
    extraHTTPHeaders: process.env.MCPMUX_ADMIN_CF_JWT
      ? { 'CF-Access-Jwt-Assertion': process.env.MCPMUX_ADMIN_CF_JWT }
      : undefined,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
