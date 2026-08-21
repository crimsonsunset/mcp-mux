#!/usr/bin/env node
/**
 * Stop a local dev session cleanly: gracefully quit the production McpMux.app
 * (macOS) and kill whatever is listening on the dev ports (Vite :1420, gateway
 * :45818, admin :45819), then wait until those ports are free.
 *
 * Recovery for the edge cases `tauri dev`'s own watcher does not cover: a
 * keychain-locked restart that stalled, an orphaned detached backend from
 * `pnpm dev:web:admin`, or a stale binary holding a port after a failed relaunch.
 *
 * Usage (repo root): pnpm dev:stop
 */

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, unlinkSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const VITE_PORT = 1420;
const GATEWAY_PORT = Number.parseInt(process.env.MCPMUX_GATEWAY_PORT ?? '45818', 10);
const ADMIN_PORT = Number.parseInt(process.env.MCPMUX_ADMIN_PORT ?? '45819', 10);
const PORTS = [VITE_PORT, GATEWAY_PORT, ADMIN_PORT];
const FREE_WAIT_MS = 10_000;
const POLL_MS = 250;

/**
 * @param {number} ms
 * @returns {Promise<void>}
 */
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Find PIDs listening on a TCP port (best-effort, cross-platform).
 * @param {number} port
 * @returns {number[]} unique listening PIDs
 */
function pidsOnPort(port) {
  if (process.platform === 'win32') {
    const out = spawnSync('netstat', ['-ano', '-p', 'tcp'], { encoding: 'utf8' }).stdout ?? '';
    const pids = out
      .split(/\r?\n/)
      .filter((line) => line.includes('LISTENING') && line.includes(`:${port} `))
      .map((line) => Number.parseInt(line.trim().split(/\s+/).pop() ?? '', 10))
      .filter((pid) => Number.isInteger(pid) && pid > 0);
    return [...new Set(pids)];
  }

  const out =
    spawnSync('lsof', [`-ti`, `tcp:${port}`, '-s', 'tcp:LISTEN'], {
      encoding: 'utf8',
    }).stdout ?? '';
  const pids = out
    .split(/\r?\n/)
    .map((line) => Number.parseInt(line.trim(), 10))
    .filter((pid) => Number.isInteger(pid) && pid > 0);
  return [...new Set(pids)];
}

/**
 * PID file written by a detached `dev:admin` supervisor (same dir as the app log).
 * @returns {string}
 */
function detachedPidPath() {
  const home = os.homedir();
  if (process.platform === 'darwin') {
    return path.join(home, 'Library/Application Support/com.mcpmux.desktop/logs/dev-admin.pid');
  }
  if (process.platform === 'win32') {
    return path.join(process.env.LOCALAPPDATA ?? home, 'com.mcpmux.desktop', 'logs', 'dev-admin.pid');
  }
  return path.join(home, '.local/share/com.mcpmux.desktop/logs/dev-admin.pid');
}

/**
 * True when `pid` still exists.
 * @param {number} pid
 * @returns {boolean}
 */
function isPidAlive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/**
 * Kill a detached `dev:admin` supervisor before ports are listening.
 * Uses the process group on Unix so compile-time children die with it.
 * @returns {boolean} true when a live supervisor was signalled
 */
function killDetachedSupervisor() {
  const pidPath = detachedPidPath();
  if (!existsSync(pidPath)) return false;

  const raw = readFileSync(pidPath, 'utf8').trim();
  const pid = Number.parseInt(raw, 10);
  if (!Number.isInteger(pid) || pid <= 0) {
    try {
      unlinkSync(pidPath);
    } catch {
      // gone
    }
    return false;
  }

  if (!isPidAlive(pid)) {
    console.log(`[dev-stop] ${new Date().toISOString()} stale detached pid ${pid} — removing ${pidPath}`);
    try {
      unlinkSync(pidPath);
    } catch {
      // gone
    }
    return false;
  }

  console.log(`[dev-stop] ${new Date().toISOString()} sending SIGTERM to detached supervisor PID ${pid}`);
  if (process.platform === 'win32') {
    killPid(pid);
  } else {
    try {
      process.kill(-pid, 'SIGTERM');
    } catch {
      killPid(pid);
    }
  }
  return true;
}

/**
 * Terminate a PID (best-effort). Uses taskkill on Windows for tree kill.
 * @param {number} pid
 */
function killPid(pid) {
  if (process.platform === 'win32') {
    spawnSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
    return;
  }
  try {
    process.kill(pid, 'SIGTERM');
  } catch {
    // already gone
  }
}

/**
 * Gracefully quit the installed McpMux.app on macOS so it releases the keychain
 * and any ports before we force-kill stragglers.
 */
function quitMacApp() {
  if (process.platform !== 'darwin') return;
  console.log(`[dev-stop] ${new Date().toISOString()} asking "McpMux" to quit via osascript`);
  const result = spawnSync('osascript', ['-e', 'tell application "McpMux" to quit'], {
    stdio: 'ignore',
  });
  if (result.status === 0) {
    console.log('[dev-stop] "McpMux" accepted the quit request.');
  } else {
    console.log(`[dev-stop] "McpMux" quit request returned status ${result.status} (likely not running).`);
  }
}

/**
 * @returns {boolean} true when no dev port has a listener
 */
function allPortsFree() {
  return PORTS.every((port) => pidsOnPort(port).length === 0);
}

async function main() {
  quitMacApp();

  let killedAny = killDetachedSupervisor();
  for (const port of PORTS) {
    for (const pid of pidsOnPort(port)) {
      console.log(`[dev-stop] ${new Date().toISOString()} sending SIGTERM to PID ${pid} on :${port}`);
      killPid(pid);
      killedAny = true;
    }
  }

  if (!killedAny) {
    console.log('[dev-stop] No dev processes found on :1420 / :45818 / :45819.');
  }

  const deadline = Date.now() + FREE_WAIT_MS;
  while (Date.now() < deadline) {
    if (allPortsFree()) {
      console.log('[dev-stop] Ports clear — ready for a fresh dev session.');
      return;
    }
    await sleep(POLL_MS);
  }

  const stuck = PORTS.filter((port) => pidsOnPort(port).length > 0);
  console.warn(`[dev-stop] Ports still busy after ${FREE_WAIT_MS}ms: ${stuck.join(', ')}.`);
  console.warn('  Inspect with: lsof -nP -iTCP -sTCP:LISTEN | grep -E "1420|45818|45819"');
  process.exit(1);
}

main();
