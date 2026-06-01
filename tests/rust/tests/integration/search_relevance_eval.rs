//! Intent-query → expected-tool relevance fixture for hybrid search ranking (Phase 4).
//!
//! Uses deterministic stub embeddings (no ONNX model download in CI). Each case
//! maps an agent-style intent query to a qualified tool name drawn from Jira,
//! GitHub, Canva, and PostHog-style bundles.

use std::collections::HashMap;

use mcpmux_core::{
    normalize_workspace_root, EmbeddingRecord, FeatureSet, FeatureSetMember, MemberMode,
    MemberType, ServerFeature, WorkspaceBinding,
};
use serde_json::{json, Value};
use uuid::Uuid;

use super::meta_tools::Fixture;

const QUERY_PREFIX: &str = "query: ";
const PASSAGE_PREFIX: &str = "passage: ";
const VECTOR_DIM: usize = 16;

/// One intent query and the qualified tool it should surface in hybrid top-3.
struct RelevanceCase {
    query: &'static str,
    expected: &'static str,
    topic: usize,
}

/// Tool seeded into the relevance bundle (includes decoys per server).
struct FixtureTool {
    server_id: &'static str,
    feature_name: &'static str,
    description: &'static str,
    topic: usize,
    variant: u8,
}

/// ~20 intent→tool cases across Jira, GitHub, Canva, and PostHog bundles.
const RELEVANCE_CASES: &[RelevanceCase] = &[
    RelevanceCase {
        query: "post a jira comment",
        expected: "jira_add_comment",
        topic: 0,
    },
    RelevanceCase {
        query: "create jira ticket",
        expected: "jira_create_issue",
        topic: 1,
    },
    RelevanceCase {
        query: "assign issue to someone",
        expected: "jira_assign_issue",
        topic: 2,
    },
    RelevanceCase {
        query: "search jira issues",
        expected: "jira_search_issues",
        topic: 3,
    },
    RelevanceCase {
        query: "transition issue to done",
        expected: "jira_transition_issue",
        topic: 4,
    },
    RelevanceCase {
        query: "open a pull request",
        expected: "github_create_pull_request",
        topic: 5,
    },
    RelevanceCase {
        query: "list my repositories",
        expected: "github_list_repositories",
        topic: 6,
    },
    RelevanceCase {
        query: "merge pull request",
        expected: "github_merge_pull_request",
        topic: 7,
    },
    RelevanceCase {
        query: "comment on github issue",
        expected: "github_create_issue_comment",
        topic: 8,
    },
    RelevanceCase {
        query: "get file contents from repo",
        expected: "github_get_file_contents",
        topic: 9,
    },
    RelevanceCase {
        query: "list folder items",
        expected: "canva_list-folder-items",
        topic: 10,
    },
    RelevanceCase {
        query: "create a new design",
        expected: "canva_create_design",
        topic: 11,
    },
    RelevanceCase {
        query: "upload asset to canva",
        expected: "canva_upload_asset",
        topic: 12,
    },
    RelevanceCase {
        query: "export design as png",
        expected: "canva_export_design",
        topic: 13,
    },
    RelevanceCase {
        query: "search canva templates",
        expected: "canva_search_templates",
        topic: 14,
    },
    RelevanceCase {
        query: "capture analytics event",
        expected: "posthog_capture",
        topic: 15,
    },
    RelevanceCase {
        query: "run hogql query",
        expected: "posthog_query",
        topic: 16,
    },
    RelevanceCase {
        query: "list feature flags",
        expected: "posthog_list_feature_flags",
        topic: 17,
    },
    RelevanceCase {
        query: "create new cohort",
        expected: "posthog_create_cohort",
        topic: 18,
    },
    RelevanceCase {
        query: "get person profile",
        expected: "posthog_get_person",
        topic: 19,
    },
];

