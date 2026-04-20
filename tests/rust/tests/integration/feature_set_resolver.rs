//! Decision-table tests for the FeatureSet resolver (pin > workspace > space-active > deny).
//!
//! Uses real SQLite repositories (via in-memory Database) rather than mocks so
//! we exercise the same code paths the gateway will at runtime.

use std::sync::Arc;

use mcpmux_core::{
    normalize_workspace_root, Client, FeatureSet, FeatureSetRepository, InboundMcpClientRepository,
    Space, SpaceRepository, WorkspaceBinding, WorkspaceBindingRepository,
};
use mcpmux_gateway::services::{FeatureSetResolverService, ResolutionSource, SessionRootsRegistry};
use mcpmux_storage::{
    Database, SqliteFeatureSetRepository, SqliteInboundMcpClientRepository, SqliteSpaceRepository,
    SqliteWorkspaceBindingRepository,
};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Fixture wiring up real SQLite-backed repos + a resolver, with a Space that
/// already has an `active_feature_set_id` so the SpaceActive tier works.
struct Fixture {
    resolver: FeatureSetResolverService,
    session_roots: Arc<SessionRootsRegistry>,
    client_repo: Arc<dyn InboundMcpClientRepository>,
    space_repo: Arc<dyn SpaceRepository>,
    binding_repo: Arc<dyn WorkspaceBindingRepository>,
    space_id: Uuid,
    active_fs_id: Uuid,
    other_fs_id: Uuid,
    client_id: Uuid,
}

