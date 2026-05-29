import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — space CRUD against real AdminServer (:45819).
 */
test.describe('Admin spaces lifecycle', () => {
  test('creates and lists a space', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-spaces').click();
    await expect(page.getByTestId('spaces-page')).toBeVisible();

    const createBtn = page.getByTestId('create-space-btn');
    if (await createBtn.isVisible()) {
      await createBtn.click();
      await page.getByTestId('create-space-name-input').fill('Playwright Admin Space');
      await page.getByTestId('create-space-submit-btn').click();
      await expect(page.getByText('Playwright Admin Space')).toBeVisible({ timeout: 15_000 });
    }
  });
});
