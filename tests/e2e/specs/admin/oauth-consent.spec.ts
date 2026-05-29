import { test, expect } from '@playwright/test';

/**
 * Admin OAuth consent — SSE trigger + HTTP consent endpoints (web admin path).
 *
 * Full authorize→token flow is covered in Rust integration tests; this spec
 * verifies the web UI listens on SSE and the consent REST surface responds.
 */
test.describe('Admin OAuth consent', () => {
  test('SSE consent event opens modal and pending fetch fails without gateway', async ({
    page,
    request,
  }) => {
    test.skip(
      !process.env.MCPMUX_ADMIN_TEST,
      'Set MCPMUX_ADMIN_TEST=1 on admin server for publish helper'
    );

    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    await request.post('/api/v1/test/events/publish', {
      data: {
        channel: 'oauth-consent-request',
        payload: { requestId: 'playwright-consent-req' },
      },
    });

    await expect(page.getByText('Authorization Failed')).toBeVisible({ timeout: 15_000 });
    await expect(
      page.getByText(/gateway service is not running|not found|expired/i)
    ).toBeVisible();
  });

  test('consent approve POST requires CSRF token', async ({ request }) => {
    const resp = await request.post('/api/v1/oauth/consent/approve', {
      data: {
        request_id: 'any',
        consent_token: 'any',
      },
    });
    expect(resp.status()).toBe(403);
  });
});