impl Fixture {
    async fn new() -> Self {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));

        let space_repo: Arc<dyn SpaceRepository> = Arc::new(SqliteSpaceRepository::new(db.clone()));
        let fs_repo: Arc<dyn FeatureSetRepository> =
            Arc::new(SqliteFeatureSetRepository::new(db.clone()));
        let client_repo: Arc<dyn InboundMcpClientRepository> =
            Arc::new(SqliteInboundMcpClientRepository::new(db.clone()));
        let binding_repo: Arc<dyn WorkspaceBindingRepository> =
            Arc::new(SqliteWorkspaceBindingRepository::new(db.clone()));

        // Use the default space (created by migration 001).
        let default_space = space_repo.get_default().await.unwrap().unwrap();
        let space_id = default_space.id;

        // Create two custom FSes so we can distinguish Pin from SpaceActive.
        let active_fs = FeatureSet::new_custom("Space Active FS", &space_id.to_string());
        let other_fs = FeatureSet::new_custom("Pinned FS", &space_id.to_string());
        fs_repo.create(&active_fs).await.unwrap();
        fs_repo.create(&other_fs).await.unwrap();
        let active_fs_id = Uuid::parse_str(&active_fs.id).unwrap();
        let other_fs_id = Uuid::parse_str(&other_fs.id).unwrap();

        // Set Space's active FS.
        space_repo
            .set_active_feature_set(&space_id, Some(&active_fs_id))
            .await
            .unwrap();

        // Create a test client with pinned_space_id set.
        let mut client = Client::new("test", "test-type");
        client.pinned_space_id = Some(space_id);
        client_repo.create(&client).await.unwrap();
        // set_pin ensures the DB columns are actually populated (create() uses
        // the legacy columns by default).
        client_repo
            .set_pin(&client.id, &space_id, None)
            .await
            .unwrap();

        let session_roots = SessionRootsRegistry::new();
        let resolver = FeatureSetResolverService::new(
            client_repo.clone(),
            space_repo.clone(),
            binding_repo.clone(),
            session_roots.clone(),
        );

        Self {
            resolver,
            session_roots,
            client_repo,
            space_repo,
            binding_repo,
            space_id,
            active_fs_id,
            other_fs_id,
            client_id: client.id,
        }
    }

    async fn set_pin(&self, fs: Option<Uuid>) {
        self.client_repo
            .set_pin(&self.client_id, &self.space_id, fs.as_ref())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn resolve_falls_through_to_space_active_when_no_pin_and_no_roots() {
    let f = Fixture::new().await;
    let r = f.resolver.resolve(&f.client_id, None).await.unwrap();
    assert_eq!(r.source, ResolutionSource::SpaceActive);
    assert_eq!(r.feature_set_id, Some(f.active_fs_id));
}

#[tokio::test]
async fn resolve_pin_wins_over_space_active() {
    let f = Fixture::new().await;
    f.set_pin(Some(f.other_fs_id)).await;

    let r = f.resolver.resolve(&f.client_id, None).await.unwrap();
    assert_eq!(r.source, ResolutionSource::Pin);
    assert_eq!(r.feature_set_id, Some(f.other_fs_id));
}

#[tokio::test]
async fn resolve_pin_wins_over_workspace_binding() {
    let f = Fixture::new().await;

    // Binding matches our session root, but pin should still win.
    let root = if cfg!(windows) {
        "d:\\work\\proj"
    } else {
        "/work/proj"
    };
    f.binding_repo
        .create(&WorkspaceBinding::new(
            f.space_id,
            normalize_workspace_root(root),
            f.other_fs_id,
        ))
        .await
        .unwrap();
    f.session_roots.set("sess-1", [root]);

    f.set_pin(Some(f.active_fs_id)).await;
    let r = f
        .resolver
        .resolve(&f.client_id, Some("sess-1"))
        .await
        .unwrap();
    assert_eq!(r.source, ResolutionSource::Pin);
    assert_eq!(r.feature_set_id, Some(f.active_fs_id));
}

#[tokio::test]
async fn resolve_workspace_binding_beats_space_active_when_no_pin() {
    let f = Fixture::new().await;

    let root = if cfg!(windows) {
        "d:\\work\\proj"
    } else {
        "/work/proj"
    };
    f.binding_repo
        .create(&WorkspaceBinding::new(
            f.space_id,
            normalize_workspace_root(root),
            f.other_fs_id,
        ))
        .await
        .unwrap();
    f.session_roots.set("sess-2", [root]);

    let r = f
        .resolver
        .resolve(&f.client_id, Some("sess-2"))
        .await
        .unwrap();
    assert_eq!(r.source, ResolutionSource::WorkspaceBinding);
    assert_eq!(r.feature_set_id, Some(f.other_fs_id));
}

#[tokio::test]
async fn resolve_deny_when_no_pin_no_binding_no_space_active() {
    let f = Fixture::new().await;
    // Clear the Space's active FS — last tier becomes Deny.
    f.space_repo
        .set_active_feature_set(&f.space_id, None)
        .await
        .unwrap();

    let r = f.resolver.resolve(&f.client_id, None).await.unwrap();
    assert_eq!(r.source, ResolutionSource::Deny);
    assert_eq!(r.feature_set_id, None);
}

#[tokio::test]
async fn resolve_longest_prefix_wins_across_multiple_bindings() {
    let f = Fixture::new().await;

    // Add two nested bindings in the same Space.
    let (outer_root, inner_root) = if cfg!(windows) {
        ("d:\\work", "d:\\work\\proj")
    } else {
        ("/work", "/work/proj")
    };
    // outer -> active_fs (just any FS we have), inner -> other_fs
    f.binding_repo
        .create(&WorkspaceBinding::new(
            f.space_id,
            normalize_workspace_root(outer_root),
            f.active_fs_id,
        ))
        .await
        .unwrap();
    f.binding_repo
        .create(&WorkspaceBinding::new(
            f.space_id,
            normalize_workspace_root(inner_root),
            f.other_fs_id,
        ))
        .await
        .unwrap();

    // Caller reports a path inside the inner binding — longest prefix wins.
    let deep = if cfg!(windows) {
        "d:\\work\\proj\\src"
    } else {
        "/work/proj/src"
    };
    f.session_roots.set("sess-deep", [deep]);

    let r = f
        .resolver
        .resolve(&f.client_id, Some("sess-deep"))
        .await
        .unwrap();
    assert_eq!(r.source, ResolutionSource::WorkspaceBinding);
    assert_eq!(r.feature_set_id, Some(f.other_fs_id));
}

#[tokio::test]
async fn resolve_falls_through_when_roots_dont_match_any_binding() {
    let f = Fixture::new().await;

    let bound = if cfg!(windows) {
        "d:\\android"
    } else {
        "/android"
    };
    let reported = if cfg!(windows) {
        "d:\\cloudflare"
    } else {
        "/cloudflare"
    };
    f.binding_repo
        .create(&WorkspaceBinding::new(
            f.space_id,
            normalize_workspace_root(bound),
            f.other_fs_id,
        ))
        .await
        .unwrap();
    f.session_roots.set("sess-3", [reported]);

    let r = f
        .resolver
        .resolve(&f.client_id, Some("sess-3"))
        .await
        .unwrap();
    assert_eq!(r.source, ResolutionSource::SpaceActive);
    assert_eq!(r.feature_set_id, Some(f.active_fs_id));
}

#[tokio::test]
async fn resolve_returns_deny_for_unknown_client() {
    let f = Fixture::new().await;
    let unknown = Uuid::new_v4();
    let r = f.resolver.resolve(&unknown, None).await.unwrap();
    assert_eq!(r.source, ResolutionSource::Deny);
    assert!(r.feature_set_id.is_none());
}
