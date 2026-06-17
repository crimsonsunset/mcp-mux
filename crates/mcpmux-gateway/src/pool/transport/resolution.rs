//! Transport configuration resolution
//!
//! Handles building the actual runtime transport configuration from
//! the static registry definition and user-specific installation settings.

use super::ResolvedTransport;
use mcpmux_core::{InstalledServer, TransportConfig as RegistryConfig, UpdatePolicy};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

const MCP_STATE_DIR_ENV: &str = "MCP_STATE_DIR";

/// Options that affect one-shot transport resolution behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransportResolutionOptions {
    /// When true, apply latest-package resolution for notify servers (explicit user update).
    pub apply_package_update: bool,
}

/// Build a merged input_values map that includes defaults for any inputs
/// not explicitly provided by the user.
fn merge_input_defaults(
    registry_transport: &RegistryConfig,
    user_values: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut merged = user_values.clone();
    let metadata = registry_transport.metadata();
    for input in &metadata.inputs {
        if !merged.contains_key(&input.id) {
            if let Some(ref default_val) = input.default {
                tracing::debug!(
                    "[TransportResolution] Using default for input '{}': '{}'",
                    input.id,
                    default_val
                );
                merged.insert(input.id.clone(), default_val.clone());
            }
        }
    }
    merged
}

/// Build transport config from registry transport and installed server
pub fn build_transport_config(
    registry_transport: &RegistryConfig,
    installed: &InstalledServer,
    base_state_dir: Option<&Path>,
    options: TransportResolutionOptions,
) -> ResolvedTransport {
    tracing::debug!(
        "[TransportResolution] Building config for {}/{} with {} input values",
        installed.space_id,
        installed.server_id,
        installed.input_values.len()
    );

    // Merge user-provided values with defaults from input definitions
    let effective_values = merge_input_defaults(registry_transport, &installed.input_values);

    match registry_transport {
        RegistryConfig::Stdio {
            command, args, env, ..
        } => {
            let resolved_command = resolve_placeholders(command, &effective_values);
            let mut resolved_args: Vec<String> = args
                .iter()
                .map(|arg| resolve_placeholders(arg, &effective_values))
                .collect();

            // Append user's extra args
            resolved_args.extend(installed.args_append.clone());

            apply_update_policy(&resolved_command, &mut resolved_args, installed, options);

            // Build env from registry + input values + env_overrides
            let mut resolved_env = HashMap::new();

            // 1. Start with registry env
            for (k, v) in env {
                let resolved_value = resolve_placeholders(v, &effective_values);
                tracing::debug!(
                    "[TransportResolution] Registry env: {}={} → {}",
                    k,
                    v,
                    resolved_value
                );
                resolved_env.insert(k.clone(), resolved_value);
            }

            // 2. Add input values (user-provided + defaults) directly as env vars
            tracing::debug!(
                "[TransportResolution] Adding {} input values as direct env vars",
                effective_values.len()
            );
            resolved_env.extend(effective_values.clone());

            // 3. Apply user's env overrides
            resolved_env.extend(installed.env_overrides.clone());

            // 4. Inject MCP_STATE_DIR if not already set
            apply_state_dir_env(&mut resolved_env, base_state_dir, installed);

            tracing::debug!(
                "[TransportResolution] Final env has {} variables",
                resolved_env.len()
            );

            ResolvedTransport::Stdio {
                command: resolved_command,
                args: resolved_args,
                env: resolved_env,
            }
        }
        RegistryConfig::Http { url, headers, .. } => {
            let resolved_url = resolve_placeholders(url, &effective_values);

            // Resolve headers from registry
            let mut resolved_headers: HashMap<String, String> = headers
                .iter()
                .map(|(k, v)| (k.clone(), resolve_placeholders(v, &effective_values)))
                .collect();

            // Add user's extra headers
            resolved_headers.extend(installed.extra_headers.clone());

            ResolvedTransport::Http {
                url: resolved_url,
                headers: resolved_headers,
            }
        }
    }
}

/// Apply per-server update policy for npx/uvx stdio transports.
fn apply_update_policy(
    command: &str,
    args: &mut [String],
    installed: &InstalledServer,
    options: TransportResolutionOptions,
) {
    if options.apply_package_update && installed.update_policy != UpdatePolicy::Pinned {
        apply_explicit_package_update(command, args, installed);
        return;
    }

    match installed.update_policy {
        UpdatePolicy::Auto => apply_auto_update_policy(command, args),
        UpdatePolicy::Pinned => apply_pinned_update_policy(command, args, installed),
        UpdatePolicy::Notify => {}
    }
}

