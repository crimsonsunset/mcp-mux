//! Admin HTTP integration with a live MCP gateway (not stub runtime).

use std::sync::Arc;
use std::time::Duration;

use mcpmux_core::{
    ApplicationServicesBuilder, EventBus, GatewayPortService, LogConfig, ServerDiscoveryService,
    ServerLogManager, SpaceRepository, SpaceService,
};
use mcpmux_gateway::admin::command_bridge::read as bridge_read;
use mcpmux_gateway::admin::{
    AdminBridgeCtx, AdminConfig, BackendBuildStamp, LiveGatewayRuntime, LiveGatewayWriteRuntime,
};
use mcpmux_gateway::services::ServerVersionProbeService;
use mcpmux_gateway::{DependenciesBuilder, GatewayConfig, GatewayServer, GatewayServerHandle};
use mcpmux_storage::{
    Database, SqliteAppSettingsRepository, SqliteCredentialRepository, SqliteFeatureSetRepository,
    SqliteInboundMcpClientRepository, SqliteInstalledServerRepository,
    SqliteOutboundOAuthRepository, SqliteServerFeatureRepository, SqliteSpaceRepository,
    SqliteWorkspaceAppearanceRepository, SqliteWorkspaceBindingRepository,
};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

use super::admin_api::{AdminClient, AdminHarness};

struct LiveGatewayFixture {
    harness: AdminHarness,
    gateway_handle: GatewayServerHandle,
    listen_url: String,
}

