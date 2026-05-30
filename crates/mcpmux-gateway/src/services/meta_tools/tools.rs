//! Built-in `mcpmux_*` meta tool implementations.
//!
//! Each tool is a unit struct implementing [`MetaTool`]. Reads execute
//! directly; writes route through the [`ApprovalBroker`] first.

use async_trait::async_trait;
use mcpmux_core::{normalize_workspace_root, DomainEvent, FeatureType, WorkspaceBinding};
use rmcp::model::{CallToolResult, Content};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tokio::sync::broadcast;
use tracing::{debug, info};
use uuid::Uuid;

use super::approval::{ApprovalPayload, ApprovalScope};
use super::registry::{feature_set_ids_fingerprint, MetaTool, MetaToolCall, MetaToolError};
use crate::services::ResolvedFeatureSet;

/// Fire a `FeatureSetMembersChanged` event so MCPNotifier pushes a
/// `tools/list_changed` notification to every connected client in the Space.
/// Used by every write tool after a successful mutation.
fn emit_tools_list_changed(event_tx: &broadcast::Sender<DomainEvent>, space_id: Uuid) {
    let _ = event_tx.send(DomainEvent::FeatureSetMembersChanged {
        space_id,
        feature_set_id: "meta-tool-write".into(),
        added_count: 0,
        removed_count: 0,
    });
}

/// Notify listeners that a workspace binding row changed.
fn emit_workspace_binding_changed(
    event_tx: &broadcast::Sender<DomainEvent>,
    space_id: Uuid,
    workspace_root: &str,
) {
    let _ = event_tx.send(DomainEvent::WorkspaceBindingChanged {
        space_id,
        workspace_root: workspace_root.to_string(),
    });
}

// NOTE: MetaToolInvoked audit events are emitted centrally by
// MetaToolRegistry::call, so individual tools don't need to fire them.

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn text_result(v: Value) -> CallToolResult {
    CallToolResult::success(vec![Content::text(v.to_string())])
}

/// Resolve the Space the caller is *actually* routed into — i.e. whichever
/// Space the resolver picks via WorkspaceBinding for this session's reported
/// roots, falling back to the default Space when no binding matches.
///
/// Every meta tool reads (and writes) inside this Space. That keeps the
/// caller's tool/FS view aligned with the tools the gateway actually exposes
/// to them, and prevents an LLM in workspace A from mutating FSes in
/// workspace B just because both sit under the same default-Space-flagged
/// row in the DB.
pub(crate) async fn caller_space_id(call: &MetaToolCall<'_>) -> Result<Uuid, MetaToolError> {
    let resolved = call
        .ctx
        .resolver
        .resolve(call.session_id, Some(call.client_id))
        .await?;
    if let Some(space_id) = resolved.space_id {
        return Ok(space_id);
    }
    // Resolver returned no space — should only happen in the pathological
    // "no default space configured" setup. Fail loudly so callers see why.
    Err(MetaToolError::Internal(
        "no Space resolved for this caller (no default Space configured?)".into(),
    ))
}

/// Full resolver output for the caller — space + binding FS ids + source.
pub(crate) async fn caller_resolution(
    call: &MetaToolCall<'_>,
) -> Result<ResolvedFeatureSet, MetaToolError> {
    call.ctx
        .resolver
        .resolve(call.session_id, Some(call.client_id))
        .await
        .map_err(|e| MetaToolError::Internal(e.to_string()))
}

/// Derive the manifest status for one server in the caller's binding.
fn derive_server_status(server_id: &str, binding_servers: &HashSet<String>) -> &'static str {
    if binding_servers.contains(server_id) {
        "enabled_via_binding"
    } else {
        "inactive"
    }
}

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
// mcpmux_list_servers — read
// ---------------------------------------------------------------------------

pub struct ListServersTool;

