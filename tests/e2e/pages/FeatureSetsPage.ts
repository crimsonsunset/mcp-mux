import { Page, Locator } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * FeatureSets page object for managing permission bundles
 */
export class FeatureSetsPage extends BasePage {
  readonly heading: Locator;
  readonly createButton: Locator;
  readonly featureSetCards: Locator;
  readonly pageRoot: Locator;

  constructor(page: Page) {
    super(page);
    this.pageRoot = page.getByTestId('featuresets-page');
    this.heading = page.getByTestId('featuresets-title');
    this.createButton = page.getByTestId('featuresets-create-btn');
    this.featureSetCards = page.locator('[data-testid^="featureset-card-"]');
  }

  async createFeatureSet(name: string, description?: string) {
    await this.createButton.click();
    await this.page.getByTestId('featuresets-create-modal').locator('input').first().fill(name);
    if (description) {
      await this.page.getByTestId('featuresets-create-modal').locator('input').nth(1).fill(description);
    }
    await this.page.getByTestId('featuresets-create-submit-btn').click();
  }

  async deleteFeatureSet(featureSetId: string) {
    await this.page.getByTestId(`featureset-card-${featureSetId}`).click();
    await this.page.getByTestId('featureset-panel-delete').click();
    await this.page.getByTestId('confirm-dialog-confirm').click();
  }
}
