#!/usr/bin/env node
/**
 * Tauri dev with web admin enabled for the session (MCPMUX_DEV_ADMIN=1).
 * Opens the HMR URL in the default browser after the admin health check passes.
 *
 * Usage (repo root): pnpm dev:admin
 */

import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { loadRepoDotEnv } from './cf-access-env.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const ADMIN_PORT = Number.parseInt(process.env.MCPMUX_ADMIN_PORT ?? '45819', 10);
const HEALTH_URL = `http://127.0.0.1:${ADMIN_PORT}/api/v1/health`;
const VITE_URL = 'http://127.0.0.1:1420';
const OPEN_WAIT_MS = 90_000;
const POLL_MS = 500;

/**
 * @param {number} ms
 * @returns {Promise<void>}
 */
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * @returns {Promise<boolean>}
 */
async function adminHealthOk() {
  try {
    const response = await fetch(HEALTH_URL, {
      method: 'GET',
      headers: { Accept: 'application/json' },
    });
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Open a URL in the system browser (macOS/Linux/Windows best-effort).
 * @param {string} url
 */
function openBrowser(url) {
  if (process.platform === 'darwin') {
    spawnSync('open', [url], { stdio: 'ignore' });
    return;
  }
  if (process.platform === 'win32') {
    spawnSync('cmd', ['/c', 'start', '', url], { stdio: 'ignore', shell: true });
    return;
  }
  spawnSync('xdg-open', [url], { stdio: 'ignore' });
}

async function waitThenOpenBrowser() {
  const deadline = Date.now() + OPEN_WAIT_MS;
  while (Date.now() < deadline) {
    if (await adminHealthOk()) {
      console.log(`[dev-admin] Admin API ready — opening ${VITE_URL} (HMR + /api proxy).`);
      console.log(`[dev-admin] Production-parity UI: http://127.0.0.1:${ADMIN_PORT}/ after pnpm build:web:admin`);
      openBrowser(VITE_URL);
      return;
    }
    await sleep(POLL_MS);
  }
  console.warn(`[dev-admin] Timed out waiting for ${HEALTH_URL}; open ${VITE_URL} manually when ready.`);
}

async function main() {
  if (!existsSync(path.join(REPO_ROOT, 'package.json'))) {
    console.error('[dev-admin] Could not locate repo root.');
    process.exit(1);
  }

  loadRepoDotEnv(REPO_ROOT);

  void waitThenOpenBrowser();

  const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm';
  const result = spawnSync(pnpm, ['dev'], {
    cwd: REPO_ROOT,
    stdio: 'inherit',
    env: {
      ...process.env,
      MCPMUX_DEV_ADMIN: '1',
      MCPMUX_DEV_PREP_DONE: '1',
    },
    shell: process.platform === 'win32',
  });
  process.exit(result.status ?? 0);
}

main();
