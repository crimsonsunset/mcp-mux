//! Routing Service - Request dispatch and permission filtering
//!
//! RoutingService handles:
//! - Listing tools/prompts/resources filtered by client grants
//! - Dispatching tool calls to the correct backend server
//! - Handling 401 errors with automatic token refresh and retry
//!
//! Uses FeatureService for permission resolution and TokenService for refresh.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use mcpmux_core::{FeatureType, LogLevel, LogSource, ServerFeature, ServerLog, ServerLogManager};
use rmcp::model::{CallToolRequestParams, CallToolResult, Content, Meta};
use serde_json::Value;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::connection::ConnectionResult;
use super::features::FeatureService;
use super::service::PoolService;
/// A tool as returned by the routing service
#[derive(Debug, Clone)]
pub struct RoutedTool {
    pub name: String,
    pub server_id: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

/// A prompt as returned by the routing service
#[derive(Debug, Clone)]
pub struct RoutedPrompt {
    pub name: String,
    pub server_id: String,
    pub description: Option<String>,
}

/// A resource as returned by the routing service
#[derive(Debug, Clone)]
pub struct RoutedResource {
    pub uri: String,
    pub server_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Result of a tool call
#[derive(Debug, Clone)]
pub struct ToolCallResult {
    pub content: Vec<Value>,
    pub structured_content: Option<Value>,
    pub is_error: bool,
    pub meta: Option<Meta>,
}

impl ToolCallResult {
    fn from_mcp_result(result: CallToolResult) -> Self {
        Self {
            content: result
                .content
                .into_iter()
                .map(|item| serde_json::to_value(item).unwrap_or(Value::Null))
                .collect(),
            structured_content: result.structured_content,
            is_error: result.is_error.unwrap_or(false),
            meta: result.meta,
        }
    }

    pub(crate) fn into_mcp_result(self) -> CallToolResult {
        let content: Vec<Content> = self
            .content
            .into_iter()
            .filter_map(|item| serde_json::from_value(item).ok())
            .collect();
        let mut result = if self.is_error {
            CallToolResult::error(content)
        } else {
            CallToolResult::success(content)
        };
        result.structured_content = self.structured_content;
        result.meta = self.meta;
        result
    }
}

/// Default timeout for MCP tool calls (60 seconds)
const TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Actionable error when a server is not in the effective enable set.
pub fn format_server_inactive_error(server_id: &str) -> String {
    format!(
        "server '{server_id}' is inactive → mcpmux_bind_current_workspace with a FeatureSet that includes this server"
    )
}

/// Actionable error when a bound server is not connected.
pub fn format_server_bound_offline_error(server_id: &str) -> String {
    format!(
        "Server '{server_id}' is bound but not connected. Run mcpmux_diagnose_server to see why."
    )
}

/// Actionable error when invoke targets a tool outside the permission set.
pub fn format_invoke_permission_denied(
    qualified_name: &str,
    server_id: &str,
    tool_name: &str,
    suggestions: &[String],
) -> String {
    if suggestions.is_empty() {
        format!(
            "tool '{qualified_name}' is not invokable with current grants (server_id='{server_id}', tool='{tool_name}')"
        )
    } else {
        format!(
            "tool '{qualified_name}' is not invokable — did you mean {}?",
            suggestions.join(", ")
        )
    }
}

/// Redirect message for direct backend `read_resource` attempts.
#[allow(dead_code)]
pub fn format_direct_read_redirect(uri: &str) -> String {
    format!(
        "Direct backend resource reads are not supported. \
         Use mcpmux_search_resources to discover readable URIs, then \
         mcpmux_read_resource to fetch one: \
         mcpmux_read_resource({{ \"uri\": \"{uri}\" }})"
    )
}

/// Redirect message for direct backend `get_prompt` attempts.
#[allow(dead_code)]
pub fn format_direct_fetch_prompt_redirect(
    qualified_name: &str,
    server_id: &str,
    prompt_name: &str,
) -> String {
    format!(
        "Direct backend prompt fetches are not supported. \
         Use mcpmux_search_prompts to discover fetchable prompts, then \
         mcpmux_fetch_prompt to fetch one: \
         mcpmux_fetch_prompt({{ \"server_id\": \"{server_id}\", \"prompt\": \"{prompt_name}\", \"args\": {{}} }}) \
         (qualified name was '{qualified_name}')"
    )
}

/// Actionable error when a server is not in the binding FeatureSet ACL.
pub fn format_server_not_in_binding_error(server_id: &str) -> String {
    format!(
        "server '{server_id}' has no readable/fetchable features with current FeatureSet grants — \
         create a FeatureSet bundle in the McpMux desktop or web UI, then bind it with \
         mcpmux_bind_current_workspace"
    )
}

/// Redirect message for direct backend `call_tool` attempts.
pub fn format_direct_call_redirect(
    qualified_name: &str,
    server_id: &str,
    tool_name: &str,
) -> String {
    format!(
        "Direct backend tool calls are not supported. Use mcpmux_invoke_tool instead: \
         mcpmux_invoke_tool({{ \"server_id\": \"{server_id}\", \"tool\": \"{tool_name}\", \"args\": {{}} }}) \
         (qualified name was '{qualified_name}')"
    )
}

/// RoutingService dispatches requests to backend MCP servers
pub struct RoutingService {
    feature_service: Arc<FeatureService>,
    pool_service: Arc<PoolService>,
    log_manager: Arc<ServerLogManager>,
}

impl RoutingService {
    /// Create a routing service. Transport re-resolve on dead connections
    /// goes through [`PoolService::reconnect_fresh_from_db`].
    pub fn new(
        feature_service: Arc<FeatureService>,
        pool_service: Arc<PoolService>,
        log_manager: Arc<ServerLogManager>,
    ) -> Self {
        Self {
            feature_service,
            pool_service,
            log_manager,
        }
    }

