#!/usr/bin/env node
'use strict';

// Managed McpMux preToolUse hook. Injects _mcpmux_context when Cursor
// reports exactly one workspace root. Fail-open on any parse error.

const CONTEXT_KEY = '_mcpmux_context';

function allow() {
  process.stdout.write(JSON.stringify({ permission: 'allow' }));
}

let raw = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
  raw += chunk;
});
process.stdin.on('end', () => {
  try {
    const payload = JSON.parse(raw || '{}');
    const roots = Array.isArray(payload.workspace_roots) ? payload.workspace_roots : [];
    if (roots.length !== 1) {
      allow();
      return;
    }
    const root = String(roots[0] || '').trim();
    if (!root) {
      allow();
      return;
    }
    const input =
      payload.tool_input &&
      typeof payload.tool_input === 'object' &&
      !Array.isArray(payload.tool_input)
        ? { ...payload.tool_input }
        : {};
    const context = { workspace_root: root };
    if (typeof payload.tool_use_id === 'string' && payload.tool_use_id) {
      context.tool_use_id = payload.tool_use_id;
    }
    input[CONTEXT_KEY] = context;
    process.stdout.write(JSON.stringify({ permission: 'allow', updated_input: input }));
  } catch {
    allow();
  }
});