#[async_trait]
impl MetaTool for ListServersTool {
    fn name(&self) -> &'static str {
        "mcpmux_list_servers"
    }

    fn description(&self) -> &'static str {
        "List every MCP server installed in the caller's resolved Space with \
         a coarse status per server: enabled_via_binding or inactive. Inactive \
         servers include `bindable_feature_set_ids` — pass one to \
         mcpmux_bind_current_workspace to activate persistently. Clone installs \
         include optional `cloned_from` (source server_id)."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let resolved = caller_resolution(&call).await?;
        let space_id = resolved
            .space_id
            .ok_or_else(|| MetaToolError::Internal("space missing".into()))?;

        let binding_features = call
            .ctx
            .feature_service
            .resolve_feature_sets(&space_id.to_string(), &resolved.feature_set_ids)
            .await?;
        let binding_servers: HashSet<String> = binding_features
            .iter()
            .map(|f| f.server_id.clone())
            .collect();

        let features = call
            .ctx
            .server_feature_repo
            .list_for_space(&space_id.to_string())
            .await?;

        let installed = call
            .ctx
            .installed_server_repo
            .list_for_space(&space_id.to_string())
            .await
            .map_err(|e| MetaToolError::Internal(e.to_string()))?;
        // Per-server lookup of effective display name (override → server_name → tail)
        // and clone lineage. Centralized so JSON output and UI agree on the label.
        struct InstalledMeta {
            display_name: String,
            cloned_from: Option<String>,
        }
        let installed_meta_by_server: HashMap<String, InstalledMeta> = installed
            .into_iter()
            .map(|s| {
                let display_name = s.display_name().to_string();
                (
                    s.server_id,
                    InstalledMeta {
                        display_name,
                        cloned_from: s.cloned_from,
                    },
                )
            })
            .collect();

        let inactive_by_server: HashMap<String, HashSet<String>> = call
            .ctx
            .feature_service
            .list_inactive_discovery_tools(&space_id.to_string(), &resolved.feature_set_ids)
            .await
            .map_err(|e| MetaToolError::Internal(e.to_string()))?
            .into_iter()
            .fold(HashMap::new(), |mut acc, entry| {
                acc.entry(entry.feature.server_id.clone())
                    .or_default()
                    .insert(entry.bindable_feature_set_id);
                acc
            });

        let mut by_server: HashMap<String, (Option<String>, usize)> = HashMap::new();
        for feature in &features {
            if feature.feature_type != FeatureType::Tool {
                continue;
            }
            let entry = by_server
                .entry(feature.server_id.clone())
                .or_insert((None, 0));
            if entry.0.is_none() {
                entry.0 = feature.display_name.clone();
            }
            entry.1 += 1;
        }

        let mut servers: Vec<Value> = by_server
            .into_iter()
            .map(|(id, (feature_display_name, tool_count))| {
                // Prefer the installed row's effective display name (override or
                // server_name) so users see "Joe Calendar" instead of the catalog name.
                let installed_meta = installed_meta_by_server.get(&id);
                let name = installed_meta
                    .map(|meta| meta.display_name.clone())
                    .or(feature_display_name)
                    .unwrap_or_else(|| id.clone());
                let status = derive_server_status(&id, &binding_servers);
                let mut entry = json!({
                    "id": id,
                    "name": name,
                    "tool_count": tool_count,
                    "status": status,
                });
                if let Some(cloned_from) = installed_meta.and_then(|meta| meta.cloned_from.as_ref())
                {
                    entry["cloned_from"] = json!(cloned_from);
                }
                if status == "inactive" {
                    if let Some(fs_ids) = inactive_by_server.get(&id) {
                        let mut ids: Vec<_> = fs_ids.iter().cloned().collect();
                        ids.sort();
                        entry["bindable_feature_set_ids"] = json!(ids);
                    }
                }
                entry
            })
            .collect();
        servers.sort_by(|a, b| {
            a.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("id").and_then(|v| v.as_str()).unwrap_or(""))
        });

        Ok(text_result(json!({ "servers": servers })))
    }
}

/// Build the active tool index from DB grants (no cache write).
async fn build_active_index(
    call: &MetaToolCall<'_>,
    space_id: &Uuid,
    resolved: &ResolvedFeatureSet,
) -> Result<Vec<crate::services::ToolIndexEntry>, MetaToolError> {
    let invokable = call
        .ctx
        .feature_service
        .get_invokable_tools_for_grants(&space_id.to_string(), &resolved.feature_set_ids)
        .await
        .map_err(|e| MetaToolError::Internal(e.to_string()))?;

    call.ctx
        .tool_discovery
        .build_index(&space_id.to_string(), &invokable)
        .await
        .map_err(|e| MetaToolError::Internal(e.to_string()))
}

