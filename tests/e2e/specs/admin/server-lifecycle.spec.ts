import { test, expect } from '@playwright/test';

import {
  attachAdminPageDiagnostics,
  waitForAdminAppReady,
  waitForServersPage,
} from './_helpers/admin-diagnostics.helpers';

/**
 * Admin web smoke — server install/browse against real AdminServer (:45819).
 */
test.describe('Admin server lifecycle smoke', () => {
  test('browses discover and my servers', async ({ page }) => {
    attachAdminPageDiagnostics(page);
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });
    await waitForAdminAppReady(page);

    await page.getByTestId('nav-discover').click();
    await expect(page.getByTestId('search-input')).toBeVisible({ timeout: 15_000 });

    await page.getByTestId('nav-my-servers').click();
    await waitForServersPage(page);
    await expect(page.getByTestId('servers-count-summary')).toBeVisible({ timeout: 15_000 });
  });
});
