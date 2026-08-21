#!/usr/bin/env node
/**
 * Cursor mcp-remote spawn wrapper — log argv/env/pwd, then exec the child.
 *
 * Drop this in front of `npx mcp-remote` in `~/.cursor/mcp.json` to re-measure
 * `${workspaceFolder}` substitution after a Cursor update. Summarize the log
 * with `pnpm probe:cursor-env:summary`.
 *
 * Usage:
 *   pnpm probe:cursor-env
 *   node scripts/cursor-env-probe.mjs --help
 *   node scripts/cursor-env-probe.mjs -y mcp-remote http://127.0.0.1:45818/mcp …
 *
 * Log default: $HOME/Desktop/mcpmux-env-probe.log
 * Override:    MCPMUX_ENV_PROBE_LOG
 */

import { spawn } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);

/**
 * Path the wrapper appends each spawn record to.
 * @returns {string}
 */
export function defaultProbeLogPath() {
  return (
    process.env.MCPMUX_ENV_PROBE_LOG || path.join(os.homedir(), 'Desktop', 'mcpmux-env-probe.log')
  );
}

/**
 * Print the `~/.cursor/mcp.json` swap recipe and default log path.
 * @param {string} [logPath]
 */
export function printSwapRecipe(logPath = defaultProbeLogPath()) {
  const snippet = {
    mcpServers: {
      mcpmux: {
        command: 'node',
        args: [
          SCRIPT_PATH,
          '-y',
          'mcp-remote',
          'http://127.0.0.1:45818/mcp',
          '--allow-http',
          '--header',
          'X-Mcpmux-Workspace:${workspaceFolder}',
          '--header',
          'X-Mcpmux-Workspace-Set:${WORKSPACE_FOLDER_PATHS}',
          '--header',
          'Authorization:Bearer ${MCPMUX_API_KEY}',
        ],
        env: { MCPMUX_API_KEY: '<your mcpk_ key>' },
      },
    },
  };
  console.log(`Cursor env-probe wrapper
Log: ${logPath}

1. Replace the mcpmux entry in ~/.cursor/mcp.json with:

${JSON.stringify(snippet, null, 2)}

2. Reload MCP in Cursor. Use editor + Agents windows until you have hundreds of spawns.
3. pnpm probe:cursor-env:summary
4. Restore the generated bridge config (Connections → Global Cursor setup).
`);
}

/**
 * Build one spawn record in the format the summarizer parses.
 * @param {string[]} argv
 * @param {NodeJS.ProcessEnv} env
 * @param {string} cwd
 * @param {Date} [now]
 * @returns {string}
 */
export function formatProbeRecord(argv, env, cwd, now = new Date()) {
  const lines = [`=== ${now.toISOString()} ===`, '--- argv ---'];
  argv.forEach((arg, i) => {
    lines.push(`[${i}] ${arg}`);
  });
  lines.push('--- env (sorted) ---');
  for (const key of Object.keys(env).sort()) {
    lines.push(`${key}=${env[key] ?? ''}`);
  }
  lines.push('--- pwd ---', cwd, '');
  return `${lines.join('\n')}\n`;
}

/**
 * Append a spawn record, then exec the remaining argv as the MCP child.
 * @param {string[]} argv
 * @param {{ logPath?: string, env?: NodeJS.ProcessEnv, cwd?: string }} [opts]
 */
export function runProbe(argv, opts = {}) {
  const logPath = opts.logPath ?? defaultProbeLogPath();
  const env = opts.env ?? process.env;
  const cwd = opts.cwd ?? process.cwd();
  appendFileSync(logPath, formatProbeRecord(argv, env, cwd));

  const [command, ...args] = argv;
  if (!command) {
    console.error('cursor-env-probe: no command to exec (pass npx / mcp-remote args)');
    process.exit(1);
  }

  const child = spawn(command, args, { stdio: 'inherit', env });
  const forward = (signal) => {
    if (!child.killed) child.kill(signal);
  };
  process.on('SIGINT', () => forward('SIGINT'));
  process.on('SIGTERM', () => forward('SIGTERM'));
  child.on('exit', (code, signal) => {
    if (signal) process.exit(1);
    process.exit(code ?? 1);
  });
  child.on('error', (err) => {
    console.error(`cursor-env-probe: failed to spawn ${command}: ${err.message}`);
    process.exit(1);
  });
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === SCRIPT_PATH;
if (isMain) {
  const args = process.argv.slice(2);
  if (args.length === 0 || args[0] === '--help' || args[0] === '-h') {
    printSwapRecipe();
    process.exit(0);
  }
  runProbe(args);
}
