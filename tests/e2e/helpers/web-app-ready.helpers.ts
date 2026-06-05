import { expect, type Page } from '@playwright/test';

const APP_READY_TIMEOUT_MS = 15_000;

const dataSyncDoneByPage = new WeakMap<Page, Promise<void>>();

/**
 * Attach console/API listeners so failures show why startup sync stalled.
 * Call once per page before the first navigation.
 */
export function attachWebPageDiagnostics(page: Page): void {
  if (!dataSyncDoneByPage.has(page)) {
    dataSyncDoneByPage.set(
      page,
      page
        .waitForEvent('console', {
          predicate: (msg) => msg.text().includes('[useDataSync] Data sync complete'),
          timeout: APP_READY_TIMEOUT_MS,
        })
        .then(() => undefined)
    );
  }

  page.on('console', (msg) => {
    const text = msg.text();
    if (msg.type() === 'error') {
      console.log(`[e2e:web] browser.error`, JSON.stringify({ text }));
      return;
    }
    if (text.includes('[useDataSync]') || text.includes('[fetchApi]')) {
      console.log(`[e2e:web] browser.console`, JSON.stringify({ type: msg.type(), text }));
    }
  });

  page.on('response', (response) => {
    const url = response.url();
    if (!url.includes('/api/v1/')) {
      return;
    }
    if (
      url.includes('/health') ||
      url.includes('/csrf-token') ||
      url.includes('/spaces') ||
      !response.ok()
    ) {
      console.log(
        `[e2e:web] browser.api`,
        JSON.stringify({ url, status: response.status(), ok: response.ok() })
      );
    }
  });

  page.on('requestfailed', (req) => {
    const url = req.url();
    if (url.includes('/api/v1/') && !url.includes('/api/v1/events')) {
      console.log(
        `[e2e:web] request.failed`,
        JSON.stringify({ url, failure: req.failure()?.errorText })
      );
    }
  });
}

/**
 * Wait until web-admin startup sync finishes (listSpaces + space switcher settled).
 * Do not use networkidle — SSE keeps the admin EventSource open indefinitely.
 */
export async function waitForWebAppReady(page: Page): Promise<void> {
  await expect(page.getByTestId('nav-dashboard')).toBeVisible({ timeout: APP_READY_TIMEOUT_MS });
  await expect(page.getByTestId('space-switcher')).toBeVisible({ timeout: APP_READY_TIMEOUT_MS });

  const syncDone = dataSyncDoneByPage.get(page);

  try {
    await Promise.race([
      syncDone ?? Promise.reject(new Error('attachWebPageDiagnostics must run before goto')),
      expect
        .poll(
          async () => {
            const text = (await page.getByTestId('space-switcher').textContent()) ?? '';
            return !text.includes('Loading');
          },
          { timeout: APP_READY_TIMEOUT_MS, intervals: [250, 500, 1000] }
        )
        .toBe(true),
    ]);
  } catch (err) {
    const switcherText =
      (await page.getByTestId('space-switcher').textContent().catch(() => null)) ?? '';
    console.log(
      `[e2e:web] waitForWebAppReady:failed`,
      JSON.stringify({ switcherText, url: page.url() })
    );
    throw new Error(
      'Web admin startup sync did not finish — space-switcher still Loading. ' +
        'Check [e2e:web] browser.api logs for /health and /spaces.',
      { cause: err instanceof Error ? err : undefined }
    );
  }

  console.log(`[e2e:web] waitForWebAppReady:done`, JSON.stringify({ url: page.url() }));
}
