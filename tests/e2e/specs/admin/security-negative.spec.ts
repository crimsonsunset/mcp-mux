import { test, expect } from '@playwright/test';

/**
 * Negative security paths for the web admin server.
 *
 * CSRF: runs whenever admin is reachable on :45819.
 * CF Access: set `MCPMUX_ADMIN_CF_TRUST_ENABLED=1` when admin has Trust CF Access on
 * (do not set `MCPMUX_ADMIN_CF_JWT` for this spec — it uses a headerless request context).
 */
test.describe('Admin security negative paths', () => {
  test('POST without CSRF token returns 403', async ({ request }) => {
    const resp = await request.post('/api/v1/gateway/stop');
    expect(resp.status()).toBe(403);

    const body = await resp.json();
    expect(String(body.error)).toMatch(/csrf/i);
  });

  test('GET /api/v1/health returns 401 without CF JWT when CF Access trust is enabled', async ({
    playwright,
  }) => {
    test.skip(
      !process.env.MCPMUX_ADMIN_CF_TRUST_ENABLED,
      'Set MCPMUX_ADMIN_CF_TRUST_ENABLED=1 when admin trust_cf_access is on'
    );

    const ctx = await playwright.request.newContext({
      baseURL: 'http://localhost:45819',
    });

    const resp = await ctx.get('/api/v1/health');
    expect(resp.status()).toBe(401);

    const body = await resp.json();
    expect(String(body.error)).toMatch(/CF Access/i);

    await ctx.dispose();
  });
});
