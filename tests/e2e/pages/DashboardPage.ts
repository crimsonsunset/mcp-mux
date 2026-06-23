import { Page, Locator } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * Dashboard/Home page object
 */
export class DashboardPage extends BasePage {
  readonly heading: Locator;
  readonly welcome: Locator;
  readonly gatewayStatus: Locator;
  readonly gatewayToggleButton: Locator;
  readonly serverCountCard: Locator;
  readonly featureSetsCard: Locator;
  readonly clientsCard: Locator;
  readonly activeSpaceCard: Locator;
  readonly connectClientHeading: Locator;
  readonly clientGrid: Locator;

  constructor(page: Page) {
    super(page);
    this.heading = page.getByTestId('dashboard-title');
    this.welcome = page.getByTestId('dashboard-welcome');
    this.gatewayStatus = page.getByTestId('connection-status-text');
    this.gatewayToggleButton = page.getByTestId('gateway-toggle-btn');
    this.serverCountCard = page.getByTestId('stat-servers');
    this.featureSetsCard = page.getByTestId('stat-featuresets');
    this.clientsCard = page.getByTestId('stat-clients');
    this.activeSpaceCard = page.getByTestId('stat-active-space');
    this.connectClientHeading = page.getByTestId('connect-client-heading');
    this.clientGrid = page.getByTestId('client-grid');
  }

  async navigate() {
    await this.goto('/');
  }

  async isGatewayRunning(): Promise<boolean> {
    const text = await this.gatewayStatus.textContent();
    return text?.toLowerCase().includes('running') ?? false;
  }

  async toggleGateway() {
    await this.gatewayToggleButton.click();
    await this.page.waitForTimeout(500);
  }

  async getServerCount(): Promise<string> {
    return (await this.page.getByTestId('stat-servers-value').textContent()) || '0/0';
  }

  async copyConfig() {
    await this.page.getByTestId('client-icon-copy-config').click();
    await this.page.getByTestId('copy-config-btn').click();
  }
}
