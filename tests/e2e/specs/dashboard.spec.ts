import { test, expect } from '@playwright/test';
import { DashboardPage } from '../pages';
import { mockAdminApi } from '../helpers/admin-api-mock';

test.beforeEach(async ({ page }) => {
  await mockAdminApi(page);
});

test.describe('Dashboard', () => {
  test('should display gateway status', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.gatewayStatus).toBeVisible();
    await expect(dashboard.gatewayStatus).toHaveText('Gateway running');
    await expect(dashboard.gatewayToggleButton).toBeVisible();
    // Gateway is mocked as running, so the button offers Stop.
    await expect(dashboard.gatewayToggleButton).toHaveText('Stop');
  });

  test('should display stats cards', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.serverCountCard).toBeVisible();
    await expect(dashboard.featureSetsCard).toBeVisible();
    await expect(dashboard.clientsCard).toBeVisible();
    await expect(dashboard.activeSpaceCard).toBeVisible();
    await expect(dashboard.workspacesCard).toBeVisible();
  });

  test('should show counts from the mocked backend', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    // One client, no servers or bundles in the fixtures.
    await expect(page.getByTestId('stat-clients-value')).toHaveText('1');
    await expect(page.getByTestId('stat-servers-value')).toHaveText('0/0');
    await expect(page.getByTestId('stat-featuresets-value')).toHaveText('0');
    await expect(page.getByTestId('stat-active-space-value')).toContainText('Personal');
  });

  test('should display the IDE connect grid', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(page.getByTestId('client-grid')).toBeVisible();
    await expect(page.getByTestId('client-icon-copy-config')).toBeVisible();
  });

  test('should copy config via JSON button', async ({ page, context, browserName }) => {
    // Clipboard permissions only work on Chromium
    test.skip(browserName !== 'chromium', 'Clipboard permissions not supported');

    await context.grantPermissions(['clipboard-read', 'clipboard-write']);

    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await dashboard.copyConfig();

    await expect(page.getByTestId('copy-config-copied')).toBeVisible({ timeout: 2000 });
  });
});
