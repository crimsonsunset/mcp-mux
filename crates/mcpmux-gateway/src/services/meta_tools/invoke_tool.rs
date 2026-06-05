//! `mcpmux_invoke_tool` — permission-checked gateway into backend MCP tools.

use async_trait::async_trait;
use rmcp::model::{CallToolResult, Content};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use tracing::debug;

use super::diagnose_server::parse_missing_required_inputs;
use super::invoke_result_filter::{apply_invoke_result_filter, parse_invoke_filter};
use super::meta_tool_common::{
    caller_resolution, caller_space_id, classify_invoke_denial, format_invoke_not_ready_action,
};
use super::registry::{MetaTool, MetaToolCall, MetaToolError};
use crate::pool::{format_invoke_permission_denied, ConnectionStatus};
use crate::services::levenshtein_suggestions;
use mcpmux_core::FeatureType;

/// Strip repeated `{server_id}_` prefixes when agents pass a qualified name from search.
pub fn normalize_invoke_tool_name(server_id: &str, tool: &str) -> String {
    let prefix = format!("{server_id}_");
    let mut bare = tool;
    while let Some(stripped) = bare.strip_prefix(&prefix) {
        bare = stripped;
    }
    bare.to_string()
}

/// First non-empty string value for any of `keys` on a JSON object (agent alias resolution).
fn first_nonempty_str(args: &Value, keys: &[&str]) -> Option<String> {
    let obj = args.as_object()?;
    for key in keys {
        let Some(value) = obj.get(*key) else {
            continue;
        };
        let Some(text) = value.as_str() else {
            continue;
        };
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    None
}

/// Resolve `server_id` from invoke call args (`server_id`, alias `serverId`, alias `server`).
pub fn resolve_invoke_server_id(args: &Value) -> Option<String> {
    first_nonempty_str(args, &["server_id", "serverId", "server"])
}

/// Resolve `tool` from invoke call args (`tool`, alias `tool_name`).
pub fn resolve_invoke_tool(args: &Value) -> Option<String> {
    first_nonempty_str(args, &["tool", "tool_name"])
}

/// Whether an invokable feature matches the caller's `tool` (bare or qualified).
fn feature_matches_tool_name(
    feature_name: &str,
    qualified_name: &str,
    tool_input: &str,
    bare: &str,
) -> bool {
    feature_name == bare || qualified_name == tool_input
}

/// Resolve backend tool arguments from `mcpmux_invoke_tool` call args.
///
/// Prefers `args`, then `params`, then `arguments` (common agent/UI aliases).
pub fn resolve_invoke_tool_args(args: &Value) -> Value {
    args.get("args")
        .or_else(|| args.get("params"))
        .or_else(|| args.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

/// Meta tool that forwards invocations to [`RoutingService::call_tool`].
pub struct InvokeToolTool;

#[async_trait]
impl MetaTool for InvokeToolTool {
    fn name(&self) -> &'static str {
        "mcpmux_invoke_tool"
    }

    fn description(&self) -> &'static str {
        "Invoke a backend MCP tool by server_id and tool (bare or qualified from \
         mcpmux_search_tools). Skip search when you already know the tool — pass \
         bare_name or qualified_name directly. Set preflight: true to check readiness \
         without calling the backend (returns { ready: true } or a structured not_ready \
         error). Requires the server to be ready and the tool in the current permission \
         set. Search results include required_params types — mcpmux_get_tool_schema is \
         optional for complex tools. Pass an optional filter to bound large payloads; omit \
         filter to return the backend response as-is."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["server_id", "tool"],
            "properties": {
                "server_id": {
                    "type": "string",
                    "description": "Registry server id (e.g. github). Aliases: server, serverId (server_id wins if multiple are set)."
                },
                "server": {
                    "type": "string",
                    "description": "Alias for server_id"
                },
                "serverId": {
                    "type": "string",
                    "description": "Alias for server_id"
                },
                "tool": {
                    "type": "string",
                    "description": "Tool name on that server — bare (e.g. list_issues) or qualified from mcpmux_search_tools (e.g. github_list_issues); bare_name in search results is the invoke value when unsure. Known tools can be invoked directly without a prior search. Alias: tool_name (tool wins if both are set)."
                },
                "tool_name": {
                    "type": "string",
                    "description": "Alias for tool"
                },
                "preflight": {
                    "type": "boolean",
                    "default": false,
                    "description": "When true, verify server and tool readiness without calling the backend. Returns { ready: true } on success or a structured not_ready error (same shape as a failed invoke)."
                },
                "args": {
                    "type": "object",
                    "description": "Arguments object passed to the backend tool. Aliases: params, arguments (args wins if multiple are set).",
                    "default": {}
                },
                "params": {
                    "type": "object",
                    "description": "Alias for args"
                },
                "arguments": {
                    "type": "object",
                    "description": "Alias for args"
                },
                "filter": {
                    "type": "object",
                    "description": "Optional result shaping (max_rows, max_bytes, fields, format). Omit to return the backend response as-is.",
                    "properties": {
                        "max_rows": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Maximum rows/items to return from large arrays"
                        },
                        "max_bytes": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Maximum UTF-8 bytes for text or serialized JSON payloads"
                        },
                        "fields": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "When set, keep only these fields on each object in list results"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["summary", "full"],
                            "description": "When max_rows is set: summary caps the sample at min(max_rows, 5); full returns up to max_rows rows. Ignored when max_rows is omitted."
                        }
                    }
                }
            }
        })
    }

    fn is_write(&self) -> bool {
        false
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let server_id = resolve_invoke_server_id(&call.args).ok_or_else(|| {
            MetaToolError::InvalidArgument("missing `server_id` (aliases: server, serverId)".into())
        })?;
        let tool_input = resolve_invoke_tool(&call.args).ok_or_else(|| {
            MetaToolError::InvalidArgument(
                "missing `tool` (aliases: tool_name; bare or qualified, e.g. \"list_issues\" or \"github_list_issues\")"
                    .into(),
            )
        })?;
        let bare_tool_name = normalize_invoke_tool_name(&server_id, &tool_input);
        let preflight = call
            .args
            .get("preflight")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let args = resolve_invoke_tool_args(&call.args);
        let filter = parse_invoke_filter(call.args.get("filter"));

        let resolved = caller_resolution(&call).await?;
        let space_id = caller_space_id(&call).await?;

        let invokable = call
            .ctx
            .feature_service
            .get_invokable_tools_for_grants(&space_id.to_string(), &resolved.feature_set_ids)
            .await
            .map_err(|e| MetaToolError::Internal(e.to_string()))?;

        let binding_features = call
            .ctx
            .feature_service
            .resolve_feature_sets(&space_id.to_string(), &resolved.feature_set_ids)
            .await
            .map_err(|e| MetaToolError::Internal(e.to_string()))?;
        let binding_servers: std::collections::HashSet<String> = binding_features
            .iter()
            .map(|f| f.server_id.clone())
            .collect();

        if !binding_servers.contains(&server_id) {
            let (reason, tool) =
                classify_invoke_denial(false, ConnectionStatus::Disconnected, false)
                    .unwrap_or(("inactive", "mcpmux_bind_current_workspace"));
            return Ok(invoke_not_ready(
                reason,
                format_invoke_not_ready_action(reason, &server_id),
                tool,
            ));
        }

        let installed = call
            .ctx
            .installed_server_repo
            .get_by_server_id(&space_id.to_string(), &server_id)
            .await
            .ok()
            .flatten();

        let pool_statuses = call.ctx.server_manager.get_all_statuses(space_id).await;
        let connection_status = pool_statuses
            .get(&server_id)
            .map(|(status, _, _, _)| *status)
            .unwrap_or(ConnectionStatus::Disconnected);
        let has_missing_inputs = installed
            .as_ref()
            .map(|server| !parse_missing_required_inputs(server).is_empty())
            .unwrap_or(false);

        if let Some((reason, tool)) =
            classify_invoke_denial(true, connection_status, has_missing_inputs)
        {
            return Ok(invoke_not_ready(
                reason,
                format_invoke_not_ready_action(reason, &server_id),
                tool,
            ));
        }

        let matched = invokable.iter().find(|f| {
            f.feature_type == FeatureType::Tool
                && f.server_id == server_id
                && feature_matches_tool_name(
                    &f.feature_name,
                    &f.qualified_name(),
                    &tool_input,
                    &bare_tool_name,
                )
        });
        let qualified_name = matched.map(|f| f.qualified_name()).unwrap_or_else(|| {
            if tool_input.starts_with(&format!("{server_id}_")) {
                tool_input.clone()
            } else {
                format!("{server_id}_{bare_tool_name}")
            }
        });
        let is_invokable = matched.map(|f| f.is_available).unwrap_or(false);

        if !is_invokable {
            if preflight {
                return Ok(invoke_not_ready(
                    "permission_denied",
                    format_invoke_not_ready_action("permission_denied", &server_id),
                    "mcpmux_search_tools",
                ));
            }
            let candidates: Vec<String> = invokable
                .iter()
                .filter(|f| f.server_id == server_id)
                .map(|f| f.feature_name.clone())
                .collect();
            let suggestions = levenshtein_suggestions(&bare_tool_name, &candidates, 5);
            return Ok(invoke_error(format_invoke_permission_denied(
                &qualified_name,
                &server_id,
                &bare_tool_name,
                &suggestions,
            )));
        }

        if preflight {
            return Ok(invoke_preflight_ok());
        }

        let effective_args = match installed {
            Some(server) => merge_default_params(args, &server.default_params),
            None => args,
        };

        let backend = call
            .ctx
            .invoke_backend
            .as_ref()
            .ok_or_else(|| MetaToolError::Internal("invoke routing not configured".into()))?;
        match backend
            .call_tool(
                space_id,
                &resolved.feature_set_ids,
                &qualified_name,
                effective_args,
            )
            .await
        {
            Ok(result) => {
                if result.is_error {
                    let content: Vec<Content> = result
                        .content
                        .into_iter()
                        .filter_map(|v| serde_json::from_value(v).ok())
                        .collect();
                    let mut mcp_result = CallToolResult::error(content);
                    mcp_result.structured_content = result.structured_content;
                    return Ok(mcp_result);
                }

                let (content, structured_content) = match filter.as_ref().filter(|f| f.has_effect())
                {
                    Some(active_filter) => apply_invoke_result_filter(
                        result.content,
                        result.structured_content,
                        active_filter,
                    ),
                    None => (result.content, result.structured_content),
                };
                let parsed_content: Vec<Content> = content
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                let mut mcp_result = CallToolResult::success(parsed_content);
                mcp_result.structured_content = structured_content;
                Ok(mcp_result)
            }
            Err(e) => Ok(invoke_error(e.to_string())),
        }
    }
}

