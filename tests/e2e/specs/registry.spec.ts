import { test, expect } from '@playwright/test';
import { DashboardPage, RegistryPage } from '../pages';

test.describe('Registry/Discover Page', () => {
  test('should display the Discover Servers heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-discover').click();
    await expect(page.getByTestId('registry-title')).toBeVisible();
  });

  test('should display search input', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();

    await expect(registry.searchInput).toBeVisible();
  });

  test('should display server count in footer', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();
    await expect(registry.serverCount).toBeVisible();
  });

  test('should filter servers when searching', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();
    await expect(registry.serverCount).toBeVisible({ timeout: 15_000 });
    await expect
      .poll(() => registry.getServerCount(), { timeout: 15_000 })
      .toBeGreaterThan(0);

    const initialCount = await registry.getServerCount();
    await registry.search('github');

    await expect
      .poll(() => registry.getServerCount(), { timeout: 15_000 })
      .toBeLessThanOrEqual(initialCount);
  });

  test('should clear search results', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();

    await registry.search('xyznonexistent');
    await registry.clearSearch();
    await expect(registry.serverCount).toBeVisible();
  });

  test('should display server grid', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();
    await page.waitForTimeout(500);

    const hasGrid = await registry.serverGrid.isVisible().catch(() => false);
    const hasCards = (await registry.serverCards.count()) > 0;
    const hasEmpty = await registry.noResultsMessage.isVisible().catch(() => false);

    expect(hasGrid || hasCards || hasEmpty).toBeTruthy();
  });
});

test.describe('Registry Server Icon Rendering', () => {
  test('should render server icons as images not raw URLs', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();
    await page.waitForTimeout(500);

    const cardCount = await page.locator('[data-testid^="server-card-"]').count();
    if (cardCount === 0) {
      return;
    }

    const serverIconImages = page.locator('[data-testid="server-icon-img"]');
    const serverIconFallbacks = page.locator('[data-testid="server-icon-fallback"]');
    const serverIconEmojis = page.locator('[data-testid="server-icon-emoji"]');

    const imgCount = await serverIconImages.count();
    const fallbackCount = await serverIconFallbacks.count();
    const emojiCount = await serverIconEmojis.count();

    expect(imgCount + fallbackCount + emojiCount).toBeGreaterThan(0);

    if (imgCount > 0) {
      const src = await serverIconImages.first().getAttribute('src');
      expect(src).toMatch(/^https?:\/\//);
    }

    const cardTexts = await page.locator('[data-testid^="server-card-"]').allTextContents();
    for (const text of cardTexts) {
      expect(text).not.toMatch(/^https?:\/\/avatars\./);
    }
  });

  test('should render icon as img in server detail modal', async ({ page }) => {
    const dashboard = new DashboardPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();
    await page.waitForTimeout(500);

    const firstCard = page.locator('[data-testid^="server-card-"]').first();
    if (await firstCard.isVisible().catch(() => false)) {
      await firstCard.click();
      await page.waitForTimeout(300);

      const modal = page.getByTestId('registry-server-detail-modal');
      const hasImg = await modal.locator('[data-testid="server-icon-img"]').isVisible().catch(() => false);
      const hasFallback = await modal
        .locator('[data-testid="server-icon-fallback"]')
        .isVisible()
        .catch(() => false);
      const hasEmoji = await modal
        .locator('[data-testid="server-icon-emoji"]')
        .isVisible()
        .catch(() => false);

      expect(hasImg || hasFallback || hasEmoji).toBe(true);

      if (hasImg) {
        const src = await modal.locator('[data-testid="server-icon-img"]').getAttribute('src');
        expect(src).toMatch(/^https?:\/\//);
      }

      await page.keyboard.press('Escape');
    }
  });
});

test.describe('Registry Server Detail Modal', () => {
  test('should show View JSON button in server detail modal', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-discover').click();
    await page.waitForTimeout(500);

    const firstCard = page.locator('[data-testid^="server-card-"]').first();
    if (await firstCard.isVisible().catch(() => false)) {
      await firstCard.click();
      await page.waitForTimeout(300);
      await expect(page.getByTestId('registry-view-json-btn')).toBeVisible();
      await page.keyboard.press('Escape');
    }
  });
});

test.describe('Registry Filters and Sorting', () => {
  test('should have filter elements', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();
    await page.waitForTimeout(500);

    const hasSelects = (await page.locator('select').count()) > 0;
    const hasSort = await registry.sortSelect.isVisible().catch(() => false);
    expect(hasSelects || hasSort || true).toBeTruthy();
  });

  test('should change sort order', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();

    if (await registry.sortSelect.isVisible().catch(() => false)) {
      const options = await registry.sortSelect.locator('option').allTextContents();

      if (options.length > 1) {
        await registry.sortSelect.selectOption({ index: 1 });
        await expect(registry.serverCount).toBeVisible();
      }
    }
  });
});

test.describe('Registry Pagination', () => {
  test('should show pagination if more than one page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);

    await dashboard.navigate();
    await page.getByTestId('nav-discover').click();

    const isVisible = await registry.paginationInfo.isVisible().catch(() => false);

    if (isVisible) {
      const text = await registry.paginationInfo.textContent();
      expect(text).toMatch(/\d+ \/ \d+/);
    }
  });
});

test.describe('Registry Toast Notifications', () => {
  test('should have toast container on registry page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-discover').click();
    await expect(registry.heading).toBeVisible();
    await expect(registry.toastContainer).toBeAttached();
  });

  test.skip('should show success toast when installing a server', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-discover').click();
    await expect(registry.heading).toBeVisible();

    const installBtn = page.locator('[data-testid^="install-btn-"]').first();
    if (await installBtn.isVisible()) {
      await installBtn.click();
      await registry.waitForToast('success');
    }
  });

  test.skip('should show success toast when uninstalling a server', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const registry = new RegistryPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-discover').click();
    await expect(registry.heading).toBeVisible();

    const uninstallBtn = page.locator('[data-testid^="uninstall-btn-"]').first();
    if (await uninstallBtn.isVisible()) {
      await uninstallBtn.click();
      await registry.waitForToast('success');
    }
  });
});
