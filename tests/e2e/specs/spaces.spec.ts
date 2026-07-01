import { test, expect } from '@playwright/test';
import { DashboardPage, SpacesPage } from '../pages';

/** Opens Spaces management via sidebar nav testid. */
async function goToSpaces(page: import('@playwright/test').Page) {
  await page.getByTestId('nav-spaces').click();
  await expect(page.getByTestId('spaces-page')).toBeVisible();
}

test.describe('Spaces Page', () => {
  test('should display the Workspaces heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await expect(page.getByTestId('spaces-title')).toBeVisible();
  });

  test('should show space management UI', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await expect(page.getByTestId('spaces-title')).toBeVisible();

    const count = await page.locator('[data-testid^="space-card-"]').count();
    expect(count).toBeGreaterThan(0);
  });

  test('should show space details elements', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await goToSpaces(page);

    const spaceCards = page.locator('[data-testid^="space-card-"]');
    expect(await spaceCards.count()).toBeGreaterThan(0);
  });
});

test.describe('Space Switcher', () => {
  test('should show current space name on dashboard', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await expect(dashboard.activeSpaceCard).toBeVisible();
    await expect(page.getByTestId('stat-active-space-value')).toBeVisible();
  });

  test('should display active space info', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    const spaceValue = page.getByTestId('stat-active-space-value');
    await expect(spaceValue).toBeVisible();
    expect((await spaceValue.textContent())?.trim()).toBeTruthy();
  });
});

test.describe('Space Management', () => {
  test('should navigate to workspaces page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await expect(page.getByTestId('spaces-title')).toBeVisible();
  });

  test('should show workspaces page content', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await expect(page.getByTestId('spaces-title')).toBeVisible();

    const count = await page.locator('[data-testid^="space-card-"]').count();
    expect(count).toBeGreaterThan(0);
  });
});

test.describe('Space Toast Notifications', () => {
  test('should have toast container on spaces page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const spacesPage = new SpacesPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await expect(page.getByTestId('spaces-title')).toBeVisible();
    await expect(spacesPage.toastContainer).toBeAttached();
  });

  test('should show create space modal with form', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await page.getByTestId('create-space-btn').click();

    await expect(page.getByTestId('create-space-modal')).toBeVisible();
    await expect(page.getByTestId('create-space-name-input')).toBeVisible();
    await expect(page.getByTestId('create-space-submit-btn')).toBeVisible();
    await expect(page.getByTestId('create-space-cancel-btn')).toBeVisible();
  });

  test('should close create space modal on cancel', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await page.getByTestId('create-space-btn').click();
    await expect(page.getByTestId('create-space-modal')).toBeVisible();

    await page.getByTestId('create-space-cancel-btn').click();
    await expect(page.getByTestId('create-space-modal')).not.toBeVisible();
  });

  test('should disable submit when name is empty', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await page.getByTestId('create-space-btn').click();

    await expect(page.getByTestId('create-space-submit-btn')).toBeDisabled();
    await page.getByTestId('create-space-name-input').fill('Test Space');
    await expect(page.getByTestId('create-space-submit-btn')).toBeEnabled();
  });

  test.skip('should show success toast on space creation', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const spacesPage = new SpacesPage(page);
    await dashboard.navigate();

    await goToSpaces(page);
    await page.getByTestId('create-space-btn').click();
    await page.getByTestId('create-space-name-input').fill('Test Toast Space');
    await page.getByTestId('create-space-submit-btn').click();

    await spacesPage.waitForToast('success');
  });

  test.skip('should show success toast on space deletion', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const spacesPage = new SpacesPage(page);
    await dashboard.navigate();

    await goToSpaces(page);

    const deleteBtn = page.locator('[data-testid^="delete-space-"]').first();
    if (await deleteBtn.isVisible()) {
      await deleteBtn.click();
      await page.getByTestId('confirm-dialog-confirm').click();
      await spacesPage.waitForToast('success');
    }
  });
});
