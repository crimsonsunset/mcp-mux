import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event';
import { open, type OpenDialogOptions } from '@tauri-apps/plugin-dialog';
import { relaunch } from '@tauri-apps/plugin-process';
import type { Update } from '@tauri-apps/plugin-updater';

import type {
  ExportClientType,
  ExportConfigRequest,
  ExportConfigResponse,
} from '@/lib/api/configExport';
import type { AdminWebSettings } from '@/lib/api/settings';

import { isTauri } from '../data/transport';

export { isTauri };

/**
 * Subscribe to a Tauri event only in the desktop shell.
 */
export async function listenWhenTauri<T>(
  event: string,
  handler: (event: Event<T>) => void
): Promise<UnlistenFn | undefined> {
  if (!isTauri()) {
    return undefined;
  }
  return listen(event, handler);
}

/**
 * Convert an absolute filesystem path to a webview-safe asset URL (desktop only).
 */
export function fileSrcFromAbsolutePath(absolutePath: string | null): string | null {
  if (!absolutePath || !isTauri()) {
    return null;
  }
  return convertFileSrc(absolutePath);
}

/**
 * Open the native file/directory picker (desktop only).
 */
export async function pickPath(
  options: OpenDialogOptions
): Promise<string | string[] | null> {
  if (!isTauri()) {
    return null;
  }
  const selected = await open(options);
  if (selected === null) {
    return null;
  }
  return selected;
}

/**
 * Flush a cold-start OAuth deep link after the consent listener is ready (desktop only).
 */
export async function flushPendingDeepLink(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke('flush_pending_deep_link');
}

/** Payload for OAuth consent deep links on desktop. */
export interface OAuthConsentDeepLinkPayload {
  requestId: string;
}

/**
 * Subscribe to OAuth consent deep-link events and flush any buffered URL (desktop only).
 */
export async function subscribeOAuthConsentRequest(
  handler: (payload: OAuthConsentDeepLinkPayload) => void
): Promise<UnlistenFn | undefined> {
  if (!isTauri()) {
    return undefined;
  }
  const unlisten = await listen<OAuthConsentDeepLinkPayload>(
    'oauth-consent-request',
    (event) => {
      handler(event.payload);
    }
  );
  void flushPendingDeepLink().catch((err) => {
    console.warn('[OAuth] flush_pending_deep_link failed:', err);
  });
  return unlisten;
}

/**
 * Subscribe to OAuth consent requests (Tauri events on desktop, SSE on web admin).
 */
export function subscribeOAuthConsentEvents(
  handler: (payload: OAuthConsentDeepLinkPayload) => void
): () => void {
  if (isTauri()) {
    let unlisten: UnlistenFn | undefined;
    void subscribeOAuthConsentRequest(handler).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }

  const source = new EventSource('/api/v1/events');
  const onConsentRequest = (event: MessageEvent<string>) => {
    try {
      const payload = JSON.parse(event.data) as OAuthConsentDeepLinkPayload;
      if (payload.requestId) {
        handler(payload);
      }
    } catch {
      // ignore malformed SSE frames
    }
  };
  source.addEventListener('oauth-consent-request', onConsentRequest);
  return () => {
    source.removeEventListener('oauth-consent-request', onConsentRequest);
    source.close();
  };
}

/**
 * Open the application logs folder in the system file manager (desktop only).
 */
export async function openLogsFolder(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke('open_logs_folder');
}

/**
 * Load web admin HTTP server settings (desktop control plane only).
 */
export async function getAdminWebSettings(): Promise<AdminWebSettings> {
  return invoke('get_admin_web_settings');
}

/**
 * Persist web admin settings and restart the admin HTTP server (desktop only).
 */
export async function updateAdminWebSettings(settings: AdminWebSettings): Promise<void> {
  await invoke('update_admin_web_settings', { settings });
}

/**
 * Reveal a space config file in the system editor (desktop only).
 */
export async function openSpaceConfigFile(spaceId: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke('open_space_config_file', { spaceId });
}

/**
 * Add McpMux to VS Code via deep link (desktop only).
 */
export async function addToVscode(gatewayUrl: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke('add_to_vscode', { gatewayUrl });
}

/**
 * Add McpMux to Cursor via deep link (desktop only).
 */
export async function addToCursor(gatewayUrl: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke('add_to_cursor', { gatewayUrl });
}

/**
 * Preview generated MCP client config JSON without writing to disk (desktop only).
 */
export async function previewConfigExport(
  request: ExportConfigRequest
): Promise<ExportConfigResponse> {
  return invoke('preview_config_export', { request });
}

/**
 * Write generated MCP client config JSON to the given file path (desktop only).
 */
export async function exportConfigToFile(
  request: ExportConfigRequest,
  path: string
): Promise<string> {
  return invoke('export_config_to_file', { request, path });
}

/**
 * Default config file paths per client type (desktop only).
 */
export async function getConfigPaths(): Promise<Record<string, string | null>> {
  return invoke('get_config_paths');
}

/**
 * Whether a config file exists at the default path for a client type (desktop only).
 */
export async function checkConfigExists(clientType: ExportClientType): Promise<boolean> {
  return invoke('check_config_exists', { clientType });
}

/**
 * Copy an existing default config to a `.json.bak` sibling before overwrite (desktop only).
 */
export async function backupExistingConfig(
  clientType: ExportClientType
): Promise<string | null> {
  return invoke('backup_existing_config', { clientType });
}

/**
 * Check the Tauri updater for an available release (desktop only).
 */
export async function checkForAvailableUpdate(): Promise<{ version: string } | null> {
  if (!isTauri()) {
    return null;
  }
  const { check } = await import('@tauri-apps/plugin-updater');
  const update = await check();
  if (!update) {
    return null;
  }
  return { version: update.version };
}

/**
 * Run the Tauri updater check and return the full update handle (desktop only).
 */
export async function checkAppUpdate(): Promise<Update | null> {
  if (!isTauri()) {
    return null;
  }
  const { check } = await import('@tauri-apps/plugin-updater');
  return check();
}

/**
 * Relaunch the desktop app after installing an update (desktop only).
 */
export async function relaunchApp(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await relaunch();
}
