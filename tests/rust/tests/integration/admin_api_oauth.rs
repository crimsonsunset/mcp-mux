//! Admin OAuth consent HTTP integration tests (Phase 7).

use std::sync::Arc;
use std::time::Duration;

use axum::{routing::post, Router};
use mcpmux_gateway::admin::AdminConfig;
use mcpmux_gateway::oauth::PkceChallenge;
use mcpmux_gateway::oauth_token;
use mcpmux_gateway::{GatewayState, PendingAuthorization};
use mcpmux_storage::{
    Database, InboundClient, InboundClientRepository, RegistrationType, JWT_SECRET_SIZE,
};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

use super::admin_api::{start_with_gateway_state, AdminClient};

async fn oauth_test_gateway_state() -> Arc<RwLock<GatewayState>> {
    let (tx, _) = broadcast::channel(16);
    let db = Arc::new(Mutex::new(Database::open_in_memory().expect("db")));
    let mut state = GatewayState::new(tx);
    state.set_database(db.clone());
    state.set_jwt_secret(Zeroizing::new([7_u8; JWT_SECRET_SIZE]));

    let now = chrono::Utc::now().to_rfc3339();
    let client = InboundClient {
        client_id: "test-client".to_string(),
        registration_type: RegistrationType::Dcr,
        client_name: "Test Client".to_string(),
        client_alias: None,
        redirect_uris: vec!["http://127.0.0.1:0/callback".to_string()],
        grant_types: vec!["authorization_code".to_string()],
        response_types: vec!["code".to_string()],
        token_endpoint_auth_method: "none".to_string(),
        scope: None,
        approved: false,
        logo_uri: None,
        client_uri: None,
        software_id: None,
        software_version: None,
        metadata_url: None,
        metadata_cached_at: None,
        metadata_cache_ttl: None,
        last_seen: None,
        created_at: now.clone(),
        updated_at: now,
        reports_roots: false,
        roots_capability_known: false,
    };
    InboundClientRepository::new(db)
        .save_client(&client)
        .await
        .expect("save client");

    Arc::new(RwLock::new(state))
}

async fn seed_pending_consent(
    gateway_state: &Arc<RwLock<GatewayState>>,
    request_id: &str,
    consent_token: &str,
    code_challenge: &str,
) {
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 + 300)
        .unwrap_or(i64::MAX);

    let pending = PendingAuthorization {
        client_id: "test-client".to_string(),
        client_name: Some("Test Client".to_string()),
        redirect_uri: "http://127.0.0.1:0/callback".to_string(),
        scope: Some("mcp".to_string()),
        state: Some("test-state".to_string()),
        code_challenge: Some(code_challenge.to_string()),
        code_challenge_method: Some("S256".to_string()),
        expires_at,
        consent_token: Some(consent_token.to_string()),
    };

    gateway_state
        .write()
        .await
        .store_pending_authorization(request_id, pending);
}

struct TokenServer {
    base_url: String,
    cancel: CancellationToken,
}

impl TokenServer {
    async fn start(gateway_state: Arc<RwLock<GatewayState>>) -> Self {
        let router = Router::new()
            .route("/oauth/token", post(oauth_token))
            .with_state(gateway_state);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind token server");
        let port = listener.local_addr().expect("addr").port();
        let base_url = format!("http://127.0.0.1:{port}");
        let cancel = CancellationToken::new();
        let shutdown = cancel.clone();

        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    shutdown.cancelled().await;
                })
                .await
                .expect("serve token server");
        });

        tokio::time::sleep(Duration::from_millis(30)).await;
        Self { base_url, cancel }
    }

    fn shutdown(self) {
        self.cancel.cancel();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_consent_csrf_required_on_post() {
    let gateway_state = oauth_test_gateway_state().await;
    seed_pending_consent(
        &gateway_state,
        "req-csrf",
        "token-csrf",
        &PkceChallenge::generate().challenge,
    )
    .await;

    let harness = start_with_gateway_state(AdminConfig::default(), gateway_state).await;
    let client = AdminClient::new(&harness.base_url, None);

    let resp = client
        .post_response(
            "/api/v1/oauth/consent/approve",
            &json!({
                "request_id": "req-csrf",
                "consent_token": "token-csrf",
            }),
        )
        .await;
    assert_eq!(resp.status(), 403);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_consent_invalid_token_returns_400() {
    let gateway_state = oauth_test_gateway_state().await;
    seed_pending_consent(
        &gateway_state,
        "req-bad-token",
        "good-token",
        &PkceChallenge::generate().challenge,
    )
    .await;

    let harness = start_with_gateway_state(AdminConfig::default(), gateway_state).await;
    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let resp = client
        .post_response(
            "/api/v1/oauth/consent/approve",
            &json!({
                "request_id": "req-bad-token",
                "consent_token": "wrong-token",
            }),
        )
        .await;
    assert_eq!(resp.status(), 400);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_consent_reject_returns_access_denied_redirect() {
    let gateway_state = oauth_test_gateway_state().await;
    seed_pending_consent(
        &gateway_state,
        "req-reject",
        "token-reject",
        &PkceChallenge::generate().challenge,
    )
    .await;

    let harness = start_with_gateway_state(AdminConfig::default(), gateway_state).await;
    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let resp = client
        .post_response(
            "/api/v1/oauth/consent/reject",
            &json!({
                "request_id": "req-reject",
                "consent_token": "token-reject",
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["success"], true);
    assert!(body["redirect_url"]
        .as_str()
        .unwrap_or("")
        .contains("error=access_denied"));

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_consent_approve_via_http_issues_token() {
    let gateway_state = oauth_test_gateway_state().await;
    let pkce = PkceChallenge::generate();
    seed_pending_consent(
        &gateway_state,
        "req-approve",
        "token-approve",
        &pkce.challenge,
    )
    .await;

    let token_server = TokenServer::start(gateway_state.clone()).await;
    let harness = start_with_gateway_state(AdminConfig::default(), gateway_state).await;
    let mut client = AdminClient::new(&harness.base_url, None);
    client.fetch_csrf_token().await;

    let pending = client
        .get_response("/api/v1/oauth/consent/pending?requestId=req-approve")
        .await;
    assert_eq!(pending.status(), 200);
    let pending_body: serde_json::Value = pending.json().await.expect("pending json");
    assert_eq!(pending_body["consentToken"], "token-approve");

    let approve = client
        .post_response(
            "/api/v1/oauth/consent/approve",
            &json!({
                "request_id": "req-approve",
                "consent_token": "token-approve",
            }),
        )
        .await;
    assert_eq!(approve.status(), 200);
    let approve_body: serde_json::Value = approve.json().await.expect("approve json");
    assert_eq!(approve_body["success"], true);
    let redirect_url = approve_body["redirect_url"].as_str().expect("redirect url");
    let code = redirect_url
        .split("code=")
        .nth(1)
        .and_then(|s| s.split('&').next())
        .expect("auth code in redirect");

    let token_resp = reqwest::Client::new()
        .post(format!("{}/oauth/token", token_server.base_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&code={code}&client_id=test-client&redirect_uri=http://127.0.0.1:0/callback&code_verifier={}",
            pkce.verifier
        ))
        .send()
        .await
        .expect("token request");
    assert_eq!(token_resp.status(), 200);
    let token_body: serde_json::Value = token_resp.json().await.expect("token json");
    assert!(token_body.get("access_token").is_some());

    harness.shutdown();
    token_server.shutdown();
}
