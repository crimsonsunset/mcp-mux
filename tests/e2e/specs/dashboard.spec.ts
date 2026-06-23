import { test, expect } from '@playwright/test';
import { DashboardPage } from '../pages';

test.describe('Dashboard', () => {
  test('should display gateway status', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.gatewayStatus).toBeVisible();
    await expect(dashboard.gatewayToggleButton).toBeVisible();
  });

  test('should display stats cards', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.serverCountCard).toBeVisible();
    await expect(dashboard.featureSetsCard).toBeVisible();
    await expect(dashboard.clientsCard).toBeVisible();
    await expect(dashboard.activeSpaceCard).toBeVisible();
  });

  test('should display connect IDEs section', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(page.getByTestId('gateway-status-card')).toBeVisible();
    await expect(dashboard.connectClientHeading).toBeVisible();
    await expect(dashboard.clientGrid).toBeVisible();
  });

  test('should copy config via JSON button', async ({ page, context, browserName }) => {
    test.skip(browserName !== 'chromium', 'Clipboard permissions not supported');

    await context.grantPermissions(['clipboard-read', 'clipboard-write']);

    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.clientGrid).toBeVisible();
    await page.getByTestId('client-icon-copy-config').first().scrollIntoViewIfNeeded();
    await page.getByTestId('client-icon-copy-config').first().click();
    await page.getByTestId('copy-config-btn').click();

    await expect(page.getByTestId('connection-url-copied')).toBeVisible({ timeout: 5000 });
  });
});