/// Build the active index and store it in the per-session search cache.
async fn build_and_cache_active_index(
    call: &MetaToolCall<'_>,
    space_id: &Uuid,
    resolved: &ResolvedFeatureSet,
    fingerprint: u64,
    session_id: &str,
) -> Result<Vec<crate::services::ToolIndexEntry>, MetaToolError> {
    let index = build_active_index(call, space_id, resolved).await?;
    call.ctx
        .search_cache
        .insert(session_id.to_string(), (fingerprint, index.clone()));
    Ok(index)
}

// ---------------------------------------------------------------------------
// mcpmux_search_tools — read
// ---------------------------------------------------------------------------

pub struct SearchToolsTool;

#[async_trait]
impl MetaTool for SearchToolsTool {
    fn name(&self) -> &'static str {
        "mcpmux_search_tools"
    }

    fn description(&self) -> &'static str {
        "Search backend tools in the caller's resolved Space. By default only \
         invokable (active/bound) tools match. Set include_inactive: true to \
         also match tools in unbound FeatureSets (annotated with status and \
         bindable_feature_set_id — activate via mcpmux_bind_current_workspace). \
         Supports query, server_id filter, detail_level (name | description | \
         schema), and pagination."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "server_id": { "type": "string" },
                "include_inactive": {
                    "type": "boolean",
                    "default": false,
                    "description": "When true, include tools from FeatureSets not bound to this workspace (inactive matches carry bindable_feature_set_id)"
                },
                "detail_level": {
                    "type": "string",
                    "enum": ["name", "description", "schema"],
                    "default": "description"
                },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 },
                "cursor": { "type": "string" }
            }
        })
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let started = Instant::now();
        let query_id: String = Uuid::new_v4()
            .to_string()
            .chars()
            .filter(|c| *c != '-')
            .take(8)
            .collect();

        let resolved = caller_resolution(&call).await?;
        let space_id = caller_space_id(&call).await?;

        let query_str = call.args.get("query").and_then(|v| v.as_str());

        let detail_level = call
            .args
            .get("detail_level")
            .and_then(|v| v.as_str())
            .and_then(crate::services::tool_discovery::DetailLevel::parse)
            .unwrap_or(crate::services::tool_discovery::DetailLevel::Description);

        let limit = call
            .args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        let include_inactive = call
            .args
            .get("include_inactive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let fingerprint = feature_set_ids_fingerprint(&resolved.feature_set_ids);

        info!(
            query_id = %query_id,
            session_id = ?call.session_id,
            fingerprint,
            query_len = query_str.map(str::len).unwrap_or(0),
            detail_level = ?detail_level,
            limit,
            include_inactive,
            "[search] call entry"
        );
        if let Some(query) = query_str {
            debug!(query_id = %query_id, query, "[search] query text");
        }

        let mut index_cache_hit = false;
        let active_index = if let Some(session_id) = call.session_id {
            if let Some(entry) = call.ctx.search_cache.get(session_id) {
                let (cached_fp, cached_index) = entry.value();
                if *cached_fp == fingerprint {
                    index_cache_hit = true;
                    cached_index.clone()
                } else {
                    drop(entry);
                    build_and_cache_active_index(
                        &call,
                        &space_id,
                        &resolved,
                        fingerprint,
                        session_id,
                    )
                    .await?
                }
            } else {
                build_and_cache_active_index(&call, &space_id, &resolved, fingerprint, session_id)
                    .await?
            }
        } else {
            build_active_index(&call, &space_id, &resolved).await?
        };

        let mut index = active_index.clone();

        let server_id_filter = call.args.get("server_id").and_then(|v| v.as_str());
        let mut inactive_tool_count = 0usize;

        if include_inactive {
            let inactive = call
                .ctx
                .feature_service
                .list_inactive_discovery_tools(&space_id.to_string(), &resolved.feature_set_ids)
                .await
                .map_err(|e| MetaToolError::Internal(e.to_string()))?;
            inactive_tool_count = inactive.len();
            let inactive_index =
                crate::services::tool_discovery::ToolDiscoveryService::build_inactive_index(
                    &inactive,
                );
            let active_keys: HashSet<(String, String)> = index
                .iter()
                .map(|e| (e.server_id.clone(), e.feature_name.clone()))
                .collect();
            for entry in inactive_index {
                let key = (entry.server_id.clone(), entry.feature_name.clone());
                if !active_keys.contains(&key) {
                    index.push(entry);
                }
            }
            index.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
        }

        let hybrid = query_str.and(call.session_id).map(|session_id| {
            crate::services::tool_discovery::SearchContext {
                embeddings: call.ctx.embeddings.as_ref(),
                embedding_cache: call.ctx.embedding_cache.as_ref(),
                session_id,
                fingerprint,
                active_index: active_index.as_slice(),
                index_cache_hit,
            }
        });

        let result = crate::services::tool_discovery::ToolDiscoveryService::search(
            &index,
            query_str,
            server_id_filter,
            detail_level,
            limit,
            call.args.get("cursor").and_then(|v| v.as_str()),
            Some(query_id.as_str()),
            hybrid,
        );

        let top_qualified_name = result
            .tools
            .first()
            .and_then(|tool| tool.get("qualified_name"))
            .and_then(|value| value.as_str())
            .unwrap_or("");

        info!(
            query_id = %query_id,
            ranking = result.ranking,
            total = result.total,
            returned = result.tools.len(),
            top_qualified_name,
            top_fused_score = ?result.top_fused_score,
            total_ms = started.elapsed().as_millis() as u64,
            "[search] result summary"
        );

        let mut payload = json!({
            "tools": result.tools,
            "next_cursor": result.next_cursor,
            "total": result.total,
            "ranking": result.ranking,
            "scope": if include_inactive { "active_and_inactive" } else { "active_only" },
        });

        if include_inactive && inactive_tool_count > 50 && server_id_filter.is_none() {
            payload["hint"] = json!("Narrow with `server_id` for faster results.");
        }

        if !include_inactive && result.total == 0 {
            payload["hint"] = json!(
                "No active tools matched. Retry with include_inactive: true to discover \
                 bindable capability, or call mcpmux_list_feature_sets then \
                 mcpmux_bind_current_workspace with a feature_set_id."
            );
        } else if include_inactive && result.total == 0 {
            let catalog = call
                .ctx
                .tool_discovery
                .build_catalog_index(&space_id.to_string())
                .await
                .map_err(|e| MetaToolError::Internal(e.to_string()))?;
            let catalog_result = crate::services::tool_discovery::ToolDiscoveryService::search(
                &catalog,
                query_str,
                call.args.get("server_id").and_then(|v| v.as_str()),
                detail_level,
                limit,
                call.args.get("cursor").and_then(|v| v.as_str()),
                Some(query_id.as_str()),
                None,
            );
            if catalog_result.total > 0 {
                payload["hint"] = json!(
                    "Matching tools exist in this Space but no FeatureSet contains them. \
                     Ask the user to create a bundle in the McpMux desktop or web UI \
                     (Workspaces → Feature Sets), then mcpmux_bind_current_workspace \
                     with the new feature_set_id."
                );
            }
        }

        Ok(text_result(payload))
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
// Writes — each goes through the ApprovalBroker before mutating state.
// ---------------------------------------------------------------------------

/// Common path for every write tool: build payload, ask broker, run the
/// mutation. Returns the broker's decision so the caller can proceed only
/// on success. `mutate` is the thing that runs post-approval and is
/// expected to emit `tools/list_changed` when relevant.
pub(crate) async fn with_approval<F, Fut, T>(
    call: &MetaToolCall<'_>,
    tool_name: &'static str,
    summary: String,
    diff: Option<Value>,
    affects_other_clients: bool,
    raw_args: Value,
    mutate: F,
) -> Result<T, MetaToolError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, MetaToolError>>,
{
    let payload = ApprovalPayload {
        tool_name: tool_name.to_string(),
        summary,
        diff,
        raw_args,
        affects_other_clients,
    };
    call.ctx
        .approval_broker
        .request_approval(call.client_id, tool_name, payload)
        .await?;
    mutate().await
}

fn parse_uuid_arg(args: &Value, field: &str) -> Result<Uuid, MetaToolError> {
    let s = args
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| MetaToolError::InvalidArgument(format!("missing `{field}`")))?;
    Uuid::parse_str(s)
        .map_err(|_| MetaToolError::InvalidArgument(format!("`{field}` is not a UUID: {s}")))
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
