//! In-memory tool index for meta-gateway search and schema lookup.
//!
//! Built from Space [`ServerFeature`] rows and filtered to the caller's
//! invokable tool set before search/schema operations run.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use mcpmux_core::{FeatureType, ServerFeature, ServerFeatureRepository};

use crate::pool::InactiveDiscoveryEntry;
use serde_json::{json, Value};
use tracing::{debug, trace};

use super::discovery_rank::{filter_and_rank_traced, lexical_score, RankTraceContext};
use super::embedding::{EmbeddingService, EmbeddingState};

/// How much detail search results include per matched tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLevel {
    Name,
    Description,
    Schema,
}

impl DetailLevel {
    /// Parse a wire-level detail level string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "name" => Some(Self::Name),
            "description" => Some(Self::Description),
            "schema" => Some(Self::Schema),
            _ => None,
        }
    }
}

/// In-memory active tool index for a resolved binding (search cache value).
pub type ToolIndex = Vec<ToolIndexEntry>;

/// Cached embedding for one active tool (hybrid search).
#[derive(Debug, Clone)]
pub struct DocEmbedding {
    pub qualified_name: String,
    pub vector: Vec<f32>,
}

/// Per-binding hybrid search inputs (embedding cache + active corpus).
pub struct SearchContext<'a> {
    pub embeddings: &'a EmbeddingService,
    pub embedding_cache: &'a DashMap<String, (u64, Vec<DocEmbedding>)>,
    pub session_id: &'a str,
    pub fingerprint: u64,
    /// Active-only index used as the semantic embedding corpus.
    pub active_index: &'a [ToolIndexEntry],
    pub index_cache_hit: bool,
}

/// One searchable tool entry in the Space index.
#[derive(Debug, Clone)]
pub struct ToolIndexEntry {
    pub server_id: String,
    pub feature_name: String,
    pub qualified_name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
    pub is_available: bool,
    /// `inactive` when matched via `include_inactive` discovery widening.
    pub status: Option<String>,
    pub bindable_feature_set_id: Option<String>,
}

/// Paginated search output.
#[derive(Debug, Clone)]
pub struct SearchToolsResult {
    pub tools: Vec<Value>,
    pub next_cursor: Option<String>,
    pub total: usize,
    /// Ranking mode used for this result set (`hybrid` or `lexical`).
    pub ranking: &'static str,
    /// Fused or lexical score of the top-ranked match when a query was provided.
    pub top_fused_score: Option<f64>,
}

/// Lexical weight for hybrid score fusion.
///
/// Tuned against the 20-case intent→tool relevance fixture in
/// `tests/rust/tests/integration/search_relevance_eval.rs` (Phase 4). At 0.4/0.6
/// hybrid passes all fixture cases in top-3 while lexical-only passes ~11/20;
/// lowering lexical (e.g. 0.3) risks exact-name queries losing to semantic noise,
/// raising it (e.g. 0.5) drops intent-only queries with zero token overlap.
const LEXICAL_FUSION_WEIGHT: f32 = 0.4;

/// Semantic weight for hybrid score fusion (complement of [`LEXICAL_FUSION_WEIGHT`]).
const SEMANTIC_FUSION_WEIGHT: f32 = 0.6;

/// Service that builds and queries a tool index for a Space.
pub struct ToolDiscoveryService {
    server_feature_repo: Arc<dyn ServerFeatureRepository>,
}

impl ToolDiscoveryService {
    /// Create a discovery service backed by the Space feature repository.
    pub fn new(server_feature_repo: Arc<dyn ServerFeatureRepository>) -> Self {
        Self {
            server_feature_repo,
        }
    }

    /// Build an index of every tool installed in `space_id` (ignores FeatureSet ACL).
    pub async fn build_catalog_index(&self, space_id: &str) -> Result<Vec<ToolIndexEntry>> {
        let features = self.server_feature_repo.list_for_space(space_id).await?;
        let mut index: Vec<ToolIndexEntry> = features
            .into_iter()
            .filter(|f| f.feature_type == FeatureType::Tool)
            .map(|f| ToolIndexEntry {
                server_id: f.server_id.clone(),
                feature_name: f.feature_name.clone(),
                qualified_name: f.qualified_name(),
                description: f.description.clone(),
                input_schema: extract_input_schema(f.raw_json.as_ref()),
                is_available: f.is_available,
                status: None,
                bindable_feature_set_id: None,
            })
            .collect();
        index.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
        Ok(index)
    }

