import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — connections page (ported from clients.wdio.ts TC-CL-001).
 */
test.describe('Admin connections page', () => {
  test('shows Connections heading and Workspaces routing hint', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-clients').click();
    await expect(page.locator('body')).toContainText('Connections');
    await expect(page.locator('body')).toContainText('Workspaces');
  });
});