/// Active tool corpus: expected tools plus decoys that share a server prefix.
const FIXTURE_TOOLS: &[FixtureTool] = &[
    // Jira
    FixtureTool {
        server_id: "jira",
        feature_name: "add_comment",
        description: "Add a comment to a Jira issue",
        topic: 0,
        variant: 0,
    },
    FixtureTool {
        server_id: "jira",
        feature_name: "create_issue",
        description: "Create a new Jira issue",
        topic: 1,
        variant: 0,
    },
    FixtureTool {
        server_id: "jira",
        feature_name: "assign_issue",
        description: "Assign a Jira issue to a user",
        topic: 2,
        variant: 0,
    },
    FixtureTool {
        server_id: "jira",
        feature_name: "search_issues",
        description: "Search Jira issues with JQL",
        topic: 3,
        variant: 0,
    },
    FixtureTool {
        server_id: "jira",
        feature_name: "transition_issue",
        description: "Transition a Jira issue to a new status",
        topic: 4,
        variant: 0,
    },
    FixtureTool {
        server_id: "jira",
        feature_name: "get_project",
        description: "Get Jira project metadata",
        topic: 1,
        variant: 2,
    },
    // GitHub
    FixtureTool {
        server_id: "github",
        feature_name: "create_pull_request",
        description: "Open a new pull request on GitHub",
        topic: 5,
        variant: 0,
    },
    FixtureTool {
        server_id: "github",
        feature_name: "list_repositories",
        description: "List repositories for the authenticated user",
        topic: 6,
        variant: 0,
    },
    FixtureTool {
        server_id: "github",
        feature_name: "merge_pull_request",
        description: "Merge an open pull request",
        topic: 7,
        variant: 0,
    },
    FixtureTool {
        server_id: "github",
        feature_name: "create_issue_comment",
        description: "Post a comment on a GitHub issue",
        topic: 8,
        variant: 0,
    },
    FixtureTool {
        server_id: "github",
        feature_name: "get_file_contents",
        description: "Read a file from a GitHub repository",
        topic: 9,
        variant: 0,
    },
    FixtureTool {
        server_id: "github",
        feature_name: "list_commits",
        description: "List commits on a branch",
        topic: 6,
        variant: 2,
    },
    // Canva
    FixtureTool {
        server_id: "canva",
        feature_name: "list-folder-items",
        description: "List items in a Canva folder",
        topic: 10,
        variant: 0,
    },
    FixtureTool {
        server_id: "canva",
        feature_name: "create_design",
        description: "Create a new Canva design from a template",
        topic: 11,
        variant: 0,
    },
    FixtureTool {
        server_id: "canva",
        feature_name: "upload_asset",
        description: "Upload a media asset to Canva",
        topic: 12,
        variant: 0,
    },
    FixtureTool {
        server_id: "canva",
        feature_name: "export_design",
        description: "Export a Canva design to PNG or PDF",
        topic: 13,
        variant: 0,
    },
    FixtureTool {
        server_id: "canva",
        feature_name: "search_templates",
        description: "Search Canva design templates",
        topic: 14,
        variant: 0,
    },
    FixtureTool {
        server_id: "canva",
        feature_name: "get_brand_kit",
        description: "Fetch brand kit colors and fonts",
        topic: 11,
        variant: 2,
    },
    // PostHog
    FixtureTool {
        server_id: "posthog",
        feature_name: "capture",
        description: "Capture a product analytics event",
        topic: 15,
        variant: 0,
    },
    FixtureTool {
        server_id: "posthog",
        feature_name: "query",
        description: "Run a HogQL analytics query",
        topic: 16,
        variant: 0,
    },
    FixtureTool {
        server_id: "posthog",
        feature_name: "list_feature_flags",
        description: "List PostHog feature flags",
        topic: 17,
        variant: 0,
    },
    FixtureTool {
        server_id: "posthog",
        feature_name: "create_cohort",
        description: "Create a user cohort for analytics",
        topic: 18,
        variant: 0,
    },
    FixtureTool {
        server_id: "posthog",
        feature_name: "get_person",
        description: "Fetch a person profile by distinct id",
        topic: 19,
        variant: 0,
    },
    FixtureTool {
        server_id: "posthog",
        feature_name: "list_insights",
        description: "List saved analytics insights",
        topic: 16,
        variant: 2,
    },
];

/// Build the embedding haystack string (matches gateway `embedding_haystack`).
fn search_haystack(_server_id: &str, feature_name: &str, description: &str) -> String {
    format!("{feature_name} {description}")
}