    /// Build an index for `space_id`, retaining only tools present in `invokable`.
    pub async fn build_index(
        &self,
        space_id: &str,
        invokable: &[ServerFeature],
    ) -> Result<Vec<ToolIndexEntry>> {
        let invokable_keys: HashSet<(String, String)> = invokable
            .iter()
            .filter(|f| f.feature_type == FeatureType::Tool)
            .map(|f| (f.server_id.clone(), f.feature_name.clone()))
            .collect();

        let features = self.server_feature_repo.list_for_space(space_id).await?;
        let mut index: Vec<ToolIndexEntry> = features
            .into_iter()
            .filter(|f| {
                f.feature_type == FeatureType::Tool
                    && invokable_keys.contains(&(f.server_id.clone(), f.feature_name.clone()))
            })
            .map(|f| ToolIndexEntry {
                server_id: f.server_id.clone(),
                feature_name: f.feature_name.clone(),
                qualified_name: f.qualified_name(),
                description: f.description.clone(),
                input_schema: extract_input_schema(f.raw_json.as_ref()),
                is_available: f.is_available,
                status: None,
                bindable_feature_set_id: None,
            })
            .collect();

        index.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
        Ok(index)
    }

    /// Build index entries for tools that exist in a FeatureSet but are not invokable yet.
    pub fn build_inactive_index(entries: &[InactiveDiscoveryEntry]) -> Vec<ToolIndexEntry> {
        let mut index: Vec<ToolIndexEntry> = entries
            .iter()
            .map(|entry| {
                let f = &entry.feature;
                ToolIndexEntry {
                    server_id: f.server_id.clone(),
                    feature_name: f.feature_name.clone(),
                    qualified_name: f.qualified_name(),
                    description: f.description.clone(),
                    input_schema: extract_input_schema(f.raw_json.as_ref()),
                    is_available: f.is_available,
                    status: Some("inactive".to_string()),
                    bindable_feature_set_id: Some(entry.bindable_feature_set_id.clone()),
                }
            })
            .collect();
        index.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
        index
    }

    /// Search the index with optional query, server filter, and pagination.
    #[allow(clippy::too_many_arguments)]
    pub fn search(
        index: &[ToolIndexEntry],
        query: Option<&str>,
        server_id: Option<&str>,
        detail_level: DetailLevel,
        limit: usize,
        cursor: Option<&str>,
        query_id: Option<&str>,
        hybrid: Option<SearchContext<'_>>,
    ) -> SearchToolsResult {
        let limit = limit.clamp(1, 100);
        let offset = cursor.and_then(|c| c.parse::<usize>().ok()).unwrap_or(0);

        let haystack_fn = |entry: &ToolIndexEntry| entry_search_haystack(entry);

        let (mut ranked, top_lexical_score) = if let Some(query_id) = query_id {
            let trace = RankTraceContext { query_id };
            filter_and_rank_traced(
                index,
                query,
                server_id,
                |entry| entry.server_id.as_str(),
                haystack_fn,
                &trace,
            )
        } else {
            use super::discovery_rank::filter_and_rank;
            (
                filter_and_rank(
                    index,
                    query,
                    server_id,
                    |entry| entry.server_id.as_str(),
                    haystack_fn,
                ),
                None,
            )
        };

        let (ranking, top_fused_score) =
            if let (Some(query), Some(query_id), Some(ctx)) = (query, query_id, hybrid) {
                rank_with_hybrid(
                    &mut ranked,
                    query,
                    query_id,
                    ctx,
                    haystack_fn,
                    top_lexical_score,
                )
            } else {
                ("lexical", top_lexical_score)
            };

        let total = ranked.len();
        let page: Vec<Value> = ranked
            .iter()
            .skip(offset)
            .take(limit)
            .map(|entry| entry_to_json(entry, detail_level))
            .collect();

        let next_offset = offset + page.len();
        let next_cursor = if next_offset < total {
            Some(next_offset.to_string())
        } else {
            None
        };

        SearchToolsResult {
            tools: page,
            next_cursor,
            total,
            ranking,
            top_fused_score,
        }
    }

