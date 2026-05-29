import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — meta tools settings section (ported from meta-tools.wdio.ts TC-MT-002).
 */
test.describe('Admin meta tools settings', () => {
  test('renders meta tools section with grants and audit log', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('nav-settings').click();
    await expect(page.getByTestId('settings-meta-tools-section')).toBeVisible();
    await expect(page.getByTestId('meta-tool-grants-panel')).toBeVisible();
    await expect(page.getByTestId('meta-tool-audit-log')).toBeVisible();
  });

  test('SSE meta-tool-approval-request opens approval dialog', async ({ page, request }) => {
    test.skip(
      !process.env.MCPMUX_ADMIN_TEST,
      'Set MCPMUX_ADMIN_TEST=1 on admin server for publish helper'
    );

    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await request.post('/api/v1/test/events/publish', {
      data: {
        channel: 'meta-tool-approval-request',
        payload: {
          request_id: 'playwright-bind-req',
          client_id: 'cursor',
          payload: {
            tool_name: 'mcpmux_bind_current_workspace',
            summary: 'Bind FeatureSet android-dev to this workspace',
            diff: {
              before: ['tool_a'],
              after: ['tool_a', 'tool_b'],
              added: ['tool_b'],
              removed: [],
            },
            raw_args: { feature_set_id: 'fs-1' },
            affects_other_clients: true,
          },
          expires_at_unix_secs: Math.floor(Date.now() / 1000) + 60,
        },
      },
    });

    await expect(page.getByTestId('meta-tool-approval-dialog')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByTestId('meta-tool-approval-allow-once')).toBeVisible();
    await expect(page.getByTestId('meta-tool-approval-deny')).toBeVisible();
    await expect(page.getByTestId('meta-tool-approval-cross-client-warning')).toBeVisible();
  });
});
