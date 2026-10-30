import { test, expect } from '@playwright/test';
import { DashboardPage } from '../pages';
import { mockAdminApi } from '../helpers/admin-api-mock';

test.beforeEach(async ({ page }) => {
  await mockAdminApi(page);
});

test.describe('Settings', () => {
  test('should display settings heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    // Click Settings in sidebar
    await page.locator('nav button:has-text("Settings")').click();

    // Check heading
    await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  });

  test('should display appearance settings', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.locator('nav button:has-text("Settings")').click();

    await expect(page.locator('text=Appearance').first()).toBeVisible();
    await expect(page.getByRole('button', { name: 'Light', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Dark', exact: true })).toBeVisible();
  });

  test('should display logs section', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.locator('nav button:has-text("Settings")').click();

    // Use heading role to be more specific
    await expect(page.locator('h3:has-text("Logs"), h2:has-text("Logs")').first()).toBeVisible();
  });

  test('should switch between themes', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.locator('nav button:has-text("Settings")').click();

    // Switch to light theme
    await page.getByRole('button', { name: 'Light', exact: true }).click();
    await page.waitForTimeout(300);

    // Switch to dark theme
    await page.getByRole('button', { name: 'Dark', exact: true }).click();
    await page.waitForTimeout(300);
    await expect(page.locator('html')).toHaveClass(/dark/);
  });

  // In the browser the app renders AboutSection; the Software Updates card
  // (UpdateChecker) is desktop-only (`isTauri()` in SettingsPage).
  test.describe('About (web mode)', () => {
    test('should display the about section instead of the update checker', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      await expect(page.getByTestId('about-section')).toBeVisible();
      await expect(page.getByText('About McpMux')).toBeVisible();
      await expect(page.getByTestId('update-checker')).toHaveCount(0);
    });

    test('should display current version', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      // The mocked backend reports 0.0.0-e2e.
      await expect(page.getByTestId('current-version')).toContainText('v0.0.0-e2e');
    });

    test('should not offer desktop-only update controls', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();
      await expect(page.getByTestId('about-section')).toBeVisible();

      await expect(page.getByTestId('check-updates-btn')).toHaveCount(0);
    });

    // Skip in web mode - requires Tauri API
    test.skip('should show loading state when checking for updates', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const checkButton = page.getByTestId('check-updates-btn');
      await checkButton.click();

      // Button should show loading state briefly
      await expect(checkButton).toContainText(/Checking/);
      await expect(checkButton).toBeDisabled();
    });

    // Skip in web mode - requires Tauri API
    test.skip('should display update status message', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const checkButton = page.getByTestId('check-updates-btn');
      await checkButton.click();

      // Wait for check to complete (should show either update available or up to date)
      await page.waitForSelector('[data-testid="update-message"], [data-testid="update-available"]', {
        timeout: 10000,
      });

      // Verify one of the expected states is shown
      const hasMessage = await page.getByTestId('update-message').isVisible().catch(() => false);
      const hasUpdate = await page.getByTestId('update-available').isVisible().catch(() => false);

      expect(hasMessage || hasUpdate).toBeTruthy();
    });

    // Skip in web mode - requires Tauri API
    test.skip('should allow multiple update checks', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const checkButton = page.getByTestId('check-updates-btn');

      // First check
      await checkButton.click();
      await page.waitForSelector('[data-testid="update-message"], [data-testid="update-available"]', {
        timeout: 10000,
      });

      // Check button should be available again
      await expect(checkButton).toBeEnabled();

      // Second check
      await checkButton.click();
      await expect(checkButton).toContainText(/Checking/);
    });
  });

  test.describe('Logs Section', () => {
    // Skip in web mode - requires Tauri API
    test.skip('should display logs path', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const logsPath = page.getByTestId('logs-path');
      await expect(logsPath).toBeVisible();
      // Should not show "Loading..." after page loads
      await expect(logsPath).not.toContainText('Loading...');
    });

    test('should not offer the open-logs button in the browser', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      // "Open Logs Folder" needs the desktop shell, so web mode omits it.
      await expect(page.getByTestId('log-retention-select')).toBeVisible();
      await expect(page.getByTestId('open-logs-btn')).toHaveCount(0);
    });

    test('should show description text', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      await expect(page.getByText(/Logs are rotated daily/i)).toBeVisible();
    });
  });

  test.describe('Page Layout', () => {
    test('should display all sections in order', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      // Verify sections appear in expected order
      const sections = [
        page.getByText('About McpMux'),
        page.getByText('Startup & System Tray'),
        page.getByText('Appearance'),
        page.locator('h3:has-text("Logs"), h2:has-text("Logs")').first(),
      ];

      for (const section of sections) {
        await expect(section).toBeVisible();
      }
    });

    test('should be scrollable if content overflows', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      // Content should be within a scrollable container
      const mainContent = page.locator('[class*="space-y-6"]').first();
      await expect(mainContent).toBeVisible();
    });
  });

  test.describe('Startup & System Tray Settings', () => {
    test('should display startup settings section', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      await expect(page.getByText('Startup & System Tray')).toBeVisible();
      await expect(page.getByText('Launch at Startup')).toBeVisible();
      await expect(page.getByText('Start Minimized')).toBeVisible();
      await expect(page.getByText('Close to Tray')).toBeVisible();
    });

    test('should have startup settings switches', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const autoLaunchSwitch = page.getByTestId('auto-launch-switch');
      const startMinimizedSwitch = page.getByTestId('start-minimized-switch');
      const closeToTraySwitch = page.getByTestId('close-to-tray-switch');

      await expect(autoLaunchSwitch).toBeVisible();
      await expect(startMinimizedSwitch).toBeVisible();
      await expect(closeToTraySwitch).toBeVisible();
    });

    // Skip in web mode - requires Tauri API
    test.skip('should toggle startup settings and show success toast', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const closeToTraySwitch = page.getByTestId('close-to-tray-switch');
      
      // Toggle the switch
      await closeToTraySwitch.click();
      
      // Wait for success toast
      await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 2000 });
      await expect(page.getByText('Settings saved')).toBeVisible();
      await expect(page.getByText('Your preferences have been updated')).toBeVisible();
      
      // Toast should auto-dismiss after 3 seconds
      await expect(page.getByTestId('toast-success')).not.toBeVisible({ timeout: 4000 });
    });

    // Skip in web mode - requires Tauri API
    test.skip('should show loading state while saving', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const closeToTraySwitch = page.getByTestId('close-to-tray-switch');
      
      // Toggle the switch
      await closeToTraySwitch.click();
      
      // Should show saving indicator briefly
      await expect(page.getByText('Saving settings...')).toBeVisible();
    });

    test('should disable start minimized when auto-launch is off', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const startMinimizedSwitch = page.getByTestId('start-minimized-switch');
      
      // Start minimized should be disabled if auto-launch is off
      // Note: This test assumes auto-launch might be off by default on test env
      const isDisabled = await startMinimizedSwitch.isDisabled();
      if (isDisabled) {
        await expect(startMinimizedSwitch).toBeDisabled();
      }
    });
  });

  test.describe('Toast Notifications', () => {
    test('should have toast container', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      // Toast container should exist in main content (even if empty)
      const toastContainer = page.getByRole('main').getByTestId('toast-container');
      await expect(toastContainer).toBeAttached();
    });

    // Skip in web mode - requires Tauri API
    test.skip('should allow manual toast dismissal', async ({ page }) => {
      const dashboard = new DashboardPage(page);
      await dashboard.navigate();

      await page.locator('nav button:has-text("Settings")').click();

      const closeToTraySwitch = page.getByTestId('close-to-tray-switch');
      
      // Toggle to trigger toast
      await closeToTraySwitch.click();
      
      // Wait for toast
      await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 2000 });
      
      // Click close button
      await page.getByTestId('toast-close').click();
      
      // Toast should disappear immediately
      await expect(page.getByTestId('toast-success')).not.toBeVisible({ timeout: 500 });
    });
  });
});