    /// List tools available to a client based on their grants
    ///
    /// Returns tools from all connected servers, filtered by the client's feature set grants.
    pub async fn list_tools(
        &self,
        space_id: Uuid,
        feature_set_ids: &[String],
    ) -> Result<Vec<RoutedTool>> {
        let space_id_str = space_id.to_string();

        // Resolve feature sets to allowed features
        let allowed_features = self
            .feature_service
            .get_invokable_tools_for_grants(&space_id_str, feature_set_ids)
            .await?;

        // Filter to just tools
        let tools: Vec<RoutedTool> = allowed_features
            .iter()
            .filter(|f| f.feature_type == FeatureType::Tool && f.is_available)
            .map(|f| RoutedTool {
                name: f.qualified_name(), // server_id/tool_name for disambiguation
                server_id: f.server_id.clone(),
                description: f.description.clone(),
                input_schema: None, // Raw JSON is used in handlers now
            })
            .collect();

        debug!(
            "[RoutingService] Listed {} tools for grants {:?}",
            tools.len(),
            feature_set_ids
        );

        Ok(tools)
    }

    /// List prompts available to a client based on their grants
    pub async fn list_prompts(
        &self,
        space_id: Uuid,
        feature_set_ids: &[String],
    ) -> Result<Vec<RoutedPrompt>> {
        let space_id_str = space_id.to_string();

        let allowed_features = self
            .feature_service
            .get_prompts_for_grants(&space_id_str, feature_set_ids)
            .await?;

        let prompts: Vec<RoutedPrompt> = allowed_features
            .iter()
            .filter(|f| f.feature_type == FeatureType::Prompt && f.is_available)
            .map(|f| RoutedPrompt {
                name: f.qualified_name(),
                server_id: f.server_id.clone(),
                description: f.description.clone(),
            })
            .collect();

        debug!(
            "[RoutingService] Listed {} prompts for grants {:?}",
            prompts.len(),
            feature_set_ids
        );

        Ok(prompts)
    }

    /// List resources available to a client based on their grants
    pub async fn list_resources(
        &self,
        space_id: Uuid,
        feature_set_ids: &[String],
    ) -> Result<Vec<RoutedResource>> {
        let space_id_str = space_id.to_string();

        let allowed_features = self
            .feature_service
            .get_resources_for_grants(&space_id_str, feature_set_ids)
            .await?;

        let resources: Vec<RoutedResource> = allowed_features
            .iter()
            .filter(|f| f.feature_type == FeatureType::Resource && f.is_available)
            .map(|f| RoutedResource {
                uri: f.qualified_name(), // Use qualified name (prefix.resource_name)
                server_id: f.server_id.clone(),
                name: f.display_name.clone(),
                description: f.description.clone(),
            })
            .collect();

        debug!(
            "[RoutingService] Listed {} resources for grants {:?}",
            resources.len(),
            feature_set_ids
        );

        Ok(resources)
    }

