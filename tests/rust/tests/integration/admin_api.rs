//! Admin HTTP integration tests.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use mcpmux_core::{
    ApplicationServices, ApplicationServicesBuilder, EventBus, GatewayPortService, LogConfig,
    ServerDiscoveryService, ServerLogManager, SpaceRepository, SpaceService,
};
use mcpmux_gateway::admin::{
    command_bridge::read as bridge_read,
    build_admin_router, format_bridge_error_message, new_csrf_token_store, test_valid_jwt,
    test_validator, AdminConfig, AdminState, AdminBridgeCtx, CF_ACCESS_JWT_HEADER,
    StubGatewayRuntime, StubGatewayWriteRuntime, AdminEventHub, AdminUiEventBus,
};
use mcpmux_storage::{
    Database, SqliteAppSettingsRepository, SqliteCredentialRepository, SqliteFeatureSetRepository,
    SqliteInboundMcpClientRepository, SqliteInstalledServerRepository, SqliteServerFeatureRepository,
    SqliteSpaceRepository, SqliteWorkspaceAppearanceRepository, SqliteWorkspaceBindingRepository,
};
use mcpmux_gateway::admin::CSRF_HEADER;
use reqwest::{Client, RequestBuilder, Response};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// In-memory `ApplicationServices` for admin integration tests.
pub async fn in_memory_services() -> (Arc<ApplicationServices>, Arc<AdminBridgeCtx>) {
    let temp_dir = tempfile::TempDir::new().expect("tempdir");
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let event_bus = Arc::new(EventBus::new());
    let space_repo: Arc<dyn SpaceRepository> = Arc::new(SqliteSpaceRepository::new(db.clone()));
    let installed_repo = Arc::new(SqliteInstalledServerRepository::new(
        db.clone(),
        Arc::new(mcpmux_storage::FieldEncryptor::new(&[7_u8; 32]).expect("encryptor")),
    ));
    let feature_set_repo = Arc::new(SqliteFeatureSetRepository::new(db.clone()));
    let client_repo = Arc::new(SqliteInboundMcpClientRepository::new(db.clone()));
    let credential_repo = Arc::new(SqliteCredentialRepository::new(
        db.clone(),
        Arc::new(mcpmux_storage::FieldEncryptor::new(&[9_u8; 32]).expect("encryptor")),
    ));
    let settings_repo = Arc::new(SqliteAppSettingsRepository::new(db.clone()));
    let gateway_port_service = Arc::new(GatewayPortService::new(settings_repo.clone()));
    let workspace_binding_repository = Arc::new(SqliteWorkspaceBindingRepository::new(db.clone()));
    let workspace_appearance_repository = Arc::new(SqliteWorkspaceAppearanceRepository::new(db.clone()));
    let server_feature_repository = Arc::new(SqliteServerFeatureRepository::new(db.clone()));

    let services = Arc::new(
        ApplicationServicesBuilder::new()
            .with_event_bus(event_bus.clone())
            .with_space_repo(space_repo.clone())
            .with_installed_server_repo(installed_repo)
            .with_feature_set_repo(feature_set_repo.clone())
            .with_server_feature_repo(server_feature_repository.clone())
            .with_client_repo(client_repo)
            .with_credential_repo(credential_repo)
            .build()
            .expect("build ApplicationServices"),
    );
    let space_service = SpaceService::with_feature_set_repository(space_repo, feature_set_repo.clone());
    let data_dir = temp_dir.path().join("data");
    let spaces_dir = data_dir.join("spaces");
    std::fs::create_dir_all(&spaces_dir).expect("create spaces");
    let bridge = Arc::new(AdminBridgeCtx {
        services: services.clone(),
        spaces_dir,
        data_dir: data_dir.clone(),
        gateway_port_service: gateway_port_service.clone(),
        server_discovery: Arc::new(ServerDiscoveryService::new(data_dir.clone(), data_dir.join("spaces"))),
        settings_repository: settings_repo,
        workspace_binding_repository,
        workspace_appearance_repository,
        server_feature_repository,
        server_log_manager: Arc::new(ServerLogManager::new(LogConfig {
            base_dir: data_dir.join("logs"),
            max_file_size: 1024 * 1024,
            max_files: 5,
            compress: false,
        })),
        space_service: Arc::new(space_service),
        gateway_runtime: Arc::new(StubGatewayRuntime),
        gateway_writes: Arc::new(StubGatewayWriteRuntime {
            gateway_port_service: Some(gateway_port_service),
            gateway_state: None,
        }),
        feature_set_repository: feature_set_repo,
        auto_launch_enabled: Some(false),
        app_version: "0.0.0-test".to_string(),
        bundle_version: None,
    });

    (services, bridge)
}

