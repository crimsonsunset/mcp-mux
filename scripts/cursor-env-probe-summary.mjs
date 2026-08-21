#!/usr/bin/env node
/**
 * Summarize a cursor-env-probe log using the Aug 20, 2026 study cuts.
 *
 * Usage:
 *   pnpm probe:cursor-env:summary
 *   node scripts/cursor-env-probe-summary.mjs [log-path]
 *   node scripts/cursor-env-probe-summary.mjs --self-check
 *
 * Default log: $HOME/Desktop/mcpmux-env-probe.log
 * Override:    MCPMUX_ENV_PROBE_LOG or the first positional arg
 */

import { readFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);

/**
 * Default probe log path (same as the wrapper).
 * @returns {string}
 */
export function defaultProbeLogPath() {
  return (
    process.env.MCPMUX_ENV_PROBE_LOG || path.join(os.homedir(), 'Desktop', 'mcpmux-env-probe.log')
  );
}

/**
 * @typedef {{
 *   argv: string[],
 *   env: Record<string, string>,
 *   workspaceHeader: string | null,
 *   isAgent: boolean,
 *   folderSet: string[],
 * }} ProbeSpawn
 */

/**
 * Parse one or more `=== … ===` records from a probe log.
 * @param {string} text
 * @returns {ProbeSpawn[]}
 */
export function parseProbeLog(text) {
  const blocks = text.split(/^=== /m).filter((block) => block.trim());
  return blocks.map(parseBlock);
}

/**
 * @param {string} block
 * @returns {ProbeSpawn}
 */
function parseBlock(block) {
  const argv = [];
  /** @type {Record<string, string>} */
  const env = {};
  let section = '';
  for (const line of block.split('\n')) {
    if (line === '--- argv ---') {
      section = 'argv';
      continue;
    }
    if (line === '--- env (sorted) ---') {
      section = 'env';
      continue;
    }
    if (line === '--- pwd ---') {
      section = 'pwd';
      continue;
    }
    if (section === 'argv') {
      const match = line.match(/^\[(\d+)\] (.*)$/);
      if (match) argv[Number(match[1])] = match[2];
    } else if (section === 'env') {
      const eq = line.indexOf('=');
      if (eq > 0) env[line.slice(0, eq)] = line.slice(eq + 1);
    }
  }
  return {
    argv,
    env,
    workspaceHeader: extractWorkspaceHeader(argv),
    isAgent: env.CURSOR_AGENT === '1',
    folderSet: parseFolderSet(env.WORKSPACE_FOLDER_PATHS ?? ''),
  };
}

/**
 * Active-folder value from the workspace header, or a legacy argv[0] path.
 * @param {string[]} argv
 * @returns {string | null}
 */
export function extractWorkspaceHeader(argv) {
  for (const arg of argv) {
    if (arg?.startsWith('X-Mcpmux-Workspace:') && !arg.startsWith('X-Mcpmux-Workspace-Set:')) {
      return arg.slice('X-Mcpmux-Workspace:'.length);
    }
  }
  const first = argv[0];
  if (!first) return null;
  if (first === '${workspaceFolder}' || first.startsWith('/') || first.startsWith('${')) {
    return first;
  }
  return /^[A-Za-z]:[\\/]/.test(first) ? first : null;
}

/**
 * Split Cursor's comma-separated folder set, dropping unexpanded templates.
 * @param {string} raw
 * @returns {string[]}
 */
export function parseFolderSet(raw) {
  return raw
    .split(',')
    .map((entry) => entry.trim())
    .filter((entry) => entry && !entry.includes('${'));
}

/**
 * @param {string | null} header
 * @returns {boolean}
 */
function isUnresolvedWorkspace(header) {
  return header === '${workspaceFolder}' || header === '' || header == null;
}

/**
 * @typedef {{
 *   spawns: number,
 *   unresolved: number,
 *   resolved: number,
 *   editorSpawns: number,
 *   editorUnresolved: number,
 *   agentSpawns: number,
 *   agentUnresolved: number,
 *   folderCounts: Record<string, number>,
 *   multiResolved: number,
 *   memberOfSet: number,
 *   firstOfSet: number,
 *   unexpandedSet: number,
 * }} ProbeSummary
 */

/**
 * Compute the Aug 20 study cuts from parsed spawns.
 * @param {ProbeSpawn[]} spawns
 * @returns {ProbeSummary}
 */