    /// Call a tool on a backend server
    pub async fn call_tool(
        &self,
        space_id: Uuid,
        feature_set_ids: &[String],
        tool_name: &str,
        arguments: Value,
    ) -> Result<ToolCallResult> {
        let space_id_str = space_id.to_string();

        // 1. Authorize against the grants. The qualified name must match exactly one
        //    available tool; a name two servers could both claim (a shared prefix, or
        //    `gh` + `evil_delete` against `gh_evil` + `delete`) is refused rather
        //    than routed to whichever came first.
        let allowed_features = self
            .feature_service
            .get_invokable_tools_for_grants(&space_id_str, feature_set_ids)
            .await?;

        let feature =
            match_feature(&allowed_features, FeatureType::Tool, tool_name).map_err(|e| {
                warn!("[RoutingService] {}", e);
                e
            })?;

        // Whether the tool says calling it twice is harmless; only then is a call
        // whose result was an auth error run again after reconnecting.
        let retry_safe = feature.is_some_and(Self::tool_is_retry_safe);

        let (server_id, actual_tool_name) = match feature {
            Some(f) => (f.server_id.clone(), f.feature_name.clone()),
            None => {
                // Not invokable with these grants. Keep the fork's precise message:
                // it names the server when the prefix cache knows the tool.
                let known = self
                    .feature_service
                    .find_server_for_qualified_tool(&space_id_str, tool_name)
                    .await?;
                warn!(
                    "[RoutingService] Tool '{}' not in the resolved feature set ({} tools available)",
                    tool_name,
                    allowed_features
                        .iter()
                        .filter(|f| f.feature_type == FeatureType::Tool && f.is_available)
                        .count()
                );
                return Err(match known {
                    Some((server_id, actual_tool_name)) => {
                        anyhow!(format_invoke_permission_denied(
                            tool_name,
                            &server_id,
                            &actual_tool_name,
                            &[],
                        ))
                    }
                    None => anyhow!("Tool '{}' not found", tool_name),
                });
            }
        };

        info!(
            "[RoutingService] Checking authorization for tool '{}' (server: {}, actual_name: {})",
            tool_name, server_id, actual_tool_name
        );
        info!(
            "[RoutingService] Feature sets to check: {:?}",
            feature_set_ids
        );
        info!(
            "[RoutingService] Total allowed features: {}",
            allowed_features.len()
        );

        info!("[RoutingService] Tool '{}' is ALLOWED", tool_name);

        info!(
            "[RoutingService] Calling tool {} on server {}",
            actual_tool_name, server_id
        );

        // Log the tool call attempt
        self.log(
            &space_id,
            &server_id,
            LogLevel::Info,
            format!("Calling tool: {}", actual_tool_name),
            Some(serde_json::json!({
                "tool": actual_tool_name,
                "arguments": arguments
            })),
        )
        .await;

        // Define the call operation
        // Function to execute the call on the instance
        async fn execute_call(
            pool: Arc<PoolService>,
            space_id: Uuid,
            server_id: String,
            tool_name: String,
            args: Value,
        ) -> Result<ToolCallResult> {
            let instance = pool
                .get_instance(space_id, &server_id)
                .ok_or_else(|| anyhow!("Server not connected: {}", server_id))?;

            // We need to get the service handle (peer) which is cloneable
            // But we don't have direct access to it via with_client easily because with_client
            // passes &McpClient (RunningService).
            // We can assume RunningService is not cloneable but its peer() returns a Service handle which is.
            // Let's use with_client to get the handle out.
            let client_handle = instance.with_client(|client| client.peer().clone());

            match client_handle {
                Some(client) => {
                    let mut params = CallToolRequestParams::new(tool_name.to_string());
                    params.arguments = args.as_object().cloned();

                    // Wrap call_tool with timeout to prevent hanging
                    let res = tokio::time::timeout(TOOL_CALL_TIMEOUT, client.call_tool(params))
                        .await
                        .map_err(|_| anyhow!("Tool call timed out after {:?}", TOOL_CALL_TIMEOUT))?
                        .map_err(|e| anyhow!("MCP call failed: {}", e))?;

                    Ok(ToolCallResult::from_mcp_result(res))
                }
                None => Err(anyhow!("Server instance has no active client")),
            }
        }

        // 3. Dispatch the call with retry logic
        // NOTE: Preemptive token refresh is no longer needed here.
        // RMCP's AuthClient with DatabaseCredentialStore handles token refresh
        // automatically on every HTTP request when needed.
        info!(
            "[RoutingService] Executing tool call: {} on {} (timeout: {:?})",
            actual_tool_name, server_id, TOOL_CALL_TIMEOUT
        );

        let call_start = std::time::Instant::now();
        match execute_call(
            self.pool_service.clone(),
            space_id,
            server_id.clone(),
            actual_tool_name.clone(),
            arguments.clone(),
        )
        .await
        {
            Ok(result) => {
                let duration = call_start.elapsed();
                if result.is_error {
                    // Check if this is an auth error embedded in the tool result.
                    // Some servers (e.g., Atlassian) return 401 as tool results rather than
                    // HTTP errors. The SDK refreshes the token successfully, but the server's
                    // internal session may be stale. A fresh MCP connection fixes this.
                    if Self::result_is_auth_failure(&result.content, true) {
                        warn!(
                            "[RoutingService] Auth error in tool result for {}/{}, attempting auto-reconnect",
                            server_id, actual_tool_name
                        );
                        self.record_call_outcome(
                            &space_id,
                            &server_id,
                            false,
                            Some("auth error in tool result"),
                        );
                        self.log(
                            &space_id,
                            &server_id,
                            LogLevel::Warn,
                            format!(
                                "Auth error in tool result for '{}' - auto-reconnecting",
                                actual_tool_name
                            ),
                            Some(serde_json::json!({ "result": result.content, "duration_ms": duration.as_millis() })),
                        )
                        .await;

                        match self
                            .pool_service
                            .reconnect_instance(space_id, &server_id)
                            .await
                        {
                            // The result may come from a call that ran: only
                            // tools marked read-only or idempotent run again.
                            // Others return the result; the next call uses the
                            // fresh connection.
                            ConnectionResult::Connected { .. } if !retry_safe => {
                                info!(
                                    "[RoutingService] Reconnected {}; not re-running {} (not marked read-only or idempotent)",
                                    server_id, actual_tool_name
                                );
                                Ok(result)
                            }
                            ConnectionResult::Connected { .. } => {
                                info!(
                                    "[RoutingService] Reconnected {}, retrying tool call: {}",
                                    server_id, actual_tool_name
                                );

                                let retry_start = std::time::Instant::now();
                                match execute_call(
                                    self.pool_service.clone(),
                                    space_id,
                                    server_id.clone(),
                                    actual_tool_name.clone(),
                                    arguments.clone(),
                                )
                                .await
                                {
                                    Ok(retry_result) => {
                                        let retry_duration = retry_start.elapsed();
                                        if retry_result.is_error {
                                            warn!(
                                                "[RoutingService] Tool retry still has error: {} (duration: {:?})",
                                                actual_tool_name, retry_duration
                                            );
                                            self.record_call_outcome(
                                                &space_id,
                                                &server_id,
                                                false,
                                                Some("tool result is_error after reconnect"),
                                            );
                                        } else {
                                            info!(
                                                "[RoutingService] Tool retry succeeded after reconnect: {} (duration: {:?})",
                                                actual_tool_name, retry_duration
                                            );
                                            self.record_call_outcome(
                                                &space_id, &server_id, true, None,
                                            );
                                        }
                                        self.log(
                                            &space_id,
                                            &server_id,
                                            LogLevel::Info,
                                            format!(
                                                "Tool '{}' retried after auto-reconnect (is_error={})",
                                                actual_tool_name, retry_result.is_error
                                            ),
                                            Some(serde_json::json!({ "retry_duration_ms": retry_duration.as_millis() })),
                                        )
                                        .await;
                                        Ok(retry_result)
                                    }
                                    Err(retry_err) => {
                                        warn!(
                                            "[RoutingService] Tool retry transport error: {} - {}",
                                            actual_tool_name, retry_err
                                        );
                                        self.log(
                                            &space_id,
                                            &server_id,
                                            LogLevel::Error,
                                            format!(
                                                "Tool '{}' still failing after reconnect",
                                                actual_tool_name
                                            ),
                                            Some(serde_json::json!({ "error": retry_err.to_string() })),
                                        )
                                        .await;
                                        // Return original tool result since it has the error details
                                        Ok(result)
                                    }
                                }
                            }
                            other => {
                                warn!(
                                    "[RoutingService] Auto-reconnect failed for {}: {:?}",
                                    server_id, other
                                );
                                self.log(
                                    &space_id,
                                    &server_id,
                                    LogLevel::Error,
                                    format!(
                                        "Auto-reconnect failed for tool '{}' - manual reconnection required",
                                        actual_tool_name
                                    ),
                                    Some(serde_json::json!({ "reconnect_result": format!("{:?}", other) })),
                                )
                                .await;
                                Ok(result)
                            }
                        }
                    } else {
                        warn!(
                            "[RoutingService] Tool execution error: {} (duration: {:?})",
                            actual_tool_name, duration
                        );
                        self.record_call_outcome(
                            &space_id,
                            &server_id,
                            false,
                            Some("tool result is_error"),
                        );
                        self.log(
                            &space_id,
                            &server_id,
                            LogLevel::Error,
                            format!("Tool execution error: {}", actual_tool_name),
                            Some(serde_json::json!({ "result": result.content, "duration_ms": duration.as_millis() }))
                        ).await;
                        Ok(result)
                    }
                } else {
                    // Even on "success" (is_error=false), some servers (e.g., Atlassian)
                    // return auth errors as plain text content like {"code":401,"message":"Unauthorized"}.
                    // Detect these and auto-reconnect + retry.
                    if Self::result_is_auth_failure(&result.content, false) {
                        warn!(
                            "[RoutingService] Auth error in successful tool result for {}/{}, attempting auto-reconnect",
                            server_id, actual_tool_name
                        );
                        self.record_call_outcome(
                            &space_id,
                            &server_id,
                            false,
                            Some("auth error in tool result"),
                        );
                        self.log(
                            &space_id,
                            &server_id,
                            LogLevel::Warn,
                            format!(
                                "Auth error in tool result for '{}' (is_error=false) - auto-reconnecting",
                                actual_tool_name
                            ),
                            Some(serde_json::json!({ "result": result.content, "duration_ms": duration.as_millis() })),
                        )
                        .await;

                        match self
                            .pool_service
                            .reconnect_instance(space_id, &server_id)
                            .await
                        {
                            // The result may come from a call that ran: only
                            // tools marked read-only or idempotent run again.
                            // Others return the result; the next call uses the
                            // fresh connection.
                            ConnectionResult::Connected { .. } if !retry_safe => {
                                info!(
                                    "[RoutingService] Reconnected {}; not re-running {} (not marked read-only or idempotent)",
                                    server_id, actual_tool_name
                                );
                                Ok(result)
                            }
                            ConnectionResult::Connected { .. } => {
                                info!(
                                    "[RoutingService] Reconnected {}, retrying tool call: {}",
                                    server_id, actual_tool_name
                                );

                                let retry_start = std::time::Instant::now();
                                match execute_call(
                                    self.pool_service.clone(),
                                    space_id,
                                    server_id.clone(),
                                    actual_tool_name.clone(),
                                    arguments.clone(),
                                )
                                .await
                                {
                                    Ok(retry_result) => {
                                        let retry_duration = retry_start.elapsed();
                                        info!(
                                            "[RoutingService] Tool retry result: {} (is_error={}, duration: {:?})",
                                            actual_tool_name, retry_result.is_error, retry_duration
                                        );
                                        self.record_call_outcome(
                                            &space_id,
                                            &server_id,
                                            !retry_result.is_error,
                                            retry_result
                                                .is_error
                                                .then_some("tool result is_error after reconnect"),
                                        );
                                        self.log(
                                            &space_id,
                                            &server_id,
                                            LogLevel::Info,
                                            format!(
                                                "Tool '{}' retried after auto-reconnect (is_error={})",
                                                actual_tool_name, retry_result.is_error
                                            ),
                                            Some(serde_json::json!({ "retry_duration_ms": retry_duration.as_millis() })),
                                        )
                                        .await;
                                        Ok(retry_result)
                                    }
                                    Err(retry_err) => {
                                        warn!(
                                            "[RoutingService] Tool retry transport error: {} - {}",
                                            actual_tool_name, retry_err
                                        );
                                        Ok(result)
                                    }
                                }
                            }
                            other => {
                                warn!(
                                    "[RoutingService] Auto-reconnect failed for {}: {:?}",
                                    server_id, other
                                );
                                Ok(result)
                            }
                        }
                    } else {
                        info!(
                            "[RoutingService] Tool executed successfully: {} (duration: {:?})",
                            actual_tool_name, duration
                        );
                        self.record_call_outcome(&space_id, &server_id, true, None);
                        self.log(
                            &space_id,
                            &server_id,
                            LogLevel::Info,
                            format!("Tool executed successfully: {}", actual_tool_name),
                            Some(serde_json::json!({ "duration_ms": duration.as_millis() })),
                        )
                        .await;
                        Ok(result)
                    }
                }
            }
            Err(e) => {
                let duration = call_start.elapsed();
                let err_str = e.to_string().to_lowercase();
                let reconnect_path = reconnect_path_for_error(&err_str);
                let trigger = failure_trigger_label(reconnect_path);

                self.record_call_outcome(&space_id, &server_id, false, Some(&err_str));
                self.log_backend_call_failure(
                    &space_id,
                    &server_id,
                    &actual_tool_name,
                    &err_str,
                    trigger,
                );

                if let Some(path) = reconnect_path {
                    self.log(
                        &space_id,
                        &server_id,
                        LogLevel::Warn,
                        format!(
                            "{trigger} error on tool '{actual_tool_name}' - auto-reconnecting"
                        ),
                        Some(serde_json::json!({ "error": e.to_string(), "duration_ms": duration.as_millis(), "trigger": trigger })),
                    )
                    .await;

                    let reconnect_result = match path {
                        ReconnectPath::OAuth => {
                            self.pool_service
                                .reconnect_instance(space_id, &server_id)
                                .await
                        }
                        ReconnectPath::Fresh => {
                            self.reconnect_fresh_from_db(space_id, &server_id).await
                        }
                    };
                    info!(
                        server_id = %server_id,
                        ok = reconnect_result.is_connected(),
                        "reconnect attempted after call_tool failure"
                    );

                    match reconnect_result {
                        ConnectionResult::Connected { .. } => {
                            info!(
                                "[RoutingService] Reconnected {}, retrying tool call: {}",
                                server_id, actual_tool_name
                            );

                            let retry_start = std::time::Instant::now();
                            match execute_call(
                                self.pool_service.clone(),
                                space_id,
                                server_id.clone(),
                                actual_tool_name.clone(),
                                arguments.clone(),
                            )
                            .await
                            {
                                Ok(result) => {
                                    let retry_duration = retry_start.elapsed();
                                    info!(
                                        "[RoutingService] Tool retry succeeded: {} (duration: {:?})",
                                        actual_tool_name, retry_duration
                                    );
                                    self.record_call_outcome(&space_id, &server_id, true, None);
                                    self.log(
                                        &space_id,
                                        &server_id,
                                        LogLevel::Info,
                                        format!(
                                            "Tool '{}' succeeded after auto-reconnect",
                                            actual_tool_name
                                        ),
                                        Some(serde_json::json!({ "retry_duration_ms": retry_duration.as_millis() })),
                                    )
                                    .await;
                                    Ok(result)
                                }
                                Err(retry_err) => {
                                    warn!(
                                        "[RoutingService] Tool retry also failed: {} - {}",
                                        actual_tool_name, retry_err
                                    );
                                    self.record_call_outcome(
                                        &space_id,
                                        &server_id,
                                        false,
                                        Some(&retry_err.to_string()),
                                    );
                                    self.log(
                                        &space_id,
                                        &server_id,
                                        LogLevel::Error,
                                        format!(
                                            "Tool '{}' still failing after reconnect - manual reconnection required",
                                            actual_tool_name
                                        ),
                                        Some(serde_json::json!({ "error": retry_err.to_string() })),
                                    )
                                    .await;
                                    Err(anyhow!(
                                        "Server '{}' {trigger} error persists after auto-reconnect. Please disconnect and connect again. Error: {}",
                                        server_id,
                                        retry_err
                                    ))
                                }
                            }
                        }
                        other => {
                            warn!(
                                "[RoutingService] Auto-reconnect failed for {}: {:?}",
                                server_id, other
                            );
                            self.log(
                                &space_id,
                                &server_id,
                                LogLevel::Error,
                                format!(
                                    "Auto-reconnect failed for tool '{}' - manual reconnection required",
                                    actual_tool_name
                                ),
                                Some(serde_json::json!({ "reconnect_result": format!("{:?}", other) })),
                            )
                            .await;
                            Err(anyhow!(
                                "Server '{}' requires reconnection. Auto-reconnect failed. Please disconnect and connect again.",
                                server_id
                            ))
                        }
                    }
                } else {
                    self.log(
                        &space_id,
                        &server_id,
                        LogLevel::Error,
                        format!("Tool call failed: {}", e),
                        Some(serde_json::json!({ "error": e.to_string(), "duration_ms": duration.as_millis(), "trigger": trigger })),
                    )
                    .await;
                    Err(e)
                }
            }
        }
    }

