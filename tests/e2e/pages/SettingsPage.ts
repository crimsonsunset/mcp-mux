import { Page, Locator } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * Settings page object
 */
export class SettingsPage extends BasePage {
  readonly heading: Locator;
  readonly lightThemeButton: Locator;
  readonly darkThemeButton: Locator;
  readonly systemThemeButton: Locator;
  readonly openLogsButton: Locator;
  readonly logsPath: Locator;
  readonly appearanceSection: Locator;
  readonly logsSection: Locator;
  readonly startupSection: Locator;
  readonly autoLaunchSwitch: Locator;
  readonly startMinimizedSwitch: Locator;
  readonly closeToTraySwitch: Locator;
  readonly toastContainer: Locator;

  constructor(page: Page) {
    super(page);
    this.heading = page.getByTestId('settings-title');
    this.lightThemeButton = page.getByTestId('theme-light-btn');
    this.darkThemeButton = page.getByTestId('theme-dark-btn');
    this.systemThemeButton = page.getByTestId('theme-system-btn');
    this.openLogsButton = page.getByTestId('open-logs-btn');
    this.logsPath = page.getByTestId('logs-path');
    this.appearanceSection = page.getByTestId('settings-appearance-section');
    this.logsSection = page.getByTestId('settings-logs-section');
    this.startupSection = page.getByTestId('settings-startup-section');
    this.autoLaunchSwitch = page.getByTestId('auto-launch-switch');
    this.startMinimizedSwitch = page.getByTestId('start-minimized-switch');
    this.closeToTraySwitch = page.getByTestId('close-to-tray-switch');
    this.toastContainer = page.getByTestId('toast-container');
  }

  async selectTheme(theme: 'light' | 'dark' | 'system') {
    switch (theme) {
      case 'light':
        await this.lightThemeButton.click();
        break;
      case 'dark':
        await this.darkThemeButton.click();
        break;
      case 'system':
        await this.systemThemeButton.click();
        break;
    }
  }

  async getActiveTheme(): Promise<string> {
    if (await this.lightThemeButton.getAttribute('class').then((c) => c?.includes('primary'))) {
      return 'light';
    }
    if (await this.darkThemeButton.getAttribute('class').then((c) => c?.includes('primary'))) {
      return 'dark';
    }
    return 'system';
  }

  async waitForToast(type: 'success' | 'error' | 'warning' | 'info', timeout = 5000) {
    await this.page.getByTestId(`toast-${type}`).waitFor({ timeout });
  }

  async getToastText() {
    const toast = this.toastContainer.locator('[role="alert"]').first();
    return toast.textContent();
  }

  async closeToast() {
    await this.page.getByTestId('toast-close').first().click();
  }
}
