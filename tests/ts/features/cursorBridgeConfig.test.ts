import { describe, it, expect } from 'vitest';
import {
  buildCursorBridgeMcpJson,
  localMcpUrlForCursorBridge,
} from '../../../apps/desktop/src/features/clients/cursor-bridge-config.helpers';

const API_KEY = 'mcpk_test_key_123';

describe('buildCursorBridgeMcpJson', () => {
  it('should inline the api key so the auth header needs no substitution', () => {
    const args = JSON.parse(buildCursorBridgeMcpJson(API_KEY, 'http://127.0.0.1:45818')).mcpServers
      .mcpmux.args as string[];

    expect(args).toContain(`Authorization:Bearer ${API_KEY}`);
  });

  it('should not route the key through env', () => {
    const entry = JSON.parse(buildCursorBridgeMcpJson(API_KEY, 'http://127.0.0.1:45818')).mcpServers
      .mcpmux;

    expect(entry.env).toBeUndefined();
  });

  it('should leave no template in the auth header', () => {
    // The regression this guards: a Cursor respawn that skips env substitution
    // sends the literal `${MCPMUX_API_KEY}`, the gateway 401s, and Cursor
    // surfaces only a connection timeout.
    const args = JSON.parse(buildCursorBridgeMcpJson(API_KEY, 'http://127.0.0.1:45818')).mcpServers
      .mcpmux.args as string[];
    const authHeader = args.find((arg) => arg.startsWith('Authorization:'));

    expect(authHeader).toBeDefined();
    expect(authHeader).not.toContain('${');
  });

  it('should keep the per-window workspace headers as templates', () => {
    // These two must stay variables — unlike the key, their value differs per
    // Cursor window, which is the whole point of the global bridge entry.
    const args = JSON.parse(buildCursorBridgeMcpJson(API_KEY, 'http://127.0.0.1:45818')).mcpServers
      .mcpmux.args as string[];

    expect(args).toContain('X-Mcpmux-Workspace:${workspaceFolder}');
    expect(args).toContain('X-Mcpmux-Workspace-Set:${WORKSPACE_FOLDER_PATHS}');
  });
});

describe('localMcpUrlForCursorBridge', () => {
  it('should keep a custom loopback port', () => {
    expect(localMcpUrlForCursorBridge('http://127.0.0.1:45999')).toBe('http://127.0.0.1:45999/mcp');
  });

  it('should rewrite a public tunnel url to the local bind', () => {
    expect(localMcpUrlForCursorBridge('https://mcp.example.com')).toBe(
      'http://127.0.0.1:45818/mcp'
    );
  });
});
