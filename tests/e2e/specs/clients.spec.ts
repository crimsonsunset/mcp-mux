import { test, expect } from '@playwright/test';
import { DashboardPage, ClientsPage } from '../pages';
import { mockAdminApi } from '../helpers/admin-api-mock';

test.beforeEach(async ({ page }) => {
  await mockAdminApi(page);
});

test.describe('Connections Page', () => {
  test('should display the Connections heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();

    // Click Clients in sidebar
    await page.locator('nav button:has-text("Clients")').click();

    await expect(clients.heading).toBeVisible();
    await expect(clients.heading).toHaveText('Connections');
  });

  test('should describe that routing lives in Workspaces', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.locator('nav button:has-text("Clients")').click();

    // Routing is configured in Projects (workspace bindings), not per-client.
    // The callout lives in the side panel, so open a card first.
    await page.locator('[data-testid^="client-card-"]').first().click();
    await expect(page.getByText('Routing is workspace-driven')).toBeVisible();
    await expect(page.getByTestId('client-open-workspaces-btn')).toBeVisible();
  });

  test('should show description text', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.locator('nav button:has-text("Clients")').click();

    await expect(page.getByText(/Approved AI clients\./)).toBeVisible();
  });

  test('should show the client list, not the empty state', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.locator('nav button:has-text("Clients")').click();

    // The mock registers one approved client.
    await expect(page.locator('[data-testid^="client-card-"]')).toHaveCount(1);
    await expect(page.getByText("Let's hook up your first IDE")).toHaveCount(0);
  });

  test('should display the client card with its name', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.locator('nav button:has-text("Clients")').click();

    const card = page.getByTestId('client-card-client-e2e');
    await expect(card).toBeVisible();
    await expect(card).toContainText('E2E Client');
  });
});

test.describe('Connection Details', () => {
  test('should show last-seen indicator on connection cards', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.locator('nav button:has-text("Clients")').click();

    // Each card surfaces "Last seen …" — pure observability (no routing bits).
    const firstCard = page.locator('[data-testid^="client-card-"]').first();
    await expect(firstCard).toBeVisible();
    await expect(firstCard).toContainText(/Last seen/);
  });

  test('should route routing config to Workspaces from the side panel', async ({
    page,
  }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.locator('nav button:has-text("Clients")').click();

    await page.locator('[data-testid^="client-card-"]').first().click();

    // The side panel's "routing is workspace-driven" callout exposes a
    // button that sends the user to Projects.
    await expect(page.getByTestId('client-open-workspaces-btn')).toBeVisible();

    // Legacy per-client controls MUST NOT be present any more.
    await expect(page.locator('text=Quick Settings')).toHaveCount(0);
    await expect(page.locator('text=Connection Mode')).toHaveCount(0);
    await expect(page.locator('text=Effective Features')).toHaveCount(0);
    await expect(page.locator('text=Advanced Permissions')).toHaveCount(0);
  });
});

test.describe('Connection lifecycle', () => {
  test('should have refresh button if available', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.locator('nav button:has-text("Clients")').click();

    const refreshButton = page.getByRole('button', { name: /Refresh/ });
    // Always rendered on the Connections header.
    await expect(refreshButton).toBeVisible();
  });
});

test.describe('Connections toast container', () => {
  test('should have toast container on Connections page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();

    await page.locator('nav button:has-text("Clients")').click();
    await expect(clients.heading).toBeVisible();

    await expect(clients.toastContainer).toBeAttached();
  });

  // Skip in web mode - requires Tauri API for the save-alias command.
  test.skip('should toast on display-name save', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();

    await page.locator('nav button:has-text("Clients")').click();

    const clientCards = page.locator('[data-testid^="client-card-"]');
    const count = await clientCards.count();

    if (count > 0) {
      await clientCards.first().click();

      // Type into the display-name input and hit save.
      const aliasInput = page.getByPlaceholder(/./).first();
      await aliasInput.fill('New Alias');
      await page.getByRole('button', { name: /Save/ }).click();

      await clients.waitForToast('success');
      expect(await clients.getToastText()).toMatch(/Saved/);
    }
  });

  // Skip in web mode - requires Tauri API for revoke.
  test.skip('should toast on revoke', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const clients = new ClientsPage(page);
    await dashboard.navigate();

    await page.locator('nav button:has-text("Clients")').click();

    const clientCards = page.locator('[data-testid^="client-card-"]');
    const count = await clientCards.count();

    if (count > 0) {
      await clientCards.first().click();

      page.on('dialog', (dialog) => dialog.accept());
      await page.getByRole('button', { name: /Revoke connection/ }).click();
      // Confirm dialog
      await page.getByRole('button', { name: /Revoke/ }).click();

      await clients.waitForToast('success');
      expect(await clients.getToastText()).toMatch(/revoked/);
    }
  });
});
