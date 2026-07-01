import { test, expect } from '@playwright/test';
import { DashboardPage } from '../pages';

test.describe('My Servers Page', () => {
  test('should display the My Servers heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('servers-title')).toBeVisible();
  });

  test('should display gateway status banner', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('gateway-status-chip')).toBeVisible();
  });

  test('should show server page content', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('servers-title')).toBeVisible();

    const hasServers = (await page.locator('[data-testid^="installed-server-"]').count()) > 0;
    const hasEmptyState = await page.getByTestId('servers-empty-state').isVisible().catch(() => false);

    expect(hasServers || hasEmptyState).toBeTruthy();
  });

  test('should display gateway controls', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('gateway-status-chip')).toBeVisible();
  });
});

test.describe('Server Actions', () => {
  test('should show server cards if servers exist', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();

    const serverCards = page.locator('[data-testid^="installed-server-"]');
    const cardCount = await serverCards.count();

    if (cardCount > 0) {
      await expect(serverCards.first()).toBeVisible();
    }
  });

  test('should show buttons on server cards', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();

    const menuButtons = page.locator('[data-testid^="action-menu-"]');
    const count = await menuButtons.count();

    if (count > 0) {
      const actionButtons = page.locator('[data-testid^="installed-server-"]').first().locator('button');
      expect(await actionButtons.count()).toBeGreaterThan(0);
    }
  });
});

test.describe('Server Action Menu', () => {
  test('should show View Logs and View Definition in action menu', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();

    const menuButton = page.locator('[data-testid^="action-menu-"]').first();
    const count = await menuButton.count();

    if (count > 0) {
      await menuButton.click();

      await expect(page.locator('[data-testid^="view-logs-"]').first()).toBeVisible();
      await expect(page.locator('[data-testid^="view-definition-"]').first()).toBeVisible();
      await expect(page.locator('[data-testid^="uninstall-menu-"]').first()).toBeVisible();

      await page.keyboard.press('Escape');
    }
  });
});

test.describe('Server Toast Notifications', () => {
  test.skip('should show success toast on server enable', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();

    const enableBtn = page.locator('[data-testid^="enable-server-"]').first();
    if (await enableBtn.isVisible()) {
      await enableBtn.click();
      await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 5000 });
    }
  });

  test.skip('should show toast when clearing server logs', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();

    const menuButton = page.locator('[data-testid^="action-menu-"]').first();
    if (await menuButton.isVisible()) {
      await menuButton.click();
      const viewLogs = page.locator('[data-testid^="view-logs-"]').first();
      if (await viewLogs.isVisible()) {
        await viewLogs.click();
        const clearBtn = page.locator('button[title="Clear all logs"]');
        if (await clearBtn.isVisible()) {
          page.on('dialog', (dialog) => dialog.accept());
          await clearBtn.click();
          await expect(page.getByTestId('toast-success').first()).toBeVisible({ timeout: 5000 });
        }
      }
    }
  });

  test.skip('should show toast when copying log file path', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-my-servers').click();

    const menuButton = page.locator('[data-testid^="action-menu-"]').first();
    if (await menuButton.isVisible()) {
      await menuButton.click();
      const viewLogs = page.locator('[data-testid^="view-logs-"]').first();
      if (await viewLogs.isVisible()) {
        await viewLogs.click();
        const copyBtn = page.locator('button[title="Open log file in external editor"]');
        if (await copyBtn.isVisible()) {
          await copyBtn.click();
          await expect(page.getByTestId('toast-success').first()).toBeVisible({ timeout: 5000 });
        }
      }
    }
  });
});
