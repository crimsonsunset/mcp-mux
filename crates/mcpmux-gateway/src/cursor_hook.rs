//! One-click install of the managed Cursor `preToolUse` workspace-context hook.
//!
//! Writes `~/.cursor/hooks/mcpmux-workspace-context.js` and merges one
//! `preToolUse` entry into plain `~/.cursor/hooks.json`. Refuses JSONC and
//! non-object shapes; preserves every unrelated hook.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Value};

const SCRIPT_NAME: &str = "mcpmux-workspace-context.js";
const SCRIPT_SOURCE: &str = include_str!("../scripts/mcpmux-workspace-context.js");
const MATCHER: &str = "MCP:mcpmux_.*";
const TIMEOUT_SECS: u64 = 5;

/// Result of install / status / uninstall.
#[derive(Debug, Clone, Serialize)]
pub struct CursorHookResult {
    pub action: String,
    pub installed: bool,
    pub hooks_path: String,
    pub script_path: String,
    pub backed_up: Option<String>,
    pub error: Option<String>,
    pub jsonc_refused: bool,
    pub manual_entry: String,
}

impl CursorHookResult {
    /// JSON body for the admin HTTP handlers.
    pub fn into_json(self) -> Value {
        serde_json::to_value(self).expect("CursorHookResult serializes")
    }
}

fn cursor_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "home directory not found".to_string())?;
    Ok(home.join(".cursor"))
}

fn hooks_json_path() -> Result<PathBuf, String> {
    Ok(cursor_dir()?.join("hooks.json"))
}

fn script_path() -> Result<PathBuf, String> {
    Ok(cursor_dir()?.join("hooks").join(SCRIPT_NAME))
}

fn hook_command(script: &Path) -> String {
    format!("node {}", script.display())
}

fn managed_entry(script: &Path) -> Value {
    json!({
        "command": hook_command(script),
        "matcher": MATCHER,
        "timeout": TIMEOUT_SECS,
    })
}

fn manual_entry_text(script: &Path) -> String {
    serde_json::to_string_pretty(&json!({
        "version": 1,
        "hooks": {
            "preToolUse": [managed_entry(script)]
        }
    }))
    .unwrap_or_else(|_| "{}".into())
}

fn empty_result(action: &str, error: Option<String>, jsonc_refused: bool) -> CursorHookResult {
    let hooks = hooks_json_path()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let script = script_path()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let manual = script_path()
        .map(|p| manual_entry_text(&p))
        .unwrap_or_else(|_| "{}".into());
    CursorHookResult {
        action: action.into(),
        installed: false,
        hooks_path: hooks,
        script_path: script,
        backed_up: None,
        error,
        jsonc_refused,
        manual_entry: manual,
    }
}

fn is_managed_entry(value: &Value, script: &Path) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    let command = obj.get("command").and_then(Value::as_str).unwrap_or("");
    command.contains(SCRIPT_NAME) || command == hook_command(script)
}

fn parse_hooks_json(existing: &str) -> Result<Value, String> {
    serde_json::from_str(existing).map_err(|_| {
        "hooks.json is not plain JSON (JSONC/comments or invalid syntax). \
         Merge the manual entry yourself — McpMux will not rewrite this file."
            .to_string()
    })
}

fn merge_pre_tool_use(existing: Option<&str>, script: &Path) -> Result<String, String> {
    let mut root = match existing {
        Some(raw) if !raw.trim().is_empty() => parse_hooks_json(raw)?,
        _ => json!({ "version": 1, "hooks": {} }),
    };
    let obj = root
        .as_object_mut()
        .ok_or_else(|| "hooks.json root must be a JSON object".to_string())?;
    obj.entry("version").or_insert(json!(1));
    let hooks = obj.entry("hooks").or_insert_with(|| json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or_else(|| "hooks.json `hooks` key must be an object".to_string())?;
    let list = hooks_obj.entry("preToolUse").or_insert_with(|| json!([]));
    let arr = list
        .as_array_mut()
        .ok_or_else(|| "hooks.json `hooks.preToolUse` must be an array".to_string())?;
    arr.retain(|entry| !is_managed_entry(entry, script));
    arr.push(managed_entry(script));
    serde_json::to_string_pretty(&root).map_err(|e| format!("failed to serialize hooks.json: {e}"))
}

fn remove_managed_entry(existing: &str, script: &Path) -> Result<String, String> {
    let mut root = parse_hooks_json(existing)?;
    if let Some(arr) = root
        .pointer_mut("/hooks/preToolUse")
        .and_then(Value::as_array_mut)
    {
        arr.retain(|entry| !is_managed_entry(entry, script));
    }
    serde_json::to_string_pretty(&root).map_err(|e| format!("failed to serialize hooks.json: {e}"))
}

fn write_with_backup(path: &Path, contents: &str) -> Result<Option<String>, String> {
    let mut backed_up = None;
    if path.exists() {
        let bak = PathBuf::from(format!("{}.mcpmux-bak", path.display()));
        std::fs::copy(path, &bak).map_err(|e| format!("failed to back up hooks.json: {e}"))?;
        backed_up = Some(bak.to_string_lossy().into_owned());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("failed to create ~/.cursor: {e}"))?;
    }
    std::fs::write(path, contents).map_err(|e| format!("failed to write hooks.json: {e}"))?;
    Ok(backed_up)
}