/// Apply Auto-mode package resolution for npx/uvx stdio transports.
fn apply_auto_update_policy(command: &str, args: &mut [String]) {
    match command {
        "npx" => inject_npx_latest(args),
        "uvx" | "uv" => run_uv_tool_upgrade(command, args),
        _ => {}
    }
}

/// One-shot user update: pin to probed semver when known, else re-resolve `@latest`.
fn apply_explicit_package_update(command: &str, args: &mut [String], installed: &InstalledServer) {
    match command {
        "npx" => {
            if let Some(version) = installed
                .latest_available_version
                .as_deref()
                .filter(|value| is_semver_like(value))
            {
                inject_npx_pinned(args, version);
            } else {
                inject_npx_latest(args);
            }
        }
        "uvx" | "uv" => run_uv_tool_upgrade(command, args),
        _ => {}
    }
}

/// Returns true for npm dist-tags that do not pin an exact installed semver.
pub fn npm_version_tag_is_floating(tag: &str) -> bool {
    matches!(
        tag.trim().trim_start_matches('@').to_ascii_lowercase().as_str(),
        "latest" | "*" | "next" | "beta" | "canary" | "stable" | "release"
    )
}

/// Returns true when `version` looks like a concrete semver (not a dist-tag).
pub fn is_semver_like(version: &str) -> bool {
    !parse_version_parts(version).is_empty()
}

/// Split a semver-ish string into numeric comparison parts.
fn parse_version_parts(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('=')
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Enforce an exact semver pin for Pinned-policy servers.
fn apply_pinned_update_policy(command: &str, args: &mut [String], installed: &InstalledServer) {
    let Some(pinned) = installed
        .pinned_version
        .as_deref()
        .filter(|v| !v.is_empty())
    else {
        return;
    };

    warn_if_pinned_version_differs(installed, pinned);

    match command {
        "npx" => inject_npx_pinned(args, pinned),
        "uvx" | "uv" => inject_uvx_pinned(command, args, pinned),
        _ => {}
    }
}

/// Log when a pin differs from the cached latest probe (informational only).
fn warn_if_pinned_version_differs(installed: &InstalledServer, pinned: &str) {
    let Some(latest) = installed
        .latest_available_version
        .as_deref()
        .filter(|v| !v.is_empty())
    else {
        return;
    };

    if pinned != latest {
        tracing::warn!(
            "[TransportResolution] Pinned version {} differs from latest available {} for {}/{}",
            pinned,
            latest,
            installed.space_id,
            installed.server_id
        );
    }
}

/// Inject `@latest` into the npx package argument so npm re-resolves the registry tag.
fn inject_npx_latest(args: &mut [String]) {
    let Some(index) = find_npx_package_arg_index(args) else {
        return;
    };
    let injected = inject_npm_version_tag(&args[index], "latest");
    tracing::debug!(
        "[TransportResolution] Auto update policy: npx package {} → {}",
        args[index],
        injected
    );
    args[index] = injected;
}

/// Inject `@<semver>` into the npx package argument for Pinned policy.
fn inject_npx_pinned(args: &mut [String], version: &str) {
    let Some(index) = find_npx_package_arg_index(args) else {
        return;
    };
    let injected = inject_npm_version_tag(&args[index], version);
    tracing::debug!(
        "[TransportResolution] Pinned update policy: npx package {} → {}",
        args[index],
        injected
    );
    args[index] = injected;
}

/// Inject `==<semver>` into the uvx / `uv run` package argument for Pinned policy.
fn inject_uvx_pinned(command: &str, args: &mut [String], version: &str) {
    let Some(index) = find_uv_package_arg_index(command, args) else {
        return;
    };
    let injected = inject_uv_version_tag(&args[index], version);
    tracing::debug!(
        "[TransportResolution] Pinned update policy: uv package {} → {}",
        args[index],
        injected
    );
    args[index] = injected;
}

/// Run `uv tool upgrade <pkg>` before spawn; failures are logged and do not block connection.
fn run_uv_tool_upgrade(command: &str, args: &[String]) {
    let Some(package) = extract_uv_package_name(command, args) else {
        return;
    };

    tracing::debug!(
        "[TransportResolution] Auto update policy: running uv tool upgrade for {}",
        package
    );

    match Command::new("uv")
        .args(["tool", "upgrade", &package])
        .output()
    {
        Ok(output) if output.status.success() => {
            tracing::debug!(
                "[TransportResolution] uv tool upgrade succeeded for {}",
                package
            );
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(
                "[TransportResolution] uv tool upgrade failed for {} (status {:?}): {}",
                package,
                output.status.code(),
                stderr.trim()
            );
        }
        Err(err) => {
            tracing::warn!(
                "[TransportResolution] uv tool upgrade could not run for {}: {}",
                package,
                err
            );
        }
    }
}

/// Index of the npm package argument for npx (`-y` flag skips to the next positional).
fn find_npx_package_arg_index(args: &[String]) -> Option<usize> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if matches!(arg, "-y" | "--yes") {
            let next = index + 1;
            if next < args.len() && !args[next].starts_with('-') {
                return Some(next);
            }
        }
        index += 1;
    }

    args.iter()
        .position(|arg| !arg.starts_with('-') && arg != "--")
}

