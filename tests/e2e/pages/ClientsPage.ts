import { Page, Locator } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * Clients page object for viewing connected AI clients
 */
export class ClientsPage extends BasePage {
  readonly heading: Locator;
  readonly pageRoot: Locator;
  readonly clientCards: Locator;
  readonly emptyState: Locator;
  readonly refreshButton: Locator;
  readonly workspacesLink: Locator;

  constructor(page: Page) {
    super(page);
    this.pageRoot = page.getByTestId('clients-page');
    this.heading = page.getByTestId('clients-title');
    this.clientCards = page.locator('[data-testid^="client-card-"]');
    this.emptyState = page.getByTestId('clients-empty-connect');
    this.refreshButton = page.getByTestId('clients-refresh-btn');
    this.workspacesLink = page.getByTestId('clients-workspaces-link');
  }

  async getClientCount(): Promise<number> {
    return await this.clientCards.count();
  }

  async revokeClient() {
    await this.page.getByTestId('client-revoke-btn').click();
    await this.page.getByTestId('confirm-dialog-confirm').click();
  }
}