fn write_script(script: &Path) -> Result<(), String> {
    if let Some(parent) = script.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create ~/.cursor/hooks: {e}"))?;
    }
    std::fs::write(script, SCRIPT_SOURCE).map_err(|e| format!("failed to write hook script: {e}"))
}

fn has_managed_entry(path: &Path, script: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let raw =
        std::fs::read_to_string(path).map_err(|e| format!("failed to read hooks.json: {e}"))?;
    let root = parse_hooks_json(&raw)?;
    Ok(root
        .pointer("/hooks/preToolUse")
        .and_then(Value::as_array)
        .is_some_and(|arr| arr.iter().any(|e| is_managed_entry(e, script))))
}

/// Current install state of the managed Cursor hook.
pub fn status() -> CursorHookResult {
    let Ok(hooks_path) = hooks_json_path() else {
        return empty_result("status", Some("home directory not found".into()), false);
    };
    let Ok(script) = script_path() else {
        return empty_result("status", Some("home directory not found".into()), false);
    };
    match has_managed_entry(&hooks_path, &script) {
        Ok(has_entry) => {
            let installed = has_entry && script.exists();
            CursorHookResult {
                action: "status".into(),
                installed,
                hooks_path: hooks_path.to_string_lossy().into_owned(),
                script_path: script.to_string_lossy().into_owned(),
                backed_up: None,
                error: None,
                jsonc_refused: false,
                manual_entry: manual_entry_text(&script),
            }
        }
        Err(e) => {
            let refused = e.contains("not plain JSON");
            empty_result("status", Some(e), refused)
        }
    }
}

/// Install or update the managed `preToolUse` hook.
pub fn install() -> CursorHookResult {
    let hooks_path = match hooks_json_path() {
        Ok(p) => p,
        Err(e) => return empty_result("error", Some(e), false),
    };
    let script = match script_path() {
        Ok(p) => p,
        Err(e) => return empty_result("error", Some(e), false),
    };
    if let Err(e) = write_script(&script) {
        return empty_result("error", Some(e), false);
    }
    let existing = if hooks_path.exists() {
        match std::fs::read_to_string(&hooks_path) {
            Ok(s) => Some(s),
            Err(e) => {
                return empty_result(
                    "error",
                    Some(format!("failed to read hooks.json: {e}")),
                    false,
                )
            }
        }
    } else {
        None
    };
    let existed = existing.is_some();
    let merged = match merge_pre_tool_use(existing.as_deref(), &script) {
        Ok(m) => m,
        Err(e) => {
            let refused = e.contains("not plain JSON") || e.contains("must be");
            return empty_result("error", Some(e), refused);
        }
    };
    match write_with_backup(&hooks_path, &merged) {
        Ok(backed_up) => CursorHookResult {
            action: if existed { "updated" } else { "created" }.into(),
            installed: true,
            hooks_path: hooks_path.to_string_lossy().into_owned(),
            script_path: script.to_string_lossy().into_owned(),
            backed_up,
            error: None,
            jsonc_refused: false,
            manual_entry: manual_entry_text(&script),
        },
        Err(e) => empty_result("error", Some(e), false),
    }
}

/// Remove the managed hook entry and delete the managed script.
pub fn uninstall() -> CursorHookResult {
    let hooks_path = match hooks_json_path() {
        Ok(p) => p,
        Err(e) => return empty_result("error", Some(e), false),
    };
    let script = match script_path() {
        Ok(p) => p,
        Err(e) => return empty_result("error", Some(e), false),
    };
    let mut backed_up = None;
    if hooks_path.exists() {
        let raw = match std::fs::read_to_string(&hooks_path) {
            Ok(s) => s,
            Err(e) => {
                return empty_result(
                    "error",
                    Some(format!("failed to read hooks.json: {e}")),
                    false,
                )
            }
        };
        let rewritten = match remove_managed_entry(&raw, &script) {
            Ok(s) => s,
            Err(e) => {
                let refused = e.contains("not plain JSON");
                return empty_result("error", Some(e), refused);
            }
        };
        match write_with_backup(&hooks_path, &rewritten) {
            Ok(bak) => backed_up = bak,
            Err(e) => return empty_result("error", Some(e), false),
        }
    }
    if script.exists() {
        if let Err(e) = std::fs::remove_file(&script) {
            return empty_result(
                "error",
                Some(format!("failed to delete hook script: {e}")),
                false,
            );
        }
    }
    CursorHookResult {
        action: "uninstalled".into(),
        installed: false,
        hooks_path: hooks_path.to_string_lossy().into_owned(),
        script_path: script.to_string_lossy().into_owned(),
        backed_up,
        error: None,
        jsonc_refused: false,
        manual_entry: manual_entry_text(&script),
    }
}
