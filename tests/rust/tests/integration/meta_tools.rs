//! End-to-end tests for the `mcpmux_*` self-management meta tools.
//!
//! Exercises the full path through the [`MetaToolRegistry`]:
//!   * read tools return structured payloads
//!   * write tools gate on the [`ApprovalBroker`] and only mutate state on Allow
//!   * denial / timeout / no-publisher surface as `CallToolResult::error`
//!   * "always-allow" persists for subsequent calls in the same session

use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use mcpmux_core::{
    normalize_workspace_root, Client, DomainEvent, FeatureSet, FeatureSetMember,
    FeatureSetRepository, InboundMcpClientRepository, InputDefinition, InstalledServer,
    InstalledServerRepository, LogConfig, MemberMode, MemberType, ServerDefinition, ServerFeature,
    ServerFeatureRepository, ServerLogManager, ServerSource, SpaceRepository, TransportConfig,
    TransportMetadata, WorkspaceBinding, WorkspaceBindingRepository,
};
use mcpmux_gateway::pool::{
    CachedFeatures, ConnectionService, FeatureService, OutboundOAuthManager, ServerKey,
    ServerManager, TokenService,
};
use mcpmux_gateway::services::{
    meta_tools, ApprovalBroker, ApprovalDecision, ApprovalPayload, ApprovalPublisher,
    FeatureSetResolverService, MetaToolRegistry, PrefixCacheService, SessionRootsRegistry,
    META_TOOL_APPROVAL_EVENT,
};
use mcpmux_storage::{
    generate_master_key, Database, FieldEncryptor, InboundClientRepository,
    SqliteFeatureSetRepository, SqliteInboundMcpClientRepository, SqliteInstalledServerRepository,
    SqliteServerFeatureRepository, SqliteSpaceRepository, SqliteWorkspaceBindingRepository,
};
use serde_json::{json, Value};
use tests::mocks::{MockCredentialRepository, MockOutboundOAuthRepository};
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

struct Fixture {
    registry: Arc<MetaToolRegistry>,
    broker: Arc<ApprovalBroker>,
    #[allow(dead_code)]
    client_repo: Arc<dyn InboundMcpClientRepository>,
    feature_set_repo: Arc<dyn FeatureSetRepository>,
    server_feature_repo: Arc<dyn ServerFeatureRepository>,
    binding_repo: Arc<dyn WorkspaceBindingRepository>,
    installed_server_repo: Arc<dyn InstalledServerRepository>,
    session_roots: Arc<SessionRootsRegistry>,
    feature_service: Arc<FeatureService>,
    space_id: Uuid,
    /// Opaque client identity (UUID-as-string here; in production for DCR
    /// clients this can be a `client_metadata` URL).
    client_id: String,
    session_id: String,
    fs_android_id: Uuid,
    github_tool_id: Uuid,
    event_rx: broadcast::Receiver<DomainEvent>,
    server_manager: Arc<ServerManager>,
}

fn test_encryptor() -> Arc<FieldEncryptor> {
    let key = generate_master_key().expect("generate key");
    Arc::new(FieldEncryptor::new(&key).expect("create encryptor"))
}

fn test_log_manager() -> Arc<ServerLogManager> {
    let base_dir = std::env::temp_dir().join(format!("mcpmux-meta-tools-logs-{}", Uuid::new_v4()));
    Arc::new(ServerLogManager::new(LogConfig {
        base_dir,
        max_file_size: 1024 * 1024,
        max_files: 5,
        compress: false,
    }))
}

fn test_server_manager(
    event_tx: broadcast::Sender<DomainEvent>,
    feature_service: Arc<FeatureService>,
    prefix_cache: Arc<PrefixCacheService>,
) -> Arc<ServerManager> {
    let credential_repo = Arc::new(MockCredentialRepository::new());
    let oauth_repo = Arc::new(MockOutboundOAuthRepository::new());
    let token_service = Arc::new(TokenService::new(
        credential_repo.clone(),
        oauth_repo.clone(),
    ));
    let oauth_manager = Arc::new(OutboundOAuthManager::new());
    let connection_service = Arc::new(ConnectionService::new(
        token_service,
        oauth_manager,
        credential_repo,
        oauth_repo,
        prefix_cache.clone(),
    ));
    Arc::new(ServerManager::new(
        event_tx,
        feature_service,
        connection_service,
        prefix_cache,
    ))
}

fn stdio_definition_with_required_input(server_id: &str, input_id: &str) -> ServerDefinition {
    ServerDefinition {
        id: server_id.to_string(),
        name: server_id.to_string(),
        description: None,
        alias: None,
        auth: None,
        icon: None,
        transport: TransportConfig::Stdio {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "pkg".to_string()],
            env: Default::default(),
            metadata: TransportMetadata {
                inputs: vec![InputDefinition {
                    id: input_id.to_string(),
                    label: input_id.to_string(),
                    r#type: "text".to_string(),
                    required: true,
                    secret: true,
                    description: None,
                    default: None,
                    placeholder: None,
                    obtain_url: None,
                    obtain_instructions: None,
                }],
            },
        },
        categories: vec![],
        publisher: None,
        source: ServerSource::Bundled,
        badges: vec![],
        hosting_type: Default::default(),
        license: None,
        license_url: None,
        installation: None,
        capabilities: None,
        sponsored: None,
        media: None,
        changelog_url: None,
    }
}

async fn seed_diagnose_servers(f: &Fixture) {
    let space_id = f.space_id.to_string();

    let github_def = stdio_definition_with_required_input("github", "github_token");
    let github = InstalledServer::new(&space_id, "github")
        .with_definition(&github_def)
        .with_input("github_token", "secret");
    f.installed_server_repo.install(&github).await.unwrap();
    f.server_manager
        .set_connected(
            &ServerKey::new(f.space_id, "github"),
            CachedFeatures::default(),
        )
        .await;

    let firebase_def = stdio_definition_with_required_input("firebase", "api_key");
    let firebase = InstalledServer::new(&space_id, "firebase")
        .with_definition(&firebase_def)
        .with_input("api_key", "key");
    f.installed_server_repo.install(&firebase).await.unwrap();
    f.server_manager
        .set_error(
            &ServerKey::new(f.space_id, "firebase"),
            "Connection refused".to_string(),
        )
        .await;
}

