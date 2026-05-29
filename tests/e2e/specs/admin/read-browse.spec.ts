import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — browse read-only views against real AdminServer (:45819).
 *
 * Requires admin mode enabled with frontendDist served. No writes.
 */
test.describe('Admin read browse smoke', () => {
  test('loads SPA and browses Spaces, My Servers, Settings', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-spaces').click();
    await expect(page.getByTestId('spaces-page')).toBeVisible();
    await expect(page.locator('[data-testid^="space-card-"]').first()).toBeVisible();

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('servers-page')).toBeVisible();
    await expect(page.getByTestId('servers-count-summary')).toBeVisible();

    await page.getByTestId('nav-settings').click();
    await expect(page.getByTestId('settings-startup-section')).toBeVisible();
    await expect(page.getByTestId('settings-gateway-section')).toBeVisible();
    await expect(page.getByTestId('logs-path')).toBeVisible();
  });
});
