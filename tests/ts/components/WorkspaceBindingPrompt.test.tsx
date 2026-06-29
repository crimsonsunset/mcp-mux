/**
 * WorkspaceBindingPanel — the "map this folder?" prompt and its disable switch.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { invoke } from '@tauri-apps/api/core';
import { renderWithI18n } from '../render-with-i18n.helpers';

const { workspaceHandlers } = vi.hoisted(() => ({
  workspaceHandlers: new Map<string, (payload: unknown) => void>(),
}));

vi.mock('@/lib/api/workspaceBindings', () => ({
  createWorkspaceBinding: vi.fn(),
  listWorkspaceBindings: vi.fn().mockResolvedValue([]),
  validateWorkspaceRoot: vi.fn().mockResolvedValue('/home/u/proj'),
  getWorkspaceEffectiveFeatures: vi.fn().mockResolvedValue({
    workspace_root: '/home/u/proj',
    source: 'unbound',
    binding_id: null,
    space_id: 's1',
    space_name: 'Default',
    feature_sets: [{ id: 'fs1', name: 'Starter', feature_set_type: 'starter' }],
    tools: [],
    prompts: [],
    resources: [],
    server_totals: {},
  }),
}));

vi.mock('@/lib/api/spaces', () => ({
  listSpaces: vi.fn().mockResolvedValue([{ id: 's1', name: 'Default', is_default: true }]),
}));

vi.mock('@/lib/api/featureSets', () => ({
  isStarterFeatureSet: vi.fn(() => true),
  listFeatureSets: vi
    .fn()
    .mockResolvedValue([
      { id: 'fs1', name: 'Starter', feature_set_type: 'starter', space_id: 's1', is_deleted: false },
    ]),
}));

vi.mock('@/lib/api/machines', () => ({
  listMachines: vi.fn().mockResolvedValue([]),
  getLocalMachineId: vi.fn().mockResolvedValue(null),
  getClientMachineId: vi.fn().mockResolvedValue(null),
}));

vi.mock('@/lib/backend/events', () => ({
  useWorkspaceEvents: () => ({
    subscribe: (channel: string, cb: (payload: unknown) => void) => {
      workspaceHandlers.set(channel, cb);
      return () => workspaceHandlers.delete(channel);
    },
    subscribeMany: vi.fn(() => () => {}),
  }),
  useWorkspaceEventListener: vi.fn(),
}));

import { WorkspaceBindingPanel } from '@/features/workspaces/workspace-binding-panel.component';
import { useBindingPanelStore } from '@/stores/bindingPanelStore';

const PROMPT_COPY = /You just opened this folder in a connected app/i;

/** Invoke the captured `workspace-needs-binding` listener with a payload. */
async function fireNeedsBinding(overrides: Record<string, unknown> = {}) {
  const cb = workspaceHandlers.get('workspace-needs-binding');
  if (!cb) throw new Error('workspace-needs-binding listener was not registered');
  await cb({
    client_id: 'c',
    session_id: 's',
    space_id: 's1',
    workspace_root: '/home/u/proj',
    ...overrides,
  });
}

function mockPromptEnabled(enabled: boolean) {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    if (cmd === 'get_workspace_mapping_prompt_enabled') return enabled;
    return undefined;
  });
}

describe('WorkspaceBindingPanel – mapping prompt toggle', () => {
  beforeEach(() => {
    workspaceHandlers.clear();
    vi.mocked(invoke).mockReset();
    useBindingPanelStore.getState().close();
  });

  it('shows the panel when the prompt setting is enabled', async () => {
    mockPromptEnabled(true);
    renderWithI18n(<WorkspaceBindingPanel />);
    await fireNeedsBinding();
    expect(await screen.findByTestId('workspace-binding-panel')).toBeTruthy();
    expect(await screen.findByText(PROMPT_COPY)).toBeTruthy();
  });

  it('does NOT show the panel when the prompt setting is disabled', async () => {
    mockPromptEnabled(false);
    renderWithI18n(<WorkspaceBindingPanel />);
    await fireNeedsBinding();
    await waitFor(() => expect(screen.queryByTestId('workspace-binding-panel')).toBeNull());
  });

  it('the in-panel "stop asking" link disables the setting and closes', async () => {
    const user = userEvent.setup();
    mockPromptEnabled(true);
    renderWithI18n(<WorkspaceBindingPanel />);
    await fireNeedsBinding();
    await screen.findByTestId('workspace-binding-panel');

    await user.click(screen.getByTestId('workspace-binding-disable-prompt'));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('set_workspace_mapping_prompt_enabled', {
        enabled: false,
      }),
    );
    await waitFor(() => expect(screen.queryByTestId('workspace-binding-panel')).toBeNull());
  });

  it('locks the Space picker when the folder is base-dir scoped', async () => {
    mockPromptEnabled(true);
    renderWithI18n(<WorkspaceBindingPanel />);
    await fireNeedsBinding({ space_locked: true });
    await screen.findByTestId('workspace-binding-panel');

    const picker = screen.getByTestId('workspace-binding-space-picker') as HTMLSelectElement;
    expect(picker.disabled).toBe(true);
  });

  it('leaves the Space picker editable for an ordinary unmapped folder', async () => {
    mockPromptEnabled(true);
    renderWithI18n(<WorkspaceBindingPanel />);
    await fireNeedsBinding({ space_locked: false });
    await screen.findByTestId('workspace-binding-panel');

    const picker = screen.getByTestId('workspace-binding-space-picker') as HTMLSelectElement;
    expect(picker.disabled).toBe(false);
  });
});
