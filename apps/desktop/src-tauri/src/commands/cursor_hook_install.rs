//! Tauri IPC wrappers for the managed Cursor `preToolUse` hook installer.

use mcpmux_gateway::cursor_hook::{self, CursorHookResult};

/// Current install state of the managed Cursor hook.
#[tauri::command]
pub fn cursor_hook_status() -> CursorHookResult {
    cursor_hook::status()
}

/// Install or update the managed `preToolUse` hook.
#[tauri::command]
pub fn install_cursor_hook() -> CursorHookResult {
    cursor_hook::install()
}

/// Remove the managed hook entry and delete the managed script.
#[tauri::command]
pub fn uninstall_cursor_hook() -> CursorHookResult {
    cursor_hook::uninstall()
}
