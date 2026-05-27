//! Diagnostic helpers and `mcpmux_diagnose_server` meta tool.
//!
//! Logic ported from [`dashboard.helpers.ts`](../../../../apps/desktop/src/features/dashboard/dashboard.helpers.ts):
//! missing required inputs, health buckets, and a redacted transport config view.

use std::collections::HashMap;

use async_trait::async_trait;
use mcpmux_core::{FeatureType, InstalledServer, LogLevel, ServerDefinition, TransportConfig};
use rmcp::model::CallToolResult;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::registry::{MetaTool, MetaToolCall, MetaToolError};
use super::tools::{caller_space_id, text_result};
use crate::pool::ConnectionStatus;

/// Operator-facing health bucket for a single installed server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerHealth {
    Healthy,
    Error,
    AuthRequired,
    NeedsSetup,
    Disconnected,
}

impl ServerHealth {
    /// Whether this bucket counts as unhealthy for no-arg diagnose filtering.
    pub fn is_unhealthy(self) -> bool {
        !matches!(self, Self::Healthy)
    }
}

/// Redacted transport configuration (keys only for secrets; no input values).
#[derive(Debug, Clone, Serialize, Default)]
pub struct ConfigView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub env_keys: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub header_keys: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub input_keys: Vec<String>,
}

/// Returns IDs of required transport inputs that have no user value.
///
/// Mirrors `hasMissingRequiredInputs` in the dashboard: uses `cached_definition`
/// transport metadata and treats empty strings as missing. Invalid JSON yields
/// an empty list (same as the TS `catch` path).
pub fn parse_missing_required_inputs(installed: &InstalledServer) -> Vec<String> {
    let Some(definition) = installed.get_definition() else {
        return Vec::new();
    };

    let values = &installed.input_values;
    let mut missing = Vec::new();

    for input in &definition.transport.metadata().inputs {
        if !input.required {
            continue;
        }
        let has_value = values
            .get(&input.id)
            .is_some_and(|v| !v.is_empty());
        if !has_value {
            missing.push(input.id.clone());
        }
    }

    missing.sort();
    missing
}

/// Whether any required input is unset (see [`parse_missing_required_inputs`]).
pub fn has_missing_required_inputs(installed: &InstalledServer) -> bool {
    !parse_missing_required_inputs(installed).is_empty()
}

/// Map runtime connection status and setup state to a diagnose health bucket.
///
/// Priority matches the dashboard attention panel: missing inputs win over
/// runtime status; then error, then OAuth required, then disconnected.
pub fn classify_health(status: ConnectionStatus, has_missing_inputs: bool) -> ServerHealth {
    if has_missing_inputs {
        return ServerHealth::NeedsSetup;
    }

    match status {
        ConnectionStatus::Error => ServerHealth::Error,
        ConnectionStatus::AuthRequired => ServerHealth::AuthRequired,
        ConnectionStatus::Disconnected => ServerHealth::Disconnected,
        ConnectionStatus::Connected
        | ConnectionStatus::Connecting
        | ConnectionStatus::Refreshing
        | ConnectionStatus::Authenticating => ServerHealth::Healthy,
    }
}

/// Build a redacted config view from the installed server's cached definition.
///
/// Secret input values are never included; only transport shape and key names.
pub fn build_config_view(installed: &InstalledServer) -> ConfigView {
    let Some(definition) = installed.get_definition() else {
        return ConfigView::default();
    };

    build_config_view_from_definition(&definition)
}

fn build_config_view_from_definition(definition: &ServerDefinition) -> ConfigView {
    let metadata = definition.transport.metadata();
    let mut input_keys: Vec<String> = metadata.inputs.iter().map(|i| i.id.clone()).collect();
    input_keys.sort();
    input_keys.dedup();

    match &definition.transport {
        TransportConfig::Stdio { command, args, env, .. } => {
            let mut env_keys: Vec<String> = env.keys().cloned().collect();
            env_keys.sort();

            ConfigView {
                transport_type: Some("stdio".to_string()),
                command: Some(command.clone()),
                url: None,
                args: args.clone(),
                env_keys,
                header_keys: Vec::new(),
                input_keys,
            }
        }
        TransportConfig::Http { url, headers, .. } => {
            let mut header_keys: Vec<String> = headers.keys().cloned().collect();
            header_keys.sort();

            ConfigView {
                transport_type: Some("http".to_string()),
                command: None,
                url: Some(url.clone()),
                args: Vec::new(),
                env_keys: Vec::new(),
                header_keys,
                input_keys,
            }
        }
    }
}

