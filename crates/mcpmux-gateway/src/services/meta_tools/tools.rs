//! Built-in `mcpmux_*` meta tool implementations.
//!
//! Each tool is a unit struct implementing [`MetaTool`]. Reads execute
//! directly; writes route through the [`ApprovalBroker`] first.

use async_trait::async_trait;
use mcpmux_core::{normalize_workspace_root, FeatureType, WorkspaceBinding};
use rmcp::model::CallToolResult;
use serde_json::{json, Value};
use std::collections::HashSet;
use tracing::info;

use super::approval::ApprovalScope;
use super::meta_tool_common::{
    caller_resolution, caller_space_id, emit_tools_list_changed, emit_workspace_binding_changed,
    parse_uuid_arg, text_result, with_approval,
};
use super::registry::{MetaTool, MetaToolCall, MetaToolError};

// Phase 3 will point token_budget at the per-tool modules directly.
pub use super::list_servers::ListServersTool;
pub use super::search_tools::SearchToolsTool;

// NOTE: MetaToolInvoked audit events are emitted centrally by
// MetaToolRegistry::call, so individual tools don't need to fire them.

// ---------------------------------------------------------------------------
// mcpmux_list_all_tools — read (not registered on the agent surface; desktop/admin only)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct ListAllToolsTool;

#[async_trait]
#[allow(dead_code)]
impl MetaTool for ListAllToolsTool {
    fn name(&self) -> &'static str {
        "mcpmux_list_all_tools"
    }

    fn description(&self) -> &'static str {
        "Operator/diagnostic: list every tool installed in the caller's resolved \
         Space (ignores FeatureSet filter on the roster). Each entry includes \
         server_available (seen on the connected server) and invokable (callable \
         via mcpmux_invoke_tool with current grants). Agents should prefer \
         mcpmux_search_tools for discovery — only invokable tools can be invoked."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "server_id": {
                    "type": "string",
                    "description": "Optional filter to one server id"
                }
            }
        })
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let resolved = caller_resolution(&call).await?;
        let space_id = caller_space_id(&call).await?;
        let server_filter = call.args.get("server_id").and_then(|v| v.as_str());

        let invokable = call
            .ctx
            .feature_service
            .get_invokable_tools_for_grants(&space_id.to_string(), &resolved.feature_set_ids)
            .await
            .map_err(|e| MetaToolError::Internal(e.to_string()))?;
        // Match by (server_id, feature_name) — prefix aliases differ from raw catalog rows.
        let invokable_keys: HashSet<(String, String)> = invokable
            .iter()
            .filter(|f| f.feature_type == FeatureType::Tool && f.is_available)
            .map(|f| (f.server_id.clone(), f.feature_name.clone()))
            .collect();

        let features = call
            .ctx
            .server_feature_repo
            .list_for_space(&space_id.to_string())
            .await?;
        let tools: Vec<_> = features
            .iter()
            .filter(|f| f.feature_type == FeatureType::Tool)
            .filter(|f| server_filter.is_none_or(|sid| f.server_id == sid))
            .map(|f| {
                let qualified_name = f.qualified_name();
                json!({
                    "server_id": f.server_id,
                    "qualified_name": qualified_name,
                    "description": f.description,
                    "server_available": f.is_available,
                    "invokable": invokable_keys.contains(&(f.server_id.clone(), f.feature_name.clone())),
                })
            })
            .collect();
        let total_invokable = tools
            .iter()
            .filter(|t| t.get("invokable") == Some(&json!(true)))
            .count();
        Ok(text_result(json!({
            "tools": tools,
            "total_installed": tools.len(),
            "total_invokable": total_invokable,
            "hint": "Use mcpmux_search_tools for agent discovery. Only invokable tools can be invoked with current FeatureSet grants.",
        })))
    }
}

// ---------------------------------------------------------------------------
// mcpmux_list_feature_sets — read
// ---------------------------------------------------------------------------

pub struct ListFeatureSetsTool;

#[async_trait]
impl MetaTool for ListFeatureSetsTool {
    fn name(&self) -> &'static str {
        "mcpmux_list_feature_sets"
    }

    fn description(&self) -> &'static str {
        "List every FeatureSet defined in the caller's resolved Space — \
         built-ins and custom. Each entry carries `id`, `name`, `description`, \
         `type`, `is_builtin`, and `status` (`active` when bound to this \
         workspace, `inactive` when available to bind). To activate capability, \
         call mcpmux_bind_current_workspace with an inactive entry's `id`."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let resolved = caller_resolution(&call).await?;
        let space_id = caller_space_id(&call).await?;
        let space = call
            .ctx
            .space_repo
            .get(&space_id)
            .await?
            .ok_or_else(|| MetaToolError::Internal("space missing".into()))?;
        let bound_ids: HashSet<String> = resolved.feature_set_ids.iter().cloned().collect();
        let sets = call
            .ctx
            .feature_set_repo
            .list_by_space(&space_id.to_string())
            .await?;
        let sets: Vec<_> = sets
            .iter()
            .filter(|fs| !fs.is_deleted)
            .map(|fs| {
                let status = if bound_ids.contains(&fs.id) {
                    "active"
                } else {
                    "inactive"
                };
                json!({
                    "id": fs.id,
                    "name": fs.name,
                    "description": fs.description,
                    "type": fs.feature_set_type,
                    "is_builtin": fs.is_builtin,
                    "status": status,
                })
            })
            .collect();
        Ok(text_result(
            json!({ "space_id": space.id, "feature_sets": sets }),
        ))
    }
}