/// Merge per-server default params under caller-supplied args.
///
/// Produces `{ ...defaults, ...caller_args }` — caller wins on key collision.
/// Returns `args` unchanged when `defaults` is empty or `args` is not an Object.
fn merge_default_params(args: Value, defaults: &HashMap<String, Value>) -> Value {
    if defaults.is_empty() {
        return args;
    }
    let Value::Object(caller_map) = args else {
        debug!("merge_default_params: args is not an Object; server defaults not applied");
        return args;
    };
    let mut merged: Map<String, Value> = defaults
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    merged.extend(caller_map);
    Value::Object(merged)
}

/// Build a structured MCP error payload for invoke failures.
fn invoke_error(message: String) -> CallToolResult {
    let payload = json!({
        "error": "invoke_failed",
        "message": message,
    });
    CallToolResult::error(vec![Content::text(payload.to_string())])
}

/// Build a structured not-ready denial before backend dispatch.
fn invoke_not_ready(reason: &str, action: String, tool: &str) -> CallToolResult {
    let payload = json!({
        "error": "not_ready",
        "reason": reason,
        "action": action,
        "tool": tool,
    });
    CallToolResult::error(vec![Content::text(payload.to_string())])
}

/// Successful preflight response — readiness verified, no backend call.
fn invoke_preflight_ok() -> CallToolResult {
    CallToolResult::success(vec![Content::text(json!({ "ready": true }).to_string())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_invoke_tool_args_prefers_args_over_params() {
        let call_args = json!({
            "args": { "owner": "a" },
            "params": { "owner": "b" }
        });
        assert_eq!(
            resolve_invoke_tool_args(&call_args),
            json!({ "owner": "a" })
        );
    }

    #[test]
    fn resolve_invoke_tool_args_falls_back_to_params() {
        let call_args = json!({ "params": { "repo": "mcp-mux" } });
        assert_eq!(
            resolve_invoke_tool_args(&call_args),
            json!({ "repo": "mcp-mux" })
        );
    }

    #[test]
    fn resolve_invoke_tool_args_defaults_to_empty_object() {
        assert_eq!(resolve_invoke_tool_args(&json!({})), json!({}));
    }

    #[test]
    fn resolve_invoke_tool_args_falls_back_to_arguments() {
        let call_args = json!({ "arguments": { "id": "page-1" } });
        assert_eq!(
            resolve_invoke_tool_args(&call_args),
            json!({ "id": "page-1" })
        );
    }

    #[test]
    fn resolve_invoke_tool_args_prefers_args_over_arguments() {
        let call_args = json!({
            "args": { "id": "a" },
            "arguments": { "id": "b" }
        });
        assert_eq!(resolve_invoke_tool_args(&call_args), json!({ "id": "a" }));
    }

    #[test]
    fn resolve_invoke_server_id_accepts_aliases() {
        assert_eq!(
            resolve_invoke_server_id(&json!({ "server_id": "github" })),
            Some("github".to_string())
        );
        assert_eq!(
            resolve_invoke_server_id(&json!({ "server": "notion" })),
            Some("notion".to_string())
        );
        assert_eq!(
            resolve_invoke_server_id(&json!({ "serverId": "jira" })),
            Some("jira".to_string())
        );
    }

    #[test]
    fn resolve_invoke_server_id_prefers_server_id_over_aliases() {
        let call_args = json!({
            "server_id": "canonical",
            "server": "alias",
            "serverId": "other"
        });
        assert_eq!(
            resolve_invoke_server_id(&call_args),
            Some("canonical".to_string())
        );
    }

    #[test]
    fn resolve_invoke_tool_accepts_tool_name_alias() {
        assert_eq!(
            resolve_invoke_tool(&json!({ "tool_name": "notion-fetch" })),
            Some("notion-fetch".to_string())
        );
    }

    #[test]
    fn resolve_invoke_tool_prefers_tool_over_tool_name() {
        let call_args = json!({
            "tool": "bare",
            "tool_name": "alias"
        });
        assert_eq!(resolve_invoke_tool(&call_args), Some("bare".to_string()));
    }

    #[test]
    fn normalize_invoke_tool_name_strips_server_prefix() {
        assert_eq!(
            normalize_invoke_tool_name("github", "github_list_issues"),
            "list_issues"
        );
    }

    #[test]
    fn normalize_invoke_tool_name_passes_bare_through() {
        assert_eq!(
            normalize_invoke_tool_name("github", "list_issues"),
            "list_issues"
        );
    }

    #[test]
    fn normalize_invoke_tool_name_strips_repeated_prefix() {
        assert_eq!(
            normalize_invoke_tool_name("github", "github_github_list_issues"),
            "list_issues"
        );
    }

    #[test]
    fn feature_matches_tool_name_accepts_qualified_or_bare() {
        assert!(feature_matches_tool_name(
            "list_issues",
            "github_list_issues",
            "github_list_issues",
            "list_issues"
        ));
        assert!(feature_matches_tool_name(
            "list_issues",
            "github_list_issues",
            "list_issues",
            "list_issues"
        ));
        assert!(!feature_matches_tool_name(
            "other_tool",
            "github_other_tool",
            "list_issues",
            "list_issues"
        ));
    }
}
