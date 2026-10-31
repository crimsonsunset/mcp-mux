//! Web admin server logs are scoped to the Space they are asked for.
//!
//! Logs live at `logs/<space>/<server>/`. The admin bridge used to read, clear
//! and locate them under the default Space no matter which Space the UI was
//! viewing (upstream #251 fixed the same bug in the desktop commands).

use std::path::PathBuf;
use std::sync::Arc;

use mcpmux_core::{
    ApplicationServicesBuilder, EventBus, GatewayPortService, LogConfig, LogSource,
    ServerDiscoveryService, ServerLog, ServerLogManager, SpaceRepository, SpaceService,
};
use mcpmux_gateway::admin::command_bridge::{read, write};
use mcpmux_gateway::admin::StubGatewayWriteRuntime;
use mcpmux_gateway::admin::{AdminBridgeCtx, BackendBuildStamp, StubGatewayRuntime};
use mcpmux_storage::{
    InboundClientRepository, SqliteMachineRepository, SqliteSpaceBaseDirRepository,
    SqliteSpaceBuiltinConfigRepository, SqliteSpaceRepository, SqliteWorkspaceAppearanceRepository,
    SqliteWorkspaceBindingRepository,
};
use tempfile::TempDir;
use tests::db::TestDatabase;
use tests::fixtures;
use tests::mocks::{
    MockAppSettingsRepository, MockFeatureSetRepository, MockInboundMcpClientRepository,
    MockInstalledServerRepository, MockServerFeatureRepository,
};
use tokio::sync::Mutex;
use uuid::Uuid;

const SERVER_ID: &str = "com.example.server";

struct Harness {
    ctx: AdminBridgeCtx,
    logs: Arc<ServerLogManager>,
    default_space: Uuid,
    other_space: Uuid,
    _dir: TempDir,
}

impl Harness {
    /// Two Spaces (the first is the default), real log files on disk, stubbed runtimes.
    async fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Mutex::new(TestDatabase::in_memory().db));

        let space_repo = Arc::new(SqliteSpaceRepository::new(db.clone()));
        let default_space = fixtures::default_test_space();
        let other_space = fixtures::test_space("Other");
        SpaceRepository::create(&*space_repo, &default_space)
            .await
            .unwrap();
        SpaceRepository::create(&*space_repo, &other_space)
            .await
            .unwrap();
        SpaceRepository::set_default(&*space_repo, &default_space.id)
            .await
            .unwrap();

        let settings = Arc::new(MockAppSettingsRepository::new());
        let installed = Arc::new(MockInstalledServerRepository::new());
        let features = Arc::new(MockServerFeatureRepository::new());
        let feature_sets = Arc::new(MockFeatureSetRepository::new());

        let services = Arc::new(
            ApplicationServicesBuilder::new()
                .with_event_bus(Arc::new(EventBus::new()))
                .with_space_repo(space_repo.clone())
                .with_installed_server_repo(installed.clone())
                .with_feature_set_repo(feature_sets.clone())
                .with_server_feature_repo(features.clone())
                .with_client_repo(Arc::new(MockInboundMcpClientRepository::new()))
                .with_credential_repo(Arc::new(tests::mocks::MockCredentialRepository::new()))
                .build()
                .unwrap(),
        );

        let logs = Arc::new(ServerLogManager::new(LogConfig {
            base_dir: dir.path().join("logs"),
            max_file_size: 1024 * 1024,
            max_files: 3,
            compress: false,
        }));

        let ctx = AdminBridgeCtx {
            services,
            spaces_dir: dir.path().join("spaces"),
            data_dir: dir.path().to_path_buf(),
            gateway_port_service: Arc::new(GatewayPortService::new(settings.clone())),
            server_discovery: Arc::new(ServerDiscoveryService::new(
                PathBuf::from(dir.path()),
                dir.path().join("spaces"),
            )),
            settings_repository: settings,
            installed_server_repository: installed,
            workspace_binding_repository: Arc::new(SqliteWorkspaceBindingRepository::new(
                db.clone(),
            )),
            machine_repository: Arc::new(SqliteMachineRepository::new(db.clone())),
            inbound_client_repository: Arc::new(InboundClientRepository::new(db.clone())),
            workspace_appearance_repository: Arc::new(SqliteWorkspaceAppearanceRepository::new(
                db.clone(),
            )),
            server_feature_repository: features,
            server_log_manager: logs.clone(),
            space_service: Arc::new(SpaceService::new(space_repo)),
            gateway_runtime: Arc::new(StubGatewayRuntime),
            gateway_writes: Arc::new(StubGatewayWriteRuntime::default()),
            feature_set_repository: feature_sets,
            space_base_dir_repository: Arc::new(SqliteSpaceBaseDirRepository::new(db.clone())),
            space_builtin_config_repository: Arc::new(SqliteSpaceBuiltinConfigRepository::new(db)),
            auto_launch_enabled: None,
            app_version: "test".into(),
            bundle_version: None,
            backend_build: BackendBuildStamp::default(),
        };

        Self {
            ctx,
            logs,
            default_space: default_space.id,
            other_space: other_space.id,
            _dir: dir,
        }
    }

    async fn log(&self, space: Uuid, message: &str) {
        self.logs
            .append(
                &space.to_string(),
                SERVER_ID,
                ServerLog::new(mcpmux_core::LogLevel::Info, LogSource::Stderr, message),
            )
            .await
            .unwrap();
    }

    async fn messages(&self, space: Uuid) -> Vec<String> {
        let value = read::get_server_logs(
            &self.ctx,
            SERVER_ID.to_string(),
            space.to_string(),
            Some(50),
            None,
        )
        .await
        .unwrap();
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["message"].as_str().unwrap().to_string())
            .collect()
    }
}

