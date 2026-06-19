import { test, expect } from '@playwright/test';
import { DashboardPage, SpacesPage } from '../pages';

/** Opens Spaces management (not folder Workspaces bindings). */
async function goToSpaces(page: import('@playwright/test').Page) {
  await page.getByTestId('nav-spaces').click();
  await expect(page.getByTestId('spaces-page')).toBeVisible();
}

test.describe('Spaces Page', () => {
  test('should display the Workspaces heading', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    
    // Check main page heading (h1 specifically)
    await expect(page.locator('h1:has-text("Workspaces")')).toBeVisible();
  });

  test('should show space management UI', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    await expect(page.locator('h1:has-text("Workspaces")')).toBeVisible();
    
    // Page should have some content
    const content = page.locator('[class*="rounded"]');
    const count = await content.count();
    expect(count).toBeGreaterThan(0);
  });

  test('should show space details elements', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    
    // Each space should have a name
    const spaceNames = page.locator('[class*="font-medium"], [class*="font-semibold"]');
    const count = await spaceNames.count();
    
    expect(count).toBeGreaterThan(0);
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
    await expect(spaceValue).not.toHaveText('');
  });
});

test.describe('Space Management', () => {
  test('should navigate to workspaces page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    
    // Verify page loaded correctly with Workspaces h1 heading
    await expect(page.locator('h1:has-text("Workspaces")')).toBeVisible();
  });

  test('should show workspaces page content', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    await expect(page.locator('h1:has-text("Workspaces")')).toBeVisible();
    
    // Page should have content elements
    const content = page.locator('[class*="rounded"]');
    const count = await content.count();
    expect(count).toBeGreaterThan(0);
  });
});

test.describe('Space Toast Notifications', () => {
  test('should have toast container on spaces page', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const spacesPage = new SpacesPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    await expect(page.locator('h1:has-text("Workspaces")')).toBeVisible();
    
    await expect(spacesPage.toastContainer).toBeAttached();
  });

  test('should show create space modal with form', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    
    // Open create modal
    await page.getByTestId('create-space-btn').click();
    
    // Modal should be visible
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
    
    // Submit should be disabled without a name
    await expect(page.getByTestId('create-space-submit-btn')).toBeDisabled();
    
    // Type a name
    await page.getByTestId('create-space-name-input').fill('Test Space');
    await expect(page.getByTestId('create-space-submit-btn')).toBeEnabled();
  });

  // Skip in web mode - requires Tauri API
  test.skip('should show success toast on space creation', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const spacesPage = new SpacesPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    
    await page.getByTestId('create-space-btn').click();
    await page.getByTestId('create-space-name-input').fill('Test Toast Space');
    await page.getByTestId('create-space-submit-btn').click();
    
    await spacesPage.waitForToast('success');
    const toastText = await spacesPage.getToastText();
    expect(toastText).toContain('Space created');
  });

  // Removed: "Set Active" toast test — gateway routing is workspace-root-driven,
  // there is no per-Space active toggle anymore.

  // Skip in web mode - requires Tauri API
  test.skip('should show success toast on space deletion', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    const spacesPage = new SpacesPage(page);
    await dashboard.navigate();
    
    await goToSpaces(page);
    
    // Find a deletable space
    const deleteBtn = page.locator('[data-testid^="delete-space-"]').first();
    if (await deleteBtn.isVisible()) {
      page.on('dialog', dialog => dialog.accept());
      await deleteBtn.click();
      
      await spacesPage.waitForToast('success');
      const toastText = await spacesPage.getToastText();
      expect(toastText).toContain('Space deleted');
    }
  });
});
