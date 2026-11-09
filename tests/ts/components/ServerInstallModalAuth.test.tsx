/**
 * The deep-link install modal (`mcpmux://install?server=...`) shows an auth
 * badge for the server being installed. A `basic` (username/password) server
 * must not be labeled "API Key".
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, act } from '@testing-library/react';
import type { AuthConfig, ServerDefinition } from '@/types/registry';

const { handlers, mockGetServerDefinition } = vi.hoisted(() => ({
  handlers: new Map<string, (e: { payload: unknown }) => void>(),
  mockGetServerDefinition: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((name: string, cb: (e: { payload: unknown }) => void) => {
    handlers.set(name, cb);
    return Promise.resolve(() => {});
  }),
}));
vi.mock('@/lib/api/spaces', () => ({
  listSpaces: vi.fn().mockResolvedValue([{ id: 'space-1', name: 'My Space' }]),
}));
vi.mock('@/lib/api/registry', () => ({
  getServerDefinition: mockGetServerDefinition,
  installServer: vi.fn(),
  listInstalledServers: vi.fn().mockResolvedValue([]),
}));
vi.mock('@/stores', () => ({
  useViewSpace: () => ({ id: 'space-1', name: 'My Space' }),
}));

import { renderWithI18n } from '../render-with-i18n.helpers';
import { ServerInstallModal } from '@/components/ServerInstallModal';

function serverWithAuth(auth: AuthConfig): ServerDefinition {
  return {
    id: 'com.example.server',
    name: 'Example Server',
    description: null,
    alias: null,
    icon: null,
    auth,
    transport: { type: 'stdio', command: 'npx', args: [], env: {}, metadata: { inputs: [] } },
    categories: [],
    publisher: null,
    source: { type: 'Registry', url: 'https://api.mcpmux.com', name: 'McpMux Registry' },
  } as ServerDefinition;
}

async function openDeepLink() {
  await act(async () => {
    handlers.get('server-install-request')?.({ payload: { serverId: 'com.example.server' } });
  });
}

describe('ServerInstallModal auth badge', () => {
  beforeEach(() => {
    handlers.clear();
    mockGetServerDefinition.mockReset();
  });

  it.each<[AuthConfig, string]>([
    [{ type: 'api_key', instructions: null }, 'API Key'],
    [{ type: 'optional_api_key', instructions: null }, 'API Key'],
    [{ type: 'basic', instructions: null }, 'Username & Password'],
    [{ type: 'oauth' }, 'OAuth'],
  ])('labels %j as "%s"', async (auth, label) => {
    mockGetServerDefinition.mockResolvedValue(serverWithAuth(auth));
    renderWithI18n(<ServerInstallModal />);
    await openDeepLink();

    const info = await screen.findByTestId('install-modal-server-info');
    expect(info).toHaveTextContent(label);
  });

  it('shows no auth badge for a server without auth', async () => {
    mockGetServerDefinition.mockResolvedValue(serverWithAuth({ type: 'none' }));
    renderWithI18n(<ServerInstallModal />);
    await openDeepLink();

    const info = await screen.findByTestId('install-modal-server-info');
    expect(info).not.toHaveTextContent('API Key');
    expect(info).not.toHaveTextContent('Username & Password');
  });
});