impl Fixture {
    async fn new() -> Self {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));

        let space_repo: Arc<dyn SpaceRepository> = Arc::new(SqliteSpaceRepository::new(db.clone()));
        let feature_set_repo: Arc<dyn FeatureSetRepository> =
            Arc::new(SqliteFeatureSetRepository::new(db.clone()));
        let client_repo: Arc<dyn InboundMcpClientRepository> =
            Arc::new(SqliteInboundMcpClientRepository::new(db.clone()));
        let binding_repo: Arc<dyn WorkspaceBindingRepository> =
            Arc::new(SqliteWorkspaceBindingRepository::new(db.clone()));
        let server_feature_repo: Arc<dyn ServerFeatureRepository> =
            Arc::new(SqliteServerFeatureRepository::new(db.clone()));
        let installed_server_repo: Arc<dyn InstalledServerRepository> = Arc::new(
            SqliteInstalledServerRepository::new(db.clone(), test_encryptor()),
        );

        let default_space = space_repo.get_default().await.unwrap().unwrap();
        let space_id = default_space.id;

        // Two FSes we'll flip between in the tests.
        let fs_android = FeatureSet::new_custom("Android Dev", space_id.to_string());
        let fs_full = FeatureSet::new_custom("Full Access", space_id.to_string());
        feature_set_repo.create(&fs_android).await.unwrap();
        feature_set_repo.create(&fs_full).await.unwrap();
        let fs_android_id = Uuid::parse_str(&fs_android.id).unwrap();
        let fs_full_id = Uuid::parse_str(&fs_full.id).unwrap();

        // Seed two tools in server_features for the tools listing test.
        //
        // Tool names are stored bare; qualified_name() prepends the server
        // prefix, so e.g. ("github", "create_issue") → "github_create_issue".
        let mut feature1 = ServerFeature::tool(space_id, "github", "create_issue");
        feature1.display_name = Some("GitHub".into());
        feature1.description = Some("Create an issue".into());
        let mut feature2 = ServerFeature::tool(space_id, "firebase", "deploy");
        feature2.display_name = Some("Firebase".into());
        feature2.description = Some("Deploy to Firebase".into());
        server_feature_repo.upsert(&feature1).await.unwrap();
        server_feature_repo.upsert(&feature2).await.unwrap();
        let github_tool_id = feature1.id;

        // The space's auto-seeded Default FS is the resolver's baseline
        // when no binding matches — no "set active FS" step needed.
        let _ = fs_full_id;

        // Create test client — routing is per-session-root now, not per-client.
        let client = Client::new("TestClient", "test-type");
        let client_id = client.id.to_string();
        client_repo.create(&client).await.unwrap();

        let session_roots = SessionRootsRegistry::new();
        let session_id = "sess-meta".to_string();

        let inbound_client_repo = Arc::new(InboundClientRepository::new(db.clone()));
        let resolver = Arc::new(FeatureSetResolverService::new(
            space_repo.clone(),
            binding_repo.clone(),
            session_roots.clone(),
            inbound_client_repo.clone(),
        ));

        let prefix_cache = Arc::new(PrefixCacheService::new());
        let feature_service = Arc::new(FeatureService::new(
            server_feature_repo.clone(),
            feature_set_repo.clone(),
            prefix_cache.clone(),
        ));

        let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(500)));
        let (tx, event_rx) = broadcast::channel::<DomainEvent>(32);
        let log_manager = test_log_manager();
        let server_manager =
            test_server_manager(tx.clone(), feature_service.clone(), prefix_cache.clone());

        let registry = meta_tools::build_default_registry(
            client_repo.clone(),
            space_repo.clone(),
            feature_set_repo.clone(),
            binding_repo.clone(),
            server_feature_repo.clone(),
            installed_server_repo.clone(),
            resolver,
            feature_service.clone(),
            None,
            None,
            session_roots.clone(),
            broker.clone(),
            tx,
            None,
            server_manager.clone(),
            log_manager,
            std::env::temp_dir().join(format!("mcpmux-meta-tools-{}", Uuid::new_v4())),
        );

        Self {
            registry,
            broker,
            client_repo,
            feature_set_repo,
            server_feature_repo,
            binding_repo,
            installed_server_repo,
            session_roots,
            feature_service,
            space_id,
            client_id,
            session_id,
            fs_android_id,
            github_tool_id,
            event_rx,
            server_manager,
        }
    }

    /// Attach a publisher that always auto-approves with the given decision.
    fn attach_auto_publisher(&self, decision: ApprovalDecision) {
        let broker = self.broker.clone();
        let publisher: ApprovalPublisher = Arc::new(move |req| {
            let b = broker.clone();
            async move {
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    b.respond(
                        &req.request_id,
                        &req.client_id,
                        &req.payload.tool_name,
                        decision,
                    );
                });
                true
            }
            .boxed()
        });
        // set_publisher is async; drive it synchronously via a current-runtime block_on
        // is unavailable here, so we spawn and detach — publisher is in place before
        // any request is made because tokio::test is single-threaded by default.
        let b = self.broker.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                b.set_publisher(publisher).await;
            });
        });
    }

    /// Attach a publisher that fans approval requests into the admin SSE bus.
    fn attach_sse_publisher(&self, ui_bus: Arc<mcpmux_gateway::admin::AdminUiEventBus>) {
        let publisher: ApprovalPublisher = Arc::new(move |req| {
            let bus = ui_bus.clone();
            async move {
                if let Ok(payload) = serde_json::to_value(&req) {
                    bus.publish(META_TOOL_APPROVAL_EVENT, payload);
                }
                true
            }
            .boxed()
        });
        let b = self.broker.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                b.set_publisher(publisher).await;
            });
        });
    }

    fn result_json(result: &rmcp::model::CallToolResult) -> Value {
        // CallToolResult's Content is opaque; round-trip through JSON and
        // pluck out the first text payload.
        let raw = serde_json::to_value(result).unwrap();
        raw.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("text"))
            .and_then(|t| t.as_str())
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .unwrap_or(raw)
    }

    fn is_error(result: &rmcp::model::CallToolResult) -> bool {
        result.is_error.unwrap_or(false)
    }

    /// Call the registry and normalize errors to `CallToolResult::error` the
    /// same way [`McpMuxGatewayHandler::call_tool`] does, so tests can assert
    /// the wire behaviour uniformly.
    async fn call_tool_as_handler_would(
        &self,
        name: &str,
        args: Value,
    ) -> rmcp::model::CallToolResult {
        match self
            .registry
            .call(name, &self.client_id, Some(&self.session_id), args)
            .await
        {
            Ok(r) => r,
            Err(e) => e.into_call_tool_result(),
        }
    }
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn list_all_tools_not_in_agent_registry() {
    let f = Fixture::new().await;
    let names: Vec<_> = f
        .registry
        .list_as_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert!(
        !names.iter().any(|n| n == "mcpmux_list_all_tools"),
        "catalog firehose removed from agent surface: {names:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn list_feature_sets_returns_space_contents() {
    let f = Fixture::new().await;
    let result = f
        .registry
        .call(
            "mcpmux_list_feature_sets",
            &f.client_id,
            Some(&f.session_id),
            json!({}),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    let sets = body.get("feature_sets").unwrap().as_array().unwrap();
    // Seed created 2 custom FSes + the auto-seeded Default.
    assert_eq!(sets.len(), 3, "Default + 2 custom expected");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_feature_sets_marks_bound_vs_inactive() {
    let f = Fixture::new().await;
    let fs_id = bind_github_only_to_session_root(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_list_feature_sets",
            &f.client_id,
            Some(&f.session_id),
            json!({}),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    let sets = body.get("feature_sets").unwrap().as_array().unwrap();
    let github_fs = sets
        .iter()
        .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(fs_id.as_str()))
        .unwrap();
    assert_eq!(github_fs.get("status"), Some(&json!("active")));
    let android = sets
        .iter()
        .find(|s| s.get("name").and_then(|v| v.as_str()) == Some("Android Dev"))
        .unwrap();
    assert_eq!(android.get("status"), Some(&json!("inactive")));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_default_empty_suggests_widen_or_bind() {
    let f = Fixture::new().await;
    let _fs_id = github_only_fs(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": "issue" }),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("total"), Some(&json!(0)));
    assert_eq!(body.get("scope"), Some(&json!("active_only")));
    let hint = body.get("hint").and_then(|v| v.as_str()).unwrap_or("");
    assert!(hint.contains("include_inactive"));
    assert!(hint.contains("mcpmux_list_feature_sets"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_first_meta_call_resolves_bound_workspace() {
    let f = Fixture::new().await;
    let root = "/tmp/mcpmux-root-race-first-search";
    let fs_id = github_only_fs(&f).await;

    f.session_roots.set_roots_capable(&f.session_id, true);
    let binding = WorkspaceBinding::new(normalize_workspace_root(root), f.space_id, fs_id.clone());
    f.binding_repo.create(&binding).await.unwrap();
    // Outcome of ensure_roots_probed before meta-tool dispatch (no prior tools/list).
    f.session_roots.set(&f.session_id, [root]);

    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": "issue" }),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("scope"), Some(&json!("active_only")));
    assert!(
        body.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1,
        "bound workspace should surface active tools on first search: {body}"
    );
    let tools = body.get("tools").unwrap().as_array().unwrap();
    assert!(
        tools
            .iter()
            .any(|t| t.get("qualified_name") == Some(&json!("github_create_issue"))),
        "expected bound github tool in first search_tools result: {tools:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_pending_roots_returns_empty_active_only() {
    let f = Fixture::new().await;
    let root = "/tmp/mcpmux-root-race-pending";
    let fs_id = github_only_fs(&f).await;

    f.session_roots.set_roots_capable(&f.session_id, true);
    let binding = WorkspaceBinding::new(normalize_workspace_root(root), f.space_id, fs_id);
    f.binding_repo.create(&binding).await.unwrap();
    // Binding exists but roots not probed yet — PendingRoots → empty grants.

    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": "issue" }),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("total"), Some(&json!(0)));
    assert_eq!(body.get("scope"), Some(&json!("active_only")));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_include_inactive_surfaces_bindable_github() {
    let f = Fixture::new().await;
    let fs_id = github_only_fs(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": "issue", "include_inactive": true }),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("scope"), Some(&json!("active_and_inactive")));
    assert!(body.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
    let tool = body
        .get("tools")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t.get("qualified_name") == Some(&json!("github_create_issue")))
        .expect("inactive github tool in results");
    assert_eq!(tool.get("status"), Some(&json!("inactive")));
    assert_eq!(tool.get("bindable_feature_set_id"), Some(&json!(fs_id)));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_include_inactive_no_bundle_suggests_author_in_mux() {
    let f = Fixture::new().await;

    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": "deploy", "include_inactive": true }),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("total"), Some(&json!(0)));
    let hint = body.get("hint").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        hint.contains("create a bundle") || hint.contains("Feature Sets"),
        "expected author-bundle hint, got: {hint}"
    );
}

/// Seed a PostHog-scale bundle (`tool_count` tools on one server) for inactive-scan perf tests.
async fn seed_large_inactive_bundle(f: &Fixture, server_id: &str, tool_count: usize) -> String {
    let space_id = f.space_id.to_string();
    let features: Vec<ServerFeature> = (0..tool_count)
        .map(|i| ServerFeature::tool(&space_id, server_id, format!("capture_event_{i}")))
        .collect();
    f.server_feature_repo.upsert_many(&features).await.unwrap();

    let mut fs = FeatureSet::new_custom("PostHog clone", space_id.clone());
    for feature in &features {
        fs.members.push(FeatureSetMember {
            id: Uuid::new_v4().to_string(),
            feature_set_id: fs.id.clone(),
            member_type: MemberType::Feature,
            member_id: feature.id.to_string(),
            mode: MemberMode::Include,
            surfaced: false,
        });
    }
    let fs_id = fs.id.clone();
    f.feature_set_repo.create(&fs).await.unwrap();
    fs_id
}

#[tokio::test(flavor = "multi_thread")]
async fn search_include_inactive_large_bundle_completes_under_two_seconds() {
    let f = Fixture::new().await;
    let tool_count = 450;
    seed_large_inactive_bundle(&f, "posthog", tool_count).await;

    let start = std::time::Instant::now();
    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "include_inactive": true, "limit": 100 }),
        )
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert!(!Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("scope"), Some(&json!("active_and_inactive")));
    assert!(
        body.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= tool_count as u64,
        "expected at least {tool_count} inactive tools"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "inactive scan took {elapsed:?}, expected < 2s"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn search_include_inactive_large_set_suggests_server_id_filter() {
    let f = Fixture::new().await;
    seed_large_inactive_bundle(&f, "analytics", 51).await;

    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "include_inactive": true, "limit": 10 }),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    let hint = body.get("hint").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        hint.contains("server_id"),
        "expected server_id filter hint, got: {hint}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_second_call_hits_active_index_cache() {
    let f = Fixture::new().await;
    bind_github_only_to_session_root(&f).await;

    let args = json!({ "query": "issue" });
    let result1 = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            args.clone(),
        )
        .await
        .unwrap();
    let body1 = Fixture::result_json(&result1);
    assert!(
        body1.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1,
        "first search should return active tools: {body1}"
    );
    assert!(f.registry.search_cache_contains(&f.session_id));

    f.server_feature_repo
        .delete(&f.github_tool_id)
        .await
        .unwrap();

    let result2 = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            args,
        )
        .await
        .unwrap();
    let body2 = Fixture::result_json(&result2);
    assert!(
        body2.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1,
        "cache hit should return cached tools despite DB deletion: {body2}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_cache_evicted_on_workspace_binding_changed() {
    let f = Fixture::new().await;
    let root = "/tmp/mcpmux-list-servers-test";
    bind_github_only_to_session_root(&f).await;

    f.registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": "issue" }),
        )
        .await
        .unwrap();
    assert!(f.registry.search_cache_contains(&f.session_id));

    f.session_roots.evict_search_cache_for_workspace_root(root);

    assert!(!f.registry.search_cache_contains(&f.session_id));
    assert!(!f.registry.embedding_cache_contains(&f.session_id));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_cache_evicted_on_session_disconnect() {
    let f = Fixture::new().await;
    bind_github_only_to_session_root(&f).await;

    f.registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": "issue" }),
        )
        .await
        .unwrap();
    assert!(f.registry.search_cache_contains(&f.session_id));

    f.session_roots.remove(&f.session_id);

    assert!(!f.registry.search_cache_contains(&f.session_id));
    assert!(!f.registry.embedding_cache_contains(&f.session_id));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_ranking_lexical_when_model_absent() {
    let f = Fixture::new().await;
    bind_github_only_to_session_root(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": "create issue" }),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    assert_eq!(
        body.get("ranking").and_then(|v| v.as_str()),
        Some("lexical"),
        "without a ready embedding model search must label itself lexical: {body}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_embedding_cache_hit_on_second_query() {
    let f = Fixture::new().await;
    let fs_id = bind_github_only_to_session_root(&f).await;
    let resolved_ids = vec![fs_id.to_string()];
    let fingerprint = meta_tools::feature_set_ids_fingerprint(&resolved_ids);

    f.registry.context().embedding_cache.insert(
        f.session_id.clone(),
        (
            fingerprint,
            vec![mcpmux_gateway::services::DocEmbedding {
                qualified_name: "github_create_issue".to_string(),
                vector: vec![1.0, 0.0, 0.0],
            }],
        ),
    );

    let args = json!({ "query": "issue" });
    f.registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            args.clone(),
        )
        .await
        .unwrap();
    assert!(f.registry.embedding_cache_contains(&f.session_id));

    f.registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            args,
        )
        .await
        .unwrap();
    let entry = f
        .registry
        .context()
        .embedding_cache
        .get(&f.session_id)
        .expect("embedding cache entry");
    assert_eq!(entry.0, fingerprint);
    assert_eq!(entry.1.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_embedding_cache_evicted_on_workspace_binding_changed() {
    let f = Fixture::new().await;
    let root = "/tmp/mcpmux-list-servers-test";
    bind_github_only_to_session_root(&f).await;

    f.registry
        .context()
        .embedding_cache
        .insert(f.session_id.clone(), (0, vec![]));
    assert!(f.registry.embedding_cache_contains(&f.session_id));

    f.session_roots.evict_search_cache_for_workspace_root(root);

    assert!(!f.registry.embedding_cache_contains(&f.session_id));
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_does_not_promote_tools_into_advertised_list() {
    let f = Fixture::new().await;
    let meta_count = f.registry.list_as_tools().len();
    let fs_id = bind_github_only_to_session_root(&f).await;

    let advertised = f
        .feature_service
        .get_advertised_tools_for_grants(&f.space_id.to_string(), &[fs_id])
        .await
        .unwrap();
    assert!(
        advertised.is_empty(),
        "binding must not surface backend tools into tools/list"
    );
    assert_eq!(f.registry.list_as_tools().len(), meta_count);
}

fn server_status(body: &Value, server_id: &str) -> String {
    body.get("servers")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(server_id))
        .unwrap()
        .get("status")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

async fn github_only_fs(f: &Fixture) -> String {
    let mut fs = FeatureSet::new_custom("GitHub only", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: f.github_tool_id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    let id = fs.id.clone();
    f.feature_set_repo.create(&fs).await.unwrap();
    id
}

async fn bind_github_only_to_session_root(f: &Fixture) -> String {
    use mcpmux_core::WorkspaceBinding;

    let fs_id = github_only_fs(f).await;
    let root = "/tmp/mcpmux-list-servers-test";
    f.session_roots.set_roots_capable(&f.session_id, true);
    f.session_roots.set(&f.session_id, [root]);
    let binding = WorkspaceBinding::new(normalize_workspace_root(root), f.space_id, fs_id.clone());
    f.binding_repo.create(&binding).await.unwrap();
    fs_id
}

#[tokio::test(flavor = "multi_thread")]
async fn list_servers_marks_unbound_servers_inactive() {
    let f = Fixture::new().await;
    let result = f
        .registry
        .call(
            "mcpmux_list_servers",
            &f.client_id,
            Some(&f.session_id),
            json!({}),
        )
        .await
        .unwrap();
    assert!(!Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    let servers = body.get("servers").unwrap().as_array().unwrap();
    assert_eq!(servers.len(), 2);
    assert_eq!(server_status(&body, "github"), "inactive");
    assert_eq!(server_status(&body, "firebase"), "inactive");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_servers_inactive_includes_bindable_feature_set_ids() {
    let f = Fixture::new().await;
    let fs_id = github_only_fs(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_list_servers",
            &f.client_id,
            Some(&f.session_id),
            json!({}),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    let github = body
        .get("servers")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s.get("id").and_then(|v| v.as_str()) == Some("github"))
        .unwrap();
    assert_eq!(github.get("status"), Some(&json!("inactive")));
    let bindable = github
        .get("bindable_feature_set_ids")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(bindable.iter().any(|v| v.as_str() == Some(fs_id.as_str())));
}

#[tokio::test(flavor = "multi_thread")]
async fn list_servers_shows_enabled_via_binding() {
    let f = Fixture::new().await;
    bind_github_only_to_session_root(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_list_servers",
            &f.client_id,
            Some(&f.session_id),
            json!({}),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    assert_eq!(server_status(&body, "github"), "enabled_via_binding");
    assert_eq!(server_status(&body, "firebase"), "inactive");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_servers_includes_cloned_from_for_clone_installs() {
    let f = Fixture::new().await;
    let space_id = f.space_id.to_string();

    let posthog = InstalledServer::new(&space_id, "posthog");
    f.installed_server_repo.install(&posthog).await.unwrap();
    let posthog_work = InstalledServer::new(&space_id, "posthog-work").with_cloned_from("posthog");
    f.installed_server_repo
        .install(&posthog_work)
        .await
        .unwrap();

    let mut clone_tool = ServerFeature::tool(f.space_id, "posthog-work", "capture");
    clone_tool.display_name = Some("PostHog (work)".into());
    f.registry
        .context()
        .server_feature_repo
        .upsert(&clone_tool)
        .await
        .unwrap();

    let result = f
        .registry
        .call(
            "mcpmux_list_servers",
            &f.client_id,
            Some(&f.session_id),
            json!({}),
        )
        .await
        .unwrap();
    let body = Fixture::result_json(&result);
    let clone_entry = body
        .get("servers")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s.get("id").and_then(|v| v.as_str()) == Some("posthog-work"))
        .expect("clone server in manifest");
    assert_eq!(
        clone_entry.get("cloned_from").and_then(|v| v.as_str()),
        Some("posthog")
    );

    let github_entry = body
        .get("servers")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s.get("id").and_then(|v| v.as_str()) == Some("github"))
        .expect("github in manifest");
    assert!(github_entry.get("cloned_from").is_none());
}

// ---------------------------------------------------------------------------
// Writes — gated by ApprovalBroker
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn write_without_publisher_returns_approval_required() {
    let f = Fixture::new().await;
    let input = if cfg!(windows) {
        "D:\\Projects\\Approval\\"
    } else {
        "/proj/approval"
    };
    f.session_roots.set(&f.session_id, [input]);
    let result = f
        .call_tool_as_handler_would(
            "mcpmux_bind_current_workspace",
            json!({ "feature_set_id": f.fs_android_id.to_string() }),
        )
        .await;
    assert!(Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    assert_eq!(
        body.get("error").unwrap().as_str().unwrap(),
        "approval_required"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn write_rejected_on_deny_leaves_state_unchanged() {
    let f = Fixture::new().await;
    f.attach_auto_publisher(ApprovalDecision::Deny);

    let before_bindings = f.binding_repo.list().await.unwrap().len();

    let input = if cfg!(windows) {
        "D:\\Projects\\Denied\\"
    } else {
        "/proj/denied"
    };
    f.session_roots.set(&f.session_id, [input]);
    let result = f
        .call_tool_as_handler_would(
            "mcpmux_bind_current_workspace",
            json!({ "feature_set_id": f.fs_android_id.to_string() }),
        )
        .await;
    assert!(Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    assert_eq!(
        body.get("error").unwrap().as_str().unwrap(),
        "approval_denied"
    );

    let after_bindings = f.binding_repo.list().await.unwrap().len();
    assert_eq!(after_bindings, before_bindings);
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_approval_surfaces_on_admin_sse_and_approve_writes_binding() {
    let f = Fixture::new().await;
    let ui_bus = Arc::new(mcpmux_gateway::admin::AdminUiEventBus::new());
    let mut sse_rx = ui_bus.subscribe();
    f.attach_sse_publisher(ui_bus);

    let input = if cfg!(windows) {
        "D:\\Projects\\WebAdmin\\"
    } else {
        "/proj/web-admin-bind"
    };
    f.session_roots.set(&f.session_id, [input]);

    let registry = f.registry.clone();
    let client_id = f.client_id.clone();
    let session_id = f.session_id.clone();
    let fs_id = f.fs_android_id.to_string();
    let broker = f.broker.clone();

    let bind_task = tokio::spawn(async move {
        registry
            .call(
                "mcpmux_bind_current_workspace",
                &client_id,
                Some(&session_id),
                json!({ "feature_set_id": fs_id }),
            )
            .await
    });

    let ui_event = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sse_rx.recv().await {
                Ok(ev) if ev.channel == META_TOOL_APPROVAL_EVENT => return ev.payload,
                Ok(_) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    panic!("SSE bus closed before approval request");
                }
            }
        }
    })
    .await
    .expect("approval request on admin SSE");

    let request_id = ui_event
        .get("request_id")
        .and_then(|v| v.as_str())
        .expect("request_id in SSE payload");
    broker.respond(
        request_id,
        &f.client_id,
        "mcpmux_bind_current_workspace",
        ApprovalDecision::AllowOnce,
    );

    let result = bind_task.await.expect("bind task").expect("bind call");
    assert!(!Fixture::is_error(&result));

    let bindings = f.binding_repo.list_for_space(&f.space_id).await.unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].workspace_root, normalize_workspace_root(input));
    assert_eq!(
        bindings[0].feature_set_ids,
        vec![f.fs_android_id.to_string()]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_deny_via_admin_sse_leaves_state_unchanged() {
    let f = Fixture::new().await;
    let ui_bus = Arc::new(mcpmux_gateway::admin::AdminUiEventBus::new());
    let mut sse_rx = ui_bus.subscribe();
    f.attach_sse_publisher(ui_bus);

    let before_bindings = f.binding_repo.list().await.unwrap().len();
    let input = if cfg!(windows) {
        "D:\\Projects\\WebDenied\\"
    } else {
        "/proj/web-admin-deny"
    };
    f.session_roots.set(&f.session_id, [input]);

    let registry = f.registry.clone();
    let client_id = f.client_id.clone();
    let session_id = f.session_id.clone();
    let fs_id = f.fs_android_id.to_string();
    let broker = f.broker.clone();

    let bind_task = tokio::spawn(async move {
        registry
            .call(
                "mcpmux_bind_current_workspace",
                &client_id,
                Some(&session_id),
                json!({ "feature_set_id": fs_id }),
            )
            .await
    });

    let ui_event = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sse_rx.recv().await {
                Ok(ev) if ev.channel == META_TOOL_APPROVAL_EVENT => return ev.payload,
                Ok(_) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    panic!("SSE bus closed before approval request");
                }
            }
        }
    })
    .await
    .expect("approval request on admin SSE");

    let request_id = ui_event
        .get("request_id")
        .and_then(|v| v.as_str())
        .expect("request_id in SSE payload");
    broker.respond(
        request_id,
        &f.client_id,
        "mcpmux_bind_current_workspace",
        ApprovalDecision::Deny,
    );

    let result = match bind_task.await.expect("bind task") {
        Ok(r) => r,
        Err(e) => e.into_call_tool_result(),
    };
    assert!(Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    assert_eq!(
        body.get("error").unwrap().as_str().unwrap(),
        "approval_denied"
    );

    let after_bindings = f.binding_repo.list().await.unwrap().len();
    assert_eq!(after_bindings, before_bindings);
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_current_workspace_fails_when_no_roots_reported() {
    let f = Fixture::new().await;
    f.attach_auto_publisher(ApprovalDecision::AllowOnce);
    // NOTE: session_roots intentionally NOT populated.

    let result = f
        .call_tool_as_handler_would(
            "mcpmux_bind_current_workspace",
            json!({ "feature_set_id": f.fs_android_id.to_string() }),
        )
        .await;
    assert!(Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    assert_eq!(
        body.get("error").unwrap().as_str().unwrap(),
        "invalid_argument"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_current_workspace_creates_binding_with_normalized_root() {
    let f = Fixture::new().await;
    f.attach_auto_publisher(ApprovalDecision::AllowOnce);
    let input = if cfg!(windows) {
        "D:\\Projects\\Android\\MyApp\\"
    } else {
        "/home/me/projects/android/myapp/"
    };
    f.session_roots.set(&f.session_id, [input]);

    let result = f
        .registry
        .call(
            "mcpmux_bind_current_workspace",
            &f.client_id,
            Some(&f.session_id),
            json!({ "feature_set_id": f.fs_android_id.to_string() }),
        )
        .await
        .unwrap();
    assert!(!Fixture::is_error(&result));

    let bindings = f.binding_repo.list_for_space(&f.space_id).await.unwrap();
    assert_eq!(bindings.len(), 1);
    let stored = &bindings[0].workspace_root;
    // Drive-letter lowercased, trailing separator trimmed.
    assert_eq!(stored, &normalize_workspace_root(input));
    assert!(!stored.ends_with('/') && !stored.ends_with('\\'));
    // Binding points at the concrete FS we passed in.
    assert_eq!(bindings[0].space_id, f.space_id);
    assert_eq!(
        bindings[0].feature_set_ids,
        vec![f.fs_android_id.to_string()]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_current_workspace_layers_onto_existing_binding() {
    let f = Fixture::new().await;
    f.attach_auto_publisher(ApprovalDecision::AllowOnce);
    let input = if cfg!(windows) {
        "D:\\Projects\\Android\\MyApp\\"
    } else {
        "/home/me/projects/android/myapp/"
    };
    let normalized = normalize_workspace_root(input);
    f.session_roots.set(&f.session_id, [input]);

    let fs_full_id = {
        let sets = f
            .feature_set_repo
            .list_by_space(&f.space_id.to_string())
            .await
            .unwrap();
        let full = sets
            .iter()
            .find(|fs| fs.name == "Full Access")
            .expect("Full Access FS");
        Uuid::parse_str(&full.id).unwrap()
    };

    // Seed an existing binding (simulates Workspaces UI or prior bind).
    let starter =
        WorkspaceBinding::new(normalized.clone(), f.space_id, f.fs_android_id.to_string());
    f.binding_repo.create(&starter).await.unwrap();

    let result = f
        .registry
        .call(
            "mcpmux_bind_current_workspace",
            &f.client_id,
            Some(&f.session_id),
            json!({ "feature_set_id": fs_full_id.to_string() }),
        )
        .await
        .unwrap();
    assert!(!Fixture::is_error(&result));

    let bindings = f.binding_repo.list_for_space(&f.space_id).await.unwrap();
    assert_eq!(bindings.len(), 1, "must not insert a second binding row");
    assert_eq!(bindings[0].id, starter.id, "must reuse existing binding id");
    assert_eq!(bindings[0].workspace_root, normalized);
    assert_eq!(bindings[0].feature_set_ids.len(), 2);
    assert!(bindings[0]
        .feature_set_ids
        .contains(&f.fs_android_id.to_string()));
    assert!(bindings[0]
        .feature_set_ids
        .contains(&fs_full_id.to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_current_workspace_rebind_is_idempotent() {
    let f = Fixture::new().await;
    f.attach_auto_publisher(ApprovalDecision::AllowOnce);
    let input = if cfg!(windows) {
        "D:\\Projects\\Android\\Rebind\\"
    } else {
        "/home/me/projects/android/rebind/"
    };
    f.session_roots.set(&f.session_id, [input]);
    let fs_id = github_only_fs(&f).await;
    let args = json!({ "feature_set_id": fs_id });

    f.registry
        .call(
            "mcpmux_bind_current_workspace",
            &f.client_id,
            Some(&f.session_id),
            args.clone(),
        )
        .await
        .unwrap();

    let result = f
        .registry
        .call(
            "mcpmux_bind_current_workspace",
            &f.client_id,
            Some(&f.session_id),
            args,
        )
        .await
        .unwrap();
    assert!(!Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("already_bound"), Some(&json!(true)));

    let bindings = f.binding_repo.list_for_space(&f.space_id).await.unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].feature_set_ids, vec![fs_id]);
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_current_workspace_second_session_inherits_binding() {
    let f = Fixture::new().await;
    f.attach_auto_publisher(ApprovalDecision::AllowOnce);
    let input = if cfg!(windows) {
        "D:\\Projects\\Android\\Persist\\"
    } else {
        "/home/me/projects/android/persist/"
    };
    f.session_roots.set_roots_capable(&f.session_id, true);
    f.session_roots.set(&f.session_id, [input]);
    let fs_id = github_only_fs(&f).await;

    f.registry
        .call(
            "mcpmux_bind_current_workspace",
            &f.client_id,
            Some(&f.session_id),
            json!({ "feature_set_id": fs_id }),
        )
        .await
        .unwrap();

    let new_session = "sess-bind-inherit";
    f.session_roots.set_roots_capable(new_session, true);
    f.session_roots.set(new_session, [input]);

    let resolved = f
        .registry
        .context()
        .resolver
        .resolve(Some(new_session), Some(&f.client_id))
        .await
        .unwrap();
    assert!(
        resolved.feature_set_ids.iter().any(|id| id == &fs_id),
        "second session should resolve the bound FeatureSet"
    );

    let tools = f
        .feature_service
        .get_tools_for_grants(&f.space_id.to_string(), &resolved.feature_set_ids)
        .await
        .unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].server_id, "github");
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_feature_set_argument_rejected() {
    let f = Fixture::new().await;
    let input = if cfg!(windows) {
        "D:\\Projects\\Invalid\\"
    } else {
        "/proj/invalid"
    };
    f.session_roots.set(&f.session_id, [input]);
    let result = f
        .call_tool_as_handler_would(
            "mcpmux_bind_current_workspace",
            json!({ "feature_set_id": "not-a-uuid" }),
        )
        .await;
    assert!(Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    assert_eq!(
        body.get("error").unwrap().as_str().unwrap(),
        "invalid_argument"
    );
}

// ---------------------------------------------------------------------------
// Registry list-as-tools shape
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn registry_advertises_every_default_tool_with_annotations() {
    let f = Fixture::new().await;
    let tools = f.registry.list_as_tools();
    let names: Vec<_> = tools.iter().map(|t| t.name.to_string()).collect();
    for expected in [
        "mcpmux_list_feature_sets",
        "mcpmux_list_servers",
        "mcpmux_bind_current_workspace",
        "mcpmux_diagnose_server",
    ] {
        assert!(names.iter().any(|n| n == expected), "missing {expected}");
    }
    for removed in [
        "mcpmux_enable_server",
        "mcpmux_disable_server",
        "mcpmux_create_feature_set",
        "mcpmux_describe_resolution",
        "mcpmux_describe_workspace",
    ] {
        assert!(
            !names.iter().any(|n| n == removed),
            "{removed} should be removed; got {names:?}"
        );
    }
    // bind_current_workspace is the sole write tool.
    let write_tools: Vec<_> = tools
        .iter()
        .filter(|t| {
            t.annotations
                .as_ref()
                .and_then(|a| a.destructive_hint)
                .unwrap_or(false)
        })
        .map(|t| t.name.to_string())
        .collect();
    assert_eq!(write_tools, vec!["mcpmux_bind_current_workspace"]);
}

// ---------------------------------------------------------------------------
// MetaToolInvoked audit emission + master switch
// ---------------------------------------------------------------------------

/// Build a bare registry (no fixture sugar) so tests can subscribe to the
/// event bus before the first call or flip the master-switch setting.
async fn bare_registry(
    settings_repo: Option<Arc<dyn mcpmux_core::AppSettingsRepository>>,
) -> (
    Arc<MetaToolRegistry>,
    String,
    broadcast::Sender<DomainEvent>,
    broadcast::Receiver<DomainEvent>,
) {
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let space_repo: Arc<dyn SpaceRepository> = Arc::new(SqliteSpaceRepository::new(db.clone()));
    let feature_set_repo: Arc<dyn FeatureSetRepository> =
        Arc::new(SqliteFeatureSetRepository::new(db.clone()));
    let client_repo: Arc<dyn InboundMcpClientRepository> =
        Arc::new(SqliteInboundMcpClientRepository::new(db.clone()));
    let binding_repo: Arc<dyn WorkspaceBindingRepository> =
        Arc::new(SqliteWorkspaceBindingRepository::new(db.clone()));
    let server_feature_repo: Arc<dyn ServerFeatureRepository> =
        Arc::new(SqliteServerFeatureRepository::new(db.clone()));
    let installed_server_repo: Arc<dyn InstalledServerRepository> = Arc::new(
        SqliteInstalledServerRepository::new(db.clone(), test_encryptor()),
    );

    let _space = space_repo.get_default().await.unwrap().unwrap();
    let client = Client::new("c", "t");
    let client_id = client.id.to_string();
    client_repo.create(&client).await.unwrap();

    let inbound_client_repo = Arc::new(InboundClientRepository::new(db.clone()));
    let resolver = Arc::new(FeatureSetResolverService::new(
        space_repo.clone(),
        binding_repo.clone(),
        SessionRootsRegistry::new(),
        inbound_client_repo.clone(),
    ));
    let prefix_cache = Arc::new(PrefixCacheService::new());
    let feature_service = Arc::new(FeatureService::new(
        server_feature_repo.clone(),
        feature_set_repo.clone(),
        prefix_cache.clone(),
    ));
    let (tx, rx) = broadcast::channel::<DomainEvent>(32);
    let log_manager = test_log_manager();
    let server_manager = test_server_manager(tx.clone(), feature_service.clone(), prefix_cache);
    let registry = meta_tools::build_default_registry(
        client_repo,
        space_repo,
        feature_set_repo,
        binding_repo,
        server_feature_repo,
        installed_server_repo,
        resolver,
        feature_service,
        None,
        None,
        SessionRootsRegistry::new(),
        Arc::new(ApprovalBroker::new()),
        tx.clone(),
        settings_repo,
        server_manager,
        log_manager,
        std::env::temp_dir().join(format!("mcpmux-bare-registry-{}", Uuid::new_v4())),
    );
    (registry, client_id, tx, rx)
}

#[tokio::test(flavor = "multi_thread")]
async fn read_tool_emits_meta_tool_invoked_with_decision_read() {
    let (registry, client_id, _tx, mut rx) = bare_registry(None).await;

    registry
        .call("mcpmux_list_servers", &client_id, Some("s"), json!({}))
        .await
        .unwrap();

    let evt = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("receive within 200ms")
        .expect("event");
    match evt {
        DomainEvent::MetaToolInvoked {
            tool_name,
            decision,
            ..
        } => {
            assert_eq!(tool_name, "mcpmux_list_servers");
            assert_eq!(decision, "read");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn denied_write_emits_meta_tool_invoked_with_decision_deny() {
    let (registry, client_id, _tx, mut rx) = bare_registry(None).await;

    // No publisher → write fails with ApprovalRequiredNoDesktop, which the
    // registry's central audit-logger records as `approval_required`.
    let _ = registry
        .call(
            "mcpmux_bind_current_workspace",
            &client_id,
            Some("s"),
            json!({ "feature_set_id": Uuid::new_v4().to_string() }),
        )
        .await;
    let evt = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("receive within 200ms")
        .expect("event");
    match evt {
        DomainEvent::MetaToolInvoked {
            decision,
            tool_name,
            ..
        } => {
            assert_eq!(tool_name, "mcpmux_bind_current_workspace");
            // bind_current_workspace bails on "invalid_args" (missing reported
            // roots) before it reaches the approval broker — the audit
            // logger records the bail-out reason, not approval_required.
            assert_eq!(decision, "invalid_args");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn master_switch_toggles_registry_visibility() {
    use mcpmux_storage::SqliteAppSettingsRepository;

    // Same DB so the settings repo and the registry see one another.
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let settings_repo: Arc<dyn mcpmux_core::AppSettingsRepository> =
        Arc::new(SqliteAppSettingsRepository::new(db.clone()));
    settings_repo
        .set("gateway.meta_tools_enabled", "false")
        .await
        .unwrap();

    let space_repo: Arc<dyn SpaceRepository> = Arc::new(SqliteSpaceRepository::new(db.clone()));
    let feature_set_repo: Arc<dyn FeatureSetRepository> =
        Arc::new(SqliteFeatureSetRepository::new(db.clone()));
    let client_repo: Arc<dyn InboundMcpClientRepository> =
        Arc::new(SqliteInboundMcpClientRepository::new(db.clone()));
    let binding_repo: Arc<dyn WorkspaceBindingRepository> =
        Arc::new(SqliteWorkspaceBindingRepository::new(db.clone()));
    let server_feature_repo: Arc<dyn ServerFeatureRepository> =
        Arc::new(SqliteServerFeatureRepository::new(db.clone()));
    let installed_server_repo: Arc<dyn InstalledServerRepository> = Arc::new(
        SqliteInstalledServerRepository::new(db.clone(), test_encryptor()),
    );
    let inbound_client_repo = Arc::new(InboundClientRepository::new(db.clone()));
    let resolver = Arc::new(FeatureSetResolverService::new(
        space_repo.clone(),
        binding_repo.clone(),
        SessionRootsRegistry::new(),
        inbound_client_repo.clone(),
    ));
    let prefix_cache = Arc::new(PrefixCacheService::new());
    let feature_service = Arc::new(FeatureService::new(
        server_feature_repo.clone(),
        feature_set_repo.clone(),
        prefix_cache.clone(),
    ));
    let (tx, _) = broadcast::channel::<DomainEvent>(16);
    let log_manager = test_log_manager();
    let server_manager = test_server_manager(tx.clone(), feature_service.clone(), prefix_cache);
    let registry = meta_tools::build_default_registry(
        client_repo,
        space_repo,
        feature_set_repo,
        binding_repo,
        server_feature_repo,
        installed_server_repo,
        resolver,
        feature_service,
        None,
        None,
        SessionRootsRegistry::new(),
        Arc::new(ApprovalBroker::new()),
        tx,
        Some(settings_repo.clone()),
        server_manager,
        log_manager,
        std::env::temp_dir().join(format!("mcpmux-meta-switch-{}", Uuid::new_v4())),
    );

    assert!(!registry.is_enabled().await, "initially disabled");

    settings_repo
        .set("gateway.meta_tools_enabled", "true")
        .await
        .unwrap();
    assert!(registry.is_enabled().await, "flipped back on");

    // Missing key → default on (fresh install).
    settings_repo
        .delete("gateway.meta_tools_enabled")
        .await
        .unwrap();
    assert!(registry.is_enabled().await, "missing key defaults on");
}

// Silence unused-import warnings from helper imports that only some tests exercise.
#[allow(dead_code)]
fn _unused(_: ApprovalPayload) {}

// ---------------------------------------------------------------------------
// Diagnose server (mcpmux_diagnose_server)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_server_registered_in_tools_list() {
    let f = Fixture::new().await;
    let names: Vec<_> = f
        .registry
        .list_as_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert!(
        names.iter().any(|n| n == "mcpmux_diagnose_server"),
        "expected mcpmux_diagnose_server in {names:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_no_arg_returns_only_unhealthy_servers() {
    let f = Fixture::new().await;
    seed_diagnose_servers(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_diagnose_server",
            &f.client_id,
            Some(&f.session_id),
            json!({}),
        )
        .await
        .unwrap();
    assert!(!Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    let servers = body.get("servers").unwrap().as_array().unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(
        servers[0].get("server_id").unwrap().as_str().unwrap(),
        "firebase"
    );
    assert_eq!(servers[0].get("health").unwrap().as_str().unwrap(), "error");
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_explicit_server_id_returns_target_regardless_of_health() {
    let f = Fixture::new().await;
    seed_diagnose_servers(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_diagnose_server",
            &f.client_id,
            Some(&f.session_id),
            json!({ "server_id": "github" }),
        )
        .await
        .unwrap();
    assert!(!Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    let servers = body.get("servers").unwrap().as_array().unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(
        servers[0].get("server_id").unwrap().as_str().unwrap(),
        "github"
    );
    assert_eq!(
        servers[0].get("health").unwrap().as_str().unwrap(),
        "healthy"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_include_logs_false_omits_logs_block() {
    let f = Fixture::new().await;
    seed_diagnose_servers(&f).await;

    let result = f
        .registry
        .call(
            "mcpmux_diagnose_server",
            &f.client_id,
            Some(&f.session_id),
            json!({ "server_id": "firebase", "include_logs": false }),
        )
        .await
        .unwrap();
    assert!(!Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    let entry = &body.get("servers").unwrap().as_array().unwrap()[0];
    assert!(entry.get("logs").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_surfaces_missing_required_inputs() {
    let f = Fixture::new().await;
    let space_id = f.space_id.to_string();

    let def = stdio_definition_with_required_input("needs-setup", "github_token");
    let server = InstalledServer::new(&space_id, "needs-setup").with_definition(&def);
    f.installed_server_repo.install(&server).await.unwrap();

    let result = f
        .registry
        .call(
            "mcpmux_diagnose_server",
            &f.client_id,
            Some(&f.session_id),
            json!({ "server_id": "needs-setup" }),
        )
        .await
        .unwrap();
    assert!(!Fixture::is_error(&result));
    let body = Fixture::result_json(&result);
    let entry = &body.get("servers").unwrap().as_array().unwrap()[0];
    assert_eq!(
        entry.get("health").unwrap().as_str().unwrap(),
        "needs_setup"
    );
    let missing = entry
        .get("missing_required_inputs")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].as_str().unwrap(), "github_token");
}
