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

import { execSync, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DEV_PORTS = [1420, 45818, 45819];
const WAIT_TIMEOUT_MS = 15_000;
const POLL_MS = 250;

/**
 * Run a shell command and return trimmed stdout, or empty string on failure.
 * @param {string} command
 * @returns {string}
 */
function run(command) {
  try {
    return execSync(command, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
  } catch {
    return '';
  }
}

/**
 * @param {number} ms
 * @returns {Promise<void>}
 */
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * PIDs listening on a TCP port (platform-specific).
 * @param {number} port
 * @returns {number[]}
 */
function pidsOnPort(port) {
  if (process.platform === 'win32') {
    const lines = run(`netstat -ano -p tcp`).split('\n');
    const pids = new Set();
    for (const line of lines) {
      if (!line.includes(`:${port}`) || !line.includes('LISTENING')) {
        continue;
      }
      const pid = Number.parseInt(line.trim().split(/\s+/).at(-1) ?? '', 10);
      if (Number.isFinite(pid) && pid > 0) {
        pids.add(pid);
      }
    }
    return [...pids];
  }

  const output = run(`lsof -ti tcp:${port}`);
  if (!output) {
    return [];
  }
  return output
    .split('\n')
    .map((value) => Number.parseInt(value, 10))
    .filter((pid) => Number.isFinite(pid) && pid > 0);
}

/**
 * Full command line for a PID.
 * @param {number} pid
 * @returns {string}
 */
function processCommand(pid) {
  if (process.platform === 'win32') {
    return run(`wmic process where ProcessId=${pid} get CommandLine /value`);
  }
  return run(`ps -p ${pid} -o command=`);
}

/**
 * Whether a process belongs to this repo's dev stack (safe to stop).
 * @param {number} pid
 * @returns {boolean}
 */
function isRepoDevProcess(pid) {
  const cmd = processCommand(pid).toLowerCase();
  if (!cmd) {
    return false;
  }

  const repo = REPO_ROOT.toLowerCase();
  const markers = [
    repo,
    `${path.sep}mcp-mux${path.sep}`,
    `${path.sep}mcpmux${path.sep}`,
    'target/debug/mcpmux',
    'target\\debug\\mcpmux',
    '@mcpmux/desktop',
    'tauri dev',
    'vite/bin/vite',
  ];

  return markers.some((marker) => cmd.includes(marker));
}

/**
 * Send SIGTERM (or taskkill) to a PID.
 * @param {number} pid
 * @param {boolean} force
 */
function killPid(pid, force = false) {
  if (process.platform === 'win32') {
    run(force ? `taskkill /F /PID ${pid}` : `taskkill /PID ${pid}`);
    return;
  }
  run(force ? `kill -9 ${pid}` : `kill ${pid}`);
}

/**
 * Quit the installed McpMux.app so dev can bind :45818 (macOS only).
 */
function quitInstalledApp() {
  if (process.platform !== 'darwin') {
    return;
  }
  run(`osascript -e 'tell application "McpMux" to quit'`);
}

/**
 * Stop repo-scoped dev processes on dev ports and stray pnpm/tauri/vite trees.
 */
function stopDevStack() {
  quitInstalledApp();

  const seen = new Set();
  for (const port of DEV_PORTS) {
    for (const pid of pidsOnPort(port)) {
      if (seen.has(pid) || !isRepoDevProcess(pid)) {
        continue;
      }
      seen.add(pid);
      killPid(pid, false);
    }
  }

  if (process.platform !== 'win32') {
    run(`pkill -f '${REPO_ROOT}.*(@mcpmux/desktop dev|tauri dev|vite/bin/vite)'`);
  }

  for (const pid of seen) {
    killPid(pid, true);
  }
}

/**
 * @returns {boolean}
 */
function portsAreFree() {
  return DEV_PORTS.every((port) => pidsOnPort(port).length === 0);
}

/**
 * Block until dev ports are free or timeout.
 * @returns {Promise<boolean>}
 */
async function waitForPortsFree() {
  const deadline = Date.now() + WAIT_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (portsAreFree()) {
      return true;
    }
    await sleep(POLL_MS);
  }
  return portsAreFree();
}

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
  stopDevStack();

  const ready = await waitForPortsFree();
  if (!ready) {
    const busy = DEV_PORTS.filter((port) => pidsOnPort(port).length > 0);
    console.error(
      `[dev-env] Ports still in use after ${WAIT_TIMEOUT_MS}ms: ${busy.join(', ')}`,
    );
    console.error('[dev-env] Quit McpMux.app or stop the process holding the port, then retry.');
    process.exit(1);
  }

  console.log('[dev-env] Ports 1420, 45818, and 45819 are free.');
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
