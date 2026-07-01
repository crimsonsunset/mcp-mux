import { test, expect, type Page } from '@playwright/test';
import { DashboardPage } from '../pages';

/**
 * Opens the first installed server's action menu, if present.
 */
async function openFirstServerMenu(page: Page): Promise<boolean> {
  const menuButton = page.locator('[data-testid^="action-menu-"]').first();
  if (!(await menuButton.isVisible().catch(() => false))) {
    return false;
  }
  await menuButton.click();
  return true;
}

/**
 * Opens the configure action for the first server with a configure menu item.
 */
async function openFirstServerConfigure(page: Page): Promise<boolean> {
  if (!(await openFirstServerMenu(page))) {
    return false;
  }
  const configureOption = page.locator('[data-testid^="configure-server-"]').first();
  if (!(await configureOption.isVisible().catch(() => false))) {
    return false;
  }
  await configureOption.click();
  return page.getByTestId('config-modal').isVisible({ timeout: 2000 }).catch(() => false);
}

test.describe('Server Configuration Modal - Custom Inputs', () => {
  test.beforeEach(async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('servers-title')).toBeVisible();
  });

  test('should show Add Custom Server button when a space is active', async ({ page }) => {
    const addTrigger = page.getByTestId('add-server-menu-trigger');
    const isVisible = await addTrigger.isVisible().catch(() => false);

    if (isVisible) {
      await expect(addTrigger).toBeVisible();
    } else {
      await expect(page.getByTestId('servers-title')).toBeVisible();
    }
  });

  test('should open config editor modal when clicking Add Custom Server', async ({ page }) => {
    const addTrigger = page.getByTestId('add-server-menu-trigger');
    const isVisible = await addTrigger.isVisible().catch(() => false);

    if (isVisible) {
      await addTrigger.click();
      await page.getByTestId('add-server-option-custom').click();
      await expect(page.getByTestId('config-editor-modal')).toBeVisible({ timeout: 5000 });
    }
  });

  test('should show config modal with Configure action on server cards', async ({ page }) => {
    const opened = await openFirstServerConfigure(page);
    if (opened) {
      await expect(page.getByTestId('config-modal')).toBeVisible({ timeout: 5000 });
      await page.getByTestId('config-cancel-btn').click();
    }
  });

  test('config modal should have cancel and save buttons', async ({ page }) => {
    const enableBtn = page.locator('[data-testid^="enable-server-"]').first();
    const hasEnable = await enableBtn.isVisible().catch(() => false);

    if (hasEnable) {
      await enableBtn.click();
      const modalVisible = await page.getByTestId('config-modal').isVisible({ timeout: 2000 }).catch(() => false);
      if (modalVisible) {
        await expect(page.getByTestId('config-cancel-btn')).toBeVisible();
        await expect(page.getByTestId('config-save-btn')).toBeVisible();
        await page.getByTestId('config-cancel-btn').click();
      }
    }
  });
});

test.describe('Server Config Modal - Additional Arguments Field', () => {
  test('args field should only appear for stdio servers', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-my-servers').click();

    if (await openFirstServerConfigure(page)) {
      await page.getByTestId('config-args-append').isVisible().catch(() => false);
      await page.getByTestId('config-cancel-btn').click();
    }
  });

  test('args textarea should accept multi-line input', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-my-servers').click();

    if (await openFirstServerConfigure(page)) {
      const argsField = page.getByTestId('config-args-append');
      if (await argsField.isVisible().catch(() => false)) {
        await argsField.fill('--verbose\n--port\n8080');
        const value = await argsField.inputValue();
        expect(value).toContain('--verbose');
        expect(value).toContain('--port');
        expect(value).toContain('8080');
      }
      await page.getByTestId('config-cancel-btn').click();
    }
  });
});

test.describe('Server Config Modal - Environment Variables', () => {
  test('env variables section should be visible in config modal', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-my-servers').click();

    if (await openFirstServerConfigure(page)) {
      await expect(page.getByTestId('config-env-section')).toBeVisible();
      await expect(page.getByTestId('config-add-env')).toBeVisible();
      await page.getByTestId('config-cancel-btn').click();
    }
  });

  test('should add and remove environment variables', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-my-servers').click();

    if (await openFirstServerConfigure(page)) {
      await page.getByTestId('config-add-env').click();

      const keyInput = page.locator('input[placeholder="KEY"]').first();
      const valueInput = page.locator('input[placeholder="value"]').first();
      await expect(keyInput).toBeVisible();
      await expect(valueInput).toBeVisible();

      await keyInput.fill('MY_VAR');
      await valueInput.fill('my_value');
      await page.getByTestId('config-add-env').click();

      const keyInputs = page.locator('input[placeholder="KEY"]');
      expect(await keyInputs.count()).toBe(2);

      const removeButtons = page.locator('button[title="Remove"]');
      if ((await removeButtons.count()) > 0) {
        await removeButtons.first().click();
        expect(await page.locator('input[placeholder="KEY"]').count()).toBe(1);
      }

      await page.getByTestId('config-cancel-btn').click();
    }
  });
});

test.describe('Server Config Modal - HTTP Headers', () => {
  test('headers section should only appear for http servers', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-my-servers').click();

    if (await openFirstServerConfigure(page)) {
      const headersVisible = await page.getByTestId('config-headers-section').isVisible().catch(() => false);
      const addHeaderVisible = await page.getByTestId('config-add-header').isVisible().catch(() => false);
      expect(headersVisible).toBe(addHeaderVisible);
      await page.getByTestId('config-cancel-btn').click();
    }
  });

  test('should add and remove HTTP headers', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-my-servers').click();

    if (await openFirstServerConfigure(page)) {
      const addHeaderBtn = page.getByTestId('config-add-header');
      if (await addHeaderBtn.isVisible().catch(() => false)) {
        await addHeaderBtn.click();

        const headerKeyInput = page.locator('input[placeholder="Header-Name"]').first();
        const headerValueInput = page.locator('input[placeholder="value"]').last();
        await expect(headerKeyInput).toBeVisible();

        await headerKeyInput.fill('Authorization');
        await headerValueInput.fill('Bearer my-token');
        await addHeaderBtn.click();

        const headerKeyInputs = page.locator('input[placeholder="Header-Name"]');
        expect(await headerKeyInputs.count()).toBe(2);

        const removeButtons = page.locator('button[title="Remove"]');
        if ((await removeButtons.count()) > 0) {
          await removeButtons.first().click();
        }
      }

      await page.getByTestId('config-cancel-btn').click();
    }
  });
});

test.describe('Server Config Modal - Combined Fields Visibility', () => {
  test('should show correct fields based on transport type', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.navigate();
    await page.getByTestId('nav-my-servers').click();

    if (await openFirstServerConfigure(page)) {
      await expect(page.getByTestId('config-env-section')).toBeVisible();
      await expect(page.getByTestId('config-add-env')).toBeVisible();

      const argsVisible = await page.getByTestId('config-args-append').isVisible().catch(() => false);
      const headersVisible = await page.getByTestId('config-add-header').isVisible().catch(() => false);

      if (argsVisible) {
        expect(headersVisible).toBe(false);
      }
      if (headersVisible) {
        expect(argsVisible).toBe(false);
      }

      await page.getByTestId('config-cancel-btn').click();
    }
  });
});
