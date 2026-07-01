import { Page, Locator } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * Spaces management page object
 */
export class SpacesPage extends BasePage {
  readonly heading: Locator;
  readonly createSpaceButton: Locator;
  readonly spaceCards: Locator;

  constructor(page: Page) {
    super(page);
    this.heading = page.getByTestId('spaces-title');
    this.createSpaceButton = page.getByTestId('create-space-btn');
    this.spaceCards = page.locator('[data-testid^="space-card-"]');
  }

  async getSpaceCount(): Promise<number> {
    return await this.spaceCards.count();
  }

  async createSpace(name: string) {
    await this.createSpaceButton.click();
    await this.page.getByTestId('create-space-name-input').fill(name);
    await this.page.getByTestId('create-space-submit-btn').click();
  }

  async deleteSpace(spaceId: string) {
    await this.page.getByTestId(`delete-space-${spaceId}`).click();
    await this.page.getByTestId('confirm-dialog-confirm').click();
  }
}
