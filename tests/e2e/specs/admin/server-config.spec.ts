import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — discover + my servers config path (subset of server-config.wdio.ts).
 */
test.describe('Admin server config browse', () => {
  test('discover search and my servers page load', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-discover').click();
    const search = page.getByTestId('search-input');
    await expect(search).toBeVisible();
    await search.fill('PostgreSQL');
    await expect(page.getByTestId('registry-server-grid')).toBeVisible({ timeout: 15_000 });
    await expect(page.locator('[data-testid^="server-card-"]').first()).toBeVisible({
      timeout: 15_000,
    });

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('servers-page')).toBeVisible();
    await expect(page.getByTestId('servers-count-summary')).toBeVisible();
  });
});
