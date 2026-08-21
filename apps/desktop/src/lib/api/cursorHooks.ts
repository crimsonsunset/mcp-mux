import { invoke } from '@tauri-apps/api/core';

/** Result of a Cursor hook install, status, or uninstall command. */
export interface CursorHookResult {
  action: string;
  installed: boolean;
  hooks_path: string;
  script_path: string;
  backed_up: string | null;
  error: string | null;
  jsonc_refused: boolean;
  manual_entry: string;
}

/**
 * Read whether the managed `preToolUse` hook is installed.
 */
export async function getCursorHookStatus(): Promise<CursorHookResult> {
  return invoke('cursor_hook_status');
}

/**
 * Write the managed script and merge one `preToolUse` entry into hooks.json.
 */
export async function installCursorHook(): Promise<CursorHookResult> {
  return invoke('install_cursor_hook');
}

/**
 * Remove the managed hook entry and delete the managed script.
 */
export async function uninstallCursorHook(): Promise<CursorHookResult> {
  return invoke('uninstall_cursor_hook');
}
