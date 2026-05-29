import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — settings sections (ported from settings.wdio.ts TC-ST-001/006).
 */
test.describe('Admin settings page', () => {
  test('loads appearance, logs, gateway, and theme controls', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-settings').click();
    await expect(page.getByTestId('settings-startup-section')).toBeVisible();
    await expect(page.getByTestId('settings-gateway-section')).toBeVisible();
    await expect(page.getByTestId('logs-path')).toBeVisible();

    await expect(page.locator('body')).toContainText('Appearance');
    await expect(page.locator('body')).toContainText('Logs');

    const themeButtons = page.getByTestId('theme-buttons');
    await expect(themeButtons).toBeVisible();
    await expect(page.getByTestId('theme-light-btn')).toBeVisible();
    await expect(page.getByTestId('theme-dark-btn')).toBeVisible();
    await expect(page.getByTestId('theme-system-btn')).toBeVisible();
  });
});
