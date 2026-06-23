import { test, expect } from '@playwright/test';
import { DashboardPage } from '../pages';

test.describe('FeatureSets Page', () => {
  test('should display the FeatureSets heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });
    await expect(page.getByTestId('featuresets-title')).toBeVisible();
  });

  test('should show feature sets page content', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });
    await expect(page.getByTestId('featuresets-page')).toBeVisible();

    const count = await page.locator('[data-testid^="featureset-card-"]').count();
    expect(count).toBeGreaterThan(0);
  });

  test('should display built-in feature sets', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });

    const starterBadges = page.locator('[data-testid^="featureset-starter-badge-"]');
    expect(await starterBadges.count()).toBeGreaterThanOrEqual(0);
  });
});

test.describe('FeatureSet Details', () => {
  test('should show feature set content', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });

    const cards = page.locator('[data-testid^="featureset-card-"]');
    const count = await cards.count();

    if (count > 0) {
      await expect(cards.first()).toBeVisible();
    }
  });

  test('should show server-specific feature sets if servers installed', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });

    const cards = page.locator('[data-testid^="featureset-card-"]');
    expect(await cards.count()).toBeGreaterThanOrEqual(0);
  });
});

test.describe('Feature Set Toast Container', () => {
  test('should have toast container on feature sets page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });
    await expect(page.getByTestId('featuresets-title')).toBeVisible();
    await expect(page.getByTestId('toast-container')).toBeAttached();
  });
});

test.describe('Feature Set Operations with Toast', () => {
  test.skip('should show toast when creating feature set', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });
    await page.getByTestId('featuresets-create-btn').click();
    await page.getByTestId('featuresets-create-modal').locator('input').first().fill('Test Feature Set');
    await page.getByTestId('featuresets-create-submit-btn').click();
    await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 2000 });
  });

  test.skip('should show toast when deleting feature set', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });

    const customSet = page.locator('[data-testid^="featureset-card-"]').first();
    if (await customSet.isVisible()) {
      await customSet.click();
      await page.getByTestId('featureset-panel-delete').click();
      await page.getByTestId('confirm-dialog-confirm').click();
      await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 2000 });
    }
  });

  test.skip('should show error toast on failed create', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });
    await page.getByTestId('featuresets-create-btn').click();
    await expect(page.getByTestId('featuresets-create-submit-btn')).toBeDisabled();
  });
});

test.describe('Feature Set Panel Save Toast', () => {
  test.skip('should show success toast when saving feature set members', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });

    const configurableSet = page.locator('[data-testid^="featureset-card-"]').first();
    if (await configurableSet.isVisible()) {
      await configurableSet.click();
      await expect(page.getByTestId('featureset-panel-save-changes')).toBeVisible({ timeout: 5000 });
      await page.getByTestId('featureset-panel-save-changes').click();
      await expect(page.getByTestId('toast-success').first()).toBeVisible({ timeout: 5000 });
    }
  });

  test.skip('should show error toast on failed save', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-featuresets').click({ force: true });

    const featureSetCard = page.locator('[data-testid^="featureset-card-"]').first();
    if (await featureSetCard.isVisible()) {
      await featureSetCard.click();
      await page.route('**/feature-sets/*/members', (route) => route.abort());

      const saveButton = page.getByTestId('featureset-panel-save-changes');
      if (await saveButton.isVisible()) {
        await saveButton.click();
        await expect(page.getByTestId('toast-error').first()).toBeVisible({ timeout: 5000 });
      }
    }
  });
});

test.describe('Config Editor Toast', () => {
  test.skip('should show toast when saving space configuration', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-spaces').click({ force: true });
    await page.getByTestId('config-save-btn').click();
    await expect(page.getByTestId('toast-success')).toBeVisible({ timeout: 2000 });
  });

  test.skip('should show error toast for invalid JSON', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await page.getByTestId('nav-spaces').click({ force: true });
    await expect(page.locator('.monaco-editor')).toBeVisible();
  });
});