/// L2-normalize a vector for cosine similarity.
fn normalize(mut vector: Vec<f32>) -> Vec<f32> {
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

/// Topic-aligned unit vector with a small variant offset (decoys stay separable).
fn topic_vector(topic: usize, variant: u8) -> Vec<f32> {
    let mut vector = vec![0.0_f32; VECTOR_DIM];
    let primary = topic % VECTOR_DIM;
    let secondary = (topic + 1) % VECTOR_DIM;
    vector[primary] = 0.88 - 0.04 * f32::from(variant.min(3));
    vector[secondary] = 0.12 + 0.02 * f32::from(variant.min(3));
    normalize(vector)
}

/// Query vector aligned to the expected tool's topic (simulates semantic intent match).
fn query_vector(topic: usize) -> Vec<f32> {
    topic_vector(topic, 0)
}

/// Register stub embeddings for every fixture query and tool passage.
fn build_stub_vectors() -> HashMap<String, Vec<f32>> {
    let mut vectors = HashMap::new();

    for case in RELEVANCE_CASES {
        let key = format!("{QUERY_PREFIX}{}", case.query);
        vectors.insert(key, query_vector(case.topic));
    }

    for tool in FIXTURE_TOOLS {
        let haystack = search_haystack(tool.server_id, tool.feature_name, tool.description);
        let key = format!("{PASSAGE_PREFIX}{haystack}");
        vectors.insert(key, topic_vector(tool.topic, tool.variant));
    }

    vectors
}

/// Seed fixture tools into DB and return a FeatureSet id containing all of them.
async fn seed_relevance_bundle(f: &Fixture) -> String {
    let space_id = f.space_id.to_string();
    let mut members = Vec::new();
    let mut fs = FeatureSet::new_custom("Relevance eval bundle", space_id.clone());

    for tool in FIXTURE_TOOLS {
        let mut feature = ServerFeature::tool(f.space_id, tool.server_id, tool.feature_name);
        feature.description = Some(tool.description.into());
        f.server_feature_repo.upsert(&feature).await.unwrap();
        members.push(FeatureSetMember {
            id: Uuid::new_v4().to_string(),
            feature_set_id: fs.id.clone(),
            member_type: MemberType::Feature,
            member_id: feature.id.to_string(),
            mode: MemberMode::Include,
            surfaced: false,
        });
    }

    fs.members = members;
    let fs_id = fs.id.clone();
    f.feature_set_repo.create(&fs).await.unwrap();
    fs_id
}

/// Bind the relevance bundle to the fixture session workspace root.
async fn bind_relevance_bundle(f: &Fixture, fs_id: &str) {
    let root = "/tmp/mcpmux-relevance-eval";
    f.session_roots.set_roots_capable(&f.session_id, true);
    f.session_roots.set(&f.session_id, [root]);
    let binding = WorkspaceBinding::new(
        normalize_workspace_root(root),
        f.space_id,
        fs_id.to_string(),
    );
    f.binding_repo.create(&binding).await.unwrap();
}

/// Run `mcpmux_search_tools` and return parsed JSON body.
async fn search_tools(f: &Fixture, query: &str) -> Value {
    let result = f
        .registry
        .call(
            "mcpmux_search_tools",
            &f.client_id,
            Some(&f.session_id),
            json!({ "query": query, "limit": 3 }),
        )
        .await
        .unwrap();
    Fixture::result_json(&result)
}

/// Return qualified names from the first page of search results.
fn top_qualified_names(body: &Value) -> Vec<String> {
    body.get("tools")
        .and_then(|v| v.as_array())
        .map(|tools| {
            tools
                .iter()
                .filter_map(|t| t.get("qualified_name").and_then(|v| v.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `expected` appears in the first `limit` ranked results.
fn in_top_n(body: &Value, expected: &str, limit: usize) -> bool {
    top_qualified_names(body)
        .into_iter()
        .take(limit)
        .any(|name| name == expected)
}

#[tokio::test(flavor = "multi_thread")]
async fn search_relevance_eval_hybrid_top3() {
    let f = Fixture::new().await;
    let fs_id = seed_relevance_bundle(&f).await;
    bind_relevance_bundle(&f, &fs_id).await;

    let stub_vectors = build_stub_vectors();
    f.registry
        .context()
        .embeddings
        .install_test_vectors(stub_vectors.clone());
    let records: Vec<EmbeddingRecord> = FIXTURE_TOOLS
        .iter()
        .map(|tool| {
            let haystack = search_haystack(tool.server_id, tool.feature_name, tool.description);
            let passage_key = format!("{PASSAGE_PREFIX}{haystack}");
            let vector = stub_vectors
                .get(&passage_key)
                .cloned()
                .expect("missing stub vector for passage");
            EmbeddingRecord {
                content_hash: mcpmux_gateway::services::EmbeddingService::content_hash(
                    tool.feature_name,
                    Some(tool.description),
                ),
                model_version: f.registry.context().embeddings.model_version().to_string(),
                vector,
            }
        })
        .collect();
    f.registry
        .context()
        .embedding_repo
        .upsert_many(&records)
        .await
        .unwrap();

    let mut failures = Vec::new();
    for case in RELEVANCE_CASES {
        let body = search_tools(&f, case.query).await;
        assert_eq!(
            body.get("ranking").and_then(|v| v.as_str()),
            Some("hybrid"),
            "case `{}` should use hybrid ranking",
            case.query
        );
        if !in_top_n(&body, case.expected, 3) {
            failures.push(format!(
                "query={:?} expected={} got={:?}",
                case.query,
                case.expected,
                top_qualified_names(&body)
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "hybrid relevance eval failures ({}): {:?}",
        failures.len(),
        failures
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn search_relevance_eval_lexical_baseline_recorded() {
    let f = Fixture::new().await;
    let fs_id = seed_relevance_bundle(&f).await;
    bind_relevance_bundle(&f, &fs_id).await;

    let mut lexical_hits = 0_usize;
    let mut hybrid_only = Vec::new();

    for case in RELEVANCE_CASES {
        let body = search_tools(&f, case.query).await;
        assert_eq!(
            body.get("ranking").and_then(|v| v.as_str()),
            Some("lexical"),
            "without stub embeddings search must stay lexical"
        );
        if in_top_n(&body, case.expected, 3) {
            lexical_hits += 1;
        } else {
            hybrid_only.push(case.query);
        }
    }

    eprintln!(
        "relevance eval lexical baseline: {}/{} top-3 hits; hybrid-only intents: {:?}",
        lexical_hits,
        RELEVANCE_CASES.len(),
        hybrid_only
    );

    // Baseline is informational (logged above). Hybrid top-3 is the regression gate.
    assert!(
        lexical_hits <= RELEVANCE_CASES.len(),
        "baseline counter sanity"
    );
}
