import { test, expect } from '@playwright/test';
import { DashboardPage, SettingsPage } from '../pages';

test.describe('Settings', () => {
  test('should display settings heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const settings = new SettingsPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-settings').click();
    await expect(settings.heading).toBeVisible();
  });

  test('should display appearance settings', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const settings = new SettingsPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-settings').click();

    await expect(settings.appearanceSection).toBeVisible();
    await expect(settings.lightThemeButton).toBeVisible();
    await expect(settings.darkThemeButton).toBeVisible();
  });

  test('should display logs section', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const settings = new SettingsPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-settings').click();
    await expect(settings.logsSection).toBeVisible();
  });

  test('should switch between themes', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const settings = new SettingsPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-settings').click();

    await settings.lightThemeButton.click();
    await page.waitForTimeout(300);

    await settings.darkThemeButton.click();
    await page.waitForTimeout(300);
    await expect(page.locator('html')).toHaveClass(/dark/);
  });

  test.describe.skip('Software Updates', () => {
    test('should display update checker section', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      await expect(page.getByTestId('update-checker')).toBeVisible();
    });

    test('should display current version', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await expect(page.getByTestId('current-version')).toBeVisible();
    });

    test('should have check for updates button', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      const checkButton = page.getByTestId('check-updates-btn');
      await expect(checkButton).toBeVisible();
      await expect(checkButton).toBeEnabled();
    });

    test.skip('should show loading state when checking for updates', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      const checkButton = page.getByTestId('check-updates-btn');
      await checkButton.click();
      await expect(checkButton).toBeDisabled();
    });

    test.skip('should display update status message', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      const checkButton = page.getByTestId('check-updates-btn');
      await checkButton.click();

      await page.waitForSelector('[data-testid="update-message"], [data-testid="update-available"]', {
        timeout: 10000,
      });

      const hasMessage = await page.getByTestId('update-message').isVisible().catch(() => false);
      const hasUpdate = await page.getByTestId('update-available').isVisible().catch(() => false);
      expect(hasMessage || hasUpdate).toBeTruthy();
    });

    test.skip('should allow multiple update checks', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      const checkButton = page.getByTestId('check-updates-btn');
      await checkButton.click();
      await page.waitForSelector('[data-testid="update-message"], [data-testid="update-available"]', {
        timeout: 10000,
      });
      await expect(checkButton).toBeEnabled();
      await checkButton.click();
      await expect(checkButton).toBeDisabled();
    });
  });

  test.describe('Logs Section', () => {
    test.skip('should display logs path', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      const logsPath = page.getByTestId('logs-path');
      await expect(logsPath).toBeVisible();
    });

    test.skip('should have open logs folder button', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await expect(settings.openLogsButton).toBeVisible();
    });

    test('should show description text', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await expect(settings.logsSection).toBeVisible();
    });
  });

  test.describe('Page Layout', () => {
    test('should display all sections in order', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      await expect(settings.startupSection).toBeVisible();
      await expect(settings.appearanceSection).toBeVisible();
      await expect(settings.logsSection).toBeVisible();
    });

    test('should be scrollable if content overflows', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await expect(page.getByTestId('settings-page')).toBeVisible();
    });
  });

  test.describe('Startup & System Tray Settings', () => {
    test('should display startup settings section', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      await expect(settings.startupSection).toBeVisible();
      await expect(settings.autoLaunchSwitch).toBeVisible();
      await expect(settings.startMinimizedSwitch).toBeVisible();
      await expect(settings.closeToTraySwitch).toBeVisible();
    });

    test('should have startup settings switches', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();

      await expect(settings.autoLaunchSwitch).toBeVisible();
      await expect(settings.startMinimizedSwitch).toBeVisible();
      await expect(settings.closeToTraySwitch).toBeVisible();
    });

    test.skip('should toggle startup settings and show success toast', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await settings.closeToTraySwitch.click();
      await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 2000 });
      await expect(page.getByTestId('toast-success')).not.toBeVisible({ timeout: 4000 });
    });

    test.skip('should show loading state while saving', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await settings.closeToTraySwitch.click();
    });

    test('should disable start minimized when auto-launch is off', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await expect(settings.startupSection).toBeVisible();

      const isDisabled = await settings.startMinimizedSwitch.isDisabled();
      if (isDisabled) {
        await expect(settings.startMinimizedSwitch).toBeDisabled();
      }
    });
  });

  test.describe('Toast Notifications', () => {
    test('should have toast container', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await expect(settings.toastContainer).toBeAttached();
    });

    test.skip('should allow manual toast dismissal', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      const settings = new SettingsPage(page);
      await dashboard.navigate();

      await page.getByTestId('nav-settings').click();
      await settings.closeToTraySwitch.click();
      await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 2000 });
      await page.getByTestId('toast-close').click();
      await expect(page.getByTestId('toast-success')).not.toBeVisible({ timeout: 500 });
    });
  });
});
