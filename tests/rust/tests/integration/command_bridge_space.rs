//! Integration tests for `command_bridge::space` against in-memory SQLite.

use std::sync::Arc;

use mcpmux_core::{
    ApplicationServices, ApplicationServicesBuilder, EventBus, SpaceRepository,
};
use mcpmux_gateway::admin::command_bridge::space::{
    self, SpaceBridgeCtx, UpdateSpaceInput,
};
use mcpmux_storage::{Database, SqliteSpaceRepository};
use tempfile::TempDir;
use tokio::sync::Mutex;
use uuid::Uuid;

struct SpaceBridgeHarness {
    services: Arc<ApplicationServices>,
    spaces_dir: TempDir,
}

impl SpaceBridgeHarness {
    async fn new() -> Self {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let event_bus = Arc::new(EventBus::new());
        let space_repo: Arc<dyn SpaceRepository> = Arc::new(SqliteSpaceRepository::new(db));
        let services = Arc::new(
            ApplicationServicesBuilder::new()
                .with_event_bus(event_bus)
                .with_space_repo(space_repo)
                .build()
                .expect("build ApplicationServices"),
        );
        let spaces_dir = TempDir::new().expect("temp spaces dir");

        Self {
            services,
            spaces_dir,
        }
    }

    fn ctx(&self) -> SpaceBridgeCtx<'_> {
        SpaceBridgeCtx {
            services: &self.services,
            spaces_dir: self.spaces_dir.path(),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn list_spaces_includes_seeded_default() {
    let harness = SpaceBridgeHarness::new().await;
    let spaces = space::list_spaces(&harness.ctx()).await.unwrap();
    assert!(!spaces.is_empty());
    assert!(spaces.iter().any(|s| s.is_default));
}

#[tokio::test(flavor = "multi_thread")]
async fn create_get_update_delete_round_trip() {
    let harness = SpaceBridgeHarness::new().await;
    let ctx = harness.ctx();

    let created = space::create_space(&ctx, "Work".to_string(), Some("briefcase".to_string()))
        .await
        .unwrap();
    assert_eq!(created.name, "Work");
    assert_eq!(created.icon.as_deref(), Some("briefcase"));

    let config_path = ctx.config_path(&created.id.to_string());
    assert!(config_path.is_file());

    let loaded = space::get_space(&ctx, created.id).await.unwrap().unwrap();
    assert_eq!(loaded.name, "Work");

    let updated = space::update_space(
        &ctx,
        created.id,
        UpdateSpaceInput {
            name: Some("Work Updated".to_string()),
            icon: None,
            description: Some("Notes".to_string()),
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.name, "Work Updated");
    assert_eq!(updated.description.as_deref(), Some("Notes"));

    space::delete_space(&ctx, created.id).await.unwrap();
    assert!(space::get_space(&ctx, created.id).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn read_space_config_creates_default_template() {
    let harness = SpaceBridgeHarness::new().await;
    let ctx = harness.ctx();
    let space_id = Uuid::new_v4().to_string();

    let content = space::read_space_config(&ctx, &space_id).await.unwrap();
    assert!(content.contains("mcpServers"));
    assert!(ctx.config_path(&space_id).is_file());
}

#[tokio::test(flavor = "multi_thread")]
async fn save_space_config_validates_json() {
    let harness = SpaceBridgeHarness::new().await;
    let ctx = harness.ctx();
    let created = space::create_space(&ctx, "Config".to_string(), None)
        .await
        .unwrap();
    let space_id = created.id.to_string();

    let err = space::save_space_config(&ctx, &space_id, "{ not json")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Invalid JSON"));

    space::save_space_config(
        &ctx,
        &space_id,
        r#"{"mcpServers":{"demo":{"command":"echo"}}}"#,
    )
    .await
    .unwrap();

    let read_back = space::read_space_config(&ctx, &space_id).await.unwrap();
    assert!(read_back.contains("demo"));
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_server_from_config() {
    let harness = SpaceBridgeHarness::new().await;
    let ctx = harness.ctx();
    let created = space::create_space(&ctx, "Servers".to_string(), None)
        .await
        .unwrap();
    let space_id = created.id.to_string();

    space::save_space_config(
        &ctx,
        &space_id,
        r#"{"mcpServers":{"keep":{"command":"a"},"drop":{"command":"b"}}}"#,
    )
    .await
    .unwrap();

    let removed = space::remove_server_from_config(&ctx, &space_id, "drop")
        .await
        .unwrap();
    assert!(removed);

    let missing = space::remove_server_from_config(&ctx, &space_id, "missing")
        .await
        .unwrap();
    assert!(!missing);

    let content = space::read_space_config(&ctx, &space_id).await.unwrap();
    assert!(content.contains("keep"));
    assert!(!content.contains("drop"));
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_default_space_is_rejected() {
    let harness = SpaceBridgeHarness::new().await;
    let ctx = harness.ctx();
    let default = space::list_spaces(&ctx)
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.is_default)
        .expect("seeded default space");

    let err = space::delete_space(&ctx, default.id).await.unwrap_err();
    assert!(err.to_string().contains("Cannot delete the default space"));
}
