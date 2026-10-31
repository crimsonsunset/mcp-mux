/**
 * Projects page: a client-id (`id`) mapping is shown by the client's name.
 *
 * Ported from upstream #238. The fork keeps its own Projects UI, so this pins
 * only the behavior taken from upstream: a mapping keyed by a client id reads
 * as the client's alias/name instead of a truncated id, can be found by that
 * name, and falls back to the short id when the client list is unavailable.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const { listWorkspaceBindingsMock, listOAuthClientsMock } = vi.hoisted(() => ({
  listWorkspaceBindingsMock: vi.fn(),
  listOAuthClientsMock: vi.fn(),
}));

vi.mock('@/lib/api/workspaceBindings', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/api/workspaceBindings')>();
  return {
    ...actual,
    listWorkspaceBindings: listWorkspaceBindingsMock,
    listReportedWorkspaceRoots: vi.fn().mockResolvedValue([]),
    clearUnmappedReportedRoots: vi.fn(),
    createWorkspaceBinding: vi.fn(),
    updateWorkspaceBinding: vi.fn(),
    deleteWorkspaceBinding: vi.fn(),
    getWorkspaceEffectiveFeatures: vi.fn(),
    validateWorkspaceRoot: vi.fn(),
  };
});

vi.mock('@/lib/api/featureSets', () => ({
  listFeatureSets: vi.fn().mockResolvedValue([]),
  isStarterFeatureSet: vi.fn(() => false),
}));

vi.mock('@/lib/api/gateway', () => ({
  listOAuthClients: listOAuthClientsMock,
}));

vi.mock('@/lib/api/workspaceAppearances', () => ({
  listWorkspaceAppearances: vi.fn().mockResolvedValue([]),
  deleteWorkspaceAppearance: vi.fn(),
  upsertWorkspaceAppearance: vi.fn(),
  uploadWorkspaceIcon: vi.fn(),
}));

vi.mock('@/lib/api/machines', () => ({
  listMachines: vi.fn().mockResolvedValue([]),
  getLocalMachineId: vi.fn().mockResolvedValue(null),
}));

vi.mock('@/lib/backend/events', () => ({
  useWorkspaceEvents: () => ({
    subscribe: vi.fn(() => () => {}),
    subscribeMany: vi.fn(() => () => {}),
  }),
  useWorkspaceEventListener: vi.fn(),
}));

vi.mock('@/stores', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/stores')>();
  return {
    ...actual,
    useSpaces: () => [{ id: 's1', name: 'Space One' }],
    usePendingWorkspaceNew: () => false,
    useSetPendingWorkspaceNew: () => () => {},
  };
});

vi.mock('@/hooks/use-viewer-identity.hook', () => ({
  useViewerIdentity: () => ({ machineId: null, isLoading: false }),
  ViewerIdentityProvider: ({ children }: { children: React.ReactNode }) => children,
}));

import { renderWithI18n } from '../render-with-i18n.helpers';
import { WorkspacesPage } from '@/features/workspaces/WorkspacesPage';

const CLIENT_ID = 'mcp_0123456789abcdef';
const ENTRY_TESTID = 'workspace-entry-b-client';

const client = {
  client_id: CLIENT_ID,
  client_name: 'Cursor',
  client_alias: 'Cursor (work)',
};

describe('WorkspacesPage – client mappings are named', () => {
  beforeEach(() => {
    listWorkspaceBindingsMock.mockResolvedValue([
      {
        id: 'b-client',
        workspace_root: CLIENT_ID,
        binding_type: 'id',
        space_id: 's1',
        feature_set_ids: [],
      },
    ]);
  });

  it('titles the card with the client alias, keeping the raw id as the secondary line', async () => {
    listOAuthClientsMock.mockResolvedValue([client]);
    renderWithI18n(<WorkspacesPage />);

    const card = await screen.findByTestId(ENTRY_TESTID);
    const title = card.querySelector('p[title="Cursor (work)"]');
    expect(title).not.toBeNull();
    expect(title?.textContent).toBe('Cursor (work)');
    expect(card.querySelector(`p[title="${CLIENT_ID}"]`)).not.toBeNull();
  });

  it('finds the mapping by searching the client name', async () => {
    listOAuthClientsMock.mockResolvedValue([client]);
    const user = userEvent.setup();
    renderWithI18n(<WorkspacesPage />);
    await screen.findByTestId(ENTRY_TESTID);

    await user.type(screen.getByRole('textbox'), 'work');

    expect(screen.getByTestId(ENTRY_TESTID)).toBeTruthy();
  });

  it('hides the mapping when the search matches neither the id nor the client name', async () => {
    listOAuthClientsMock.mockResolvedValue([client]);
    const user = userEvent.setup();
    renderWithI18n(<WorkspacesPage />);
    await screen.findByTestId(ENTRY_TESTID);

    await user.type(screen.getByRole('textbox'), 'windsurf');

    expect(screen.queryByTestId(ENTRY_TESTID)).toBeNull();
  });

  it('falls back to the short id when the client list is unavailable', async () => {
    listOAuthClientsMock.mockRejectedValue(new Error('gateway stopped'));
    renderWithI18n(<WorkspacesPage />);

    const card = await screen.findByTestId(ENTRY_TESTID);
    expect(card.textContent).not.toContain('Cursor');
    expect(card.textContent).toContain(CLIENT_ID.slice(0, 6));
  });
});
