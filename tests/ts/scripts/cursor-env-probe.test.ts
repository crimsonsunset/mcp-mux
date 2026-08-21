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
    expect(isSecretEnvKey('CURSOR_TRACE_ID')).toBe(false);

    const record = formatProbeRecord(
      ['npx', '-y', 'mcp-remote', '--header', 'Authorization:Bearer mcpk_abc'],
      { PATH: '/usr/bin', OPENAI_API_KEY: 'sk-test', CURSOR_TRACE_ID: 't1' },
      '/tmp',
      new Date('2026-08-21T00:00:00.000Z')
    );
    expect(record).toContain('Authorization:Bearer <redacted>');
    expect(record).not.toContain('mcpk_abc');
    expect(record).toContain('OPENAI_API_KEY=<redacted>');
    expect(record).not.toContain('sk-test');
    expect(record).toContain('CURSOR_TRACE_ID=t1');
  });
});