impl LiveGatewayFixture {
    async fn start() -> Self {
        let temp_dir = tempfile::TempDir::new().expect("tempdir");
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let event_bus = Arc::new(EventBus::new());
        let space_repo: Arc<dyn SpaceRepository> = Arc::new(SqliteSpaceRepository::new(db.clone()));
        let installed_repo = Arc::new(SqliteInstalledServerRepository::new(
            db.clone(),
            Arc::new(mcpmux_storage::FieldEncryptor::new(&[7_u8; 32]).expect("encryptor")),
        ));
        let installed_repo_for_probe = installed_repo.clone();
        let feature_set_repo = Arc::new(SqliteFeatureSetRepository::new(db.clone()));
        let client_repo = Arc::new(SqliteInboundMcpClientRepository::new(db.clone()));
        let credential_repo = Arc::new(SqliteCredentialRepository::new(
            db.clone(),
            Arc::new(mcpmux_storage::FieldEncryptor::new(&[9_u8; 32]).expect("encryptor")),
        ));
        let backend_oauth_repo = Arc::new(SqliteOutboundOAuthRepository::new(db.clone()));
        let settings_repo = Arc::new(SqliteAppSettingsRepository::new(db.clone()));
        let gateway_port_service = Arc::new(GatewayPortService::new(settings_repo.clone()));
        let workspace_binding_repository =
            Arc::new(SqliteWorkspaceBindingRepository::new(db.clone()));
        let workspace_appearance_repository =
            Arc::new(SqliteWorkspaceAppearanceRepository::new(db.clone()));
        let server_feature_repository = Arc::new(SqliteServerFeatureRepository::new(db.clone()));
        let data_dir = temp_dir.path().join("data");
        let spaces_dir = data_dir.join("spaces");
        std::fs::create_dir_all(&spaces_dir).expect("create spaces");

        let services = Arc::new(
            ApplicationServicesBuilder::new()
                .with_event_bus(event_bus.clone())
                .with_space_repo(space_repo.clone())
                .with_installed_server_repo(installed_repo.clone())
                .with_feature_set_repo(feature_set_repo.clone())
                .with_server_feature_repo(server_feature_repository.clone())
                .with_client_repo(client_repo)
                .with_credential_repo(credential_repo.clone())
                .build()
                .expect("build ApplicationServices"),
        );
        let space_service =
            SpaceService::with_feature_set_repository(space_repo.clone(), feature_set_repo.clone());
        let server_discovery = Arc::new(ServerDiscoveryService::new(
            data_dir.clone(),
            data_dir.join("spaces"),
        ));
        let server_log_manager = Arc::new(ServerLogManager::new(LogConfig {
            base_dir: data_dir.join("logs"),
            max_file_size: 1024 * 1024,
            max_files: 5,
            compress: false,
        }));

        let deps = DependenciesBuilder::new()
            .with_installed_server_repo(installed_repo)
            .with_credential_repo(credential_repo)
            .with_backend_oauth_repo(backend_oauth_repo)
            .with_feature_repo(server_feature_repository.clone())
            .with_feature_set_repo(feature_set_repo.clone())
            .with_server_discovery(server_discovery.clone())
            .with_log_manager(server_log_manager.clone())
            .with_database(db)
            .with_jwt_secret(Zeroizing::new([5_u8; mcpmux_storage::JWT_SECRET_SIZE]))
            .with_settings_repo(settings_repo.clone())
            .with_event_bus(event_bus.clone())
            .build()
            .expect("gateway dependencies");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind gateway port");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);

        let gateway_config = GatewayConfig {
            host: "127.0.0.1".to_string(),
            port,
            enable_cors: false,
        };
        let listen_url = gateway_config.base_url();
        let gateway_server = GatewayServer::new(gateway_config, deps);
        let version_probe = Arc::new(ServerVersionProbeService::new(
            installed_repo_for_probe.clone(),
            settings_repo.clone(),
            event_bus.clone(),
        ));
        let live_runtime = Arc::new(LiveGatewayRuntime::from_gateway_server(
            &gateway_server,
            gateway_port_service.clone(),
            listen_url.clone(),
        ));
        let gateway_writes = Arc::new(LiveGatewayWriteRuntime::from_gateway_server(
            &gateway_server,
            data_dir.clone(),
            installed_repo_for_probe.clone(),
            version_probe.clone(),
        ));
        let gateway_handle = gateway_server.spawn();

        let bridge = Arc::new(AdminBridgeCtx {
            services: services.clone(),
            spaces_dir,
            data_dir: data_dir.clone(),
            gateway_port_service: gateway_port_service.clone(),
            server_discovery,
            settings_repository: settings_repo,
            workspace_binding_repository,
            workspace_appearance_repository,
            server_feature_repository,
            server_log_manager,
            space_service: Arc::new(space_service),
            gateway_runtime: live_runtime,
            gateway_writes,
            feature_set_repository: feature_set_repo,
            auto_launch_enabled: Some(false),
            app_version: "0.0.0-test".to_string(),
            bundle_version: None,
            backend_build: BackendBuildStamp {
                git_sha: "test-sha".to_string(),
                ..Default::default()
            },
            version_probe,
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let harness =
            AdminHarness::start_with_bridge(AdminConfig::default(), true, services, bridge).await;

        Self {
            harness,
            gateway_handle,
            listen_url,
        }
    }

    fn shutdown(self) {
        let mut handle = self.gateway_handle;
        handle.shutdown();
        self.harness.shutdown();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn live_gateway_status_matches_bridge_and_http() {
    let fixture = LiveGatewayFixture::start().await;

    let bridge_status = bridge_read::get_gateway_status(&fixture.harness.bridge, None)
        .await
        .expect("bridge status");
    assert_eq!(bridge_status["running"], true);
    assert_eq!(bridge_status["url"], fixture.listen_url);

    let stub_would_report_not_running = false;
    assert_ne!(
        bridge_status["running"].as_bool(),
        Some(stub_would_report_not_running)
    );

    let client = AdminClient::new(&fixture.harness.base_url, None);
    let resp = client.get_response("/api/v1/gateway/status").await;
    assert_eq!(resp.status(), 200);
    let http_body: serde_json::Value = resp.json().await.expect("status json");
    assert_eq!(http_body, bridge_status);

    let pool_resp = client.get_response("/api/v1/gateway/pool-stats").await;
    assert_eq!(pool_resp.status(), 200);
    let pool_http: serde_json::Value = pool_resp.json().await.expect("pool json");
    let pool_bridge = bridge_read::get_pool_stats(&fixture.harness.bridge)
        .await
        .expect("bridge pool stats");
    assert_eq!(pool_http, pool_bridge);

    fixture.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn live_gateway_port_settings_reflects_active_listen_url() {
    let fixture = LiveGatewayFixture::start().await;

    let bridge_settings = bridge_read::get_gateway_port_settings(&fixture.harness.bridge)
        .await
        .expect("port settings");
    let active_port = bridge_settings["activePort"].as_u64().expect("active port");
    assert!(fixture.listen_url.contains(&format!(":{active_port}")));

    fixture.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn live_gateway_update_server_package_uses_live_write_runtime() {
    use mcpmux_gateway::admin::command_bridge::write::{
        update_server_package, ServerConnectionBody,
    };

    let fixture = LiveGatewayFixture::start().await;
    let space_id = "11111111-1111-1111-1111-111111111111".to_string();

    let result = update_server_package(
        &fixture.harness.bridge,
        ServerConnectionBody {
            space_id,
            server_id: "missing-server".to_string(),
        },
    )
    .await;

    let error = result.expect_err("missing server should fail");
    let message = error.to_string();
    assert!(
        !message.contains("Gateway not running"),
        "live write runtime should be wired, got: {message}"
    );
    assert!(message.contains("not found") || message.contains("Not found"));

    fixture.shutdown();
}