/// Start admin harness with a live gateway state wired for OAuth consent tests.
pub async fn start_with_gateway_state(
    config: AdminConfig,
    gateway_state: Arc<tokio::sync::RwLock<mcpmux_gateway::GatewayState>>,
) -> AdminHarness {
    let (services, bridge) = in_memory_services().await;
    let gateway_port_service = bridge.gateway_port_service.clone();
    let bridge = Arc::new(AdminBridgeCtx {
        gateway_writes: Arc::new(StubGatewayWriteRuntime {
            gateway_port_service: Some(gateway_port_service),
            gateway_state: Some(gateway_state),
        }),
        ..(*bridge).clone()
    });

    let gateway_flag = Arc::new(AtomicBool::new(true));
    let cf_validator = if config.trust_cf_access {
        config
            .cf_validator_override
            .clone()
            .or_else(|| Some(test_validator()))
    } else {
        None
    };

    let ui_event_bus = Arc::new(AdminUiEventBus::new());
    let event_hub = Arc::new(AdminEventHub::new(ui_event_bus));

    let state = AdminState {
        services: services.clone(),
        config: config.clone(),
        gateway_running: gateway_flag,
        frontend_dist: std::path::PathBuf::from("/nonexistent"),
        cf_validator,
        bridge: bridge.clone(),
        event_hub: event_hub.clone(),
        csrf_token: new_csrf_token_store(),
    };
    let router = build_admin_router(state);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind admin harness");
    let port = listener.local_addr().expect("local addr").port();
    let base_url = format!("http://127.0.0.1:{port}");
    let cancel = CancellationToken::new();
    let shutdown = cancel.clone();

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                shutdown.cancelled().await;
            })
            .await
            .expect("serve admin harness");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    AdminHarness {
        base_url,
        services,
        bridge,
        event_hub,
        cancel,
    }
}

/// Running admin server on an ephemeral loopback port.
pub struct AdminHarness {
    pub base_url: String,
    pub services: Arc<ApplicationServices>,
    pub bridge: Arc<AdminBridgeCtx>,
    pub event_hub: Arc<AdminEventHub>,
    cancel: CancellationToken,
}

impl AdminHarness {
    /// Mount the admin router and bind to `127.0.0.1:0`.
    pub async fn start(config: AdminConfig, gateway_running: bool) -> Self {
        let (services, bridge) = in_memory_services().await;
        let gateway_flag = Arc::new(AtomicBool::new(gateway_running));
        let cf_validator = if config.trust_cf_access {
            config
                .cf_validator_override
                .clone()
                .or_else(|| Some(test_validator()))
        } else {
            None
        };

        let ui_event_bus = Arc::new(AdminUiEventBus::new());
        let event_hub = Arc::new(AdminEventHub::new(ui_event_bus));

        let state = AdminState {
            services: services.clone(),
            config: config.clone(),
            gateway_running: gateway_flag,
            frontend_dist: std::path::PathBuf::from("/nonexistent"),
            cf_validator,
            bridge: bridge.clone(),
            event_hub: event_hub.clone(),
            csrf_token: new_csrf_token_store(),
        };
        let router = build_admin_router(state);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind admin harness");
        let port = listener.local_addr().expect("local addr").port();
        let base_url = format!("http://127.0.0.1:{port}");
        let cancel = CancellationToken::new();
        let shutdown = cancel.clone();

        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    shutdown.cancelled().await;
                })
                .await
                .expect("serve admin harness");
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        Self {
            base_url,
            services,
            bridge,
            event_hub,
            cancel,
        }
    }

    pub fn shutdown(self) {
        self.cancel.cancel();
    }
}

async fn assert_get_matches_bridge(
    _harness: &AdminHarness,
    client: &AdminClient,
    path: &str,
    bridge_value: serde_json::Value,
) {
    let resp = client.get_response(path).await;
    assert_eq!(resp.status(), 200, "path={path}");
    let body: serde_json::Value = resp.json().await.expect("json response");
    assert_eq!(body, bridge_value, "path={path}");
}

/// HTTP client for admin API requests with optional CF Access JWT.
pub(crate) struct AdminClient {
    inner: Client,
    base_url: String,
    cf_jwt: Option<String>,
    csrf_token: Option<String>,
}

