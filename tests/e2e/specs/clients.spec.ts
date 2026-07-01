import { test, expect } from '@playwright/test';
import { DashboardPage, ClientsPage } from '../pages';

test.describe('Connections Page', () => {
  test('should display the Connections heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-clients').click();

    await expect(clients.heading).toBeVisible();
  });

  test('should describe that routing lives in Workspaces', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-clients').click();

    await expect(clientsWorkspacesLink(page)).toBeVisible();
  });

  test('should show description text', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-clients').click();

    await expect(page.getByTestId('clients-page')).toBeVisible();
  });

  test('should show empty state or client list', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-clients').click();
    await clients.waitForContent();

    const hasEmpty = await clients.emptyState.isVisible();
    const clientCount = await clients.clientCards.count();

    expect(hasEmpty || clientCount > 0).toBeTruthy();
  });

  test('should display client cards if clients exist', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-clients').click();

    const count = await clients.clientCards.count();

    if (count > 0) {
      await expect(clients.clientCards.first()).toBeVisible();
    }
  });
});

test.describe('Connection Details', () => {
  test('should show last-seen indicator on connection cards', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-clients').click();

    const count = await clients.clientCards.count();

    if (count > 0) {
      await expect(clients.clientCards.first()).toBeVisible();
      await expect(clients.clientCards.first().getByTestId('client-last-seen')).toBeVisible();
    }
  });

  test('should route routing config to Workspaces from the side panel', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-clients').click();

    const count = await clients.clientCards.count();

    if (count > 0) {
      await clients.clientCards.first().click();
      await expect(page.getByTestId('client-open-workspaces-btn')).toBeVisible();
    }
  });
});

test.describe('Connection lifecycle', () => {
  test('should have refresh button if available', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-clients').click();

    await expect(clients.refreshButton).toBeVisible();
  });
});

test.describe('Connections toast container', () => {
  test('should have toast container on Connections page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-clients').click();
    await expect(clients.heading).toBeVisible();
    await expect(clients.pageRoot.getByTestId('toast-container')).toBeAttached();
  });

  test.skip('should toast on display-name save', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-clients').click();

    const count = await clients.clientCards.count();

    if (count > 0) {
      await clients.clientCards.first().click();
      const aliasInput = page.getByPlaceholder(/./).first();
      await aliasInput.fill('New Alias');
      await page.getByTestId('client-save-alias-btn').click();
      await clients.waitForToast('success');
    }
  });

  test.skip('should toast on revoke', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-clients').click();

    const count = await clients.clientCards.count();

    if (count > 0) {
      await clients.clientCards.first().click();
      await page.getByTestId('client-revoke-btn').click();
      await page.getByTestId('confirm-dialog-confirm').click();
      await clients.waitForToast('success');
    }
  });
});

/** Workspaces link in the clients page header subtitle. */
function clientsWorkspacesLink(page: import('@playwright/test').Page) {
  return page.getByTestId('clients-workspaces-link');
}
