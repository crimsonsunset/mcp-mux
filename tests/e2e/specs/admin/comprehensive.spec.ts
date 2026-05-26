import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — cross-page navigation subset (ported from comprehensive.wdio.ts shell).
 */
test.describe('Admin comprehensive navigation', () => {
  test('visits primary admin views in one session', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    const routes: Array<{ nav: string; marker: RegExp | string }> = [
      { nav: 'nav-spaces', marker: 'spaces-page' },
      { nav: 'nav-my-servers', marker: 'servers-page' },
      { nav: 'nav-featuresets', marker: 'featuresets-page' },
      { nav: 'nav-workspaces', marker: /Workspaces/i },
      { nav: 'nav-clients', marker: 'Connections' },
      { nav: 'nav-settings', marker: 'settings-startup-section' },
    ];

    for (const { nav, marker } of routes) {
      await page.getByTestId(nav).click();
      if (typeof marker === 'string' && marker.includes('-')) {
        await expect(page.getByTestId(marker)).toBeVisible();
      } else {
        await expect(page.locator('body')).toContainText(marker);
      }
    }
  });
});