/// Index of the package argument for uvx or `uv run` invocations.
fn find_uv_package_arg_index(command: &str, args: &[String]) -> Option<usize> {
    match command {
        "uvx" => args.iter().position(|arg| !arg.starts_with('-')),
        "uv" if args.first().map(String::as_str) == Some("run") => {
            let mut index = 1;
            while index < args.len() {
                let arg = args[index].as_str();
                if arg.starts_with('-') {
                    if matches!(arg, "-m" | "--module") {
                        index += 2;
                        continue;
                    }
                    index += 1;
                    continue;
                }
                return Some(index);
            }
            None
        }
        _ => None,
    }
}

/// Package name for uvx or `uv run` invocations.
fn extract_uv_package_name(command: &str, args: &[String]) -> Option<String> {
    let index = find_uv_package_arg_index(command, args)?;
    Some(strip_package_version(&args[index]))
}

/// Strip an existing `@version` or `==version` suffix from a package specifier.
fn strip_package_version(package: &str) -> String {
    if let Some(stripped) = package.strip_prefix("==") {
        return stripped.to_string();
    }
    split_npm_package_arg(package).0
}

/// Split an npm-style package arg into name and optional version tag.
fn split_npm_package_arg(package: &str) -> (String, Option<String>) {
    if let Some(scoped) = package.strip_prefix('@') {
        if let Some(at_idx) = scoped.find('@') {
            let split_at = 1 + at_idx;
            return (
                package[..split_at].to_string(),
                Some(package[split_at + 1..].to_string()),
            );
        }
        return (package.to_string(), None);
    }

    if let Some(at_idx) = package.rfind('@') {
        return (
            package[..at_idx].to_string(),
            Some(package[at_idx + 1..].to_string()),
        );
    }

    (package.to_string(), None)
}

/// Append or replace an npm version tag on a package argument (`pkg`, `@scope/pkg`, or `pkg@ver`).
fn inject_npm_version_tag(package: &str, tag: &str) -> String {
    let tag = tag.trim_start_matches('@');
    if tag.is_empty() {
        return package.to_string();
    }

    let (name, _) = split_npm_package_arg(package);
    format!("{name}@{tag}")
}

/// Append or replace a PEP 440 exact version on a uv package argument (`pkg` or `pkg==ver`).
fn inject_uv_version_tag(package: &str, version: &str) -> String {
    let version = version.trim_start_matches('=');
    if version.is_empty() {
        return package.to_string();
    }

    let name = strip_package_version(package);
    format!("{name}=={version}")
}

fn apply_state_dir_env(
    resolved_env: &mut HashMap<String, String>,
    base_state_dir: Option<&Path>,
    installed: &InstalledServer,
) {
    if resolved_env.contains_key(MCP_STATE_DIR_ENV) {
        return;
    }

    let Some(base_state_dir) = base_state_dir else {
        return;
    };

    let state_dir = base_state_dir
        .join("stdio")
        .join(&installed.space_id)
        .join(&installed.server_id);

    resolved_env.insert(
        MCP_STATE_DIR_ENV.to_string(),
        state_dir.to_string_lossy().to_string(),
    );
}

