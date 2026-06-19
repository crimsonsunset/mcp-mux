//! Integration tests for server update policy probe parsing, guards, and resolution.

use std::collections::HashMap;

use mcpmux_core::{InstalledServer, TransportConfig as RegistryConfig, TransportMetadata, UpdatePolicy};
use mcpmux_gateway::pool::transport::resolution::update_policy_parsing::{
    build_transport_config, parse_npx_cache_info_text, parse_npx_cache_ls_line,
    TransportResolutionOptions,
};
use mcpmux_gateway::services::server_update_policy_parsing::{
    parse_pypi_json_version, parse_uv_outdated_line, parse_uv_tool_list_line,
};
use mcpmux_gateway::services::{is_pinned, probe_update_available};
use mcpmux_gateway::ResolvedTransport;

/// Shared guard fixture vector — keep in sync with TS `shouldShowPackageUpdate` cases.
const GUARD_FIXTURES: &[(&str, Option<&str>, Option<&str>, Option<&str>, bool)] = &[
    ("floating tag blocks badge", None, Some("2.0.0"), Some("latest"), false),
    ("unknown current blocks badge", None, Some("2.0.0"), None, false),
    ("semver delta shows badge", Some("1.0.0"), Some("2.0.0"), None, true),
    ("same version hides badge", Some("2.0.0"), Some("2.0.0"), None, false),
    ("older latest hides badge", Some("2.0.0"), Some("1.0.0"), None, false),
];

#[test]
fn guard_fixture_vector_matches_probe_update_available() {
    for (label, current, latest, npm_tag, expected) in GUARD_FIXTURES {
        assert_eq!(
            probe_update_available(*current, *latest, *npm_tag),
            *expected,
            "fixture: {label}"
        );
    }
}

#[test]
fn pinned_policy_is_excluded_from_probe_badging() {
    assert!(is_pinned(UpdatePolicy::Pinned));
    assert!(!is_pinned(UpdatePolicy::Notify));
    assert!(!is_pinned(UpdatePolicy::Auto));
}

#[test]
fn parse_npx_cache_ls_line_reads_text_output() {
    let (key, specs) = parse_npx_cache_ls_line("abc123def: inngest-cloud-mcp, inngest-cloud-mcp@1.2.3")
        .expect("parse line");
    assert_eq!(key, "abc123def");
    assert_eq!(
        specs,
        vec!["inngest-cloud-mcp".to_string(), "inngest-cloud-mcp@1.2.3".to_string()]
    );
}

#[test]
fn parse_npx_cache_ls_line_ignores_empty_and_metadata_lines() {
    assert!(parse_npx_cache_ls_line("").is_none());
    assert!(parse_npx_cache_ls_line("(empty)").is_none());
}

#[test]
fn parse_npx_cache_info_text_reads_resolved_semver() {
    let text = "\
abc123:
  - inngest-cloud-mcp (inngest-cloud-mcp@1.4.2)
";
    assert_eq!(
        parse_npx_cache_info_text(text, "inngest-cloud-mcp"),
        Some("1.4.2".to_string())
    );
}

#[test]
fn parse_npx_cache_info_text_ignores_floating_tags() {
    let text = "\
abc123:
  - demo-pkg (demo-pkg@latest)
";
    assert_eq!(parse_npx_cache_info_text(text, "demo-pkg"), None);
}

#[test]
fn parse_uv_tool_list_line_reads_name_and_version() {
    let (name, version) = parse_uv_tool_list_line("ruff v0.8.6").expect("parse line");
    assert_eq!(name, "ruff");
    assert_eq!(version, "0.8.6");
}

#[test]
fn parse_uv_outdated_line_reads_arrow_format() {
    let (name, latest) =
        parse_uv_outdated_line("mcp-server v1.0.0 -> v1.2.0").expect("parse line");
    assert_eq!(name, "mcp-server");
    assert_eq!(latest, "1.2.0");
}

#[test]
fn parse_pypi_json_version_reads_info_version() {
    let body = serde_json::json!({
        "info": { "version": "0.12.4" }
    });
    assert_eq!(parse_pypi_json_version(&body), Some("0.12.4".to_string()));
}

#[test]
fn parse_pypi_json_version_returns_none_on_missing_fields() {
    assert_eq!(parse_pypi_json_version(&serde_json::json!({})), None);
}

#[test]
fn explicit_update_injects_probed_semver_for_notify_policy() {
    let transport = RegistryConfig::Stdio {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "inngest-cloud-mcp".to_string()],
        env: HashMap::new(),
        metadata: TransportMetadata { inputs: vec![] },
    };

    let mut installed = InstalledServer::new("space", "inngest").with_update_policy(UpdatePolicy::Notify);
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
            assert_eq!(args[1], "inngest-cloud-mcp@3.2.1");
        }
        _ => panic!("Expected Stdio transport"),
    }
}

#[test]
fn explicit_update_falls_back_to_latest_without_probed_semver() {
    let transport = RegistryConfig::Stdio {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "inngest-cloud-mcp".to_string()],
        env: HashMap::new(),
        metadata: TransportMetadata { inputs: vec![] },
    };

    let installed =
        InstalledServer::new("space", "inngest").with_update_policy(UpdatePolicy::Notify);

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
fn explicit_update_respects_pinned_policy() {
    let transport = RegistryConfig::Stdio {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "firebase-tools".to_string()],
        env: HashMap::new(),
        metadata: TransportMetadata { inputs: vec![] },
    };

    let mut installed = InstalledServer::new("space", "firebase")
        .with_update_policy(UpdatePolicy::Pinned)
        .with_pinned_version(Some("13.0.0".to_string()));
    installed.latest_available_version = Some("14.0.0".to_string());

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
