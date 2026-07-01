import { Page, Locator } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * Discover/Registry page object for browsing and installing servers
 */
export class RegistryPage extends BasePage {
  readonly heading: Locator;
  readonly searchInput: Locator;
  readonly serverGrid: Locator;
  readonly serverCards: Locator;
  readonly noResultsMessage: Locator;
  readonly loadingSpinner: Locator;
  readonly serverCount: Locator;
  readonly clearFiltersButton: Locator;
  readonly offlineBadge: Locator;
  readonly sortSelect: Locator;
  readonly paginationPrev: Locator;
  readonly paginationNext: Locator;
  readonly paginationInfo: Locator;

  constructor(page: Page) {
    super(page);
    this.heading = page.getByTestId('registry-title');
    this.searchInput = page.getByTestId('search-input');
    this.serverGrid = page.getByTestId('registry-server-grid');
    this.serverCards = page.locator('[data-testid^="server-card-"]');
    this.noResultsMessage = page.getByTestId('registry-empty-state');
    this.loadingSpinner = page.locator('.animate-spin');
    this.serverCount = page.getByTestId('server-count');
    this.clearFiltersButton = page.getByTestId('registry-clear-filters');
    this.offlineBadge = page.getByTestId('registry-offline-badge');
    this.sortSelect = page.getByTestId('registry-sort-select');
    this.paginationPrev = page.getByTestId('registry-pagination-prev');
    this.paginationNext = page.getByTestId('registry-pagination-next');
    this.paginationInfo = page.getByTestId('registry-pagination-info');
  }

  async search(query: string) {
    await this.searchInput.fill(query);
    await this.page.waitForTimeout(400);
  }

  async clearSearch() {
    await this.searchInput.clear();
    await this.page.waitForTimeout(400);
  }

  async getServerCount(): Promise<number> {
    const text = await this.serverCount.textContent();
    const match = text?.match(/(\d+)/);
    return match ? parseInt(match[1], 10) : 0;
  }

  async installServer(serverId: string) {
    await this.page.getByTestId(`install-btn-${serverId}`).click();
  }

  async uninstallServer(serverId: string) {
    await this.page.getByTestId(`uninstall-btn-${serverId}`).click();
  }

  async openServerDetails(serverId: string) {
    await this.page.getByTestId(`server-card-${serverId}`).click();
  }

  async closeServerDetails() {
    await this.page.keyboard.press('Escape');
  }

  async goToNextPage() {
    await this.paginationNext.click();
  }

  async goToPreviousPage() {
    await this.paginationPrev.click();
  }
}