/// Resolve placeholders like ${input:INPUT_NAME} in a string
fn resolve_placeholders(template: &str, input_values: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in input_values {
        result = result.replace(&format!("${{input:{}}}", key), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcpmux_core::{InputDefinition, TransportMetadata};

    fn make_installed(input_values: HashMap<String, String>) -> InstalledServer {
        InstalledServer::new("test-space", "test-server").with_inputs(input_values)
    }

    fn make_input(id: &str, default: Option<&str>) -> InputDefinition {
        InputDefinition {
            id: id.to_string(),
            label: id.to_string(),
            r#type: "text".to_string(),
            required: default.is_none(),
            secret: false,
            description: None,
            default: default.map(|s| s.to_string()),
            placeholder: None,
            obtain_url: None,
            obtain_instructions: None,
        }
    }

    #[test]
    fn test_default_used_when_user_provides_no_value() {
        let transport = RegistryConfig::Stdio {
            command: "node".to_string(),
            args: vec!["server.js".to_string()],
            env: HashMap::from([("LOG_LEVEL".to_string(), "${input:LOG_LEVEL}".to_string())]),
            metadata: TransportMetadata {
                inputs: vec![make_input("LOG_LEVEL", Some("info"))],
            },
        };

        let installed = make_installed(HashMap::new()); // No user values

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Stdio { env, .. } => {
                // Default should be used for placeholder resolution
                assert_eq!(env.get("LOG_LEVEL"), Some(&"info".to_string()));
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_user_value_overrides_default() {
        let transport = RegistryConfig::Stdio {
            command: "node".to_string(),
            args: vec![],
            env: HashMap::from([("LOG_LEVEL".to_string(), "${input:LOG_LEVEL}".to_string())]),
            metadata: TransportMetadata {
                inputs: vec![make_input("LOG_LEVEL", Some("info"))],
            },
        };

        let installed = make_installed(HashMap::from([(
            "LOG_LEVEL".to_string(),
            "debug".to_string(),
        )]));

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Stdio { env, .. } => {
                // User value should win over default
                assert_eq!(env.get("LOG_LEVEL"), Some(&"debug".to_string()));
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_default_resolves_in_args() {
        let transport = RegistryConfig::Stdio {
            command: "node".to_string(),
            args: vec!["--port".to_string(), "${input:PORT}".to_string()],
            env: HashMap::new(),
            metadata: TransportMetadata {
                inputs: vec![make_input("PORT", Some("8080"))],
            },
        };

        let installed = make_installed(HashMap::new());

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Stdio { args, .. } => {
                assert_eq!(args[0], "--port");
                assert_eq!(args[1], "8080");
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_default_resolves_in_command() {
        let transport = RegistryConfig::Stdio {
            command: "${input:BINARY_PATH}".to_string(),
            args: vec![],
            env: HashMap::new(),
            metadata: TransportMetadata {
                inputs: vec![make_input("BINARY_PATH", Some("/usr/local/bin/mcp"))],
            },
        };

        let installed = make_installed(HashMap::new());

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Stdio { command, .. } => {
                assert_eq!(command, "/usr/local/bin/mcp");
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_default_resolves_in_http_url() {
        let transport = RegistryConfig::Http {
            url: "https://api.example.com/${input:API_VERSION}/mcp".to_string(),
            headers: HashMap::new(),
            metadata: TransportMetadata {
                inputs: vec![make_input("API_VERSION", Some("v2"))],
            },
        };

        let installed = make_installed(HashMap::new());

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Http { url, .. } => {
                assert_eq!(url, "https://api.example.com/v2/mcp");
            }
            _ => panic!("Expected Http transport"),
        }
    }

    #[test]
    fn test_default_resolves_in_http_headers() {
        let transport = RegistryConfig::Http {
            url: "https://api.example.com/mcp".to_string(),
            headers: HashMap::from([("X-Api-Key".to_string(), "${input:API_KEY}".to_string())]),
            metadata: TransportMetadata {
                inputs: vec![make_input("API_KEY", Some("default-key"))],
            },
        };

        let installed = make_installed(HashMap::new());

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Http { headers, .. } => {
                assert_eq!(headers.get("X-Api-Key"), Some(&"default-key".to_string()));
            }
            _ => panic!("Expected Http transport"),
        }
    }

    #[test]
    fn test_multiple_defaults_some_overridden() {
        let transport = RegistryConfig::Stdio {
            command: "node".to_string(),
            args: vec![],
            env: HashMap::from([
                ("LOG_LEVEL".to_string(), "${input:LOG_LEVEL}".to_string()),
                ("PORT".to_string(), "${input:PORT}".to_string()),
                ("API_KEY".to_string(), "${input:API_KEY}".to_string()),
            ]),
            metadata: TransportMetadata {
                inputs: vec![
                    make_input("LOG_LEVEL", Some("info")),
                    make_input("PORT", Some("3000")),
                    make_input("API_KEY", None), // No default
                ],
            },
        };

        // User provides PORT and API_KEY, but not LOG_LEVEL
        let installed = make_installed(HashMap::from([
            ("PORT".to_string(), "9090".to_string()),
            ("API_KEY".to_string(), "secret123".to_string()),
        ]));

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Stdio { env, .. } => {
                // LOG_LEVEL: default used
                assert_eq!(env.get("LOG_LEVEL"), Some(&"info".to_string()));
                // PORT: user value wins
                assert_eq!(env.get("PORT"), Some(&"9090".to_string()));
                // API_KEY: user value used
                assert_eq!(env.get("API_KEY"), Some(&"secret123".to_string()));
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_no_default_leaves_placeholder_unresolved() {
        let transport = RegistryConfig::Stdio {
            command: "node".to_string(),
            args: vec![],
            env: HashMap::from([("API_KEY".to_string(), "${input:API_KEY}".to_string())]),
            metadata: TransportMetadata {
                inputs: vec![make_input("API_KEY", None)],
            },
        };

        let installed = make_installed(HashMap::new());

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Stdio { env, .. } => {
                // Without user value or default, placeholder stays unresolved in the env template
                assert_eq!(env.get("API_KEY"), Some(&"${input:API_KEY}".to_string()));
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_merge_input_defaults_only_fills_missing() {
        let transport = RegistryConfig::Stdio {
            command: "node".to_string(),
            args: vec![],
            env: HashMap::new(),
            metadata: TransportMetadata {
                inputs: vec![
                    make_input("A", Some("default_a")),
                    make_input("B", Some("default_b")),
                ],
            },
        };

        let user_values = HashMap::from([("A".to_string(), "user_a".to_string())]);

        let merged = merge_input_defaults(&transport, &user_values);

        assert_eq!(merged.get("A"), Some(&"user_a".to_string()));
        assert_eq!(merged.get("B"), Some(&"default_b".to_string()));
    }

    #[test]
    fn test_pinned_policy_injects_npx_version() {
        let transport = RegistryConfig::Stdio {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "firebase-tools".to_string()],
            env: HashMap::new(),
            metadata: TransportMetadata { inputs: vec![] },
        };

        let installed = InstalledServer::new("space", "firebase")
            .with_update_policy(UpdatePolicy::Pinned)
            .with_pinned_version(Some("13.0.0"));

        let resolved = build_transport_config(&transport, &installed, None, TransportResolutionOptions::default());

        match resolved {
            ResolvedTransport::Stdio { args, .. } => {
                assert_eq!(args[1], "firebase-tools@13.0.0");
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_explicit_update_applies_latest_for_notify_policy() {
        let transport = RegistryConfig::Stdio {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "inngest-cloud-mcp".to_string()],
            env: HashMap::new(),
            metadata: TransportMetadata { inputs: vec![] },
        };

        let installed = InstalledServer::new("space", "inngest")
            .with_update_policy(UpdatePolicy::Notify);

        let resolved = build_transport_config(
            &transport,
            &installed,
            None,
            TransportResolutionOptions {
                apply_package_update: true,
            },
        );

        match resolved {
            ResolvedTransport::Stdio { args, .. } => {
                assert_eq!(args[1], "inngest-cloud-mcp@latest");
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_explicit_update_injects_probed_semver_for_notify_policy() {
        let transport = RegistryConfig::Stdio {
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@upstash/context7-mcp@latest".to_string(),
            ],
            env: HashMap::new(),
            metadata: TransportMetadata { inputs: vec![] },
        };

        let mut installed = InstalledServer::new("space", "context7")
            .with_update_policy(UpdatePolicy::Notify);
        installed.latest_available_version = Some("3.2.1".to_string());

        let resolved = build_transport_config(
            &transport,
            &installed,
            None,
            TransportResolutionOptions {
                apply_package_update: true,
            },
        );

        match resolved {
            ResolvedTransport::Stdio { args, .. } => {
                assert_eq!(args[1], "@upstash/context7-mcp@3.2.1");
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_explicit_update_respects_pinned_policy() {
        let transport = RegistryConfig::Stdio {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "firebase-tools".to_string()],
            env: HashMap::new(),
            metadata: TransportMetadata { inputs: vec![] },
        };

        let installed = InstalledServer::new("space", "firebase")
            .with_update_policy(UpdatePolicy::Pinned)
            .with_pinned_version(Some("13.0.0"));

        let resolved = build_transport_config(
            &transport,
            &installed,
            None,
            TransportResolutionOptions {
                apply_package_update: true,
            },
        );

        match resolved {
            ResolvedTransport::Stdio { args, .. } => {
                assert_eq!(args[1], "firebase-tools@13.0.0");
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn npm_version_tag_is_floating_recognizes_latest() {
        assert!(npm_version_tag_is_floating("latest"));
        assert!(!is_semver_like("latest"));
        assert!(is_semver_like("3.2.1"));
    }
}
