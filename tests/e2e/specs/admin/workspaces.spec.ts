import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — workspaces page shell (ported from workspaces.wdio.ts TC-WS-001).
 */
test.describe('Admin workspaces page', () => {
  test('navigates to workspaces and shows create control', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-workspaces').click();
    await expect(page.getByTestId('workspace-binding-create-toggle')).toBeVisible();
    await expect(page.locator('body')).toContainText(/Workspaces/i);
  });
});