    /// Log an event
    async fn log(
        &self,
        space_id: &Uuid,
        server_id: &str,
        level: LogLevel,
        message: String,
        metadata: Option<Value>,
    ) {
        let mut log = ServerLog::new(level, LogSource::App, message);
        if let Some(meta) = metadata {
            log = log.with_metadata(meta);
        }

        if let Err(e) = self
            .log_manager
            .append(&space_id.to_string(), server_id, log)
            .await
        {
            warn!("[RoutingService] Failed to log event: {}", e);
        }
    }

    /// Record call-time stats on the pooled instance without flipping `state`.
    fn record_call_outcome(&self, space_id: &Uuid, server_id: &str, ok: bool, error: Option<&str>) {
        let Some(instance) = self.pool_service.get_instance(*space_id, server_id) else {
            return;
        };
        if ok {
            instance.record_success();
        } else {
            instance.record_failure(error.unwrap_or("unknown"));
        }
    }

    /// Structured breadcrumb for every transport-level `call_tool` failure.
    fn log_backend_call_failure(
        &self,
        space_id: &Uuid,
        server_id: &str,
        tool: &str,
        err_str: &str,
        trigger: &'static str,
    ) {
        let (instance_age_secs, consecutive_failures, requests_served) =
            match self.pool_service.get_instance(*space_id, server_id) {
                Some(instance) => {
                    let stats = instance.stats.read();
                    (
                        stats.connected_at.map(|t| t.elapsed().as_secs()),
                        stats.consecutive_failures,
                        stats.requests_served,
                    )
                }
                None => (None, 0, 0),
            };
        warn!(
            server_id = %server_id,
            space_id = %space_id,
            tool = %tool,
            error = %err_str,
            trigger,
            instance_age_secs,
            consecutive_failures,
            requests_served,
            "backend call_tool failed"
        );
    }

