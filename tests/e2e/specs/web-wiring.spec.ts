import { test } from '@playwright/test';

import {
  attachWebPageDiagnostics,
  waitForWebAppReady,
} from '../helpers/web-app-ready.helpers';

/**
 * Smoke test: Vite :1420 → proxy `/api` → AdminServer :45819 (and CF headers when configured).
 */
test.describe('Web admin wiring', () => {
  test('SPA loads and initial spaces sync completes', async ({ page }) => {
    attachWebPageDiagnostics(page);
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    await waitForWebAppReady(page);
  });
});
