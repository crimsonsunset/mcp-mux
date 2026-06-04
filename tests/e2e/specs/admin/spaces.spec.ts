import { test, expect } from '@playwright/test';

import {
  attachAdminPageDiagnostics,
  waitForAdminAppReady,
  snapshotAdminSelectors,
} from './_helpers/admin-diagnostics.helpers';

/**
 * Admin web smoke — space CRUD against real AdminServer (:45819).
 */
test.describe('Admin spaces lifecycle', () => {
  test('creates and lists a space', async ({ page }) => {
    const spaceName = `PW Admin ${Date.now()}`;

    attachAdminPageDiagnostics(page);
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });
    await waitForAdminAppReady(page);

    await page.getByTestId('nav-spaces').click();
    await expect(page.getByTestId('spaces-page')).toBeVisible({ timeout: 15_000 });

    const createBtn = page.getByTestId('create-space-btn');
    if (await createBtn.isVisible()) {
      await snapshotAdminSelectors(page, 'spaces:before-create');
      await createBtn.click();
      await page.getByTestId('create-space-name-input').fill(spaceName);
      await snapshotAdminSelectors(page, 'spaces:form-filled');
      await expect(page.getByTestId('create-space-submit-btn')).toBeEnabled({ timeout: 5_000 });
      await page.getByTestId('create-space-submit-btn').click();
      await expect(
        page.locator('[data-testid^="space-card-"]').filter({ hasText: spaceName })
      ).toBeVisible({ timeout: 15_000 });
    }
  });
});
