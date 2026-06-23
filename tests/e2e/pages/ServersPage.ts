import { Page, Locator } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * My Servers page object for managing installed servers
 */
export class ServersPage extends BasePage {
  readonly heading: Locator;
  readonly addServerButton: Locator;
  readonly gatewayStatus: Locator;
  readonly serverList: Locator;
  readonly emptyState: Locator;

  constructor(page: Page) {
    super(page);
    this.heading = page.getByTestId('servers-title');
    this.addServerButton = page.getByTestId('add-server-menu-trigger');
    this.gatewayStatus = page.getByTestId('gateway-status-chip');
    this.serverList = page.locator('[data-testid^="installed-server-"]');
    this.emptyState = page.getByTestId('servers-empty-state');
  }

  async isGatewayRunning(): Promise<boolean> {
    const text = await this.gatewayStatus.textContent();
    return text?.toLowerCase().includes('running') ?? false;
  }

  async getServerCards(): Promise<Locator> {
    return this.page.locator('[data-testid^="installed-server-"]');
  }

  async getServerByName(name: string): Promise<Locator> {
    return this.page.locator(`[data-testid^="installed-server-"]`).filter({ hasText: name }).first();
  }

  async openServerMenu(serverId: string) {
    await this.page.getByTestId(`action-menu-${serverId}`).click();
  }

  async viewServerLogs(serverId: string) {
    await this.openServerMenu(serverId);
    await this.page.getByTestId(`view-logs-${serverId}`).click();
  }

  async uninstallServer(serverId: string) {
    await this.openServerMenu(serverId);
    await this.page.getByTestId(`uninstall-menu-${serverId}`).click();
    await this.page.getByTestId('confirm-dialog-confirm').click();
  }
}
