//! Guards for fork-only storage that an upstream merge can silently drop.
//!
//! Each test pins one thing the fork added on top of upstream: a migration's
//! schema objects, an `InstalledServer` / `InboundClient` column that must
//! survive a repository rewrite, or a settings key the tunnel setup reads.
//! See `docs/planning/upstream-sync-oct-2026.md`.

use chrono::{TimeZone, Utc};
use mcpmux_core::repository::{InstalledServerRepository, MachineRepository, SpaceRepository};
use mcpmux_core::{
    keys, AppSettingsService, DefaultParamsStrategy, InstallationSource, Machine, UpdatePolicy,
};
use mcpmux_storage::{
    generate_master_key, Database, FieldEncryptor, InboundClient, InboundClientRepository,
    RegistrationType, SqliteAppSettingsRepository, SqliteInstalledServerRepository,
    SqliteMachineRepository, SqliteSpaceRepository,
};
use pretty_assertions::assert_eq;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tests::{db::TestDatabase, fixtures};
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

/// Highest migration the fork ships. Bump when a migration is added.
const FORK_SCHEMA_VERSION: i64 = 47;

/// Tables created by fork migrations 019–043.
const FORK_TABLES: &[&str] = &[
    "space_base_dirs",
    "workspace_appearances",
    "tool_embeddings",
    "workspace_binding_feature_sets",
    "machines",
    "inbound_client_api_keys",
    "workspace_binding_prompt_dismissals",
];

/// Columns added by fork migrations 020–047 (044+ are the onboarding port and
/// upstream's renumbered OAuth client columns), as `(table, column)`.
const FORK_COLUMNS: &[(&str, &str)] = &[
    ("workspace_bindings", "label"),
    ("workspace_bindings", "icon"),
    ("workspace_bindings", "client_id"),
    ("workspace_bindings", "machine_id"),
    ("workspace_bindings", "binding_type"),
    ("workspace_bindings", "git_remote_url"),
    ("workspace_bindings", "project_link_id"),
    ("feature_set_members", "surfaced"),
    ("installed_servers", "cloned_from"),
    ("installed_servers", "display_name_override"),
    ("installed_servers", "default_params"),
    ("installed_servers", "default_params_strategy"),
    ("installed_servers", "update_policy"),
    ("installed_servers", "pinned_version"),
    ("installed_servers", "latest_available_version"),
    ("installed_servers", "version_checked_at"),
    ("installed_servers", "current_version"),
    ("inbound_clients", "machine_id"),
    ("inbound_clients", "locked_space_id"),
    ("inbound_clients", "client_icon"),
    ("feature_sets", "auto_include"),
    ("outbound_oauth_clients", "client_secret_encrypted"),
    ("outbound_oauth_clients", "token_endpoint_auth_method"),
    ("outbound_oauth_clients", "client_secret_expires_at"),
];

fn object_exists(db: &Database, kind: &str, name: &str) -> bool {
    db.connection()
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = ?1 AND name = ?2",
            [kind, name],
            |r| r.get(0),
        )
        .unwrap()
}

fn column_exists(db: &Database, table: &str, column: &str) -> bool {
    db.connection()
        .query_row(
            &format!("SELECT COUNT(*) > 0 FROM pragma_table_info('{table}') WHERE name = ?1"),
            [column],
            |r| r.get(0),
        )
        .unwrap()
}

/// Every fork table and column exists after a fresh migrate. Fails if a merge
/// drops, renumbers below the ledger, or rewrites one of migrations 019–043.
#[test]
fn fork_schema_objects_exist() {
    let db = Database::open_in_memory().expect("open");

    let missing_tables: Vec<_> = FORK_TABLES
        .iter()
        .filter(|t| !object_exists(&db, "table", t))
        .collect();
    let missing_columns: Vec<_> = FORK_COLUMNS
        .iter()
        .filter(|(t, c)| !column_exists(&db, t, c))
        .collect();

    assert!(
        missing_tables.is_empty(),
        "missing tables: {missing_tables:?}"
    );
    assert!(
        missing_columns.is_empty(),
        "missing columns: {missing_columns:?}"
    );
}

