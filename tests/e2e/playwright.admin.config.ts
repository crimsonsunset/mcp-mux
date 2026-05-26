/**
 * Playwright config for web admin parity E2E (real AdminServer on :45819).
 *
 * Phase 4+ enables specs under specs/admin/. Until then all tests are ignored.
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
