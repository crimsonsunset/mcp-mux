/** Default API-key client name for the global Cursor mcp-remote bridge. */
export const CURSOR_BRIDGE_CLIENT_NAME = 'cursor-global-bridge';

const DEFAULT_LOOPBACK_PORT = '45818';

/**
 * Resolve the MCP URL a local `mcp-remote` child should hit.
 *
 * The Cursor bridge is stdio on the same machine as the gateway. An advertised
 * public/tunnel URL (Cloudflare Access) 403s because `mcp-remote` has no Access
 * cookies. Non-loopback inputs fall back to `127.0.0.1:45818`; a loopback input
 * keeps its port so a custom local bind still pastes correctly.
 */
export function localMcpUrlForCursorBridge(gatewayUrl: string): string {
  const trimmed = gatewayUrl.trim().replace(/\/$/, '');
  try {
    const parsed = new URL(trimmed.includes('://') ? trimmed : `http://${trimmed}`);
    const host = parsed.hostname.replace(/^\[|\]$/g, '').toLowerCase();
    const isLoopback = host === 'localhost' || host === '127.0.0.1' || host === '::1';
    if (isLoopback) {
      const port = parsed.port || DEFAULT_LOOPBACK_PORT;
      return `http://127.0.0.1:${port}/mcp`;
    }
  } catch {
    // advertised URL or junk — use the default local bind
  }
  return `http://127.0.0.1:${DEFAULT_LOOPBACK_PORT}/mcp`;
}

/**
 * Build the `~/.cursor/mcp.json` snippet for the global mcp-remote bridge.
 *
 * Always targets loopback (see `localMcpUrlForCursorBridge`) so the snippet is
 * paste-ready even when the UI is advertising a Cloudflare tunnel.
 *
 * Cursor resolves `${workspaceFolder}` in `args` at spawn time, so one global
 * entry routes each window to the correct workspace header. That substitution
 * is unreliable (measured at ~21% failure across 282 spawns), and when it
 * fails `mcp-remote` expands the leftover literal to an empty header value.
 * `${WORKSPACE_FOLDER_PATHS}` is not a Cursor variable, so it survives to
 * `mcp-remote`, which expands it from the child environment — giving the
 * gateway the window's full folder set even when the active folder is missing.
 * The set constrains which root the session may claim; it does not pick one.
 *
 * The key is inlined rather than referenced through `env.MCPMUX_API_KEY`. Both
 * live in this one file at the same permissions, so the indirection bought no
 * secrecy — only exposure to the same substitution flake: a Cursor MCP respawn
 * was observed sending the literal `${MCPMUX_API_KEY}`, which the gateway
 * correctly 401s while the client sits on a connection timeout. The workspace
 * variables above have to stay variables because they differ per window; a
 * constant does not.
 */
export function buildCursorBridgeMcpJson(apiKey: string, gatewayUrl: string): string {
  const mcpUrl = localMcpUrlForCursorBridge(gatewayUrl);
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
          `Authorization:Bearer ${apiKey}`,
        ],
      },
    },
  };
  return JSON.stringify(config, null, 2);
}

/**
 * Manual `~/.cursor/hooks.json` fallback when the installer refuses JSONC.
 *
 * `scriptPath` should be the absolute managed script path from the Tauri
 * status/install result. Used only as copy-paste text.
 */
export function buildCursorHookFallbackJson(scriptPath: string): string {
  return JSON.stringify(
    {
      version: 1,
      hooks: {
        preToolUse: [
          {
            command: `node ${scriptPath}`,
            matcher: 'MCP:mcpmux_.*',
            timeout: 5,
          },
        ],
      },
    },
    null,
    2
  );
}
