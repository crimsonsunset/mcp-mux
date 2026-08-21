/**
 * cursor-env-probe — recipe argv and secret redaction.
 */

import { describe, it, expect, vi } from 'vitest';

import {
  formatProbeRecord,
  isSecretEnvKey,
  printSwapRecipe,
  redactArg,
} from '../../../scripts/cursor-env-probe.mjs';

describe('cursor-env-probe', () => {
  it('prints an npx executable in the swap recipe', () => {
    const logs: string[] = [];
    const spy = vi.spyOn(console, 'log').mockImplementation((msg: string) => {
      logs.push(String(msg));
    });
    printSwapRecipe('/tmp/probe.log');
    spy.mockRestore();
    const printed = logs.join('\n');
    expect(printed).toContain('"npx"');
    expect(printed.indexOf('"npx"')).toBeLessThan(printed.indexOf('"-y"'));
  });

  it('redacts bearer tokens and secret-like env vars', () => {
    expect(redactArg('Authorization:Bearer mcpk_live_secret')).toBe(
      'Authorization:Bearer <redacted>'
    );
    expect(isSecretEnvKey('GITHUB_TOKEN')).toBe(true);
    expect(isSecretEnvKey('OPENAI_KEY')).toBe(true);
    expect(isSecretEnvKey('AWS_ACCESS_KEY_ID')).toBe(true);
    expect(isSecretEnvKey('GITHUB_PAT')).toBe(true);
    expect(isSecretEnvKey('DATABASE_URL')).toBe(true);
    expect(isSecretEnvKey('PATH')).toBe(true);
    expect(isSecretEnvKey('CURSOR_TRACE_ID')).toBe(false);
    expect(isSecretEnvKey('WORKSPACE_FOLDER_PATHS')).toBe(false);
    expect(isSecretEnvKey('VSCODE_PID')).toBe(false);

    const record = formatProbeRecord(
      ['npx', '-y', 'mcp-remote', '--header', 'Authorization:Bearer mcpk_abc'],
      {
        PATH: '/usr/bin',
        OPENAI_KEY: 'sk-test',
        AWS_ACCESS_KEY_ID: 'AKIATEST',
        CURSOR_TRACE_ID: 't1',
        WORKSPACE_FOLDER_PATHS: '/repos/alpha',
      },
      '/tmp',
      new Date('2026-08-21T00:00:00.000Z')
    );
    expect(record).toContain('Authorization:Bearer <redacted>');
    expect(record).not.toContain('mcpk_abc');
    expect(record).toContain('OPENAI_KEY=<redacted>');
    expect(record).toContain('AWS_ACCESS_KEY_ID=<redacted>');
    expect(record).toContain('PATH=<redacted>');
    expect(record).not.toContain('sk-test');
    expect(record).not.toContain('AKIATEST');
    expect(record).toContain('CURSOR_TRACE_ID=t1');
    expect(record).toContain('WORKSPACE_FOLDER_PATHS=/repos/alpha');
  });
});
