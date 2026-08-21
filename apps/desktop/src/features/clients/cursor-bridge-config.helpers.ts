/** Default API-key client name for the global Cursor mcp-remote bridge. */
export const CURSOR_BRIDGE_CLIENT_NAME = 'cursor-global-bridge';

/**
 * Build the `~/.cursor/mcp.json` snippet for the global mcp-remote bridge.
 *
 * Cursor resolves `${workspaceFolder}` in `args` at spawn time, so one global
 * entry routes each window to the correct workspace header. That substitution
 * is unreliable (measured at ~21% failure across 282 spawns), and when it
 * fails `mcp-remote` expands the leftover literal to an empty header value.
 * `${WORKSPACE_FOLDER_PATHS}` is not a Cursor variable, so it survives to
 * `mcp-remote`, which expands it from the child environment — giving the
 * gateway the window's full folder set even when the active folder is missing.
 * The set constrains which root the session may claim; it does not pick one.
 */
export function buildCursorBridgeMcpJson(apiKey: string, gatewayUrl: string): string {
  const mcpUrl = `${gatewayUrl.replace(/\/$/, '')}/mcp`;
  const config = {
    mcpServers: {
      mcpmux: {
        command: 'npx',
        args: [
          '-y',
          'mcp-remote',
          mcpUrl,
          '--allow-http',
          '--header',
          'X-Mcpmux-Workspace:${workspaceFolder}',
          '--header',
          'X-Mcpmux-Workspace-Set:${WORKSPACE_FOLDER_PATHS}',
          '--header',
          'Authorization:Bearer ${MCPMUX_API_KEY}',
        ],
        env: { MCPMUX_API_KEY: apiKey },
      },
    },
  };
  return JSON.stringify(config, null, 2);
}
