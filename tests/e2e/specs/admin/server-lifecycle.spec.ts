import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — server install/browse against real AdminServer (:45819).
 */
test.describe('Admin server lifecycle smoke', () => {
  test('browses discover and my servers', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-discover').click();
    await expect(page.getByTestId('search-input')).toBeVisible();

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('servers-page')).toBeVisible();
    await expect(page.getByTestId('servers-count-summary')).toBeVisible();
  });
});