// ---------------------------------------------------------------------------
// mcpmux_get_tool_schema — read
// ---------------------------------------------------------------------------

/// Parsed `tools` argument for schema lookup, retaining invalid entries for `missing`.
struct ToolSchemaNameRequest {
    valid_names: Vec<String>,
    invalid_entries: Vec<String>,
}

/// Parse the `tools` argument from `mcpmux_get_tool_schema` call args.
///
/// Accepts a qualified name string, a string array, or a JSON-encoded array
/// string (common when agents double-serialize through MCP clients).
fn parse_tool_schema_names(value: Option<&Value>) -> Result<ToolSchemaNameRequest, MetaToolError> {
    let Some(value) = value else {
        return Err(MetaToolError::InvalidArgument(
            "missing or invalid `tools` — expected string or string array".into(),
        ));
    };

    match value {
        Value::String(s) => {
            if let Ok(Value::Array(arr)) = serde_json::from_str(s) {
                return names_from_json_array(&arr);
            }
            Ok(ToolSchemaNameRequest {
                valid_names: vec![s.clone()],
                invalid_entries: Vec::new(),
            })
        }
        Value::Array(arr) => names_from_json_array(arr),
        _ => Err(MetaToolError::InvalidArgument(
            "missing or invalid `tools` — expected string or string array".into(),
        )),
    }
}

/// Split a JSON string array into valid qualified names and invalid entries (e.g. empty strings).
fn names_from_json_array(arr: &[Value]) -> Result<ToolSchemaNameRequest, MetaToolError> {
    let mut valid_names = Vec::new();
    let mut invalid_entries = Vec::new();

    for value in arr {
        match value.as_str() {
            Some(name) if name.trim().is_empty() => invalid_entries.push(name.to_string()),
            Some(name) => valid_names.push(name.trim().to_string()),
            None => invalid_entries.push(value.to_string()),
        }
    }

    if valid_names.is_empty() && invalid_entries.is_empty() {
        return Err(MetaToolError::InvalidArgument(
            "`tools` must contain at least one qualified name".into(),
        ));
    }

    Ok(ToolSchemaNameRequest {
        valid_names,
        invalid_entries,
    })
}

pub struct GetToolSchemaTool;

