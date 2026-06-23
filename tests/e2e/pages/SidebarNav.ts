import { Page, Locator } from '@playwright/test';

/**
 * Sidebar navigation component — all items located via stable data-testid.
 */
export class SidebarNav {
  readonly page: Page;
  readonly dashboard: Locator;
  readonly myServers: Locator;
  readonly discover: Locator;
  readonly spaces: Locator;
  readonly featureSets: Locator;
  readonly workspaces: Locator;
  readonly clients: Locator;
  readonly settings: Locator;
  readonly spaceSwitcher: Locator;
  readonly themeToggle: Locator;

  constructor(page: Page) {
    this.page = page;
    this.dashboard = page.getByTestId('nav-dashboard');
    this.myServers = page.getByTestId('nav-my-servers');
    this.discover = page.getByTestId('nav-discover');
    this.spaces = page.getByTestId('nav-spaces');
    this.featureSets = page.getByTestId('nav-featuresets');
    this.workspaces = page.getByTestId('nav-workspaces');
    this.clients = page.getByTestId('nav-clients');
    this.settings = page.getByTestId('nav-settings');
    this.spaceSwitcher = page.getByTestId('space-switcher');
    this.themeToggle = page.locator('button[title*="mode"]');
  }

  async goToDashboard() {
    await this.dashboard.click();
  }

  async goToMyServers() {
    await this.myServers.click();
  }

  async goToDiscover() {
    await this.discover.click();
  }

  async goToSpaces() {
    await this.spaces.click();
  }

  async goToFeatureSets() {
    await this.featureSets.click();
  }

  async goToWorkspaces() {
    await this.workspaces.click();
  }

  async goToClients() {
    await this.clients.click();
  }

  async goToSettings() {
    await this.settings.click();
  }

  async toggleTheme() {
    await this.themeToggle.click();
  }
}