    /// Evict + re-resolve + connect. Used for non-auth transport-closed errors.
    async fn reconnect_fresh_from_db(&self, space_id: Uuid, server_id: &str) -> ConnectionResult {
        self.pool_service
            .reconnect_fresh_from_db(space_id, server_id)
            .await
    }

    /// Check if an error string indicates authentication is needed
    fn is_auth_error(error_str: &str) -> bool {
        let lower = error_str.to_ascii_lowercase();
        let indicators = [
            "401",
            "unauthorized",
            "invalid_token",
            "token expired",
            "access token",
        ];
        indicators.iter().any(|s| lower.contains(s))
    }

    /// Check if an error string indicates the backend transport died.
    ///
    /// Matches after case/punctuation/camelCase normalize so rmcp Display
    /// variants (`Transport closed`, `connection closed: …`, `Transport channel
    /// closed`, `unexpected end of stream`) and MCP `-32000` all hit the same
    /// path. Auth is classified first by [`reconnect_path_for_error`].
    fn is_transport_closed_error(error_str: &str) -> bool {
        let lower = error_str.to_ascii_lowercase();
        if lower.contains("-32000") {
            return true;
        }
        let tokens = normalize_error_tokens(error_str);
        const PHRASES: &[&str] = &[
            "transport closed",
            "connection closed",
            "channel closed",
            "transport terminated",
            "handler terminated",
            "unexpected end of stream",
            "session expired",
            "broken pipe",
            "connection reset",
            "keep alive timeout",
            "transport send error",
            "send message error",
        ];
        if PHRASES.iter().any(|p| tokens.contains(p)) {
            return true;
        }
        tokens.contains("closed")
            && ["transport", "connection", "channel", "stream"]
                .iter()
                .any(|noun| tokens.contains(noun))
    }

