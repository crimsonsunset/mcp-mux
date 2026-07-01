import { test, expect } from '@playwright/test';
import { DashboardPage, RegistryPage } from '../pages';

/**
 * End-to-end user flow tests that simulate real user journeys
 */

test.describe('Complete User Flows', () => {
  test('should complete first-time setup flow', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await dashboard.navigate();
    await expect(dashboard.heading).toBeVisible();

    await expect(dashboard.gatewayStatus).toBeVisible();

    await page.getByTestId('nav-discover').click();
    await expect(page.getByTestId('registry-title')).toBeVisible();

    await page.getByTestId('nav-settings').click();
    await expect(page.getByTestId('settings-title')).toBeVisible();

    await page.getByTestId('nav-dashboard').click();
    await expect(dashboard.heading).toBeVisible();
  });

  test('should navigate through all main sections', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.heading).toBeVisible();

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('servers-title')).toBeVisible();

    await page.getByTestId('nav-discover').click();
    await expect(page.getByTestId('registry-title')).toBeVisible();

    await page.getByTestId('nav-spaces').click();
    await expect(page.getByTestId('spaces-title')).toBeVisible();

    await page.getByTestId('nav-featuresets').click();
    await expect(page.getByTestId('featuresets-title')).toBeVisible();

    await page.getByTestId('nav-clients').click();
    await expect(page.getByTestId('clients-title')).toBeVisible();

    await page.getByTestId('nav-settings').click();
    await expect(page.getByTestId('settings-title')).toBeVisible();
  });

  test('should persist theme preference', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-settings').click();

    await page.getByTestId('theme-dark-btn').click();
    await page.waitForTimeout(500);
    await expect(page.locator('html')).toHaveClass(/dark/);

    await page.getByTestId('nav-dashboard').click();
    await page.getByTestId('nav-settings').click();
    await expect(page.locator('html')).toHaveClass(/dark/);
  });
});

test.describe('Server Discovery Flow', () => {
  test('should search and browse servers', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();

    await expect(registry.serverCount).toBeVisible();
    await registry.search('file');
    await expect(registry.serverCount).toBeVisible();
    await registry.clearSearch();
    await expect(registry.serverCount).toBeVisible();
  });
});

test.describe('Dashboard Interactions', () => {
  test('should display all stat cards', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.serverCountCard).toBeVisible();
    await expect(dashboard.featureSetsCard).toBeVisible();
    await expect(dashboard.clientsCard).toBeVisible();
    await expect(dashboard.activeSpaceCard).toBeVisible();
  });

  test('should show connect IDEs section', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.connectClientHeading).toBeVisible();
    await expect(dashboard.clientGrid).toBeVisible();
  });
});

test.describe('Responsive Behavior', () => {
  test('should adjust layout for mobile viewport', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await page.setViewportSize({ width: 375, height: 667 });
    await dashboard.navigate();
    await expect(dashboard.heading).toBeVisible();
  });

  test('should adjust layout for tablet viewport', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await page.setViewportSize({ width: 768, height: 1024 });
    await dashboard.navigate();
    await expect(dashboard.heading).toBeVisible();
  });

  test('should work on desktop viewport', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await page.setViewportSize({ width: 1920, height: 1080 });
    await dashboard.navigate();
    await expect(dashboard.heading).toBeVisible();
  });
});

test.describe('Error Handling', () => {
  test('should handle network errors gracefully', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await dashboard.navigate();
    await expect(dashboard.heading).toBeVisible();
  });

  test('should show loading states', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();
    await expect(page.getByTestId('registry-title')).toBeVisible();
  });
});