/// Serialize a pool [`ConnectionStatus`] as the diagnose runtime status string.
fn connection_status_label(status: ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Disconnected => "disconnected",
        ConnectionStatus::Connecting => "connecting",
        ConnectionStatus::Connected => "connected",
        ConnectionStatus::Refreshing => "refreshing",
        ConnectionStatus::AuthRequired => "auth_required",
        ConnectionStatus::Authenticating => "authenticating",
        ConnectionStatus::Error => "error",
    }
}

/// Parsed arguments for [`DiagnoseServerTool`].
struct DiagnoseArgs {
    server_id: Option<String>,
    include_logs: bool,
    log_limit: usize,
    log_level_filter: Option<LogLevel>,
}

/// Parse and validate `mcpmux_diagnose_server` call arguments.
fn parse_diagnose_args(args: &Value) -> Result<DiagnoseArgs, MetaToolError> {
    let server_id = args
        .get("server_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let include_logs = args
        .get("include_logs")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let log_limit = args
        .get("log_limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50)
        .min(500) as usize;

    let log_level_filter = match args.get("log_level_filter") {
        None | Some(Value::Null) => None,
        Some(v) => {
            let Some(raw) = v.as_str() else {
                return Err(MetaToolError::InvalidArgument(
                    "`log_level_filter` must be a string".into(),
                ));
            };
            Some(LogLevel::parse(raw).ok_or_else(|| {
                MetaToolError::InvalidArgument(format!(
                    "invalid log_level_filter '{raw}'; expected trace, debug, info, warn, or error"
                ))
            })?)
        }
    };

    Ok(DiagnoseArgs {
        server_id,
        include_logs,
        log_limit,
        log_level_filter,
    })
}

/// Count installed tool features per server in a Space.
async fn tool_counts_for_space(
    call: &MetaToolCall<'_>,
    space_id: &Uuid,
) -> Result<HashMap<String, usize>, MetaToolError> {
    let features = call
        .ctx
        .server_feature_repo
        .list_for_space(&space_id.to_string())
        .await?;
    let mut counts = HashMap::new();
    for feature in features {
        if feature.feature_type != FeatureType::Tool {
            continue;
        }
        *counts.entry(feature.server_id.clone()).or_insert(0) += 1;
    }
    Ok(counts)
}

/// Build the runtime sub-object for one diagnosed server.
fn build_runtime_view(
    status: ConnectionStatus,
    flow_id: u64,
    has_connected_before: bool,
    message: Option<String>,
) -> Value {
    json!({
        "status": connection_status_label(status),
        "flow_id": flow_id,
        "has_connected_before": has_connected_before,
        "message": message,
    })
}

/// Read and serialize the log tail for one server when requested.
async fn build_logs_view(
    call: &MetaToolCall<'_>,
    space_id: &Uuid,
    server_id: &str,
    include_logs: bool,
    log_limit: usize,
    log_level_filter: Option<LogLevel>,
) -> Result<Option<Value>, MetaToolError> {
    if !include_logs {
        return Ok(None);
    }

    let entries = call
        .ctx
        .log_manager
        .read_logs(
            &space_id.to_string(),
            server_id,
            log_limit,
            log_level_filter,
        )
        .await
        .map_err(|e| MetaToolError::Internal(e.to_string()))?;

    Ok(Some(json!({
        "count": entries.len(),
        "level_filter": log_level_filter.map(|level| level.as_str()),
        "entries": entries,
    })))
}

/// Build one server entry for the diagnose response payload.
async fn build_server_diagnosis(
    call: &MetaToolCall<'_>,
    space_id: &Uuid,
    installed: &InstalledServer,
    runtime: (ConnectionStatus, u64, bool, Option<String>),
    tool_count: usize,
    args: &DiagnoseArgs,
) -> Result<Value, MetaToolError> {
    let missing = parse_missing_required_inputs(installed);
    let has_missing = !missing.is_empty();
    let (status, flow_id, has_connected_before, message) = runtime;
    let health = classify_health(status, has_missing);

    let mut entry = json!({
        "server_id": installed.server_id,
        "display_name": installed.display_name(),
        "health": health,
        "runtime": build_runtime_view(status, flow_id, has_connected_before, message),
        "config": build_config_view(installed),
        "missing_required_inputs": missing,
        "tool_count": tool_count,
    });

    if let Some(logs) = build_logs_view(
        call,
        space_id,
        &installed.server_id,
        args.include_logs,
        args.log_limit,
        args.log_level_filter,
    )
    .await?
    {
        entry["logs"] = logs;
    }

    Ok(entry)
}

/// Read-only combo diagnostic for MCP servers in the caller's resolved Space.
pub struct DiagnoseServerTool;

#[async_trait]
impl MetaTool for DiagnoseServerTool {
    fn name(&self) -> &'static str {
        "mcpmux_diagnose_server"
    }

    fn description(&self) -> &'static str {
        "Operator diagnostic: return runtime status, redacted transport config, \
         missing required inputs, and a recent log tail for MCP servers in the \
         caller's resolved Space. Omit server_id to list only unhealthy servers; \
         pass server_id to inspect one server regardless of health."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "server_id": {
                    "type": "string",
                    "description": "Optional. When omitted, only unhealthy servers are returned"
                },
                "include_logs": {
                    "type": "boolean",
                    "default": true,
                    "description": "Set false to omit the logs block"
                },
                "log_limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 500,
                    "default": 50,
                    "description": "Maximum number of log entries to return"
                },
                "log_level_filter": {
                    "type": "string",
                    "enum": ["trace", "debug", "info", "warn", "error"],
                    "description": "Minimum log level to include (inclusive)"
                }
            }
        })
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let space_id = caller_space_id(&call).await?;
        let args = parse_diagnose_args(&call.args)?;

        let installed = call
            .ctx
            .installed_server_repo
            .list_for_space(&space_id.to_string())
            .await
            .map_err(|e| MetaToolError::Internal(e.to_string()))?;

        if let Some(ref target) = args.server_id {
            if !installed.iter().any(|s| &s.server_id == target) {
                return Err(MetaToolError::InvalidArgument(format!(
                    "unknown server_id '{target}' in this Space"
                )));
            }
        }

        let statuses = call.ctx.server_manager.get_all_statuses(space_id).await;
        let tool_counts = tool_counts_for_space(&call, &space_id).await?;

        let mut servers: Vec<Value> = Vec::new();
        for server in &installed {
            if args
                .server_id
                .as_ref()
                .is_some_and(|target| &server.server_id != target)
            {
                continue;
            }

            let runtime = statuses.get(&server.server_id).cloned().unwrap_or((
                ConnectionStatus::Disconnected,
                0_u64,
                false,
                None::<String>,
            ));

            let missing = parse_missing_required_inputs(server);
            let health = classify_health(runtime.0, !missing.is_empty());

            if args.server_id.is_none() && !health.is_unhealthy() {
                continue;
            }

            let tool_count = tool_counts.get(&server.server_id).copied().unwrap_or(0);
            servers.push(
                build_server_diagnosis(&call, &space_id, server, runtime, tool_count, &args).await?,
            );
        }

        servers.sort_by(|a, b| {
            a.get("server_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("server_id").and_then(|v| v.as_str()).unwrap_or(""))
        });

        let total_unhealthy = servers
            .iter()
            .filter(|entry| {
                entry.get("health").and_then(|v| v.as_str()) != Some("healthy")
            })
            .count();

        Ok(text_result(json!({
            "space_id": space_id,
            "servers": servers,
            "total_unhealthy": total_unhealthy,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcpmux_core::{InputDefinition, TransportMetadata};

    fn stdio_definition(inputs: Vec<InputDefinition>) -> ServerDefinition {
        ServerDefinition {
            id: "test.server".to_string(),
            name: "Test".to_string(),
            description: None,
            alias: None,
            auth: None,
            icon: None,
            transport: TransportConfig::Stdio {
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "pkg".to_string()],
                env: [("GITHUB_TOKEN".to_string(), "${input:token}".to_string())]
                    .into_iter()
                    .collect(),
                metadata: TransportMetadata { inputs },
            },
            categories: vec![],
            publisher: None,
            source: Default::default(),
            badges: vec![],
            hosting_type: Default::default(),
            license: None,
            license_url: None,
            installation: None,
            capabilities: None,
            sponsored: None,
            media: None,
            changelog_url: None,
        }
    }

    fn installed_with_definition(
        definition: &ServerDefinition,
        input_values: &[(&str, &str)],
    ) -> InstalledServer {
        let mut server = InstalledServer::new("space", "test.server").with_definition(definition);
        for (key, value) in input_values {
            server = server.with_input(*key, *value);
        }
        server
    }

    fn required_input(id: &str) -> InputDefinition {
        InputDefinition {
            id: id.to_string(),
            label: id.to_string(),
            r#type: "text".to_string(),
            required: true,
            secret: true,
            description: None,
            default: None,
            placeholder: None,
            obtain_url: None,
            obtain_instructions: None,
        }
    }

    fn optional_input(id: &str) -> InputDefinition {
        InputDefinition {
            required: false,
            secret: false,
            ..required_input(id)
        }
    }

    #[test]
    fn parse_missing_returns_required_ids_without_values() {
        let def = stdio_definition(vec![
            required_input("github_token"),
            optional_input("optional_flag"),
            required_input("api_key"),
        ]);
        let installed = installed_with_definition(&def, &[]);

        assert_eq!(
            parse_missing_required_inputs(&installed),
            vec!["api_key".to_string(), "github_token".to_string()]
        );
    }

    #[test]
    fn parse_missing_treats_empty_string_as_missing() {
        let def = stdio_definition(vec![required_input("token")]);
        let installed = installed_with_definition(&def, &[("token", "")]);

        assert_eq!(
            parse_missing_required_inputs(&installed),
            vec!["token".to_string()]
        );
    }

    #[test]
    fn parse_missing_empty_when_all_required_filled() {
        let def = stdio_definition(vec![required_input("token")]);
        let installed = installed_with_definition(&def, &[("token", "secret")]);

        assert!(parse_missing_required_inputs(&installed).is_empty());
    }

    #[test]
    fn parse_missing_empty_without_cached_definition() {
        let installed = InstalledServer::new("space", "bare");

        assert!(parse_missing_required_inputs(&installed).is_empty());
    }

    #[test]
    fn parse_missing_empty_on_invalid_cached_json() {
        let mut installed = InstalledServer::new("space", "bad");
        installed.cached_definition = Some("{not json".to_string());

        assert!(parse_missing_required_inputs(&installed).is_empty());
    }

    #[test]
    fn classify_health_missing_inputs_beats_error_status() {
        assert_eq!(
            classify_health(ConnectionStatus::Error, true),
            ServerHealth::NeedsSetup
        );
    }

    #[test]
    fn classify_health_error_and_auth_and_disconnected() {
        assert_eq!(
            classify_health(ConnectionStatus::Error, false),
            ServerHealth::Error
        );
        assert_eq!(
            classify_health(ConnectionStatus::AuthRequired, false),
            ServerHealth::AuthRequired
        );
        assert_eq!(
            classify_health(ConnectionStatus::Disconnected, false),
            ServerHealth::Disconnected
        );
    }

    #[test]
    fn classify_health_connected_and_in_progress_are_healthy() {
        for status in [
            ConnectionStatus::Connected,
            ConnectionStatus::Connecting,
            ConnectionStatus::Refreshing,
            ConnectionStatus::Authenticating,
        ] {
            assert_eq!(
                classify_health(status, false),
                ServerHealth::Healthy,
                "expected healthy for {status:?}"
            );
        }
    }

    #[test]
    fn build_config_view_stdio_redacts_values() {
        let def = stdio_definition(vec![
            required_input("github_token"),
            optional_input("extra"),
        ]);
        let installed = installed_with_definition(&def, &[("github_token", "ghp_secret")]);

        let view = build_config_view(&installed);

        assert_eq!(view.transport_type.as_deref(), Some("stdio"));
        assert_eq!(view.command.as_deref(), Some("npx"));
        assert_eq!(view.args, vec!["-y", "pkg"]);
        assert_eq!(view.env_keys, vec!["GITHUB_TOKEN"]);
        assert_eq!(view.input_keys, vec!["extra", "github_token"]);
        let json = serde_json::to_string(&view).expect("serialize");
        assert!(!json.contains("ghp_secret"));
        assert!(!json.contains("${input:token}"));
    }

    #[test]
    fn build_config_view_http_includes_url_and_header_keys() {
        let definition = ServerDefinition {
            id: "remote".to_string(),
            name: "Remote".to_string(),
            description: None,
            alias: None,
            auth: None,
            icon: None,
            transport: TransportConfig::Http {
                url: "https://mcp.example.com".to_string(),
                headers: [("Authorization".to_string(), "Bearer x".to_string())]
                    .into_iter()
                    .collect(),
                metadata: TransportMetadata {
                    inputs: vec![required_input("api_key")],
                },
            },
            categories: vec![],
            publisher: None,
            source: Default::default(),
            badges: vec![],
            hosting_type: Default::default(),
            license: None,
            license_url: None,
            installation: None,
            capabilities: None,
            sponsored: None,
            media: None,
            changelog_url: None,
        };
        let installed = installed_with_definition(&definition, &[("api_key", "sk_live_secret")]);
        let view = build_config_view(&installed);

        assert_eq!(view.transport_type.as_deref(), Some("http"));
        assert_eq!(view.url.as_deref(), Some("https://mcp.example.com"));
        assert!(view.command.is_none());
        assert_eq!(view.header_keys, vec!["Authorization"]);
        assert_eq!(view.input_keys, vec!["api_key"]);
        let json = serde_json::to_string(&view).expect("serialize");
        assert!(!json.contains("Bearer x"));
        assert!(!json.contains("sk_live_secret"));
    }
}
