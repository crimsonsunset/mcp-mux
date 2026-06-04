import { test, expect } from '@playwright/test';

import {
  attachAdminPageDiagnostics,
  waitForAdminAppReady,
  waitForServersPage,
  waitForSpacesPage,
} from './_helpers/admin-diagnostics.helpers';

/**
 * Admin web smoke — browse read-only views against real AdminServer (:45819).
 *
 * Requires admin mode enabled with frontendDist served. No writes.
 */
test.describe('Admin read browse smoke', () => {
  test('loads SPA and browses Spaces, My Servers, Settings', async ({ page }) => {
    attachAdminPageDiagnostics(page);
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });
    await waitForAdminAppReady(page);

    await page.getByTestId('nav-spaces').click();
    await waitForSpacesPage(page);

    await page.getByTestId('nav-my-servers').click();
    await waitForServersPage(page);
    await expect(page.getByTestId('servers-count-summary')).toBeVisible({ timeout: 15_000 });

    await page.getByTestId('nav-settings').click();
    await expect(page.getByTestId('settings-startup-section')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('settings-gateway-section')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('logs-path')).toBeVisible({ timeout: 15_000 });
  });
});