    /// Whether a tool result is the server reporting an authentication failure
    /// (rather than tool output that happens to mention one).
    ///
    /// Some MCP servers (e.g., Atlassian) return auth errors as tool results
    /// rather than HTTP-level errors — `is_error: true` with a 401 message, or
    /// even a "successful" `{"code":401,"message":"Unauthorized"}`. The SDK may
    /// have already refreshed the token, but the server's internal session can
    /// be stale, and a fresh connection fixes it; the caller then reconnects and
    /// runs the call again. Because the retry repeats the call, only a result
    /// that is *itself* an auth error qualifies: a single short text item that
    /// is a structured 401 error, or, for `is_error` results, a short auth error
    /// message. Output that merely contains these words (an issue body, a web
    /// page, documentation) never triggers a second call.
    fn result_is_auth_failure(content: &[Value], is_error: bool) -> bool {
        const MAX_ERROR_TEXT: usize = 512;
        let [item] = content else {
            return false;
        };
        let Some(text) = item.get("text").and_then(|v| v.as_str()) else {
            return false;
        };
        let text = text.trim();
        if text.len() > MAX_ERROR_TEXT {
            return false;
        }
        if let Ok(json) = serde_json::from_str::<Value>(text) {
            // A "successful" result only counts when it is nothing but an
            // error envelope, not tool output (e.g. a fetched page's body)
            // that carries a 401 among other fields.
            if Self::json_is_auth_error(&json) && (is_error || Self::is_error_envelope(&json)) {
                return true;
            }
        }
        // A failed call whose (short) error message names an auth failure, e.g.
        // "401 Unauthorized" or {"message":"Token expired"}.
        is_error && Self::is_auth_error(&text.to_lowercase())
    }

    /// A JSON object made only of error-envelope fields.
    fn is_error_envelope(json: &Value) -> bool {
        const ENVELOPE_KEYS: [&str; 7] = [
            "code",
            "status",
            "statusCode",
            "status_code",
            "message",
            "error",
            "error_description",
        ];
        json.as_object()
            .is_some_and(|obj| obj.keys().all(|k| ENVELOPE_KEYS.contains(&k.as_str())))
    }

    /// Whether a tool's MCP annotations say running it again is harmless
    /// (`readOnlyHint` or `idempotentHint`).
    fn tool_is_retry_safe(feature: &ServerFeature) -> bool {
        let hint = |name: &str| {
            feature
                .raw_json
                .as_ref()
                .and_then(|tool| tool.get("annotations"))
                .and_then(|a| a.get(name))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        hint("readOnlyHint") || hint("idempotentHint")
    }

    /// A JSON error object carrying a 401 status/code, or an OAuth
    /// `invalid_token`/`unauthorized` error code.
    fn json_is_auth_error(json: &Value) -> bool {
        let Some(obj) = json.as_object() else {
            return false;
        };
        let is_401 = |v: &Value| v.as_u64() == Some(401) || v.as_str() == Some("401");
        let is_auth_code = |v: &Value| {
            v.as_str().is_some_and(|s| {
                matches!(
                    s.to_ascii_lowercase().as_str(),
                    "unauthorized" | "invalid_token" | "unauthenticated"
                )
            })
        };
        ["code", "status", "statusCode", "status_code"]
            .iter()
            .filter_map(|k| obj.get(*k))
            .any(|v| is_401(v) || is_auth_code(v))
            || obj
                .get("error")
                .is_some_and(|e| is_auth_code(e) || Self::json_is_auth_error(e))
    }
}

/// The one available feature of `feature_type` answering to `qualified_name`.
///
/// Features on two servers must never share a qualified name (prefixes are
/// unique, but `_` is legal inside tool names, so `gh` + `evil_delete` and
/// `gh_evil` + `delete` still collide; resource URIs aren't prefixed at all).
/// If they do, the request is refused rather than sent — with its arguments —
/// to whichever server happens to come first.
pub(crate) fn match_feature<'a>(
    features: &'a [ServerFeature],
    feature_type: FeatureType,
    qualified_name: &str,
) -> Result<Option<&'a ServerFeature>> {
    let mut matches = features.iter().filter(|f| {
        f.feature_type == feature_type && f.is_available && f.qualified_name() == qualified_name
    });
    let Some(first) = matches.next() else {
        return Ok(None);
    };
    if let Some(other) = matches.find(|f| f.server_id != first.server_id) {
        return Err(anyhow!(
            "'{}' matches features on more than one server ({} and {}); \
             rename or disable one of them",
            qualified_name,
            first.server_id,
            other.server_id
        ));
    }
    Ok(Some(first))
}

#[cfg(test)]
mod auth_failure_tests {
    use super::RoutingService;
    use serde_json::{json, Value};

