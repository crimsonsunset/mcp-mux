import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — feature set browse against real AdminServer (:45819).
 */
test.describe('Admin feature sets browse', () => {
  test('navigates to feature sets page', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-featuresets').click();
    await expect(page.getByTestId('featuresets-page')).toBeVisible();
    await expect(page.locator('body')).toContainText(/Feature/i);
  });
});
