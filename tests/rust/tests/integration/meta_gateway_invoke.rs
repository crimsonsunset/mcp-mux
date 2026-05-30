//! Integration tests for meta-gateway invoke (search → schema → invoke).

use std::sync::Arc;
use std::time::Duration;

use mcpmux_core::{
    Client, DomainEvent, FeatureSet, FeatureSetMember, FeatureSetRepository,
    InboundMcpClientRepository, InstalledServerRepository, MemberMode, MemberType, ServerFeature,
    ServerFeatureRepository, SpaceRepository, WorkspaceBindingRepository,
};
use mcpmux_gateway::pool::{
    format_direct_call_redirect, format_direct_fetch_prompt_redirect, format_direct_read_redirect,
    FeatureService, ToolCallResult,
};
use mcpmux_gateway::services::meta_tools::invoke::{
    apply_invoke_result_filter, parse_invoke_filter, shape_json_value, InvokeResultFilter,
};
use mcpmux_gateway::services::{
    meta_tools, meta_tools::DisclosureBackend, ApprovalBroker, FeatureSetResolverService,
    InvokeToolBackend, MetaToolRegistry, PrefixCacheService, SessionRootsRegistry,
};
use mcpmux_storage::{
    generate_master_key, Database, FieldEncryptor, InboundClientRepository,
    SqliteFeatureSetRepository, SqliteInboundMcpClientRepository, SqliteInstalledServerRepository,
    SqliteServerFeatureRepository, SqliteSpaceRepository, SqliteWorkspaceBindingRepository,
};
use serde_json::{json, Value};
use tests::CannedDisclosureBackend;
use tests::CannedInvokeBackend;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

struct Fixture {
    registry: Arc<MetaToolRegistry>,
    feature_service: Arc<FeatureService>,
    prefix_cache: Arc<PrefixCacheService>,
    session_roots: Arc<SessionRootsRegistry>,
    inbound_client_repo: Arc<InboundClientRepository>,
    server_feature_repo: Arc<dyn ServerFeatureRepository>,
    feature_set_repo: Arc<dyn FeatureSetRepository>,
    space_id: Uuid,
    client_id: String,
    session_id: String,
}

fn test_encryptor() -> Arc<FieldEncryptor> {
    let key = generate_master_key().expect("generate key");
    Arc::new(FieldEncryptor::new(&key).expect("create encryptor"))
}

impl Fixture {
    async fn new() -> Self {
        Self::with_backends(None, None).await
    }

    async fn with_invoke_backend(invoke_backend: Option<Arc<dyn InvokeToolBackend>>) -> Self {
        Self::with_backends(invoke_backend, None).await
    }

