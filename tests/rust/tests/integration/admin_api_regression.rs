//! Admin HTTP regression tests (event hub, SPA fallback).

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use mcpmux_gateway::admin::{
    build_admin_router, new_csrf_token_store, AdminConfig, AdminEventHub, AdminState,
    AdminUiEventBus,
};

use super::admin_api::{in_memory_services, AdminHarness};

#[tokio::test(flavor = "multi_thread")]
async fn event_hub_fan_in_tasks_bounded_after_router_rebuild() {
    let (services, bridge) = in_memory_services().await;
    let event_hub = Arc::new(AdminEventHub::new(Arc::new(AdminUiEventBus::new())));
    let csrf_token = new_csrf_token_store();

    for _ in 0..25 {
        let state = AdminState {
            services: services.clone(),
            config: AdminConfig::default(),
            gateway_running: Arc::new(AtomicBool::new(false)),
            frontend_dist: std::path::PathBuf::from("/nonexistent"),
            cf_validator: None,
            bridge: bridge.clone(),
            event_hub: event_hub.clone(),
            csrf_token: csrf_token.clone(),
        };
        let _router = build_admin_router(state);
    }

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(event_hub.active_fan_in_task_count(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn root_returns_build_hint_when_spa_missing() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let client = super::admin_api::AdminClient::new(&harness.base_url, None);

    let resp = client.get_response("/").await;
    assert_eq!(resp.status(), 503);

    let body = resp.text().await.expect("html body");
    assert!(body.contains("build:web:admin"));
    assert!(body.contains("Web admin UI not built"));

    harness.shutdown();
}