#[tokio::test]
async fn reads_the_logs_of_the_requested_space_not_the_default() {
    let h = Harness::new().await;
    h.log(h.default_space, "from default").await;
    h.log(h.other_space, "from other").await;

    assert_eq!(h.messages(h.other_space).await, vec!["from other"]);
    assert_eq!(h.messages(h.default_space).await, vec!["from default"]);
}

#[tokio::test]
async fn clearing_one_space_leaves_the_other_untouched() {
    let h = Harness::new().await;
    h.log(h.default_space, "from default").await;
    h.log(h.other_space, "from other").await;

    write::clear_server_logs(&h.ctx, SERVER_ID.to_string(), h.other_space.to_string())
        .await
        .unwrap();

    assert!(h.messages(h.other_space).await.is_empty());
    assert_eq!(h.messages(h.default_space).await, vec!["from default"]);
}

#[tokio::test]
async fn log_file_path_is_under_the_requested_space() {
    let h = Harness::new().await;

    let value = read::get_server_log_file(&h.ctx, SERVER_ID.to_string(), h.other_space.to_string())
        .await
        .unwrap();
    let path = value.as_str().unwrap();

    assert!(path.contains(&h.other_space.to_string()), "{path}");
    assert!(!path.contains(&h.default_space.to_string()), "{path}");
}

#[tokio::test]
async fn rejects_a_space_id_that_is_not_a_uuid() {
    let h = Harness::new().await;

    let read_err = read::get_server_logs(
        &h.ctx,
        SERVER_ID.to_string(),
        "../../etc".to_string(),
        None,
        None,
    )
    .await;
    let clear_err =
        write::clear_server_logs(&h.ctx, SERVER_ID.to_string(), "../../etc".to_string()).await;
    let file_err =
        read::get_server_log_file(&h.ctx, SERVER_ID.to_string(), "../../etc".to_string()).await;

    assert!(read_err.is_err());
    assert!(clear_err.is_err());
    assert!(file_err.is_err());
}
