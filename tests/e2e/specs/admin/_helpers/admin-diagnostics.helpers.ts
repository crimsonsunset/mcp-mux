import { expect, type Page } from '@playwright/test';

/** Matches playwright.admin.config `expect.timeout` — fail fast, don't burn 60s polls. */
const SELECTOR_TIMEOUT_MS = 15_000;

const dataSyncDoneByPage = new WeakMap<Page, Promise<void>>();

/**
 * Log a structured selector snapshot — mirrors S2H `logSelectorDiagnostics`.
 */
export function logAdminSelectors(
  step: string,
  payload: Record<string, unknown>
): void {
  console.log(`[e2e:admin] selectors`, JSON.stringify({ step, ...payload }));
}

/**
 * Snapshot key admin UI element counts for debugging.
 */
export async function snapshotAdminSelectors(page: Page, step: string): Promise<void> {
  const [
    navDashboard,
    navServers,
    navSpaces,
    serversPage,
    spacesPage,
    spaceSwitcher,
    spaceCards,
    mainSpinners,
  ] = await Promise.all([
    page.getByTestId('nav-dashboard').count(),
    page.getByTestId('nav-my-servers').count(),
    page.getByTestId('nav-spaces').count(),
    page.getByTestId('servers-page').count(),
    page.getByTestId('spaces-page').count(),
    page.getByTestId('space-switcher').count(),
    page.locator('[data-testid^="space-card-"]').count(),
    page.locator('main .animate-spin').count(),
  ]);

  const switcherText =
    (await page.getByTestId('space-switcher').textContent().catch(() => null))?.trim() ?? null;

  logAdminSelectors(step, {
    navDashboard,
    navServers,
    navSpaces,
    serversPage,
    spacesPage,
    spaceSwitcher,
    spaceCards,
    mainSpinners,
    switcherText,
    url: page.url(),
  });
}

/**
 * Attach browser console + request listeners for admin page diagnostics.
 * Call once per page, before navigating.
 */
export function attachAdminPageDiagnostics(page: Page): void {
  if (!dataSyncDoneByPage.has(page)) {
    dataSyncDoneByPage.set(
      page,
      page
        .waitForEvent('console', {
          predicate: (msg) => msg.text().includes('[useDataSync] Data sync complete'),
          timeout: SELECTOR_TIMEOUT_MS,
        })
        .then(() => undefined)
    );
  }

  page.on('console', (msg) => {
    const text = msg.text();
    if (msg.type() === 'error') {
      console.log(`[e2e:admin] browser.error`, JSON.stringify({ text }));
      return;
    }
    if (
      text.startsWith('[useDataSync]') ||
      text.startsWith('[fetchApi]') ||
      text.includes('Admin API')
    ) {
      console.log(`[e2e:admin] browser.log`, JSON.stringify({ text }));
    }
  });

  page.on('response', (response) => {
    const url = response.url();
    if (!url.includes('localhost:45819/api/v1/')) {
      return;
    }
    if (
      url.includes('/health') ||
      url.includes('/csrf-token') ||
      url.includes('/spaces') ||
      !response.ok()
    ) {
      console.log(
        `[e2e:admin] browser.api`,
        JSON.stringify({ url, status: response.status(), ok: response.ok() })
      );
    }
  });

  page.on('requestfailed', (req) => {
    const url = req.url();
    if (url.includes('localhost:45819') && !url.includes('/api/v1/events')) {
      console.log(
        `[e2e:admin] request.failed`,
        JSON.stringify({ url, failure: req.failure()?.errorText })
      );
    }
  });
}

/**
 * Wait until admin HTTP startup sync finishes (health + listSpaces).
 * Grounded on `SpaceSwitcher` + `useDataSync` — poll switcher label, not sidebar nav buttons.
 */
export async function waitForAdminAppReady(page: Page): Promise<void> {
  await snapshotAdminSelectors(page, 'waitForAdminAppReady:start');

  await expect(page.getByTestId('space-switcher')).toBeVisible({ timeout: SELECTOR_TIMEOUT_MS });

  const syncDone = dataSyncDoneByPage.get(page);

  try {
    await Promise.race([
      syncDone ?? Promise.reject(new Error('attachAdminPageDiagnostics must run before goto')),
      expect
        .poll(
          async () => {
            const text = (await page.getByTestId('space-switcher').textContent()) ?? '';
            return !text.includes('Loading');
          },
          { timeout: SELECTOR_TIMEOUT_MS, intervals: [250, 500, 1000] }
        )
        .toBe(true),
    ]);
  } catch (err) {
    await snapshotAdminSelectors(page, 'waitForAdminAppReady:failed');
    throw new Error(
      'Admin startup sync did not finish — space-switcher still Loading. ' +
        'Check [e2e:admin] browser.api for /health and /spaces; rebuild with `pnpm build:web:admin` after fetch-api changes.',
      { cause: err instanceof Error ? err : undefined }
    );
  }

  await snapshotAdminSelectors(page, 'waitForAdminAppReady:done');
}

/**
 * Wait for the servers-page testid with explicit timeout + diagnostic snapshot on failure.
 */
export async function waitForServersPage(page: Page): Promise<void> {
  await snapshotAdminSelectors(page, 'waitForServersPage:start');

  try {
    await expect(page.locator('main .animate-spin')).toHaveCount(0, { timeout: SELECTOR_TIMEOUT_MS });
    await expect(page.getByTestId('servers-page')).toBeVisible({ timeout: 10_000 });
  } catch (err) {
    await snapshotAdminSelectors(page, 'waitForServersPage:failed');
    throw err;
  }

  await snapshotAdminSelectors(page, 'waitForServersPage:done');
}

/**
 * Wait for the spaces grid to finish loading (cards or empty-state), not the loading spinner.
 */
export async function waitForSpacesPage(page: Page): Promise<void> {
  await snapshotAdminSelectors(page, 'waitForSpacesPage:start');

  await expect(page.getByTestId('spaces-page')).toBeVisible({ timeout: SELECTOR_TIMEOUT_MS });

  const content = page.getByTestId('spaces-page');
  try {
    await expect(content.locator('.animate-spin')).toHaveCount(0, { timeout: SELECTOR_TIMEOUT_MS });
    await expect(
      content
        .locator('[data-testid^="space-card-"]')
        .first()
        .or(content.getByText('No spaces created'))
        .or(content.getByText('No spaces match your search'))
    ).toBeVisible({ timeout: 5_000 });
  } catch (err) {
    await snapshotAdminSelectors(page, 'waitForSpacesPage:failed');
    throw err;
  }

  await snapshotAdminSelectors(page, 'waitForSpacesPage:done');
}
