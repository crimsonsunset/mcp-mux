import { test, expect } from '@playwright/test';

import {
  attachAdminPageDiagnostics,
  waitForAdminAppReady,
  waitForServersPage,
} from './_helpers/admin-diagnostics.helpers';

/**
 * Admin web smoke — cross-page navigation subset (ported from comprehensive.wdio.ts shell).
 */
test.describe('Admin comprehensive navigation', () => {
  test('visits primary admin views in one session', async ({ page }) => {
    attachAdminPageDiagnostics(page);
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });
    await waitForAdminAppReady(page);

    const routes: Array<{ nav: string; marker: string }> = [
      { nav: 'nav-spaces', marker: 'spaces-page' },
      { nav: 'nav-my-servers', marker: 'servers-page' },
      { nav: 'nav-featuresets', marker: 'featuresets-page' },
      { nav: 'nav-workspaces', marker: 'workspaces-page' },
      { nav: 'nav-clients', marker: 'clients-page' },
      { nav: 'nav-settings', marker: 'settings-startup-section' },
    ];

    for (const { nav, marker } of routes) {
      await page.getByTestId(nav).click();
      if (nav === 'nav-my-servers') {
        await waitForServersPage(page);
      } else {
        await expect(page.getByTestId(marker)).toBeVisible({ timeout: 15_000 });
      }
    }
  });
});
