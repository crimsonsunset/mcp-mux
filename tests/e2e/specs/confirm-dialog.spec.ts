import { test, expect } from '@playwright/test';
import { DashboardPage } from '../pages';

/** Opens Spaces management (not folder Workspaces bindings). */
async function goToSpaces(page: import('@playwright/test').Page) {
  await page.getByTestId('nav-spaces').click();
  await expect(page.getByTestId('spaces-page')).toBeVisible();
}

test.describe('ConfirmDialog – Spaces', () => {
  test('should show confirm dialog when clicking delete on a non-default space', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await goToSpaces(page);
    await expect(page.getByTestId('spaces-title')).toBeVisible();

    const deleteBtn = page.locator('[data-testid^="delete-space-"]').first();
    if (await deleteBtn.isVisible().catch(() => false)) {
      await deleteBtn.click();

      await expect(page.getByTestId('confirm-dialog')).toBeVisible();
      await expect(page.getByTestId('confirm-dialog-confirm')).toBeVisible();
      await expect(page.getByTestId('confirm-dialog-cancel')).toBeVisible();
      await expect(page.getByTestId('confirm-dialog-title')).toBeVisible();
    }
  });

  test('should dismiss confirm dialog on cancel without deleting', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await goToSpaces(page);
    await expect(page.getByTestId('spaces-title')).toBeVisible();

    const deleteBtn = page.locator('[data-testid^="delete-space-"]').first();
    if (await deleteBtn.isVisible().catch(() => false)) {
      const spaceBefore = await page.locator('[data-testid^="space-card-"]').count();

      await deleteBtn.click();
      await expect(page.getByTestId('confirm-dialog')).toBeVisible();
      await page.getByTestId('confirm-dialog-cancel').click();
      await expect(page.getByTestId('confirm-dialog')).not.toBeVisible();

      const spaceAfter = await page.locator('[data-testid^="space-card-"]').count();
      expect(spaceAfter).toBe(spaceBefore);
    }
  });

  test('should dismiss confirm dialog when clicking overlay', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await goToSpaces(page);
    await expect(page.getByTestId('spaces-title')).toBeVisible();

    const deleteBtn = page.locator('[data-testid^="delete-space-"]').first();
    if (await deleteBtn.isVisible().catch(() => false)) {
      await deleteBtn.click();
      await expect(page.getByTestId('confirm-dialog')).toBeVisible();

      await page.getByTestId('confirm-dialog-overlay').click({ position: { x: 5, y: 5 } });
      await expect(page.getByTestId('confirm-dialog')).not.toBeVisible();
    }
  });
});

test.describe('ConfirmDialog – Clients', () => {
  test('should show confirm dialog when clicking Remove Client', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-clients').click();
    await expect(page.getByTestId('clients-title')).toBeVisible();

    const clientCards = page.locator('[data-testid^="client-card-"]');
    const count = await clientCards.count();

    if (count > 0) {
      await clientCards.first().click();
      await page.waitForTimeout(300);

      const removeBtn = page.getByTestId('client-revoke-btn');
      if (await removeBtn.isVisible().catch(() => false)) {
        await removeBtn.click();

        await expect(page.getByTestId('confirm-dialog')).toBeVisible();
        await expect(page.getByTestId('confirm-dialog-confirm')).toBeVisible();

        await page.getByTestId('confirm-dialog-cancel').click();
        await expect(page.getByTestId('confirm-dialog')).not.toBeVisible();

        const countAfter = await clientCards.count();
        expect(countAfter).toBe(count);
      }
    }
  });
});