    /// Resolve schemas for one or more qualified tool names.
    pub fn get_schemas(
        index: &[ToolIndexEntry],
        tool_names: &[String],
        compact: bool,
    ) -> Vec<Value> {
        tool_names
            .iter()
            .filter_map(|name| {
                let entry = index.iter().find(|e| e.qualified_name == *name)?;
                Some(schema_entry_to_json(entry, compact))
            })
            .collect()
    }
}

/// Haystack text for lexical and semantic ranking (`feature_name + qualified_name + description`).
fn entry_search_haystack(entry: &ToolIndexEntry) -> String {
    format!(
        "{} {} {}",
        entry.feature_name,
        entry.qualified_name,
        entry.description.as_deref().unwrap_or("")
    )
}

/// Apply hybrid fusion when the embedding model is ready; otherwise lexical-only.
fn rank_with_hybrid<'a, T, FHaystack>(
    ranked: &mut Vec<&'a T>,
    query: &str,
    query_id: &str,
    ctx: SearchContext<'_>,
    haystack_fn: FHaystack,
    top_lexical_score: Option<f64>,
) -> (&'static str, Option<f64>)
where
    T: AsRef<ToolIndexEntry> + 'a,
    FHaystack: Fn(&T) -> String,
{
    let embedding_cache_label;
    let doc_vectors = match ctx.embedding_cache.get(ctx.session_id) {
        Some(entry) if entry.0 == ctx.fingerprint => {
            embedding_cache_label = "hit";
            entry.value().1.clone()
        }
        _ => {
            embedding_cache_label = "miss";
            Vec::new()
        }
    };

    let model_ready = matches!(ctx.embeddings.state(), EmbeddingState::Ready);
    if !model_ready {
        ctx.embeddings.ensure_init_started();
    }

    if !model_ready || ranked.is_empty() {
        log_cache_decision(
            query_id,
            ctx.index_cache_hit,
            "skipped",
            ctx.active_index.len(),
        );
        return ("lexical", top_lexical_score);
    }

    let active_keys: HashSet<&str> = ctx
        .active_index
        .iter()
        .map(|e| e.qualified_name.as_str())
        .collect();

    let doc_vectors = if doc_vectors.is_empty() {
        let documents: Vec<String> = ctx.active_index.iter().map(entry_search_haystack).collect();
        let Some(raw_vectors) = ctx.embeddings.embed_documents(&documents, Some(query_id)) else {
            log_cache_decision(
                query_id,
                ctx.index_cache_hit,
                "skipped",
                ctx.active_index.len(),
            );
            return ("lexical", top_lexical_score);
        };
        let cached: Vec<DocEmbedding> = ctx
            .active_index
            .iter()
            .zip(raw_vectors)
            .map(|(entry, vector)| DocEmbedding {
                qualified_name: entry.qualified_name.clone(),
                vector,
            })
            .collect();
        ctx.embedding_cache.insert(
            ctx.session_id.to_string(),
            (ctx.fingerprint, cached.clone()),
        );
        log_embedding_cache_reuse(query_id, false, cached.len());
        cached
    } else {
        log_embedding_cache_reuse(query_id, true, doc_vectors.len());
        doc_vectors
    };

    log_cache_decision(
        query_id,
        ctx.index_cache_hit,
        embedding_cache_label,
        ctx.active_index.len(),
    );

    let Some(query_vector) = ctx.embeddings.embed_query(query, Some(query_id)) else {
        return ("lexical", top_lexical_score);
    };

    let vector_by_name: HashMap<&str, &[f32]> = doc_vectors
        .iter()
        .map(|doc| (doc.qualified_name.as_str(), doc.vector.as_slice()))
        .collect();

    let corpus: Vec<String> = ranked.iter().map(|entry| haystack_fn(entry)).collect();
    let lexical_scores: Vec<f64> = ranked
        .iter()
        .map(|entry| lexical_score(query, &haystack_fn(entry), &corpus))
        .collect();
    let max_lexical = lexical_scores
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(1e-9);

    let mut fused_scores: Vec<f64> = Vec::with_capacity(ranked.len());
    for (idx, entry) in ranked.iter().enumerate() {
        let tool_entry = entry.as_ref();
        let norm_lexical = (lexical_scores[idx] / max_lexical) as f32;
        let semantic = if active_keys.contains(tool_entry.qualified_name.as_str()) {
            vector_by_name
                .get(tool_entry.qualified_name.as_str())
                .map(|doc_vector| EmbeddingService::cosine(&query_vector, doc_vector))
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let fused = if active_keys.contains(tool_entry.qualified_name.as_str()) {
            (LEXICAL_FUSION_WEIGHT * norm_lexical + SEMANTIC_FUSION_WEIGHT * semantic) as f64
        } else {
            lexical_scores[idx]
        };
        trace!(
            query_id,
            qualified_name = %tool_entry.qualified_name,
            lexical_score = lexical_scores[idx],
            semantic_score = semantic,
            fused_score = fused,
            "[search] entry score"
        );
        fused_scores.push(fused);
    }

    let mut scored: Vec<(&T, f64)> = ranked.drain(..).zip(fused_scores).collect();
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| haystack_fn(a.0).cmp(&haystack_fn(b.0)))
    });
    let top_fused_score = scored.first().map(|(_, score)| *score);
    *ranked = scored.into_iter().map(|(entry, _)| entry).collect();

    debug!(
        query_id,
        ranking = "hybrid",
        lexical_weight = LEXICAL_FUSION_WEIGHT,
        semantic_weight = SEMANTIC_FUSION_WEIGHT,
        "[search] fusion"
    );

    ("hybrid", top_fused_score)
}

