/**
 * Create-time / edit-time server ID helpers.
 */

import { describe, it, expect } from 'vitest';
import { normalizeServerId, pendingServerRename } from '@/lib/api/serverClone';

describe('pendingServerRename', () => {
  it('returns null when the typed ID is unchanged after normalize', () => {
    expect(pendingServerRename('slack-foj', 'Slack-Foj')).toBeNull();
    expect(pendingServerRename('slack-foj', 'slack-foj')).toBeNull();
  });

  it('returns null for empty/invalid IDs', () => {
    expect(pendingServerRename('slack-foj', '___')).toBeNull();
    expect(pendingServerRename('slack-foj', '   ')).toBeNull();
  });

  it('returns the normalized ID when it actually changes', () => {
    expect(pendingServerRename('slack-s2h-foj', 'Slack FOJ')).toBe('slackfoj');
    expect(pendingServerRename('slack-s2h-foj', 'slack-foj')).toBe('slack-foj');
  });
});

describe('normalizeServerId', () => {
  it('matches backend lowercase + strip spaces/underscores', () => {
    expect(normalizeServerId('Slack_S2H FOJ')).toBe('slacks2hfoj');
    expect(normalizeServerId('slack-foj')).toBe('slack-foj');
  });
});