impl AdminClient {
    /// Create a client targeting `base_url` with an optional JWT stub.
    pub(crate) fn new(base_url: impl Into<String>, cf_jwt: Option<&str>) -> Self {
        Self {
            inner: Client::new(),
            base_url: base_url.into(),
            cf_jwt: cf_jwt.map(str::to_string),
            csrf_token: None,
        }
    }

    fn with_headers(&self, mut req: RequestBuilder) -> RequestBuilder {
        if let Some(jwt) = &self.cf_jwt {
            req = req.header(CF_ACCESS_JWT_HEADER, jwt);
        }
        if let Some(token) = &self.csrf_token {
            req = req.header(CSRF_HEADER, token);
        }
        req
    }

    /// Fetch and cache CSRF token from the admin server.
    pub(crate) async fn fetch_csrf_token(&mut self) {
        let resp = self
            .get("/api/v1/csrf-token")
            .send()
            .await
            .expect("csrf token request");
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.expect("csrf json");
        self.csrf_token = body["token"].as_str().map(str::to_string);
    }

    /// Begin a GET request, attaching `CF-Access-Jwt-Assertion` when configured.
    fn get(&self, path: &str) -> RequestBuilder {
        self.with_headers(self.inner.get(format!("{}{}", self.base_url, path)))
    }

    fn post_json(&self, path: &str, body: &serde_json::Value) -> RequestBuilder {
        self.with_headers(
            self.inner
                .post(format!("{}{}", self.base_url, path))
                .header("Content-Type", "application/json")
                .json(body),
        )
    }

    fn put_json(&self, path: &str, body: &serde_json::Value) -> RequestBuilder {
        self.with_headers(
            self.inner
                .put(format!("{}{}", self.base_url, path))
                .header("Content-Type", "application/json")
                .json(body),
        )
    }

    fn delete_json(&self, path: &str, body: Option<&serde_json::Value>) -> RequestBuilder {
        let mut req = self
            .inner
            .delete(format!("{}{}", self.base_url, path))
            .header("Content-Type", "application/json");
        if let Some(body) = body {
            req = req.json(body);
        }
        self.with_headers(req)
    }

    /// Send GET and return the response.
    pub(crate) async fn get_response(&self, path: &str) -> Response {
        self.get(path).send().await.expect("admin GET request")
    }

    pub(crate) async fn post_response(&self, path: &str, body: &serde_json::Value) -> Response {
        self.post_json(path, body)
            .send()
            .await
            .expect("admin POST request")
    }

    pub(crate) async fn put_response(&self, path: &str, body: &serde_json::Value) -> Response {
        self.put_json(path, body)
            .send()
            .await
            .expect("admin PUT request")
    }

