//! Admin HTTP write integration tests (Phase 6).

use mcpmux_gateway::admin::command_bridge::write::CreateSpaceBody;
use mcpmux_gateway::admin::command_bridge::{read as bridge_read, write as bridge_write};
use mcpmux_gateway::admin::AdminConfig;
use serde_json::json;

use super::admin_api::{AdminClient, AdminHarness};

#[tokio::test(flavor = "multi_thread")]
async fn csrf_rejects_post_without_token() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let client = AdminClient::new(&harness.base_url, None);

    let resp = client
        .post_response("/api/v1/spaces", &json!({ "name": "Blocked" }))
        .await;
    assert_eq!(resp.status(), 403);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn csrf_allows_post_with_token() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let resp = client
        .post_response("/api/v1/spaces", &json!({ "name": "CSRF OK Space" }))
        .await;
    assert_eq!(resp.status(), 200);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn create_space_round_trip_via_http() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let create_body = json!({ "name": "Round Trip Space" });
    let create_resp = client.post_response("/api/v1/spaces", &create_body).await;
    assert_eq!(create_resp.status(), 200);
    let created: serde_json::Value = create_resp.json().await.expect("create json");
    let space_id = created["id"].as_str().expect("space id").to_string();

    let list_resp = client.get_response("/api/v1/spaces").await;
    assert_eq!(list_resp.status(), 200);
    let list: serde_json::Value = list_resp.json().await.expect("list json");
    let names: Vec<_> = list
        .as_array()
        .expect("spaces array")
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.iter().any(|n| *n == "Round Trip Space"));

    let bridge_list = bridge_read::list_spaces(&harness.bridge)
        .await
        .expect("bridge list");
    assert_eq!(list, bridge_list);

    let _ = space_id;
    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn write_endpoints_match_bridge_for_spaces_and_settings() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let create_bridge = bridge_write::create_space(
        &harness.bridge,
        CreateSpaceBody {
            name: "Bridge Dual Space".to_string(),
            icon: None,
        },
    )
    .await
    .expect("bridge create space");

    let create_http = client
        .post_response("/api/v1/spaces", &json!({ "name": "HTTP Dual Space" }))
        .await;
    assert_eq!(create_http.status(), 200);

    let space_id = create_bridge["id"].as_str().expect("id").to_string();
    let update_bridge = bridge_write::update_space(
        &harness.bridge,
        space_id.clone(),
        mcpmux_gateway::admin::command_bridge::space::UpdateSpaceInput {
            name: Some("Updated Bridge".to_string()),
            icon: None,
            description: Some("via bridge".to_string()),
        },
    )
    .await
    .expect("bridge update");

    let update_http = client
        .put_response(
            &format!("/api/v1/spaces/{space_id}"),
            &json!({ "name": "Updated Bridge", "description": "via bridge" }),
        )
        .await;
    assert_eq!(update_http.status(), 200);
    let update_body: serde_json::Value = update_http.json().await.expect("update json");
    assert_eq!(update_body["name"], update_bridge["name"]);

    let settings_bridge = bridge_write::set_meta_tools_enabled(&harness.bridge, false)
        .await
        .expect("bridge settings");
    assert_eq!(settings_bridge["ok"], true);

    let settings_http = client
        .put_response(
            "/api/v1/settings/meta-tools-enabled",
            &json!({ "enabled": false }),
        )
        .await;
    assert_eq!(settings_http.status(), 200);

    let meta_enabled = bridge_read::get_meta_tools_enabled(&harness.bridge)
        .await
        .expect("read meta");
    assert_eq!(meta_enabled, json!(false));

    let delete_http = client
        .delete_response(&format!("/api/v1/spaces/{space_id}"), None)
        .await;
    assert_eq!(delete_http.status(), 200);

    let http_created: serde_json::Value = create_http.json().await.expect("http create json");
    let http_space_id = http_created["id"].as_str().expect("http id").to_string();
    bridge_write::delete_space(&harness.bridge, http_space_id)
        .await
        .expect("bridge delete http space");

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn create_feature_set_via_http() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let default_space_id = bridge_read::list_spaces(&harness.bridge)
        .await
        .expect("list spaces")
        .as_array()
        .and_then(|spaces| spaces.first())
        .and_then(|space| space.get("id"))
        .and_then(|id| id.as_str())
        .expect("default space")
        .to_string();

    let resp = client
        .post_response(
            "/api/v1/feature-sets",
            &json!({
                "name": "Admin Test Set",
                "space_id": default_space_id,
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("feature set json");
    assert_eq!(body["name"], "Admin Test Set");

    harness.shutdown();
}
