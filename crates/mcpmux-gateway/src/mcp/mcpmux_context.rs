//! Per-call `_mcpmux_context` attached by Cursor's `preToolUse` hook.
//!
//! The reserved argument is transport metadata: parse it, validate the root,
//! then strip it before meta-tool parsing or backend forwarding.

use rmcp::model::JsonObject;
use serde_json::Value;

use crate::services::SessionRootsRegistry;
use mcpmux_core::normalize_workspace_root;

/// Reserved tool-argument key injected by the managed Cursor hook.
pub const MCPMUX_CONTEXT_KEY: &str = "_mcpmux_context";

/// Exact workspace identity carried on one `tools/call`.
#[derive(Debug, Clone)]
pub struct ExtractedCallContext {
    pub workspace_root: String,
    pub tool_use_id: Option<String>,
}

/// Remove `_mcpmux_context` from `arguments` and validate it when present.
///
/// `Ok(None)` means the call has no hook context and should use the session
/// ladder. Malformed objects and candidate-set mismatches are errors.
pub fn take_mcpmux_context(
    arguments: &mut JsonObject,
    session_id: Option<&str>,
    session_roots: &SessionRootsRegistry,
) -> Result<Option<ExtractedCallContext>, String> {
    let Some(raw) = arguments.remove(MCPMUX_CONTEXT_KEY) else {
        return Ok(None);
    };

    let obj = match raw {
        Value::Object(obj) => obj,
        _ => {
            return Err("invalid _mcpmux_context: expected an object".into());
        }
    };

    let raw_root = obj
        .get("workspace_root")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "invalid _mcpmux_context: workspace_root must be a non-empty string".to_string()
        })?;

    let workspace_root = normalize_workspace_root(raw_root);
    if workspace_root.is_empty() {
        return Err("invalid _mcpmux_context: workspace_root is empty after normalize".into());
    }

    if let Some(sid) = session_id {
        if !session_roots.is_candidate(sid, &workspace_root) {
            return Err(
                "invalid _mcpmux_context: workspace_root is not in this session's candidate set"
                    .into(),
            );
        }
    }

    let tool_use_id = obj
        .get("tool_use_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Ok(Some(ExtractedCallContext {
        workspace_root,
        tool_use_id,
    }))
}
