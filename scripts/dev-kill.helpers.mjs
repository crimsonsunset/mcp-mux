/**
 * Kill helpers for the McpMux local dev stack (Vite :1420, gateway :45818, admin :45819).
 *
 * Used by `scripts/dev-env.mjs` (`pnpm dev:stop`, `predev`). Patterns are scoped to
 * McpMux dev processes — not a generic `pkill mcpmux`.
 */

import { execSync } from 'node:child_process';
import path from 'node:path';

/** Dev ports that must be free before `pnpm dev` / `pnpm dev:admin`. */
export const DEV_PORTS = [1420, 45818, 45819];

/** Default wait for processes to release ports after SIGTERM. */
export const PORT_WAIT_TIMEOUT_MS = 15_000;

/** Poll interval while waiting for ports to clear. */
export const PORT_POLL_MS = 250;

/**
 * `pkill -f` patterns for Unix dev orphans.
 *
 * Intentionally **not** prefixed with the repo path — `pnpm --filter @mcpmux/desktop dev`
 * runs from the pnpm store and does not include `mcp-mux/` in its argv.
 */
export const UNIX_DEV_PGREP_PATTERNS = [
  '@mcpmux/desktop dev',
  '@mcpmux/desktop dev:web',
  'scripts/dev-admin.mjs',
  'scripts/dev-web-admin.mjs',
  'scripts/dev-web-admin',
  'tauri dev',
  'target/debug/mcpmux',
  'target/release/mcpmux',
];

/**
 * Command-line markers used when deciding whether a port-holding PID is ours.
 * @param {string} repoRoot
 * @returns {string[]}
 */
export function devProcessMarkers(repoRoot) {
  const repo = repoRoot.toLowerCase();
  return [
    repo,
    `${path.sep}mcp-mux${path.sep}`,
    `${path.sep}mcpmux${path.sep}`,
    '@mcpmux/desktop',
    'dev-admin.mjs',
    'dev-web-admin.mjs',
    'tauri dev',
    'target/debug/mcpmux',
    'target/release/mcpmux',
    'vite/bin/vite',
    'apps/desktop/node_modules',
  ];
}

/**
 * Run a shell command and return trimmed stdout, or empty string on failure.
 * @param {string} command
 * @returns {string}
 */
export function run(command) {
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
export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * PIDs listening on a TCP port (platform-specific).
 * @param {number} port
 * @returns {number[]}
 */
export function pidsOnPort(port) {
  if (process.platform === 'win32') {
    const lines = run('netstat -ano -p tcp').split('\n');
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
export function processCommand(pid) {
  if (process.platform === 'win32') {
    return run(`wmic process where ProcessId=${pid} get CommandLine /value`);
  }
  return run(`ps -p ${pid} -o command=`);
}

/**
 * Whether a process belongs to this repo's dev stack (safe to stop).
 * @param {number} pid
 * @param {string} repoRoot
 * @returns {boolean}
 */
export function isRepoDevProcess(pid, repoRoot) {
  const cmd = processCommand(pid).toLowerCase();
  if (!cmd) {
    return false;
  }

  return devProcessMarkers(repoRoot).some((marker) => cmd.includes(marker.toLowerCase()));
}

/**
 * Send SIGTERM or SIGKILL (or taskkill) to a PID.
 * @param {number} pid
 * @param {boolean} force
 */
export function killPid(pid, force = false) {
  if (process.platform === 'win32') {
    run(force ? `taskkill /F /PID ${pid}` : `taskkill /PID ${pid}`);
    return;
  }
  run(force ? `kill -9 ${pid}` : `kill ${pid}`);
}

/**
 * Quit the installed McpMux.app so dev can bind :45818 (macOS only).
 */
export function quitInstalledApp() {
  if (process.platform !== 'darwin') {
    return;
  }
  run(`osascript -e 'tell application "McpMux" to quit'`);
}

/**
 * Best-effort `pkill -f` for known dev-stack command lines (Unix only).
 * @param {boolean} force
 */
export function pkillDevPatterns(force = false) {
  if (process.platform === 'win32') {
    return;
  }

  const signal = force ? '-9' : '';
  for (const pattern of UNIX_DEV_PGREP_PATTERNS) {
    run(`pkill ${signal} -f '${pattern}'`.trim());
  }
}

/**
 * Stop repo-scoped dev processes and free dev ports.
 * @param {string} repoRoot
 */
export function killDevStack(repoRoot) {
  quitInstalledApp();

  pkillDevPatterns(false);

  const seen = new Set();
  for (const port of DEV_PORTS) {
    for (const pid of pidsOnPort(port)) {
      if (seen.has(pid) || !isRepoDevProcess(pid, repoRoot)) {
        continue;
      }
      seen.add(pid);
      killPid(pid, false);
    }
  }

  pkillDevPatterns(true);

  for (const pid of seen) {
    killPid(pid, true);
  }

  for (const port of DEV_PORTS) {
    for (const pid of pidsOnPort(port)) {
      if (!isRepoDevProcess(pid, repoRoot)) {
        continue;
      }
      killPid(pid, true);
    }
  }
}

/**
 * @returns {boolean}
 */
export function portsAreFree() {
  return DEV_PORTS.every((port) => pidsOnPort(port).length === 0);
}

/**
 * Block until dev ports are free or timeout.
 * @param {number} [timeoutMs]
 * @returns {Promise<boolean>}
 */
export async function waitForPortsFree(timeoutMs = PORT_WAIT_TIMEOUT_MS) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (portsAreFree()) {
      return true;
    }
    await sleep(PORT_POLL_MS);
  }
  return portsAreFree();
}

/**
 * Ports still listening after a stop attempt.
 * @returns {number[]}
 */
export function busyDevPorts() {
  return DEV_PORTS.filter((port) => pidsOnPort(port).length > 0);
}