fn log_cache_decision(
    query_id: &str,
    index_cache_hit: bool,
    embedding_cache: &str,
    active_tools: usize,
) {
    debug!(
        query_id,
        index_cache = if index_cache_hit { "hit" } else { "miss" },
        embedding_cache,
        active_tools,
        "[search] cache decision"
    );
}

fn log_embedding_cache_reuse(query_id: &str, cached: bool, docs_embedded: usize) {
    tracing::info!(
        target: "embed",
        "[embed] query_id = {}, model_state = ready, docs_embedded = {}, cached = {}",
        query_id,
        docs_embedded,
        cached
    );
}

/// Extract MCP `inputSchema` from a cached tool JSON blob.
fn extract_input_schema(raw_json: Option<&Value>) -> Option<Value> {
    raw_json.and_then(|json| {
        json.get("inputSchema")
            .or_else(|| json.get("input_schema"))
            .cloned()
    })
}

fn entry_to_json(entry: &ToolIndexEntry, detail_level: DetailLevel) -> Value {
    let mut obj = json!({
        "server_id": entry.server_id,
        "qualified_name": entry.qualified_name,
        "available": entry.is_available,
    });
    if let Some(status) = &entry.status {
        obj["status"] = json!(status);
    }
    if let Some(fs_id) = &entry.bindable_feature_set_id {
        obj["bindable_feature_set_id"] = json!(fs_id);
    }
    match detail_level {
        DetailLevel::Name => {}
        DetailLevel::Description | DetailLevel::Schema => {
            if let Some(desc) = &entry.description {
                obj["description"] = json!(desc);
            }
        }
    }
    if detail_level == DetailLevel::Schema {
        if let Some(schema) = &entry.input_schema {
            obj["input_schema"] = schema.clone();
        }
    }
    obj
}

fn schema_entry_to_json(entry: &ToolIndexEntry, compact: bool) -> Value {
    let mut obj = json!({
        "qualified_name": entry.qualified_name,
        "server_id": entry.server_id,
        "feature_name": entry.feature_name,
    });
    if !compact {
        if let Some(desc) = &entry.description {
            obj["description"] = json!(desc);
        }
    }
    if let Some(schema) = &entry.input_schema {
        obj["input_schema"] = schema.clone();
    } else {
        obj["input_schema"] = json!({"type": "object", "properties": {}});
    }
    obj
}

impl AsRef<ToolIndexEntry> for ToolIndexEntry {
    fn as_ref(&self) -> &ToolIndexEntry {
        self
    }
}
