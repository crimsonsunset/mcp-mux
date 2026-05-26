//! Admin SSE channel contract tests — one test per UI event channel.

use std::time::Duration;

use mcpmux_core::{ConnectionStatus, DiscoveredCapabilities, DomainEvent};
use mcpmux_gateway::admin::ui_events::map_domain_event_to_ui;
use reqwest::Client;
use tokio::time::timeout;
use uuid::Uuid;

use crate::admin_api::AdminHarness;

/// Read one SSE frame from a streaming response body.
async fn read_sse_frame(body: &mut reqwest::Response) -> Option<(String, serde_json::Value)> {
    let mut buffer = String::new();
    let deadline = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            chunk = body.chunk() => {
                let chunk = chunk.ok().flatten()?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                if buffer.contains("\n\n") {
                    break;
                }
            }
            _ = &mut deadline => return None,
        }
    }

    let mut event_name = String::new();
    let mut data = String::new();
    for line in buffer.lines() {
        if let Some(name) = line.strip_prefix("event:") {
            event_name = name.trim().to_string();
        } else if let Some(payload) = line.strip_prefix("data:") {
            data = payload.trim().to_string();
        }
    }
    if event_name.is_empty() || data.is_empty() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(&data).ok()?;
    Some((event_name, json))
}

/// Connect to `/api/v1/events`, emit `event`, assert SSE channel + payload match Tauri shape.
async fn assert_sse_matches_domain_event(event: DomainEvent) {
    let harness = AdminHarness::start(mcpmux_gateway::admin::AdminConfig::default(), false).await;
    let (expected_channel, expected_payload) = map_domain_event_to_ui(&event);

    let client = Client::new();
    let mut sse = client
        .get(format!("{}/api/v1/events", harness.base_url))
        .send()
        .await
        .expect("SSE connect");

    harness.services.event_bus.sender().emit(event);

    let (channel, payload) = read_sse_frame(&mut sse)
        .await
        .expect("SSE frame within timeout");
    assert_eq!(channel, expected_channel);
    assert_eq!(payload, expected_payload);

    harness.shutdown();
}

/// Connect to SSE and assert a direct UI bus publish matches Tauri emit shape.
async fn assert_sse_matches_direct_emit(channel: &str, payload: serde_json::Value) {
    let harness = AdminHarness::start(mcpmux_gateway::admin::AdminConfig::default(), false).await;

    let client = Client::new();
    let mut sse = client
        .get(format!("{}/api/v1/events", harness.base_url))
        .send()
        .await
        .expect("SSE connect");

    harness.event_hub.publish_test_event(channel, payload.clone());

    let (got_channel, got_payload) = read_sse_frame(&mut sse)
        .await
        .expect("SSE frame within timeout");
    assert_eq!(got_channel, channel);
    assert_eq!(got_payload, payload);

    harness.shutdown();
}

macro_rules! sse_domain_test {
    ($name:ident, $event:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn $name() {
            assert_sse_matches_domain_event($event).await;
        }
    };
}

sse_domain_test!(
    sse_space_changed_matches_tauri_shape,
    DomainEvent::SpaceCreated {
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        name: "Work".to_string(),
        icon: Some("briefcase".to_string()),
    }
);

sse_domain_test!(
    sse_server_changed_matches_tauri_shape,
    DomainEvent::ServerInstalled {
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        server_id: "github".to_string(),
        server_name: "GitHub".to_string(),
    }
);

sse_domain_test!(
    sse_server_status_changed_matches_tauri_shape,
    DomainEvent::ServerStatusChanged {
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        server_id: "github".to_string(),
        status: ConnectionStatus::Connected,
        flow_id: 1,
        has_connected_before: true,
        message: None,
        features: None,
    }
);

sse_domain_test!(
    sse_server_auth_progress_matches_tauri_shape,
    DomainEvent::ServerAuthProgress {
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        server_id: "github".to_string(),
        remaining_seconds: 42,
        flow_id: 7,
    }
);