export function summarizeSpawns(spawns) {
  /** @type {ProbeSummary} */
  const summary = {
    spawns: spawns.length,
    unresolved: 0,
    resolved: 0,
    editorSpawns: 0,
    editorUnresolved: 0,
    agentSpawns: 0,
    agentUnresolved: 0,
    folderCounts: {},
    multiResolved: 0,
    memberOfSet: 0,
    firstOfSet: 0,
    unexpandedSet: 0,
  };

  for (const spawn of spawns) {
    const unresolved = isUnresolvedWorkspace(spawn.workspaceHeader);
    if (unresolved) summary.unresolved += 1;
    else summary.resolved += 1;

    if (spawn.isAgent) {
      summary.agentSpawns += 1;
      if (unresolved) summary.agentUnresolved += 1;
    } else {
      summary.editorSpawns += 1;
      if (unresolved) summary.editorUnresolved += 1;
    }

    const rawSet = spawn.env.WORKSPACE_FOLDER_PATHS ?? '';
    if (rawSet.includes('${')) summary.unexpandedSet += 1;

    const n = spawn.folderSet.length;
    const key = String(n);
    summary.folderCounts[key] = (summary.folderCounts[key] ?? 0) + 1;

    if (!unresolved && n > 1 && spawn.workspaceHeader) {
      summary.multiResolved += 1;
      const active = spawn.workspaceHeader;
      if (spawn.folderSet.includes(active)) summary.memberOfSet += 1;
      if (spawn.folderSet[0] === active) summary.firstOfSet += 1;
    }
  }

  return summary;
}

/**
 * @param {number} part
 * @param {number} whole
 * @returns {string}
 */
function pct(part, whole) {
  if (whole === 0) return 'n/a';
  return `${((100 * part) / whole).toFixed(1)}%`;
}

/**
 * Print a human-readable summary.
 * @param {ProbeSummary} summary
 * @returns {string}
 */
export function formatSummary(summary) {
  const counts = Object.entries(summary.folderCounts)
    .sort(([a], [b]) => Number(a) - Number(b))
    .map(([n, c]) => `  ${n} folders: ${c}`)
    .join('\n');
  return [
    `spawns:              ${summary.spawns}`,
    `${'${workspaceFolder}'} unresolved: ${summary.unresolved} (${pct(summary.unresolved, summary.spawns)})`,
    `resolved:            ${summary.resolved}`,
    `editor:              ${summary.editorUnresolved}/${summary.editorSpawns} unresolved (${pct(summary.editorUnresolved, summary.editorSpawns)})`,
    `agent (CURSOR_AGENT): ${summary.agentUnresolved}/${summary.agentSpawns} unresolved (${pct(summary.agentUnresolved, summary.agentSpawns)})`,
    `folder-count histogram:`,
    counts || '  (none)',
    `multi-folder resolved: ${summary.multiResolved}`,
    `  active in set:     ${summary.memberOfSet} (${pct(summary.memberOfSet, summary.multiResolved)})`,
    `  active == WFP[0]:  ${summary.firstOfSet} (${pct(summary.firstOfSet, summary.multiResolved)})`,
    `unexpanded WORKSPACE_FOLDER_PATHS: ${summary.unexpandedSet}`,
  ].join('\n');
}

/**
 * Tiny fixture that fails if the parser or cuts regress.
 * @returns {void}
 */
export function selfCheck() {
  const fixture = `=== 2026-08-20T00:00:00.000Z ===
--- argv ---
[0] -y
[1] mcp-remote
[2] --header
[3] X-Mcpmux-Workspace:/repos/alpha
--- env (sorted) ---
CURSOR_AGENT=1
WORKSPACE_FOLDER_PATHS=/repos/alpha,/repos/beta
--- pwd ---
/Users/joe

=== 2026-08-20T00:00:01.000Z ===
--- argv ---
[0] -y
[1] --header
[2] X-Mcpmux-Workspace:\${workspaceFolder}
--- env (sorted) ---
WORKSPACE_FOLDER_PATHS=/repos/alpha,/repos/beta
--- pwd ---
/Users/joe

=== 2026-08-20T00:00:02.000Z ===
--- argv ---
[0] \${workspaceFolder}
--- env (sorted) ---
WORKSPACE_FOLDER_PATHS=\${WORKSPACE_FOLDER_PATHS}
--- pwd ---
/Users/joe
`;
  const summary = summarizeSpawns(parseProbeLog(fixture));
  const checks = [
    summary.spawns === 3,
    summary.unresolved === 2,
    summary.resolved === 1,
    summary.agentSpawns === 1 && summary.agentUnresolved === 0,
    summary.editorSpawns === 2 && summary.editorUnresolved === 2,
    summary.multiResolved === 1 && summary.memberOfSet === 1 && summary.firstOfSet === 1,
    summary.unexpandedSet === 1,
  ];
  if (checks.some((ok) => !ok)) {
    console.error('self-check failed\n', formatSummary(summary));
    process.exit(1);
  }
  console.log('self-check ok');
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === SCRIPT_PATH;
if (isMain) {
  const args = process.argv.slice(2);
  if (args[0] === '--self-check') {
    selfCheck();
    process.exit(0);
  }
  const logPath = args[0] || defaultProbeLogPath();
  let text;
  try {
    text = readFileSync(logPath, 'utf8');
  } catch (err) {
    console.error(`cursor-env-probe-summary: cannot read ${logPath}: ${err.message}`);
    process.exit(1);
  }
  console.log(`log: ${logPath}`);
  console.log(formatSummary(summarizeSpawns(parseProbeLog(text))));
}
