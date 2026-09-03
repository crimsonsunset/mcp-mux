/**
 * CloneAccountModal — create-time server ID is editable and not clobbered after edit.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fireEvent, screen, waitFor } from '@testing-library/react';
import { renderWithI18n } from '../render-with-i18n.helpers';
import type { ServerViewModel } from '@/types/registry';

const {
  mockCloneServer,
  mockIsCloneIdAvailable,
  mockSuggestCloneSuffix,
} = vi.hoisted(() => ({
  mockCloneServer: vi.fn(),
  mockIsCloneIdAvailable: vi.fn(),
  mockSuggestCloneSuffix: vi.fn(),
}));

vi.mock('@/lib/api/serverClone', async () => {
  const actual = await vi.importActual<typeof import('@/lib/api/serverClone')>(
    '@/lib/api/serverClone'
  );
  return {
    ...actual,
    cloneServer: mockCloneServer,
    isCloneIdAvailable: mockIsCloneIdAvailable,
    suggestCloneSuffix: mockSuggestCloneSuffix,
  };
});

import { CloneAccountModal } from '@/features/servers/CloneAccountModal';

function sourceServer(): ServerViewModel {
  return {
    id: 'slack-s2h',
    name: 'Slack S2H',
    description: null,
    alias: 'slack',
    auth: null,
    icon: null,
    transport: {
      type: 'http',
      url: 'https://mcp.slack.com',
      headers: {},
      metadata: { inputs: [] },
    },
    categories: [],
    publisher: null,
    source: { type: 'Bundled' },
    is_installed: true,
    enabled: false,
    oauth_connected: false,
    input_values: {},
    connection_status: 'disconnected',
    missing_required_inputs: false,
    last_error: null,
  };
}

describe('CloneAccountModal', () => {
  beforeEach(() => {
    mockCloneServer.mockReset();
    mockIsCloneIdAvailable.mockReset().mockResolvedValue(true);
    mockSuggestCloneSuffix.mockReset().mockResolvedValue('foj');
  });

  it('prefills an editable server ID from the derived value', async () => {
    renderWithI18n(
      <CloneAccountModal
        open
        spaceId="space-1"
        sourceServer={sourceServer()}
        onClose={() => {}}
        onCloned={() => {}}
      />
    );

    const idInput = await screen.findByTestId('clone-server-id-input');
    await waitFor(() => expect(idInput).toHaveValue('slack-s2h-foj'));
    expect(idInput).not.toHaveAttribute('readonly');
  });

  it('does not clobber the ID after the user edits it', async () => {
    renderWithI18n(
      <CloneAccountModal
        open
        spaceId="space-1"
        sourceServer={sourceServer()}
        onClose={() => {}}
        onCloned={() => {}}
      />
    );

    const idInput = await screen.findByTestId('clone-server-id-input');
    await waitFor(() => expect(idInput).toHaveValue('slack-s2h-foj'));

    fireEvent.change(idInput, { target: { value: 'slack-foj' } });
    expect(idInput).toHaveValue('slack-foj');

    fireEvent.click(screen.getByTestId('clone-suffix-suggestion-work'));
    expect(screen.getByTestId('clone-suffix-input')).toHaveValue('work');
    expect(idInput).toHaveValue('slack-foj');
  });

  it('collision-checks the typed ID and disables Create', async () => {
    mockIsCloneIdAvailable.mockImplementation(
      async (_space: string, _source: string, _suffix: string, serverId?: string) =>
        serverId !== 'taken-id'
    );

    renderWithI18n(
      <CloneAccountModal
        open
        spaceId="space-1"
        sourceServer={sourceServer()}
        onClose={() => {}}
        onCloned={() => {}}
      />
    );

    const idInput = await screen.findByTestId('clone-server-id-input');
    await waitFor(() => expect(idInput).toHaveValue('slack-s2h-foj'));
    fireEvent.change(idInput, { target: { value: 'taken-id' } });

    await waitFor(() => {
      expect(mockIsCloneIdAvailable).toHaveBeenCalledWith(
        'space-1',
        'slack-s2h',
        'foj',
        'taken-id'
      );
    });
    await waitFor(() => {
      expect(screen.getByTestId('clone-collision-error')).toBeInTheDocument();
      expect(screen.getByTestId('clone-submit-btn')).toBeDisabled();
    });
  });

  it('submits the typed ID', async () => {
    mockCloneServer.mockResolvedValue({
      server_id: 'slack-foj',
      cloned_from: 'slack-s2h',
    });
    const onCloned = vi.fn();

    renderWithI18n(
      <CloneAccountModal
        open
        spaceId="space-1"
        sourceServer={sourceServer()}
        onClose={() => {}}
        onCloned={onCloned}
      />
    );

    const idInput = await screen.findByTestId('clone-server-id-input');
    await waitFor(() => expect(idInput).toHaveValue('slack-s2h-foj'));
    fireEvent.change(idInput, { target: { value: 'slack-foj' } });

    await waitFor(() => expect(screen.getByTestId('clone-submit-btn')).toBeEnabled());
    fireEvent.click(screen.getByTestId('clone-submit-btn'));

    await waitFor(() => {
      expect(mockCloneServer).toHaveBeenCalledWith(
        'space-1',
        'slack-s2h',
        'foj',
        undefined,
        undefined,
        'slack-foj'
      );
    });
    expect(onCloned).toHaveBeenCalled();
  });
});
