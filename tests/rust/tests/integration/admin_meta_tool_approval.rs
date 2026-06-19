//! Admin HTTP path for meta-tool bind approvals (web admin SPA).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::FutureExt;
use mcpmux_gateway::admin::write_runtime::GatewayWriteRuntime;
use mcpmux_gateway::services::{
    ApprovalBroker, ApprovalDecision, ApprovalPayload, ApprovalPublisher, META_TOOL_APPROVAL_EVENT,
};
use serde_json::{json, Value};

use super::admin_api::{AdminClient, AdminHarness};

struct BrokerWriteRuntime {
    approval_broker: Arc<ApprovalBroker>,
}

#[async_trait]
impl GatewayWriteRuntime for BrokerWriteRuntime {
    async fn start_gateway(
        &self,
        _port: Option<u16>,
        _allow_dynamic_fallback: Option<bool>,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn stop_gateway(&self) -> anyhow::Result<Value> {
        Ok(json!({ "ok": true }))
    }

    async fn restart_gateway(
        &self,
        _port: Option<u16>,
        _allow_dynamic_fallback: Option<bool>,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn disconnect_server(
        &self,
        _server_id: String,
        _space_id: String,
        _logout: Option<bool>,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn connect_all_enabled_servers(&self) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn refresh_oauth_tokens_on_startup(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "servers_checked": 0,
            "tokens_refreshed": 0,
            "refresh_failed": 0,
        }))
    }

    async fn set_gateway_port(&self, _port: u16) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn enable_server_v2(
        &self,
        _space_id: String,
        _server_id: String,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn disable_server_v2(
        &self,
        _space_id: String,
        _server_id: String,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn start_auth_v2(&self, _space_id: String, _server_id: String) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn cancel_auth_v2(&self, _space_id: String, _server_id: String) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn retry_connection(
        &self,
        _space_id: String,
        _server_id: String,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn update_server_package(
        &self,
        _space_id: String,
        _server_id: String,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn logout_server(&self, _space_id: String, _server_id: String) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn respond_to_meta_tool_approval(
        &self,
        request_id: String,
        client_id: String,
        tool_name: String,
        decision: String,
    ) -> anyhow::Result<Value> {
        let decision = match decision.as_str() {
            "allow_once" => ApprovalDecision::AllowOnce,
            "always_for_this_session_and_client" => ApprovalDecision::AlwaysForThisSessionAndClient,
            "deny" => ApprovalDecision::Deny,
            other => return Err(anyhow::anyhow!("unknown decision: {other}")),
        };
        let approved = self
            .approval_broker
            .respond(&request_id, &client_id, &tool_name, decision);
        Ok(json!({ "approved": approved }))
    }

    async fn revoke_meta_tool_grant(
        &self,
        _client_id: String,
        _tool_name: String,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn update_oauth_client(
        &self,
        _client_id: String,
        _client_alias: Option<String>,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn delete_oauth_client(&self, _client_id: String) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn grant_oauth_client_feature_set(
        &self,
        _client_id: String,
        _space_id: String,
        _feature_set_id: String,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn revoke_oauth_client_feature_set(
        &self,
        _client_id: String,
        _space_id: String,
        _feature_set_id: String,
    ) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("Gateway not running"))
    }

    async fn gateway_state(
        &self,
    ) -> Option<Arc<tokio::sync::RwLock<mcpmux_gateway::GatewayState>>> {
        None
    }
}

fn sse_only_publisher(ui_bus: Arc<mcpmux_gateway::admin::AdminUiEventBus>) -> ApprovalPublisher {
    Arc::new(move |req| {
        let bus = ui_bus.clone();
        async move {
            if let Ok(payload) = serde_json::to_value(&req) {
                bus.publish(META_TOOL_APPROVAL_EVENT, payload);
            }
            true
        }
        .boxed()
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_http_approve_resolves_pending_bind_approval() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_secs(5)));
    let (services, bridge) = super::admin_api::in_memory_services().await;
    let bridge = Arc::new(mcpmux_gateway::admin::AdminBridgeCtx {
        gateway_writes: Arc::new(BrokerWriteRuntime {
            approval_broker: broker.clone(),
        }),
        ..(*bridge).clone()
    });

    let harness = AdminHarness::start_with_bridge(
        mcpmux_gateway::admin::AdminConfig::default(),
        true,
        services,
        bridge,
    )
    .await;

    let ui_bus = harness.event_hub.ui_event_bus();
    broker.set_publisher(sse_only_publisher(ui_bus)).await;

    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let broker_wait = broker.clone();
    let approval_task = tokio::spawn(async move {
        broker_wait
            .request_approval(
                "cursor-client",
                "mcpmux_bind_current_workspace",
                ApprovalPayload {
                    tool_name: "mcpmux_bind_current_workspace".into(),
                    summary: "Bind test".into(),
                    diff: None,
                    raw_args: json!({}),
                    affects_other_clients: false,
                },
            )
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;
    let pending = broker.list_pending_ids();
    assert_eq!(pending.len(), 1, "expected one pending approval");
    let request_id = pending[0].clone();

    let resp = client
        .post_response(
            "/api/v1/meta-tools/approval",
            &json!({
                "request_id": request_id,
                "client_id": "cursor-client",
                "tool_name": "mcpmux_bind_current_workspace",
                "decision": "allow_once",
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body.get("approved").and_then(|v| v.as_bool()), Some(true));

    let decision = tokio::time::timeout(Duration::from_secs(2), approval_task)
        .await
        .expect("approval timeout")
        .expect("approval task")
        .expect("approval ok");
    assert_eq!(decision, ApprovalDecision::AllowOnce);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_http_deny_resolves_pending_bind_approval() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_secs(5)));
    let (services, bridge) = super::admin_api::in_memory_services().await;
    let bridge = Arc::new(mcpmux_gateway::admin::AdminBridgeCtx {
        gateway_writes: Arc::new(BrokerWriteRuntime {
            approval_broker: broker.clone(),
        }),
        ..(*bridge).clone()
    });

    let harness = AdminHarness::start_with_bridge(
        mcpmux_gateway::admin::AdminConfig::default(),
        true,
        services,
        bridge,
    )
    .await;

    let ui_bus = harness.event_hub.ui_event_bus();
    broker.set_publisher(sse_only_publisher(ui_bus)).await;

    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let broker_wait = broker.clone();
    let approval_task = tokio::spawn(async move {
        broker_wait
            .request_approval(
                "cursor-client",
                "mcpmux_bind_current_workspace",
                ApprovalPayload {
                    tool_name: "mcpmux_bind_current_workspace".into(),
                    summary: "Bind test".into(),
                    diff: None,
                    raw_args: json!({}),
                    affects_other_clients: false,
                },
            )
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;
    let request_id = broker.list_pending_ids()[0].clone();

    let resp = client
        .post_response(
            "/api/v1/meta-tools/approval",
            &json!({
                "request_id": request_id,
                "client_id": "cursor-client",
                "tool_name": "mcpmux_bind_current_workspace",
                "decision": "deny",
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);

    let err = tokio::time::timeout(Duration::from_secs(2), approval_task)
        .await
        .expect("approval timeout")
        .expect("approval task")
        .expect_err("approval denied");
    assert!(matches!(
        err,
        mcpmux_gateway::services::meta_tools::MetaToolError::ApprovalDenied
    ));

    harness.shutdown();
}