#[async_trait]
impl MetaTool for GetToolSchemaTool {
    fn name(&self) -> &'static str {
        "mcpmux_get_tool_schema"
    }

    fn description(&self) -> &'static str {
        "Load input schemas for one or more qualified tool names before \
         invoking via mcpmux_invoke_tool. Pass tools as a single qualified \
         name string or a string array (e.g. [\"github_list_issues\"]). \
         Set compact: true to omit descriptions. Tools must be invokable \
         with current grants — use mcpmux_search_tools to discover names."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["tools"],
            "properties": {
                "tools": {
                    "oneOf": [
                        { "type": "string" },
                        { "type": "array", "items": { "type": "string" } }
                    ]
                },
                "compact": { "type": "boolean", "default": false }
            }
        })
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let resolved = caller_resolution(&call).await?;
        let space_id = caller_space_id(&call).await?;

        let schema_request = parse_tool_schema_names(call.args.get("tools"))?;
        let tool_names = schema_request.valid_names;

        let compact = call
            .args
            .get("compact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let invokable = call
            .ctx
            .feature_service
            .get_invokable_tools_for_grants(&space_id.to_string(), &resolved.feature_set_ids)
            .await
            .map_err(|e| MetaToolError::Internal(e.to_string()))?;

        let index = call
            .ctx
            .tool_discovery
            .build_index(&space_id.to_string(), &invokable)
            .await
            .map_err(|e| MetaToolError::Internal(e.to_string()))?;

        let schemas = crate::services::tool_discovery::ToolDiscoveryService::get_schemas(
            &index,
            &tool_names,
            compact,
        );

        let found_names: HashSet<String> = schemas
            .iter()
            .filter_map(|s| {
                s.get("qualified_name")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect();
        let mut missing: Vec<String> = tool_names
            .iter()
            .filter(|name| !found_names.contains(*name))
            .cloned()
            .collect();
        missing.extend(schema_request.invalid_entries);

        if missing.is_empty() {
            return Ok(text_result(json!({ "schemas": schemas })));
        }

        let missing_list: Vec<&str> = missing.iter().map(String::as_str).collect();
        Ok(text_result(json!({
            "schemas": schemas,
            "missing": missing_list,
            "message": format!(
                "{} tool(s) not invokable or unknown with current grants → use mcpmux_search_tools to discover allowed names",
                missing.len()
            ),
        })))
    }
}

// ---------------------------------------------------------------------------
// mcpmux_bind_current_workspace — write (persistent, space-wide effect)
// ---------------------------------------------------------------------------

pub struct BindCurrentWorkspaceTool;

#[async_trait]
impl MetaTool for BindCurrentWorkspaceTool {
    fn name(&self) -> &'static str {
        "mcpmux_bind_current_workspace"
    }

    fn description(&self) -> &'static str {
        "Canonical activation path: persistently append an existing FeatureSet \
         onto the caller's workspace binding (layers with existing bundles, \
         deduped). Use after mcpmux_search_tools (include_inactive: true) or \
         mcpmux_list_feature_sets to obtain feature_set_id. Every future \
         connection reporting the same root inherits the binding. Requires \
         approval; the client MUST have declared MCP roots."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["feature_set_id"],
            "properties": {
                "feature_set_id": { "type": "string" }
            }
        })
    }

    fn is_write(&self) -> bool {
        true
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let fs_id = parse_uuid_arg(&call.args, "feature_set_id")?;

        let space_id = caller_space_id(&call).await?;
        let roots = call
            .session_id
            .and_then(|sid| call.ctx.session_roots.get(sid))
            .unwrap_or_default();
        let root = roots.into_iter().next().ok_or_else(|| {
            MetaToolError::InvalidArgument(
                "caller did not report any MCP roots; cannot bind".into(),
            )
        })?;
        let normalized = normalize_workspace_root(&root);

        let fs_name = call
            .ctx
            .feature_set_repo
            .get(&fs_id.to_string())
            .await?
            .map(|fs| fs.name)
            .unwrap_or_else(|| fs_id.to_string());

        let binding_repo = call.ctx.binding_repo.clone();
        let fs_id_str = fs_id.to_string();

        // Dedup before consent: repeat binds must not re-prompt the user.
        if let Some(existing) = binding_repo
            .list()
            .await?
            .into_iter()
            .find(|b| b.workspace_root == normalized)
        {
            if existing.feature_set_ids.iter().any(|id| id == &fs_id_str) {
                return Ok(text_result(json!({
                    "ok": true,
                    "binding_id": existing.id,
                    "workspace_root": normalized,
                    "feature_set_id": fs_id,
                    "feature_set_ids": existing.feature_set_ids,
                    "already_bound": true,
                })));
            }
        }

        let summary = format!(
            "Append FeatureSet '{fs_name}' to workspace '{normalized}' binding \
             (existing bundles preserved). Affects every future connection that \
             reports this path."
        );

        let event_tx = call.ctx.domain_event_tx.clone();
        with_approval(
            &call,
            "mcpmux_bind_current_workspace",
            summary,
            None,
            true,
            call.args.clone(),
            || async move {
                let fs_id_str = fs_id.to_string();
                let existing = binding_repo
                    .list()
                    .await?
                    .into_iter()
                    .find(|b| b.workspace_root == normalized);

                let (binding_id, feature_set_ids, already_bound) = if let Some(mut binding) =
                    existing
                {
                    binding.space_id = space_id;
                    let already_bound = binding.feature_set_ids.iter().any(|id| id == &fs_id_str);
                    if !already_bound {
                        binding.feature_set_ids.push(fs_id_str.clone());
                        binding.updated_at = chrono::Utc::now();
                        binding_repo.update(&binding).await?;
                        emit_workspace_binding_changed(&event_tx, space_id, &normalized);
                    }
                    info!(
                        %space_id,
                        binding_id = %binding.id,
                        workspace_root = %normalized,
                        feature_set_id = %fs_id,
                        already_bound,
                        feature_set_count = binding.feature_set_ids.len(),
                        "[meta_tools] bind_current_workspace updated existing binding",
                    );
                    (binding.id, binding.feature_set_ids.clone(), already_bound)
                } else {
                    let binding =
                        WorkspaceBinding::new(normalized.clone(), space_id, fs_id_str.clone());
                    let binding_id = binding.id;
                    let feature_set_ids = binding.feature_set_ids.clone();
                    binding_repo.create(&binding).await?;
                    info!(
                        %space_id,
                        binding_id = %binding_id,
                        workspace_root = %normalized,
                        feature_set_id = %fs_id,
                        "[meta_tools] bind_current_workspace created binding",
                    );
                    (binding_id, feature_set_ids, false)
                };

                emit_tools_list_changed(&event_tx, space_id);
                Ok(text_result(json!({
                    "ok": true,
                    "binding_id": binding_id,
                    "workspace_root": normalized,
                    "feature_set_id": fs_id,
                    "feature_set_ids": feature_set_ids,
                    "already_bound": already_bound,
                })))
            },
        )
        .await
    }
}

// Suppress unused warning — `ApprovalScope` is re-exported for the Tauri
// surface and will land as a command argument once the dialog is wired up.
#[allow(dead_code)]
fn _unused_approval_scope(_: ApprovalScope) {}