sse_domain_test!(
    sse_server_features_refreshed_matches_tauri_shape,
    DomainEvent::ServerFeaturesRefreshed {
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        server_id: "github".to_string(),
        features: DiscoveredCapabilities::default(),
        added: vec!["a".to_string()],
        removed: vec![],
    }
);

sse_domain_test!(
    sse_feature_set_changed_matches_tauri_shape,
    DomainEvent::FeatureSetCreated {
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        feature_set_id: "fs-1".to_string(),
        name: "Custom".to_string(),
        feature_set_type: Some("custom".to_string()),
    }
);

sse_domain_test!(
    sse_client_changed_matches_tauri_shape,
    DomainEvent::ClientRegistered {
        client_id: "cursor".to_string(),
        client_name: "Cursor".to_string(),
        registration_type: Some("dcr".to_string()),
    }
);

sse_domain_test!(
    sse_client_grant_changed_matches_tauri_shape,
    DomainEvent::ClientGrantChanged {
        client_id: "cursor".to_string(),
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
    }
);

sse_domain_test!(
    sse_gateway_changed_matches_tauri_shape,
    DomainEvent::GatewayStarted {
        url: "http://127.0.0.1:45818".to_string(),
        port: 45818,
    }
);

sse_domain_test!(
    sse_mcp_notification_matches_tauri_shape,
    DomainEvent::ToolsChanged {
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        server_id: "github".to_string(),
    }
);

sse_domain_test!(
    sse_session_roots_changed_matches_tauri_shape,
    DomainEvent::SessionRootsChanged
);

sse_domain_test!(
    sse_workspace_binding_changed_matches_tauri_shape,
    DomainEvent::WorkspaceBindingChanged {
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        workspace_root: "/proj".to_string(),
    }
);

sse_domain_test!(
    sse_workspace_needs_binding_matches_tauri_shape,
    DomainEvent::WorkspaceNeedsBinding {
        client_id: "vscode".to_string(),
        session_id: "sess-1".to_string(),
        space_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        workspace_root: "/proj/unbound".to_string(),
    }
);

#[tokio::test(flavor = "multi_thread")]
async fn sse_meta_tool_invoked_matches_tauri_shape() {
    let harness = AdminHarness::start(mcpmux_gateway::admin::AdminConfig::default(), false).await;
    let event = DomainEvent::MetaToolInvoked {
        client_id: "cursor".to_string(),
        session_id: Some("sess-1".to_string()),
        tool_name: "mcpmux_search_tools".to_string(),
        decision: "read".to_string(),
        resolved_feature_set_id: Some("fs-1".to_string()),
        summary: "ok".to_string(),
    };
    let (expected_channel, mut expected_payload) = map_domain_event_to_ui(&event);

    let client = Client::new();
    let mut sse = client
        .get(format!("{}/api/v1/events", harness.base_url))
        .send()
        .await
        .expect("SSE connect");

    harness.services.event_bus.sender().emit(event);

    let (channel, payload) = timeout(Duration::from_secs(3), read_sse_frame(&mut sse))
        .await
        .expect("timeout")
        .expect("SSE frame");

    assert_eq!(channel, expected_channel);
    expected_payload
        .as_object_mut()
        .expect("object payload")
        .remove("timestamp");
    let mut got = payload;
    got.as_object_mut().expect("object payload").remove("timestamp");
    assert_eq!(got, expected_payload);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn sse_oauth_client_changed_matches_tauri_shape() {
    assert_sse_matches_direct_emit(
        "oauth-client-changed",
        serde_json::json!({
            "action": "grants_updated",
            "client_id": "cursor",
        }),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sse_session_overrides_changed_matches_tauri_shape() {
    assert_sse_matches_direct_emit(
        "session-overrides-changed",
        serde_json::json!({ "session_id": "sess-1" }),
    )
    .await;
}