/// The ledger reaches at least the fork's last migration, and versions are
/// contiguous. An upstream migration slotted into a number the fork already
/// used (instead of appended after it) shows up here as a gap or a duplicate.
#[test]
fn migration_ledger_is_contiguous_and_reaches_fork_head() {
    let db = Database::open_in_memory().expect("open");
    let versions: Vec<i64> = {
        let conn = db.connection();
        let mut stmt = conn
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };

    let expected: Vec<i64> = (1..=*versions.last().unwrap()).collect();
    assert_eq!(
        versions, expected,
        "migration versions must be 1..=N with no gaps"
    );
    assert!(
        *versions.last().unwrap() >= FORK_SCHEMA_VERSION,
        "schema stopped below the fork head ({FORK_SCHEMA_VERSION})"
    );
}

// ---------------------------------------------------------------------------
// InstalledServer
// ---------------------------------------------------------------------------

/// Every fork-only `InstalledServer` field survives install → get and
/// update → get. Guards the repository rewrite in upstream #276/#285.
#[tokio::test]
async fn installed_server_fork_fields_round_trip() {
    let test_db = TestDatabase::new();
    let db = Arc::new(Mutex::new(test_db.db));
    let key = generate_master_key().unwrap();
    let encryptor = Arc::new(FieldEncryptor::new(&key).unwrap());
    let repo = SqliteInstalledServerRepository::new(db.clone(), encryptor);
    let space = fixtures::test_space("Fork fields");
    SpaceRepository::create(&SqliteSpaceRepository::new(db), &space)
        .await
        .unwrap();

    let checked_at = Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap();
    let mut server = fixtures::test_installed_server(&space.id.to_string(), "atlassian-work")
        .with_cloned_from("atlassian")
        .with_display_name_override(Some("Atlassian (work)"))
        .with_update_policy(UpdatePolicy::Pinned)
        .with_pinned_version(Some("1.4.2"))
        .with_source(InstallationSource::UserConfig {
            file_path: PathBuf::from("/tmp/space.json"),
        });
    server
        .default_params
        .insert("cloudId".into(), json!("abc-123"));
    server.default_params.insert("limit".into(), json!(25));
    server.default_params_strategy = DefaultParamsStrategy::Override;
    server.latest_available_version = Some("1.5.0".into());
    server.current_version = Some("1.4.2".into());
    server.version_checked_at = Some(checked_at);

    repo.install(&server).await.unwrap();
    let loaded = repo.get(&server.id).await.unwrap().expect("installed");

    assert_eq!(loaded.cloned_from.as_deref(), Some("atlassian"));
    assert_eq!(
        loaded.display_name_override.as_deref(),
        Some("Atlassian (work)")
    );
    assert_eq!(loaded.update_policy, UpdatePolicy::Pinned);
    assert_eq!(loaded.pinned_version.as_deref(), Some("1.4.2"));
    assert_eq!(loaded.default_params, server.default_params);
    assert_eq!(
        loaded.default_params_strategy,
        DefaultParamsStrategy::Override
    );
    assert_eq!(loaded.latest_available_version.as_deref(), Some("1.5.0"));
    assert_eq!(loaded.current_version.as_deref(), Some("1.4.2"));
    assert_eq!(loaded.version_checked_at, Some(checked_at));
    assert_eq!(loaded.source, server.source);

    let mut edited = loaded;
    edited.display_name_override = None;
    edited.update_policy = UpdatePolicy::Auto;
    edited.default_params_strategy = DefaultParamsStrategy::Fill;
    edited.default_params.remove("limit");
    repo.update(&edited).await.unwrap();
    let reloaded = repo.get(&server.id).await.unwrap().unwrap();

    assert_eq!(reloaded.display_name_override, None);
    assert_eq!(reloaded.update_policy, UpdatePolicy::Auto);
    assert_eq!(
        reloaded.default_params_strategy,
        DefaultParamsStrategy::Fill
    );
    assert_eq!(reloaded.default_params.len(), 1);
    assert_eq!(reloaded.cloned_from.as_deref(), Some("atlassian"));
}

