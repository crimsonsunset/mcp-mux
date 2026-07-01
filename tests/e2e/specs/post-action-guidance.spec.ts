import { test, expect, type Page } from '@playwright/test';
import { DashboardPage, RegistryPage } from '../pages';

/**
 * Add Server trigger in the empty-state panel (toolbar renders a duplicate testid).
 */
function emptyStateAddServerTrigger(page: Page) {
  return page.getByTestId('servers-empty-state').getByTestId('add-server-menu-trigger');
}

test.describe('Post-Action User Guidance', () => {
  test.describe('My Servers empty state', () => {
    test('should show Discover MCP Servers button when no servers installed', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-my-servers').click();
      await expect(page.getByTestId('servers-title')).toBeVisible();

      const emptyState = page.getByTestId('servers-empty-state');
      if (!(await emptyState.isVisible().catch(() => false))) {
        test.skip(true, 'Servers already installed in this environment');
      }

      const addServerTrigger = emptyStateAddServerTrigger(page);
      await expect(addServerTrigger).toBeVisible();
      await addServerTrigger.click();
      await expect(page.getByTestId('add-server-option-discover')).toBeVisible();
    });

    test('should navigate to Discover page when clicking Discover button in empty state', async ({
      page,
    }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-my-servers').click();

      const emptyState = page.getByTestId('servers-empty-state');
      if (!(await emptyState.isVisible().catch(() => false))) {
        test.skip(true, 'Servers already installed in this environment');
      }

      const addServerTrigger = emptyStateAddServerTrigger(page);
      await addServerTrigger.click();
      await page.getByTestId('add-server-option-discover').click();

      await expect(page.getByTestId('registry-title')).toBeVisible();
    });
  });

  test.describe('Registry post-install toast', () => {
    test('should have toast container on registry page for install guidance', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const registry = new RegistryPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-discover').click();
      await expect(registry.heading).toBeVisible();
      await expect(registry.toastContainer).toBeAttached();
    });

    test.skip('should show toast with Go to My Servers action after installing', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-discover').click();

      const installBtn = page.locator('[data-testid^="install-btn-"]').first();
      if (await installBtn.isVisible()) {
        await installBtn.click();
        await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 5000 });
        await expect(page.getByTestId('toast-action')).toBeVisible();
      }
    });

    test.skip('should navigate to My Servers when clicking toast action after install', async ({
      page,
    }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-discover').click();

      const installBtn = page.locator('[data-testid^="install-btn-"]').first();
      if (await installBtn.isVisible()) {
        await installBtn.click();
        await expect(page.getByTestId('toast-action')).toBeVisible({ timeout: 5000 });
        await page.getByTestId('toast-action').click();
        await expect(page.getByTestId('servers-title')).toBeVisible();
      }
    });
  });

  test.describe('OAuth consent post-approval guidance', () => {
    test.skip('should show success state with Open Workspaces button after approval', async ({
      page,
    }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      const openWorkspacesBtn = page.getByTestId('go-to-workspaces-btn');
      await expect(openWorkspacesBtn).toBeVisible();
    });
  });
});