    #[test]
    fn tools_sharing_a_name_across_servers_are_refused() {
        use mcpmux_core::{FeatureType, ServerFeature};
        let tool = |server: &str, alias: &str, name: &str| {
            let mut f = ServerFeature::tool("space", server, name);
            f.server_alias = Some(alias.to_string());
            f.is_available = true;
            f
        };
        let find = |features: &[ServerFeature], name: &str| {
            super::match_feature(features, FeatureType::Tool, name)
                .map(|f| f.map(|f| f.server_id.clone()))
        };
        let one = vec![tool("server-a", "github", "create_issue")];
        assert_eq!(
            find(&one, "github_create_issue").unwrap().as_deref(),
            Some("server-a")
        );

        // Same alias on two servers.
        let clash = vec![
            tool("server-a", "github", "create_issue"),
            tool("server-b", "github", "create_issue"),
        ];
        assert!(find(&clash, "github_create_issue").is_err());
        assert!(find(&clash, "github_other").unwrap().is_none());

        // Distinct prefixes whose qualified names still coincide.
        let underscore = vec![
            tool("server-a", "gh", "evil_delete"),
            tool("server-b", "gh_evil", "delete"),
        ];
        assert!(find(&underscore, "gh_evil_delete").is_err());
    }

    #[test]
    fn resources_with_the_same_uri_on_two_servers_are_refused() {
        use mcpmux_core::{FeatureType, ServerFeature};
        let resource = |server: &str| {
            let mut f = ServerFeature::resource("space", server, "memo://notes");
            f.is_available = true;
            f
        };
        let one = vec![resource("server-a")];
        assert!(
            super::match_feature(&one, FeatureType::Resource, "memo://notes")
                .unwrap()
                .is_some()
        );
        let clash = vec![resource("server-a"), resource("server-b")];
        assert!(super::match_feature(&clash, FeatureType::Resource, "memo://notes").is_err());
    }

    fn text(t: &str) -> Vec<Value> {
        vec![json!({ "type": "text", "text": t })]
    }

    #[test]
    fn structured_401_results_are_auth_failures() {
        for t in [
            r#"{"code":401,"message":"Unauthorized"}"#,
            r#"{"status":"401"}"#,
            r#"{"error":"invalid_token"}"#,
            r#"{"error":{"code":401,"message":"token expired"}}"#,
        ] {
            assert!(
                RoutingService::result_is_auth_failure(&text(t), false),
                "{t}"
            );
            assert!(
                RoutingService::result_is_auth_failure(&text(t), true),
                "{t}"
            );
        }
    }

    #[test]
    fn short_auth_error_messages_count_only_when_the_call_failed() {
        for msg in [
            text("401 Unauthorized: token expired"),
            text(r#"{"message":"401 Unauthorized"}"#),
        ] {
            assert!(RoutingService::result_is_auth_failure(&msg, true));
            assert!(!RoutingService::result_is_auth_failure(&msg, false));
        }
    }

    #[test]
    fn a_successful_result_must_be_only_an_error_envelope() {
        // A fetch tool relaying another service's 401 body, with other fields.
        for t in [
            r#"{"url":"https://api.example.com","status":401,"body":"Unauthorized"}"#,
            r#"{"code":401,"message":"Unauthorized","data":[1,2,3]}"#,
        ] {
            assert!(
                !RoutingService::result_is_auth_failure(&text(t), false),
                "{t}"
            );
            // The same text as a failed call's error still counts.
            assert!(
                RoutingService::result_is_auth_failure(&text(t), true),
                "{t}"
            );
        }
        assert!(RoutingService::result_is_auth_failure(
            &text(r#"{"error":"invalid_token","error_description":"expired"}"#),
            false
        ));
    }

    #[test]
    fn only_tools_marked_read_only_or_idempotent_are_re_run() {
        use mcpmux_core::ServerFeature;
        let tool = |annotations: Value| {
            ServerFeature::tool(uuid::Uuid::nil(), "s", "t")
                .with_raw_json(json!({ "name": "t", "annotations": annotations }))
        };
        assert!(RoutingService::tool_is_retry_safe(&tool(
            json!({ "readOnlyHint": true })
        )));
        assert!(RoutingService::tool_is_retry_safe(&tool(
            json!({ "idempotentHint": true })
        )));
        assert!(!RoutingService::tool_is_retry_safe(&tool(
            json!({ "destructiveHint": true })
        )));
        assert!(!RoutingService::tool_is_retry_safe(&tool(json!({}))));
        assert!(!RoutingService::tool_is_retry_safe(&ServerFeature::tool(
            uuid::Uuid::nil(),
            "s",
            "t"
        )));
    }

    #[test]
    fn output_that_mentions_auth_words_is_not_retried() {
        // A created issue whose body happens to say "401 Unauthorized".
        let created =
            text(r#"{"id":4012,"title":"Login fails","body":"API returns 401 Unauthorized"}"#);
        assert!(!RoutingService::result_is_auth_failure(&created, false));
        // Documentation text.
        let docs = text("To call the API, send your access token in the Authorization header.");
        assert!(!RoutingService::result_is_auth_failure(&docs, false));
        // Long output is never treated as an error message.
        let long = text(&format!("unauthorized {}", "x".repeat(600)));
        assert!(!RoutingService::result_is_auth_failure(&long, true));
        // Several content items are tool output, not an error.
        let mut many = text(r#"{"code":401}"#);
        many.extend(text("more"));
        assert!(!RoutingService::result_is_auth_failure(&many, true));
    }
}

/// Which reconnect path a transport-level error should take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReconnectPath {
    OAuth,
    Fresh,
}

impl ReconnectPath {
    /// Structured-log `trigger` label for this path.
    fn trigger_label(self) -> &'static str {
        match self {
            Self::OAuth => "auth",
            Self::Fresh => "transport_closed",
        }
    }
}

/// Label for the structured `call_tool` failure log (`trigger` field).
fn failure_trigger_label(path: Option<ReconnectPath>) -> &'static str {
    path.map(ReconnectPath::trigger_label)
        .unwrap_or("unmatched")
}

/// Collapse camelCase and punctuation so `TransportClosed` / `transport_closed`
/// / `transport-closed` all become `transport closed`.
fn normalize_error_tokens(error_str: &str) -> String {
    let mut with_camel = String::with_capacity(error_str.len() + 8);
    let mut prev_was_lower = false;
    for c in error_str.chars() {
        if c.is_ascii_uppercase() && prev_was_lower {
            with_camel.push(' ');
        }
        with_camel.push(c);
        prev_was_lower = c.is_ascii_lowercase();
    }

    let mut out = String::with_capacity(with_camel.len());
    let mut prev_space = false;
    for c in with_camel.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    out
}

/// Classify a transport error for retry routing. Case-insensitive.
fn reconnect_path_for_error(err_str: &str) -> Option<ReconnectPath> {
    if RoutingService::is_auth_error(err_str) {
        Some(ReconnectPath::OAuth)
    } else if RoutingService::is_transport_closed_error(err_str) {
        Some(ReconnectPath::Fresh)
    } else {
        None
    }
}

#[cfg(test)]
mod redirect_tests {
    use super::{format_direct_call_redirect, ToolCallResult};
    use rmcp::model::{CallToolResult, Content, Meta};
    use serde_json::json;

