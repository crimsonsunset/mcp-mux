import { test, expect } from '@playwright/test';
import { DashboardPage } from '../pages';

test.describe('Navigation', () => {
  test('should load the dashboard on startup', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.heading).toBeVisible();
    await expect(dashboard.welcome).toBeVisible();
  });

  test('should navigate to settings page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-settings').click();
    await expect(page.getByTestId('settings-title')).toBeVisible();
  });

  test('should navigate to all main pages', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

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

    await page.getByTestId('nav-dashboard').click();
    await expect(dashboard.heading).toBeVisible();
  });
});
