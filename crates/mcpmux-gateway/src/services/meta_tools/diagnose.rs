//! Diagnostic helpers for `mcpmux_diagnose_server`.
//!
//! Logic ported from [`dashboard.helpers.ts`](../../../../apps/desktop/src/features/dashboard/dashboard.helpers.ts):
//! missing required inputs, health buckets, and a redacted transport config view.

use mcpmux_core::{InstalledServer, ServerDefinition, TransportConfig};
use serde::Serialize;

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