    #[test]
    fn direct_call_redirect_points_at_invoke_tool() {
        let message = format_direct_call_redirect("github_create_issue", "github", "create_issue");
        assert!(message.contains("mcpmux_invoke_tool"));
        assert!(message.contains("\"server_id\": \"github\""));
        assert!(message.contains("\"tool\": \"create_issue\""));
    }

    #[test]
    fn tool_result_round_trip_preserves_structured_content_and_meta() {
        let structured = json!({ "matches": [{ "message": "found" }] });
        let mut meta = Meta::new();
        meta.0.insert("traceId".to_string(), json!("trace-123"));

        let mut upstream = CallToolResult::structured(structured.clone());
        upstream.content = vec![Content::text("search completed")];
        upstream.meta = Some(meta.clone());

        let routed = ToolCallResult::from_mcp_result(upstream);
        let forwarded = routed.into_mcp_result();

        assert_eq!(forwarded.content, vec![Content::text("search completed")]);
        assert_eq!(forwarded.structured_content, Some(structured));
        assert_eq!(forwarded.meta, Some(meta));
        assert_eq!(forwarded.is_error, Some(false));
    }
}

#[cfg(test)]
mod call_failure_classify_tests {
    use super::{failure_trigger_label, reconnect_path_for_error, ReconnectPath, RoutingService};

    #[test]
    fn reported_minus_32000_is_transport_closed_not_auth() {
        let err = "mcp error -32000: connection closed";
        assert!(RoutingService::is_transport_closed_error(err));
        assert!(!RoutingService::is_auth_error(err));
        assert_eq!(reconnect_path_for_error(err), Some(ReconnectPath::Fresh));
        assert_eq!(
            failure_trigger_label(Some(ReconnectPath::Fresh)),
            "transport_closed"
        );
    }

    #[test]
    fn connection_closed_without_code_is_transport_closed() {
        let err = "connection closed";
        assert_eq!(reconnect_path_for_error(err), Some(ReconnectPath::Fresh));
    }

    #[test]
    fn rmcp_stdio_transport_closed_is_not_unmatched() {
        let err = "MCP call failed: Transport closed";
        assert_eq!(reconnect_path_for_error(err), Some(ReconnectPath::Fresh));
        assert!(!RoutingService::is_auth_error(err));
    }

    #[test]
    fn transport_closed_shapes_normalize_to_fresh() {
        let cases = [
            "TransportClosed",
            "transport_closed",
            "transport-closed",
            "Transport channel closed",
            "connection closed: initialize response",
            "unexpected end of stream",
            "broken pipe",
            "Session expired (HTTP 404)",
            "keep alive timeout after 30000ms",
            "connection reset by peer",
            "the transport has been closed",
        ];
        for err in cases {
            assert_eq!(
                reconnect_path_for_error(err),
                Some(ReconnectPath::Fresh),
                "{err}"
            );
        }
    }

    #[test]
    fn auth_error_still_routes_to_oauth_reconnect() {
        let err = "401 unauthorized: token expired";
        assert_eq!(reconnect_path_for_error(err), Some(ReconnectPath::OAuth));
        assert_eq!(failure_trigger_label(Some(ReconnectPath::OAuth)), "auth");
    }

    #[test]
    fn unmatched_error_does_not_retry() {
        let err = "tool not found: no_such_tool";
        assert!(reconnect_path_for_error(err).is_none());
        assert_eq!(failure_trigger_label(None), "unmatched");
        assert!(!RoutingService::is_transport_closed_error(err));
        assert!(!RoutingService::is_auth_error(err));
    }

    #[test]
    fn closed_without_transport_noun_is_unmatched() {
        assert!(reconnect_path_for_error("file closed").is_none());
        assert!(reconnect_path_for_error("request timeout after 60s").is_none());
    }

    #[test]
    fn auth_wins_when_error_also_looks_closed() {
        let err = "401 unauthorized: connection closed";
        assert_eq!(reconnect_path_for_error(err), Some(ReconnectPath::OAuth));
    }
}
