#!/usr/bin/env node
/**
 * Dev stack helpers for McpMux — stop orphaned Vite/Tauri processes, free ports
 * (1420, 45818, 45819), and optionally rebuild gateway crates before `pnpm dev`.
 *
 * Usage:
 *   node scripts/dev-env.mjs prep          # predev: quit app, stop repo orphans, wait ports
 *   node scripts/dev-env.mjs stop        # same as prep (explicit stop)
 *   node scripts/dev-env.mjs restart     # stop + rebuild gateway + exec pnpm dev
 *   node scripts/dev-env.mjs rebuild     # cargo build mcpmux-gateway + mcpmux only
 */

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  DEV_PORTS,
  PORT_WAIT_TIMEOUT_MS,
  busyDevPorts,
  killDevStack,
  waitForPortsFree,
} from './dev-kill.helpers.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

/**
 * Rebuild gateway + desktop binary so `Finished in 0.20s` is not a stale artifact.
 */
function rebuildGateway() {
  const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
  console.log('[dev-env] Rebuilding mcpmux-gateway + mcpmux…');
  const result = spawnSync(cargo, ['build', '-p', 'mcpmux-gateway', '-p', 'mcpmux'], {
    cwd: REPO_ROOT,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

/**
 * Prep dev environment: stop orphans, wait for ports.
 */
async function prep() {
  if (process.env.MCPMUX_DEV_PREP_DONE === '1') {
    return;
  }

  console.log('[dev-env] Stopping orphaned McpMux dev processes…');
  killDevStack(REPO_ROOT);

  const ready = await waitForPortsFree(PORT_WAIT_TIMEOUT_MS);
  if (!ready) {
    const busy = busyDevPorts();
    console.error(
      `[dev-env] Ports still in use after ${PORT_WAIT_TIMEOUT_MS}ms: ${busy.join(', ')}`,
    );
    console.error(
      '[dev-env] Run `pnpm dev:stop` again or see AGENTS.md (Build & Dev Commands).',
    );
    process.exit(1);
  }

  console.log(`[dev-env] Ports ${DEV_PORTS.join(', ')} are free.`);
}

/**
 * @param {string[]} argv
 */
async function main(argv) {
  const command = argv[0] ?? 'prep';
  const flags = new Set(argv.slice(1));

  switch (command) {
    case 'prep':
    case 'stop':
      await prep();
      break;

    case 'rebuild':
      rebuildGateway();
      break;

    case 'restart': {
      await prep();
      if (!flags.has('--no-rebuild')) {
        rebuildGateway();
      }
      console.log('[dev-env] Starting pnpm dev…');
      const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm';
      const result = spawnSync(pnpm, ['dev'], {
        cwd: REPO_ROOT,
        stdio: 'inherit',
        shell: process.platform === 'win32',
        env: { ...process.env, MCPMUX_DEV_PREP_DONE: '1' },
      });
      process.exit(result.status ?? 0);
      break;
    }

    default:
      console.error(`Unknown command: ${command}`);
      console.error('Usage: node scripts/dev-env.mjs [prep|stop|rebuild|restart] [--no-rebuild]');
      process.exit(1);
  }
}

if (!existsSync(path.join(REPO_ROOT, 'package.json'))) {
  console.error('[dev-env] Could not locate repo root.');
  process.exit(1);
}

main(process.argv.slice(2));
