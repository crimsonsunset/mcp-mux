import { test, expect } from '@playwright/test';

/**
 * Admin web smoke — gateway status on dashboard (ported from gateway.wdio.ts).
 */
test.describe('Admin gateway dashboard', () => {
  test('shows gateway status card and toggle', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: 30_000 });

    const statusCard = page.getByTestId('gateway-status-card');
    await expect(statusCard).toBeVisible();

    const cardText = await statusCard.textContent();
    expect(cardText).toMatch(/Gateway/i);

    const toggle = page.getByTestId('gateway-toggle-btn');
    await expect(toggle).toBeVisible();
  });
});
