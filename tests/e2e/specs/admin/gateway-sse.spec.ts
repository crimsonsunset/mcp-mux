import { test, expect } from '@playwright/test';

/**
 * Admin SSE smoke — gateway status chip updates live without page refresh.
 *
 * Requires admin server with `MCPMUX_ADMIN_TEST=1` for the publish helper.
 */
test.describe('Admin gateway SSE smoke', () => {
  test('gateway status updates via SSE without refresh', async ({ page, request }) => {
    test.skip(
      !process.env.MCPMUX_ADMIN_TEST,
      'Set MCPMUX_ADMIN_TEST=1 on admin server for publish helper'
    );

    await page.goto('/');

    await page.getByTestId('nav-my-servers').click();
    await expect(page.getByTestId('servers-page')).toBeVisible({ timeout: 30_000 });

    const chip = page.getByTestId('gateway-status-chip');
    await expect(chip).toBeVisible();
    await expect(chip).toContainText(/Gateway (Running|Stopped)/);

    await page.evaluate(() => {
      (window as unknown as { __gatewayEvents: string[] }).__gatewayEvents = [];
      const source = new EventSource('/api/v1/events');
      source.addEventListener('gateway-changed', (event: MessageEvent) => {
        (window as unknown as { __gatewayEvents: string[] }).__gatewayEvents.push(event.data);
      });
      (window as unknown as { __gatewaySse: EventSource }).__gatewaySse = source;
    });

    await request.post('/api/v1/test/events/publish', {
      data: {
        channel: 'gateway-changed',
        payload: {
          action: 'started',
          url: 'http://127.0.0.1:45818',
          port: 45818,
        },
      },
    });

    await expect
      .poll(async () =>
        page.evaluate(
          () => (window as unknown as { __gatewayEvents: string[] }).__gatewayEvents.length
        )
      )
      .toBeGreaterThan(0);

    await expect(chip).toContainText('Gateway Running');

    await request.post('/api/v1/test/events/publish', {
      data: {
        channel: 'gateway-changed',
        payload: { action: 'stopped' },
      },
    });

    await expect(chip).toContainText('Gateway Stopped');

    await page.evaluate(() => {
      const source = (window as unknown as { __gatewaySse?: EventSource }).__gatewaySse;
      source?.close();
    });
  });
});