// ---------------------------------------------------------------------------
// InboundClient
// ---------------------------------------------------------------------------

/// Build a minimal DCR client with no fork fields set.
fn dcr_client(client_id: &str) -> InboundClient {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    InboundClient {
        client_id: client_id.into(),
        registration_type: RegistrationType::Dcr,
        client_name: "Cursor".into(),
        client_alias: None,
        redirect_uris: vec!["http://127.0.0.1:8080/callback".into()],
        grant_types: vec!["authorization_code".into()],
        response_types: vec!["code".into()],
        token_endpoint_auth_method: "none".into(),
        scope: None,
        approved: true,
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
        machine_id: None,
        client_icon: None,
    }
}

/// Machine binding, icon and Space lock persist on a client, and a later
/// DCR re-registration (`save_client` upsert) does not wipe them.
#[tokio::test]
async fn inbound_client_fork_fields_survive_reregistration() {
    let test_db = TestDatabase::new();
    let db = Arc::new(Mutex::new(test_db.db));
    let repo = InboundClientRepository::new(db.clone());

    let space = fixtures::test_space("Locked");
    SpaceRepository::create(&SqliteSpaceRepository::new(db.clone()), &space)
        .await
        .unwrap();
    let machine = Machine::new("Gondor");
    SqliteMachineRepository::new(db)
        .create(&machine)
        .await
        .unwrap();

    let client = dcr_client("mcp_fork_fields");
    repo.save_client(&client).await.unwrap();
    repo.set_machine_id(&client.client_id, Some(machine.id))
        .await
        .unwrap();
    repo.update_client_icon(&client.client_id, Some("🛰️".into()))
        .await
        .unwrap();
    repo.set_locked_space(&client.client_id, Some(space.id))
        .await
        .unwrap();

    repo.save_client(&dcr_client("mcp_fork_fields"))
        .await
        .unwrap();

    let loaded = repo.get_client(&client.client_id).await.unwrap().unwrap();
    assert_eq!(loaded.machine_id, Some(machine.id));
    assert_eq!(loaded.client_icon.as_deref(), Some("🛰️"));
    assert_eq!(
        repo.get_locked_space(&client.client_id).await.unwrap(),
        Some(space.id)
    );
}

// ---------------------------------------------------------------------------
// Settings keys
// ---------------------------------------------------------------------------

/// The tunnel and machine routing read these exact keys from the live DB.
/// Upstream #267 also reads `gateway.public_base_url` to decide whether the
/// gateway is exposed, so a rename here silently turns that check off.
#[tokio::test]
async fn fork_settings_keys_are_stable_and_round_trip() {
    assert_eq!(keys::gateway::PUBLIC_URL, "gateway.public_base_url");
    assert_eq!(keys::gateway::LOCAL_MACHINE_ID, "gateway.local_machine_id");
    assert_eq!(
        keys::gateway::ADMIN_TRUST_CF_ACCESS,
        "gateway.admin_trust_cf_access"
    );
    assert_eq!(
        keys::gateway::ADMIN_CF_TEAM_DOMAIN,
        "gateway.admin_cf_team_domain"
    );

    let test_db = TestDatabase::new();
    let db = Arc::new(Mutex::new(test_db.db));
    let settings = AppSettingsService::new(Arc::new(SqliteAppSettingsRepository::new(db)));

    settings
        .set_gateway_public_url("https://mcp.example.com")
        .await
        .unwrap();
    let machine_id = uuid::Uuid::new_v4();
    settings
        .set_local_machine_id(Some(machine_id))
        .await
        .unwrap();

    assert_eq!(
        settings.get_gateway_public_url().await.as_deref(),
        Some("https://mcp.example.com")
    );
    assert_eq!(settings.get_local_machine_id().await, Some(machine_id));
}
