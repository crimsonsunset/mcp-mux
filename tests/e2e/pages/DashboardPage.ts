import { Page, Locator } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * Dashboard page object
 */
export class DashboardPage extends BasePage {
  readonly heading: Locator;
  readonly gatewayStatus: Locator;
  readonly gatewayToggleButton: Locator;
  readonly serverCountCard: Locator;
  readonly featureSetsCard: Locator;
  readonly clientsCard: Locator;
  readonly activeSpaceCard: Locator;
  readonly workspacesCard: Locator;
  readonly clientGrid: Locator;

  constructor(page: Page) {
    super(page);
    this.heading = page.getByRole('heading', { name: 'Dashboard' });
    // Test ids come from ConnectionCard and DashboardStatCards.
    this.gatewayStatus = page.getByTestId('connection-status-text');
    this.gatewayToggleButton = page.getByTestId('gateway-toggle-btn');
    this.serverCountCard = page.getByTestId('stat-servers');
    this.featureSetsCard = page.getByTestId('stat-featuresets');
    this.clientsCard = page.getByTestId('stat-clients');
    this.activeSpaceCard = page.getByTestId('stat-active-space');
    this.workspacesCard = page.getByTestId('stat-workspaces');
    this.clientGrid = page.getByTestId('client-grid');
  }

  async navigate() {
    await this.goto('/');
    await this.waitForLoad();
  }

  async isGatewayRunning(): Promise<boolean> {
    const text = await this.gatewayStatus.textContent();
    return text?.includes('running') ?? false;
  }

  async toggleGateway() {
    await this.gatewayToggleButton.click();
    // Wait for status to change
    await this.page.waitForTimeout(500);
  }

  async getServerCount(): Promise<string> {
    return (await this.page.getByTestId('stat-servers-value').textContent()) || '0/0';
  }

  async copyConfig() {
    // Open JSON config popover and click copy
    await this.page.getByTestId('client-icon-copy-config').click();
    await this.page.getByTestId('copy-config-btn').click();
  }
}