    pub(crate) async fn delete_response(
        &self,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Response {
        self.delete_json(path, body)
            .send()
            .await
            .expect("admin DELETE request")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn health_returns_200_with_valid_jwt_stub() {
    let mut config = AdminConfig {
        trust_cf_access: true,
        ..AdminConfig::default()
    };
    config.cf_validator_override = Some(test_validator());

    let harness = AdminHarness::start(config, true).await;
    let client = AdminClient::new(&harness.base_url, Some(&test_valid_jwt()));

    let resp = client.get_response("/api/v1/health").await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.expect("health json");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["gateway_running"], true);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn health_returns_401_when_cf_access_enabled_and_no_jwt() {
    let mut config = AdminConfig {
        trust_cf_access: true,
        ..AdminConfig::default()
    };
    config.cf_validator_override = Some(test_validator());

    let harness = AdminHarness::start(config, false).await;
    let client = AdminClient::new(&harness.base_url, None);

    let resp = client.get_response("/api/v1/health").await;
    assert_eq!(resp.status(), 401);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn health_returns_200_when_cf_access_disabled() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let client = AdminClient::new(&harness.base_url, None);

    let resp = client.get_response("/api/v1/health").await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.expect("health json");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["gateway_running"], false);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn read_endpoints_match_bridge_for_core_p4_routes() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let client = AdminClient::new(&harness.base_url, None);

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/spaces",
        bridge_read::list_spaces(&harness.bridge).await.expect("bridge list_spaces"),
    )
    .await;

    let default_space_id = bridge_read::list_spaces(&harness.bridge)
        .await
        .expect("bridge list spaces")
        .as_array()
        .and_then(|spaces| spaces.first())
        .and_then(|space| space.get("id"))
        .and_then(|id| id.as_str())
        .map(str::to_string)
        .expect("default space id");
    assert_get_matches_bridge(
        &harness,
        &client,
        &format!("/api/v1/spaces/{default_space_id}"),
        bridge_read::get_space(&harness.bridge, default_space_id.clone())
            .await
            .expect("bridge get_space"),
    )
    .await;

    let temp_space_id = Uuid::new_v4().to_string();
    assert_get_matches_bridge(
        &harness,
        &client,
        &format!("/api/v1/spaces/{temp_space_id}/config"),
        bridge_read::read_space_config(&harness.bridge, temp_space_id.clone())
            .await
            .expect("bridge read config"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/gateway/status",
        bridge_read::get_gateway_status(&harness.bridge, None)
            .await
            .expect("bridge gateway status"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/gateway/probe-start",
        bridge_read::probe_gateway_start(&harness.bridge, None)
            .await
            .expect("bridge gateway probe"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/gateway/port-settings",
        bridge_read::get_gateway_port_settings(&harness.bridge)
            .await
            .expect("bridge gateway port settings"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/registry/discover",
        bridge_read::discover_servers(&harness.bridge)
            .await
            .expect("bridge discover servers"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/clients",
        bridge_read::list_clients(&harness.bridge)
            .await
            .expect("bridge list clients"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/feature-sets",
        bridge_read::list_feature_sets(&harness.bridge)
            .await
            .expect("bridge list feature sets"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/workspaces/bindings",
        bridge_read::list_workspace_bindings(&harness.bridge)
            .await
            .expect("bridge list workspace bindings"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/workspaces/reported-roots",
        bridge_read::list_reported_workspace_roots(&harness.bridge)
            .await
            .expect("bridge list roots"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/workspaces/validate-root?path=/tmp",
        bridge_read::validate_workspace_root("/tmp".to_string())
            .await
            .expect("bridge validate root"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/workspaces/appearances",
        bridge_read::list_workspace_appearances(&harness.bridge)
            .await
            .expect("bridge list appearances"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/session-overrides",
        bridge_read::list_session_overrides(&harness.bridge, None)
            .await
            .expect("bridge list session overrides"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/settings/startup",
        bridge_read::get_startup_settings(&harness.bridge)
            .await
            .expect("bridge startup settings"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/settings/meta-tools-enabled",
        bridge_read::get_meta_tools_enabled(&harness.bridge)
            .await
            .expect("bridge meta tools enabled"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/app/version",
        bridge_read::get_version(&harness.bridge)
            .await
            .expect("bridge version"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/app/bundle-version",
        bridge_read::get_bundle_version(&harness.bridge)
            .await
            .expect("bridge bundle version"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/app/logs-path",
        bridge_read::get_logs_path(&harness.bridge)
            .await
            .expect("bridge logs path"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/logs/retention-days",
        bridge_read::get_log_retention_days(&harness.bridge)
            .await
            .expect("bridge retention"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/oauth/clients",
        bridge_read::get_oauth_clients(&harness.bridge)
            .await
            .expect("bridge oauth clients"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/meta-tools/grants",
        bridge_read::list_meta_tool_grants(&harness.bridge)
            .await
            .expect("bridge grants"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/server-features?spaceId=00000000-0000-0000-0000-000000000001",
        bridge_read::list_server_features(
            &harness.bridge,
            "00000000-0000-0000-0000-000000000001".to_string(),
            None,
        )
        .await
        .expect("bridge server features"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/servers/clones/available?spaceId=00000000-0000-0000-0000-000000000001&sourceServerId=demo&suffix=work",
        bridge_read::is_clone_id_available(
            &harness.bridge,
            "00000000-0000-0000-0000-000000000001".to_string(),
            "demo".to_string(),
            "work".to_string(),
        )
        .await
        .expect("bridge clone availability"),
    )
    .await;

    assert_get_matches_bridge(
        &harness,
        &client,
        "/api/v1/servers/clones/dependents?spaceId=00000000-0000-0000-0000-000000000001&sourceServerId=demo",
        bridge_read::list_clone_dependents(
            &harness.bridge,
            "00000000-0000-0000-0000-000000000001".to_string(),
            "demo".to_string(),
        )
        .await
        .expect("bridge clone dependents"),
    )
    .await;

    harness.shutdown();
}

#[test]
fn format_helper_preserves_port_in_use_sentinel() {
    let msg = format_bridge_error_message(anyhow::anyhow!("PORT_IN_USE:45818:default"));
    assert_eq!(msg, "PORT_IN_USE:45818:default");
}
