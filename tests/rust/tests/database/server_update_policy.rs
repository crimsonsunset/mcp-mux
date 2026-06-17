//! Migration 024/025 column read/write tests for server update policy.

use chrono::Utc;
use mcpmux_core::repository::{InstalledServerRepository, SpaceRepository};
use mcpmux_core::{UpdatePolicy};
use mcpmux_storage::{
    generate_master_key, FieldEncryptor, SqliteInstalledServerRepository, SqliteSpaceRepository,
};
use pretty_assertions::assert_eq;
use std::sync::Arc;
use tests::{db::TestDatabase, fixtures};
use tokio::sync::Mutex;

fn test_encryptor() -> Arc<FieldEncryptor> {
    let key = generate_master_key().expect("Failed to generate key");
    Arc::new(FieldEncryptor::new(&key).expect("Failed to create encryptor"))
}

#[tokio::test]
async fn installed_server_defaults_update_policy_to_notify() {
    let test_db = TestDatabase::new();
    let db = Arc::new(Mutex::new(test_db.db));
    let server_repo = SqliteInstalledServerRepository::new(Arc::clone(&db), test_encryptor());
    let space_repo = SqliteSpaceRepository::new(db);

    let space = fixtures::test_space("Test Space");
    SpaceRepository::create(&space_repo, &space).await.unwrap();

    let server = fixtures::test_installed_server(&space.id.to_string(), "policy-default");
    let server_id = server.id;
    InstalledServerRepository::install(&server_repo, &server)
        .await
        .expect("install server");

    let loaded = InstalledServerRepository::get(&server_repo, &server_id)
        .await
        .expect("get server")
        .expect("server exists");

    assert_eq!(loaded.update_policy, UpdatePolicy::Notify);
    assert!(loaded.pinned_version.is_none());
    assert!(loaded.latest_available_version.is_none());
    assert!(loaded.version_checked_at.is_none());
}

#[tokio::test]
async fn installed_server_persists_update_policy_and_pin() {
    let test_db = TestDatabase::new();
    let db = Arc::new(Mutex::new(test_db.db));
    let server_repo = SqliteInstalledServerRepository::new(Arc::clone(&db), test_encryptor());
    let space_repo = SqliteSpaceRepository::new(db);

    let space = fixtures::test_space("Test Space");
    SpaceRepository::create(&space_repo, &space).await.unwrap();

    let mut server = fixtures::test_installed_server(&space.id.to_string(), "policy-pinned");
    server.update_policy = UpdatePolicy::Pinned;
    server.pinned_version = Some("2.1.0".to_string());

    let server_id = server.id;
    InstalledServerRepository::install(&server_repo, &server)
        .await
        .expect("install server");

    let loaded = InstalledServerRepository::get(&server_repo, &server_id)
        .await
        .expect("get server")
        .expect("server exists");

    assert_eq!(loaded.update_policy, UpdatePolicy::Pinned);
    assert_eq!(loaded.pinned_version.as_deref(), Some("2.1.0"));
}

#[tokio::test]
async fn installed_server_update_version_cache_round_trip() {
    let test_db = TestDatabase::new();
    let db = Arc::new(Mutex::new(test_db.db));
    let server_repo = SqliteInstalledServerRepository::new(Arc::clone(&db), test_encryptor());
    let space_repo = SqliteSpaceRepository::new(db);

    let space = fixtures::test_space("Test Space");
    SpaceRepository::create(&space_repo, &space).await.unwrap();

    let server = fixtures::test_installed_server(&space.id.to_string(), "version-cache");
    let server_id = server.id;
    InstalledServerRepository::install(&server_repo, &server)
        .await
        .expect("install server");

    let checked_at = Utc::now();
    InstalledServerRepository::update_version_cache(
        &server_repo,
        &server_id,
        Some("4.5.6".to_string()),
        checked_at,
    )
    .await
    .expect("update version cache");

    let loaded = InstalledServerRepository::get(&server_repo, &server_id)
        .await
        .expect("get server")
        .expect("server exists");

    assert_eq!(
        loaded.latest_available_version.as_deref(),
        Some("4.5.6")
    );
    assert!(loaded.version_checked_at.is_some());
}

#[tokio::test]
async fn installed_server_update_preserves_version_cache_columns() {
    let test_db = TestDatabase::new();
    let db = Arc::new(Mutex::new(test_db.db));
    let server_repo = SqliteInstalledServerRepository::new(Arc::clone(&db), test_encryptor());
    let space_repo = SqliteSpaceRepository::new(db);

    let space = fixtures::test_space("Test Space");
    SpaceRepository::create(&space_repo, &space).await.unwrap();

    let mut server = fixtures::test_installed_server(&space.id.to_string(), "policy-update");
    server.update_policy = UpdatePolicy::Auto;
    server.latest_available_version = Some("9.9.9".to_string());
    server.version_checked_at = Some(Utc::now());

    let server_id = server.id;
    InstalledServerRepository::install(&server_repo, &server)
        .await
        .expect("install server");

    let mut loaded = InstalledServerRepository::get(&server_repo, &server_id)
        .await
        .expect("get server")
        .expect("server exists");
    loaded.update_policy = UpdatePolicy::Notify;
    loaded.pinned_version = Some("1.0.0".to_string());

    InstalledServerRepository::update(&server_repo, &loaded)
        .await
        .expect("update server");

    let reloaded = InstalledServerRepository::get(&server_repo, &server_id)
        .await
        .expect("get server")
        .expect("server exists");

    assert_eq!(reloaded.update_policy, UpdatePolicy::Notify);
    assert_eq!(reloaded.pinned_version.as_deref(), Some("1.0.0"));
    assert_eq!(
        reloaded.latest_available_version.as_deref(),
        Some("9.9.9")
    );
    assert!(reloaded.version_checked_at.is_some());
}
