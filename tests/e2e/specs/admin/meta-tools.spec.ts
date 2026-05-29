import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — meta tools settings section (ported from meta-tools.wdio.ts TC-MT-002).
 */
test.describe('Admin meta tools settings', () => {
  test('renders meta tools section with grants and audit log', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-settings').click();
    await expect(page.getByTestId('settings-meta-tools-section')).toBeVisible();
    await expect(page.getByTestId('meta-tool-grants-panel')).toBeVisible();
    await expect(page.getByTestId('meta-tool-audit-log')).toBeVisible();
  });
});