    async fn with_backends(
        invoke_backend: Option<Arc<dyn InvokeToolBackend>>,
        disclosure_backend: Option<Arc<dyn DisclosureBackend>>,
    ) -> Self {
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

        let client = Client::new("InvokeTestClient", "test-type");
        let client_id = client.id.to_string();
        client_repo.create(&client).await.unwrap();

        let mut list_issues = ServerFeature::tool(space_id, "github", "list_issues");
        list_issues.description = Some("List issues in a repository".into());
        list_issues.raw_json = Some(json!({
            "name": "list_issues",
            "description": "List issues in a repository",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner": { "type": "string" },
                    "repo": { "type": "string" }
                },
                "required": ["owner", "repo"]
            }
        }));
        server_feature_repo.upsert(&list_issues).await.unwrap();

        let mut grant_all = FeatureSet::new_custom("Grant GitHub", space_id.to_string());
        grant_all.members.push(FeatureSetMember {
            id: Uuid::new_v4().to_string(),
            feature_set_id: grant_all.id.clone(),
            member_type: MemberType::Feature,
            member_id: list_issues.id.to_string(),
            mode: MemberMode::Include,
            surfaced: false,
        });
        feature_set_repo.create(&grant_all).await.unwrap();

        let session_roots = SessionRootsRegistry::new();
        let session_id = "sess-invoke".to_string();

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
        let (tx, _event_rx) = broadcast::channel::<DomainEvent>(32);
        let log_manager = Arc::new(mcpmux_core::ServerLogManager::new(mcpmux_core::LogConfig {
            base_dir: std::env::temp_dir().join(format!("mcpmux-invoke-logs-{}", Uuid::new_v4())),
            max_file_size: 1024 * 1024,
            max_files: 5,
            compress: false,
        }));
        let credential_repo = Arc::new(tests::mocks::MockCredentialRepository::new());
        let oauth_repo = Arc::new(tests::mocks::MockOutboundOAuthRepository::new());
        let token_service = Arc::new(mcpmux_gateway::pool::TokenService::new(
            credential_repo.clone(),
            oauth_repo.clone(),
        ));
        let oauth_manager = Arc::new(mcpmux_gateway::pool::OutboundOAuthManager::new());
        let connection_service = Arc::new(mcpmux_gateway::pool::ConnectionService::new(
            token_service,
            oauth_manager,
            credential_repo,
            oauth_repo,
            prefix_cache.clone(),
        ));
        let server_manager = Arc::new(mcpmux_gateway::pool::ServerManager::new(
            tx.clone(),
            feature_service.clone(),
            connection_service,
            prefix_cache.clone(),
        ));

        let registry = meta_tools::build_default_registry(
            client_repo,
            space_repo,
            feature_set_repo.clone(),
            binding_repo,
            server_feature_repo.clone(),
            installed_server_repo,
            resolver,
            feature_service.clone(),
            invoke_backend,
            disclosure_backend,
            session_roots.clone(),
            broker,
            tx,
            None,
            server_manager,
            log_manager,
            std::env::temp_dir().join(format!("mcpmux-meta-invoke-{}", Uuid::new_v4())),
        );

        Self {
            registry,
            feature_service,
            prefix_cache,
            session_roots,
            inbound_client_repo,
            server_feature_repo,
            feature_set_repo,
            space_id,
            client_id,
            session_id,
        }
    }

    /// Grant a FeatureSet to the fixture client (Tier-2 resolver path).
    async fn grant_feature_set(&self, feature_set_id: &str) {
        self.inbound_client_repo
            .grant_feature_set(&self.client_id, &self.space_id.to_string(), feature_set_id)
            .await
            .unwrap();
        self.session_roots
            .set_roots_capable(&self.session_id, false);
    }

    fn result_json(result: &rmcp::model::CallToolResult) -> Value {
        let raw = serde_json::to_value(result).unwrap();
        raw.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("text"))
            .and_then(|t| t.as_str())
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .unwrap_or(raw)
    }

    async fn call(&self, name: &str, args: Value) -> rmcp::model::CallToolResult {
        match self
            .registry
            .call(name, &self.client_id, Some(&self.session_id), args)
            .await
        {
            Ok(r) => r,
            Err(e) => e.into_call_tool_result(),
        }
    }
    async fn grant_github_feature_set(&self) -> String {
        let fs_id = self
            .feature_set_repo
            .list_by_space(&self.space_id.to_string())
            .await
            .unwrap()
            .into_iter()
            .find(|fs| fs.name == "Grant GitHub")
            .unwrap()
            .id;
        self.grant_feature_set(&fs_id).await;
        fs_id
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_tool_applies_filter_end_to_end() {
    let issues: Vec<Value> = (0..20)
        .map(|i| {
            json!({
                "id": i,
                "title": format!("issue-{i}"),
                "body": format!("body-{i}")
            })
        })
        .collect();
    let payload = json!({ "issues": issues });
    let backend_result = ToolCallResult {
        content: vec![json!({
            "type": "text",
            "text": payload.to_string(),
        })],
        structured_content: Some(payload),
        is_error: false,
    };
    let invoke_backend = CannedInvokeBackend::new()
        .with_response("github_list_issues", backend_result)
        .into_arc();

    let f = Fixture::with_invoke_backend(Some(invoke_backend)).await;
    f.grant_github_feature_set().await;

    let result = f
        .call(
            "mcpmux_invoke_tool",
            json!({
                "server_id": "github",
                "tool": "list_issues",
                "args": { "owner": "mcpmux", "repo": "mcp-mux" },
                "filter": {
                    "max_rows": 3,
                    "fields": ["id", "title"],
                    "format": "summary"
                }
            }),
        )
        .await;

    assert!(!result.is_error.unwrap_or(true));
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("returned"), Some(&json!(3)));
    assert_eq!(body.get("total"), Some(&json!(20)));
    assert_eq!(body.get("truncated"), Some(&json!(true)));
    let sample = body.get("issues").and_then(|v| v.as_array()).unwrap();
    assert_eq!(sample.len(), 3);
    assert_eq!(sample[0], json!({ "id": 0, "title": "issue-0" }));

    let structured = result
        .structured_content
        .expect("structured content shaped");
    assert_eq!(structured.get("returned"), Some(&json!(3)));
    let structured_sample = structured.get("issues").and_then(|v| v.as_array()).unwrap();
    assert_eq!(structured_sample.len(), 3);
    assert_eq!(structured_sample[0], json!({ "id": 0, "title": "issue-0" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn advertised_tools_empty_without_surfaced_members() {
    let f = Fixture::new().await;
    let fs_ids = vec![
        f.feature_set_repo
            .list_by_space(&f.space_id.to_string())
            .await
            .unwrap()
            .into_iter()
            .find(|fs| fs.name == "Grant GitHub")
            .unwrap()
            .id,
    ];

    let advertised = f
        .feature_service
        .get_advertised_tools_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();
    assert!(advertised.is_empty(), "no surfaced members by default");

    let invokable = f
        .feature_service
        .get_invokable_tools_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();
    assert_eq!(invokable.len(), 1);
    assert_eq!(invokable[0].feature_name, "list_issues");
}

#[tokio::test(flavor = "multi_thread")]
async fn github_read_path_enable_search_schema() {
    let f = Fixture::new().await;

    let servers = f.call("mcpmux_list_servers", json!({})).await;
    let body = Fixture::result_json(&servers);
    let github = body
        .get("servers")
        .and_then(|s| s.as_array())
        .and_then(|arr| arr.iter().find(|s| s.get("id") == Some(&json!("github"))))
        .expect("github server listed");
    assert_eq!(github.get("status"), Some(&json!("inactive")));

    f.grant_github_feature_set().await;

    let search = f
        .call(
            "mcpmux_search_tools",
            json!({
                "query": "list issues",
                "server_id": "github",
                "detail_level": "description"
            }),
        )
        .await;
    let search_body = Fixture::result_json(&search);
    let tools = search_body.get("tools").unwrap().as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0].get("qualified_name"),
        Some(&json!("github_list_issues"))
    );

    let schema = f
        .call(
            "mcpmux_get_tool_schema",
            json!({ "tools": "github_list_issues" }),
        )
        .await;
    let schema_body = Fixture::result_json(&schema);
    let schemas = schema_body.get("schemas").unwrap().as_array().unwrap();
    assert_eq!(schemas.len(), 1);
    assert!(schemas[0].get("input_schema").is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_denied_when_server_inactive() {
    let f = Fixture::new().await;
    let result = f
        .call(
            "mcpmux_invoke_tool",
            json!({
                "server_id": "github",
                "tool": "list_issues",
                "args": { "owner": "mcpmux", "repo": "mcp-mux" }
            }),
        )
        .await;
    assert!(result.is_error.unwrap_or(false));
    let body = Fixture::result_json(&result);
    let message = body.get("message").and_then(|m| m.as_str()).unwrap_or("");
    assert!(message.contains("inactive"));
    assert!(message.contains("mcpmux_bind_current_workspace"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_empty_when_server_inactive_until_widened() {
    let f = Fixture::new().await;
    let search = f
        .call(
            "mcpmux_search_tools",
            json!({ "query": "list", "server_id": "github" }),
        )
        .await;
    let body = Fixture::result_json(&search);
    assert_eq!(body.get("total"), Some(&json!(0)));
    assert!(body
        .get("hint")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .contains("include_inactive"));

    let widened = f
        .call(
            "mcpmux_search_tools",
            json!({
                "query": "list",
                "server_id": "github",
                "include_inactive": true
            }),
        )
        .await;
    let wide_body = Fixture::result_json(&widened);
    assert!(wide_body.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
    let tool = wide_body
        .get("tools")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap();
    assert_eq!(tool.get("status"), Some(&json!("inactive")));
    assert!(tool
        .get("bindable_feature_set_id")
        .and_then(|v| v.as_str())
        .is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn direct_backend_call_redirect_message() {
    let msg = format_direct_call_redirect("github_list_issues", "github", "list_issues");
    assert!(msg.contains("mcpmux_invoke_tool"));
    assert!(msg.contains("github"));
    assert!(msg.contains("list_issues"));
}

#[tokio::test(flavor = "multi_thread")]
async fn registry_lists_new_meta_tools() {
    let f = Fixture::new().await;
    let names: Vec<String> = f
        .registry
        .list_as_tools()
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    assert!(names.iter().any(|n| n == "mcpmux_search_tools"));
    assert!(names.iter().any(|n| n == "mcpmux_get_tool_schema"));
    assert!(names.iter().any(|n| n == "mcpmux_invoke_tool"));
    assert!(names.iter().any(|n| n == "mcpmux_search_resources"));
    assert!(names.iter().any(|n| n == "mcpmux_read_resource"));
    assert!(names.iter().any(|n| n == "mcpmux_search_prompts"));
    assert!(names.iter().any(|n| n == "mcpmux_fetch_prompt"));
    assert!(names.iter().any(|n| n == "mcpmux_diagnose_server"));
    assert!(
        !names.iter().any(|n| n == "mcpmux_list_all_tools"),
        "list_all_tools removed from agent registry"
    );
    assert_eq!(names.len(), 11);
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_input_schema_includes_filter() {
    let f = Fixture::new().await;
    let invoke = f
        .registry
        .list_as_tools()
        .into_iter()
        .find(|t| t.name.as_ref() == "mcpmux_invoke_tool")
        .expect("invoke tool registered");
    let schema = invoke.input_schema;
    assert!(schema
        .get("properties")
        .and_then(|p| p.get("filter"))
        .is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_result_no_filter_passes_through() {
    let items: Vec<Value> = (0..100).map(|i| json!({ "id": i })).collect();
    let payload = json!({ "items": items.clone() });

    let shaped = shape_json_value(payload, &InvokeResultFilter::default());

    assert_eq!(
        shaped
            .get("items")
            .and_then(|v| v.as_array())
            .unwrap()
            .len(),
        100
    );
    assert!(shaped.get("truncated").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_result_explicit_filter_limits_rows() {
    let items: Vec<Value> = (0..30)
        .map(|i| json!({ "id": i, "label": format!("row-{i}") }))
        .collect();
    let filter = parse_invoke_filter(Some(&json!({ "max_rows": 5, "fields": ["id"] }))).unwrap();

    let shaped = shape_json_value(Value::Array(items), &filter);

    assert_eq!(shaped.get("returned"), Some(&json!(5)));
    assert_eq!(shaped.get("total"), Some(&json!(30)));
    assert_eq!(shaped.get("truncated"), Some(&json!(true)));
    let sample = shaped.get("items").and_then(|v| v.as_array()).unwrap();
    assert_eq!(sample.len(), 5);
    assert_eq!(sample[0], json!({ "id": 0 }));
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_result_filter_shapes_text_content_blocks() {
    let rows: Vec<Value> = (0..80).map(|i| json!({ "n": i })).collect();
    let content = vec![json!({
        "type": "text",
        "text": json!({ "results": rows }).to_string(),
    })];
    let filter = parse_invoke_filter(Some(&json!({ "max_rows": 10 }))).unwrap();

    let (shaped_content, _) = apply_invoke_result_filter(content, None, &filter);
    let text = shaped_content[0]
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();

    assert_eq!(parsed.get("returned"), Some(&json!(10)));
    assert_eq!(parsed.get("total"), Some(&json!(80)));
    assert_eq!(parsed.get("truncated"), Some(&json!(true)));
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_result_explicit_max_bytes_plain_text() {
    let text = "x".repeat(200);
    let content = vec![json!({ "type": "text", "text": text })];
    let filter = parse_invoke_filter(Some(&json!({ "max_bytes": 80 }))).unwrap();

    let (shaped_content, _) = apply_invoke_result_filter(content, None, &filter);
    let parsed: Value = serde_json::from_str(
        shaped_content[0]
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap(),
    )
    .unwrap();

    assert_eq!(parsed.get("truncated"), Some(&json!(true)));
    assert_eq!(parsed.get("total"), Some(&json!(200)));
}

#[tokio::test(flavor = "multi_thread")]
async fn partial_feature_set_binding_limits_search_and_invoke() {
    let f = Fixture::new().await;

    let mut create_issue = ServerFeature::tool(f.space_id, "github", "create_issue");
    create_issue.description = Some("Create an issue".into());
    create_issue.raw_json = Some(json!({
        "name": "create_issue",
        "description": "Create an issue",
        "inputSchema": { "type": "object" }
    }));
    f.server_feature_repo.upsert(&create_issue).await.unwrap();

    let list_issues = f
        .server_feature_repo
        .list_for_space(&f.space_id.to_string())
        .await
        .unwrap()
        .into_iter()
        .find(|feat| feat.feature_name == "list_issues")
        .unwrap();

    let mut partial_fs = FeatureSet::new_custom("Partial GitHub", f.space_id.to_string());
    partial_fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: partial_fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: list_issues.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&partial_fs).await.unwrap();
    f.grant_feature_set(&partial_fs.id).await;
    let fs_ids = vec![partial_fs.id.clone()];
    let invokable = f
        .feature_service
        .get_invokable_tools_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();
    assert_eq!(invokable.len(), 1);
    assert_eq!(invokable[0].feature_name, "list_issues");

    let search = f
        .call(
            "mcpmux_search_tools",
            json!({ "query": "issue", "server_id": "github" }),
        )
        .await;
    let search_body = Fixture::result_json(&search);
    let tools = search_body.get("tools").unwrap().as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0].get("qualified_name"),
        Some(&json!("github_list_issues"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn surfaced_tool_appears_in_advertised_set() {
    let f = Fixture::new().await;

    let list_issues = f
        .server_feature_repo
        .list_for_space(&f.space_id.to_string())
        .await
        .unwrap()
        .into_iter()
        .find(|feat| feat.feature_name == "list_issues")
        .unwrap();

    let mut surfaced_fs = FeatureSet::new_custom("Surfaced GitHub", f.space_id.to_string());
    surfaced_fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: surfaced_fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: list_issues.id.to_string(),
        mode: MemberMode::Include,
        surfaced: true,
    });
    f.feature_set_repo.create(&surfaced_fs).await.unwrap();

    let fs_ids = vec![surfaced_fs.id.clone()];
    let advertised = f
        .feature_service
        .get_advertised_tools_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();

    assert_eq!(advertised.len(), 1);
    assert_eq!(advertised[0].feature_name, "list_issues");
    assert_eq!(advertised[0].qualified_name(), "github_list_issues");
}

#[tokio::test(flavor = "multi_thread")]
async fn direct_backend_call_gate_allows_surfaced_only() {
    let f = Fixture::new().await;

    let features = f
        .server_feature_repo
        .list_for_space(&f.space_id.to_string())
        .await
        .unwrap();
    let list_issues = features
        .iter()
        .find(|feat| feat.feature_name == "list_issues")
        .unwrap();

    let mut get_me = ServerFeature::tool(f.space_id, "github", "get_me");
    get_me.description = Some("Get authenticated GitHub user".into());
    f.server_feature_repo.upsert(&get_me).await.unwrap();

    let mut mixed_fs = FeatureSet::new_custom("Mixed Surfaced GitHub", f.space_id.to_string());
    mixed_fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: mixed_fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: list_issues.id.to_string(),
        mode: MemberMode::Include,
        surfaced: true,
    });
    mixed_fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: mixed_fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: get_me.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&mixed_fs).await.unwrap();
    f.grant_feature_set(&mixed_fs.id).await;
    let fs_ids = vec![mixed_fs.id.clone()];
    let invokable = f
        .feature_service
        .get_invokable_tools_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();
    let advertised = f
        .feature_service
        .get_advertised_tools_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();

    assert_eq!(invokable.len(), 2);
    assert_eq!(advertised.len(), 1);
    assert_eq!(advertised[0].qualified_name(), "github_list_issues");

    let is_surfaced = |qualified_name: &str| {
        advertised
            .iter()
            .any(|feature| feature.qualified_name() == qualified_name)
    };
    assert!(is_surfaced("github_list_issues"));
    assert!(!is_surfaced("github_get_me"));
}

#[tokio::test(flavor = "multi_thread")]
async fn get_tool_schema_accepts_string_array() {
    let f = Fixture::new().await;
    f.grant_github_feature_set().await;

    let result = f
        .call(
            "mcpmux_get_tool_schema",
            json!({ "tools": ["github_list_issues"] }),
        )
        .await;
    let body = Fixture::result_json(&result);
    let schemas = body.get("schemas").unwrap().as_array().unwrap();
    assert_eq!(schemas.len(), 1);
    assert_eq!(
        schemas[0].get("qualified_name"),
        Some(&json!("github_list_issues"))
    );
    assert!(body.get("missing").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn get_tool_schema_accepts_json_encoded_array_string() {
    let f = Fixture::new().await;
    f.grant_github_feature_set().await;

    let result = f
        .call(
            "mcpmux_get_tool_schema",
            json!({ "tools": "[\"github_list_issues\"]" }),
        )
        .await;
    let body = Fixture::result_json(&result);
    let schemas = body.get("schemas").unwrap().as_array().unwrap();
    assert_eq!(schemas.len(), 1);
    assert_eq!(
        schemas[0].get("qualified_name"),
        Some(&json!("github_list_issues"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn get_tool_schema_reports_missing_tools() {
    let f = Fixture::new().await;
    f.grant_github_feature_set().await;

    let result = f
        .call(
            "mcpmux_get_tool_schema",
            json!({ "tools": ["github_list_issues", "github_create_issue"] }),
        )
        .await;
    let body = Fixture::result_json(&result);
    let schemas = body.get("schemas").unwrap().as_array().unwrap();
    assert_eq!(schemas.len(), 1);
    let missing = body.get("missing").unwrap().as_array().unwrap();
    assert_eq!(missing, &[json!("github_create_issue")]);
    assert!(body.get("message").and_then(|m| m.as_str()).is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_max_bytes_truncates_json_array_without_max_rows() {
    let rows: Vec<Value> = (0..40)
        .map(|i| json!({ "id": i, "label": format!("row-{i}-padding-value") }))
        .collect();
    let payload = json!({ "items": rows });
    let backend_result = ToolCallResult {
        content: vec![json!({
            "type": "text",
            "text": payload.to_string(),
        })],
        structured_content: None,
        is_error: false,
    };
    let invoke_backend = CannedInvokeBackend::new()
        .with_response("github_list_issues", backend_result)
        .into_arc();

    let f = Fixture::with_invoke_backend(Some(invoke_backend)).await;
    f.grant_github_feature_set().await;

    let result = f
        .call(
            "mcpmux_invoke_tool",
            json!({
                "server_id": "github",
                "tool": "list_issues",
                "args": { "owner": "mcpmux", "repo": "mcp-mux" },
                "filter": { "max_bytes": 512 }
            }),
        )
        .await;

    assert!(!result.is_error.unwrap_or(true));
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("truncated"), Some(&json!(true)));
}

#[tokio::test(flavor = "multi_thread")]
async fn get_tool_schema_reports_empty_string_in_missing() {
    let f = Fixture::new().await;
    f.grant_github_feature_set().await;

    let result = f
        .call(
            "mcpmux_get_tool_schema",
            json!({ "tools": ["github_list_issues", ""] }),
        )
        .await;
    let body = Fixture::result_json(&result);
    let missing = body.get("missing").unwrap().as_array().unwrap();
    assert!(missing.contains(&json!("")));
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_filter_shapes_structured_insights_payload() {
    let insights: Vec<Value> = (0..16)
        .map(|i| {
            json!({
                "name": format!("Insight {i}"),
                "short_id": format!("ins-{i}"),
                "extra": "noise"
            })
        })
        .collect();
    let payload = json!({ "insights": insights });
    let backend_result = ToolCallResult {
        content: vec![json!({
            "type": "text",
            "text": "legacy plain summary",
        })],
        structured_content: Some(payload),
        is_error: false,
    };
    let invoke_backend = CannedInvokeBackend::new()
        .with_response("posthog-personal-gait_insights-list", backend_result)
        .into_arc();

    let f = Fixture::with_invoke_backend(Some(invoke_backend)).await;

    let tool = ServerFeature::tool(f.space_id, "posthog-personal-gait", "insights-list");
    f.server_feature_repo.upsert(&tool).await.unwrap();
    let mut fs = FeatureSet::new_custom("GAIT insights", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: tool.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;
    let result = f
        .call(
            "mcpmux_invoke_tool",
            json!({
                "server_id": "posthog-personal-gait",
                "tool": "insights-list",
                "args": {},
                "filter": { "max_rows": 3, "fields": ["name", "short_id"] }
            }),
        )
        .await;

    assert!(!result.is_error.unwrap_or(true));
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("returned"), Some(&json!(3)));
    assert_eq!(body.get("total"), Some(&json!(16)));
    assert_eq!(body.get("truncated"), Some(&json!(true)));
    let sample = body.get("insights").and_then(|v| v.as_array()).unwrap();
    assert_eq!(sample.len(), 3);
    assert_eq!(
        sample[0],
        json!({ "name": "Insight 0", "short_id": "ins-0" })
    );

    let structured = result.structured_content.expect("structured shaped");
    assert_eq!(structured.get("returned"), Some(&json!(3)));
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_filter_shapes_posthog_paginated_results_in_content_json() {
    let results: Vec<Value> = (0..16)
        .map(|i| {
            json!({
                "name": format!("Insight {i}"),
                "short_id": format!("ins-{i}"),
                "description": "noise"
            })
        })
        .collect();
    let payload = json!({
        "count": 16,
        "next": null,
        "previous": null,
        "results": results,
    });
    let backend_result = ToolCallResult {
        content: vec![json!({
            "type": "text",
            "text": payload.to_string(),
        })],
        structured_content: None,
        is_error: false,
    };
    let invoke_backend = CannedInvokeBackend::new()
        .with_response("posthog-personal-gait_insights-list", backend_result)
        .into_arc();

    let f = Fixture::with_invoke_backend(Some(invoke_backend)).await;

    let tool = ServerFeature::tool(f.space_id, "posthog-personal-gait", "insights-list");
    f.server_feature_repo.upsert(&tool).await.unwrap();
    let mut fs = FeatureSet::new_custom("GAIT insights paginated", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: tool.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;
    let result = f
        .call(
            "mcpmux_invoke_tool",
            json!({
                "server_id": "posthog-personal-gait",
                "tool": "insights-list",
                "args": {},
                "filter": { "max_rows": 3, "fields": ["name", "short_id"] }
            }),
        )
        .await;

    assert!(!result.is_error.unwrap_or(true));
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("returned"), Some(&json!(3)));
    assert_eq!(body.get("total"), Some(&json!(16)));
    assert_eq!(body.get("truncated"), Some(&json!(true)));
    let sample = body.get("results").and_then(|v| v.as_array()).unwrap();
    assert_eq!(sample.len(), 3);

    let structured = result.structured_content.expect("structured mirrored");
    assert_eq!(structured.get("returned"), Some(&json!(3)));
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_filter_shapes_posthog_paginated_results_in_content_yaml() {
    let mut yaml = String::from("count: 16\nnext: null\nprevious: null\nresults[16]:\n");
    for i in 0..16 {
        yaml.push_str(&format!(
            "  - name: Insight {i}\n    short_id: ins-{i}\n    description: noise\n"
        ));
    }
    let backend_result = ToolCallResult {
        content: vec![json!({
            "type": "text",
            "text": yaml,
        })],
        structured_content: None,
        is_error: false,
    };
    let invoke_backend = CannedInvokeBackend::new()
        .with_response("posthog-personal-gait_insights-list", backend_result)
        .into_arc();

    let f = Fixture::with_invoke_backend(Some(invoke_backend)).await;

    let tool = ServerFeature::tool(f.space_id, "posthog-personal-gait", "insights-list");
    f.server_feature_repo.upsert(&tool).await.unwrap();
    let mut fs = FeatureSet::new_custom("GAIT insights yaml", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: tool.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;
    let result = f
        .call(
            "mcpmux_invoke_tool",
            json!({
                "server_id": "posthog-personal-gait",
                "tool": "insights-list",
                "args": {},
                "filter": { "max_rows": 3, "fields": ["name", "short_id"] }
            }),
        )
        .await;

    assert!(!result.is_error.unwrap_or(true));
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("returned"), Some(&json!(3)));
    assert_eq!(body.get("total"), Some(&json!(16)));
    assert_eq!(body.get("truncated"), Some(&json!(true)));
    let sample = body.get("results").and_then(|v| v.as_array()).unwrap();
    assert_eq!(sample.len(), 3);

    let structured = result.structured_content.expect("structured mirrored");
    assert_eq!(structured.get("returned"), Some(&json!(3)));
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_filter_aggregates_multi_block_content() {
    let blocks: Vec<Value> = (0..8)
        .map(|i| {
            json!({
                "type": "text",
                "text": json!({ "name": format!("row-{i}"), "value": i }).to_string(),
            })
        })
        .collect();
    let backend_result = ToolCallResult {
        content: blocks,
        structured_content: None,
        is_error: false,
    };
    let invoke_backend = CannedInvokeBackend::new()
        .with_response("github_list_issues", backend_result)
        .into_arc();

    let f = Fixture::with_invoke_backend(Some(invoke_backend)).await;
    f.grant_github_feature_set().await;

    let result = f
        .call(
            "mcpmux_invoke_tool",
            json!({
                "server_id": "github",
                "tool": "list_issues",
                "args": {},
                "filter": { "max_rows": 3, "fields": ["name"] }
            }),
        )
        .await;

    assert!(!result.is_error.unwrap_or(true));
    let body = Fixture::result_json(&result);
    assert_eq!(body.get("returned"), Some(&json!(3)));
    assert_eq!(body.get("total"), Some(&json!(8)));
    let sample = body.get("items").and_then(|v| v.as_array()).unwrap();
    assert_eq!(sample.len(), 3);
    assert_eq!(sample[0], json!({ "name": "row-0" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn advertised_resources_empty_without_surfaced_members() {
    let f = Fixture::new().await;

    let mut resource = ServerFeature::resource(f.space_id, "github", "github://docs/readme");
    resource.description = Some("GitHub readme resource".into());
    f.server_feature_repo.upsert(&resource).await.unwrap();

    let mut fs = FeatureSet::new_custom("Grant GitHub resource", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: resource.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;
    let fs_ids = vec![fs.id.clone()];
    let advertised = f
        .feature_service
        .get_advertised_resources_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();
    assert!(advertised.is_empty());

    let readable = f
        .feature_service
        .get_readable_resources_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();
    assert_eq!(readable.len(), 1);
    assert_eq!(readable[0].feature_name, "github://docs/readme");
}

#[tokio::test(flavor = "multi_thread")]
async fn surfaced_resource_appears_in_advertised_set() {
    let f = Fixture::new().await;

    let mut resource = ServerFeature::resource(f.space_id, "github", "github://docs/readme");
    f.server_feature_repo.upsert(&resource).await.unwrap();

    let mut fs = FeatureSet::new_custom("Surfaced resource", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: resource.id.to_string(),
        mode: MemberMode::Include,
        surfaced: true,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;

    let fs_ids = vec![fs.id.clone()];
    let advertised = f
        .feature_service
        .get_advertised_resources_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();
    assert_eq!(advertised.len(), 1);
    assert_eq!(advertised[0].feature_name, "github://docs/readme");
}

#[tokio::test(flavor = "multi_thread")]
async fn surfaced_prompt_appears_in_advertised_set() {
    let f = Fixture::new().await;

    let mut prompt = ServerFeature::prompt(f.space_id, "github", "summarize_issue");
    prompt.description = Some("Summarize a GitHub issue".into());
    f.server_feature_repo.upsert(&prompt).await.unwrap();

    let mut fs = FeatureSet::new_custom("Surfaced prompt", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: prompt.id.to_string(),
        mode: MemberMode::Include,
        surfaced: true,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;

    let fs_ids = vec![fs.id.clone()];
    let advertised = f
        .feature_service
        .get_advertised_prompts_for_grants(&f.space_id.to_string(), &fs_ids)
        .await
        .unwrap();
    assert_eq!(advertised.len(), 1);
    assert_eq!(advertised[0].feature_name, "summarize_issue");
}

#[tokio::test(flavor = "multi_thread")]
async fn search_resources_returns_readable_matches() {
    let f = Fixture::new().await;

    let mut resource = ServerFeature::resource(f.space_id, "github", "github://docs/readme");
    resource.description = Some("Project readme".into());
    f.server_feature_repo.upsert(&resource).await.unwrap();

    let mut fs = FeatureSet::new_custom("Resource search", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: resource.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;
    let search = f
        .call(
            "mcpmux_search_resources",
            json!({
                "query": "readme",
                "server_id": "github",
                "detail_level": "description"
            }),
        )
        .await;
    let body = Fixture::result_json(&search);
    let resources = body.get("resources").unwrap().as_array().unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(
        resources[0].get("uri"),
        Some(&json!("github://docs/readme"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn search_prompts_returns_fetchable_matches() {
    let f = Fixture::new().await;

    let mut prompt = ServerFeature::prompt(f.space_id, "github", "summarize_issue");
    prompt.description = Some("Summarize issue text".into());
    f.server_feature_repo.upsert(&prompt).await.unwrap();

    let mut fs = FeatureSet::new_custom("Prompt search", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: prompt.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;
    let search = f
        .call(
            "mcpmux_search_prompts",
            json!({
                "query": "summarize",
                "server_id": "github",
                "detail_level": "description"
            }),
        )
        .await;
    let body = Fixture::result_json(&search);
    let prompts = body.get("prompts").unwrap().as_array().unwrap();
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].get("prompt"), Some(&json!("summarize_issue")));
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_levenshtein_suggests_near_tool_name() {
    let f = Fixture::new().await;
    f.grant_github_feature_set().await;

    let result = f
        .call(
            "mcpmux_invoke_tool",
            json!({
                "server_id": "github",
                "tool": "list_isses",
                "args": {}
            }),
        )
        .await;

    assert!(result.is_error.unwrap_or(false));
    let body = Fixture::result_json(&result);
    let message = body.get("message").and_then(|v| v.as_str()).unwrap();
    assert!(message.contains("did you mean"));
    assert!(message.contains("github_list_issues"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_tools_tf_idf_ranks_list_issues_first() {
    let f = Fixture::new().await;

    let mut list_pulls = ServerFeature::tool(f.space_id, "github", "list_pulls");
    list_pulls.description = Some("List pull requests for issues backlog".into());
    f.server_feature_repo.upsert(&list_pulls).await.unwrap();

    let mut fs = FeatureSet::new_custom("GitHub both tools", f.space_id.to_string());
    for feature in [
        f.server_feature_repo
            .list_for_space(&f.space_id.to_string())
            .await
            .unwrap()
            .into_iter()
            .find(|feat| feat.feature_name == "list_issues")
            .unwrap(),
        list_pulls,
    ] {
        fs.members.push(FeatureSetMember {
            id: Uuid::new_v4().to_string(),
            feature_set_id: fs.id.clone(),
            member_type: MemberType::Feature,
            member_id: feature.id.to_string(),
            mode: MemberMode::Include,
            surfaced: false,
        });
    }
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;
    let search = f
        .call(
            "mcpmux_search_tools",
            json!({
                "query": "issues",
                "server_id": "github",
                "detail_level": "name"
            }),
        )
        .await;
    let body = Fixture::result_json(&search);
    let tools = body.get("tools").unwrap().as_array().unwrap();
    assert_eq!(tools.len(), 2);
    assert_eq!(
        tools[0].get("qualified_name"),
        Some(&json!("github_list_issues"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn read_resource_routes_clone_server_not_inactive_parent() {
    let uri = "posthog://skills/audit/all";
    let disclosure_backend = CannedDisclosureBackend::new()
        .with_resource_read(
            "posthog-personal-gait",
            uri,
            vec![json!({ "uri": uri, "text": "audit skill body" })],
        )
        .into_arc();
    let f = Fixture::with_backends(None, Some(disclosure_backend)).await;

    let parent = ServerFeature::resource(f.space_id, "posthog-personal", uri);
    f.server_feature_repo.upsert(&parent).await.unwrap();

    let clone = ServerFeature::resource(f.space_id, "posthog-personal-gait", uri);
    f.server_feature_repo.upsert(&clone).await.unwrap();

    let ambiguous = f
        .feature_service
        .find_server_for_resource(&f.space_id.to_string(), uri)
        .await
        .unwrap();
    assert_eq!(
        ambiguous.as_deref(),
        Some("posthog-personal"),
        "Space-wide lookup is ambiguous for clones"
    );

    let mut fs = FeatureSet::new_custom("GAIT PostHog skills", f.space_id.to_string());
    fs.members.push(FeatureSetMember {
        id: Uuid::new_v4().to_string(),
        feature_set_id: fs.id.clone(),
        member_type: MemberType::Feature,
        member_id: clone.id.to_string(),
        mode: MemberMode::Include,
        surfaced: false,
    });
    f.feature_set_repo.create(&fs).await.unwrap();
    f.grant_feature_set(&fs.id).await;
    let read = f.call("mcpmux_read_resource", json!({ "uri": uri })).await;
    let body = Fixture::result_json(&read);
    assert_eq!(body.get("uri"), Some(&json!(uri)));
    let contents = body.get("contents").unwrap().as_array().unwrap();
    assert_eq!(contents.len(), 1);
    assert_eq!(contents[0].get("text"), Some(&json!("audit skill body")));
}

#[test]
fn direct_read_and_fetch_redirect_messages() {
    let read = format_direct_read_redirect("posthog://skills/foo");
    assert!(read.contains("mcpmux_read_resource"));
    assert!(read.contains("posthog://skills/foo"));

    let fetch =
        format_direct_fetch_prompt_redirect("github_summarize_issue", "github", "summarize_issue");
    assert!(fetch.contains("mcpmux_fetch_prompt"));
    assert!(fetch.contains("summarize_issue"));
}
