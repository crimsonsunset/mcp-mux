//! Feature Service Facade - Unified API delegating to specialized services

use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;

use crate::pool::instance::McpClient;
use crate::services::{PrefixCacheService, SessionOverrideRegistry};
use mcpmux_core::{FeatureSetRepository, FeatureType, ServerFeature, ServerFeatureRepository};

use super::{
    CachedFeatures, FeatureDiscoveryService, FeatureResolutionService, FeatureRoutingService,
};

pub use super::resolution::InactiveDiscoveryEntry;

/// Unified facade providing all feature operations (Facade pattern)
pub struct FeatureService {
    discovery: Arc<FeatureDiscoveryService>,
    resolution: Arc<FeatureResolutionService>,
    routing: Arc<FeatureRoutingService>,
    session_overrides: Arc<SessionOverrideRegistry>,
}

impl FeatureService {
    pub fn new(
        feature_repo: Arc<dyn ServerFeatureRepository>,
        feature_set_repo: Arc<dyn FeatureSetRepository>,
        prefix_cache: Arc<PrefixCacheService>,
        session_overrides: Arc<SessionOverrideRegistry>,
    ) -> Self {
        let discovery = Arc::new(FeatureDiscoveryService::new(feature_repo.clone()));

        let resolution = Arc::new(FeatureResolutionService::new(
            feature_repo.clone(),
            feature_set_repo.clone(),
            prefix_cache.clone(),
        ));

        let routing = Arc::new(FeatureRoutingService::new(
            feature_repo.clone(),
            prefix_cache.clone(),
        ));

        Self {
            discovery,
            resolution,
            routing,
            session_overrides,
        }
    }

    // Delegate to FeatureDiscoveryService
    pub async fn discover_and_cache(
        &self,
        space_id: &str,
        server_id: &str,
        client: &McpClient,
    ) -> Result<CachedFeatures> {
        self.discovery
            .discover_and_cache(space_id, server_id, client)
            .await
    }

    pub async fn mark_unavailable(&self, space_id: &str, server_id: &str) -> Result<()> {
        self.discovery.mark_unavailable(space_id, server_id).await
    }

    pub async fn delete_for_server(&self, space_id: &str, server_id: &str) -> Result<()> {
        self.discovery.delete_for_server(space_id, server_id).await
    }

    // Delegate to FeatureResolutionService
    pub async fn resolve_feature_sets(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
    ) -> Result<Vec<ServerFeature>> {
        self.resolution
            .resolve_feature_sets(space_id, feature_set_ids, None)
            .await
    }

    /// Get all available features for a space (optionally filtered by type)
    pub async fn get_all_features_for_space(
        &self,
        space_id: &str,
        filter_type: Option<FeatureType>,
    ) -> Result<Vec<ServerFeature>> {
        self.resolution
            .get_all_features_for_space(space_id, filter_type)
            .await
    }

    /// Catalog tools in the Space that require binding a FeatureSet before invoke.
    pub async fn list_inactive_discovery_tools(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<InactiveDiscoveryEntry>> {
        let invokable = self
            .get_invokable_tools_for_grants(space_id, feature_set_ids, session_id)
            .await?;
        let invokable_keys: HashSet<(String, String)> = invokable
            .iter()
            .filter(|f| f.feature_type == FeatureType::Tool)
            .map(|f| (f.server_id.clone(), f.feature_name.clone()))
            .collect();
        self.resolution
            .list_inactive_tools_for_discovery(space_id, &invokable_keys)
            .await
    }

    /// Resolve granted feature sets to tools invokable via search/invoke ACL.
    pub async fn get_invokable_tools_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        self.get_features_for_grants(
            space_id,
            feature_set_ids,
            session_id,
            Some(FeatureType::Tool),
        )
        .await
    }

    /// Tools promoted into client `tools/list` (surfaced backend tools only).
    pub async fn get_advertised_tools_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        if feature_set_ids.is_empty() {
            return Ok(Vec::new());
        }

        let invokable = self
            .get_invokable_tools_for_grants(space_id, feature_set_ids, session_id)
            .await?;
        let surfaced_ids = self
            .resolution
            .resolve_surfaced_feature_ids(feature_set_ids)
            .await?;

        Ok(invokable
            .into_iter()
            .filter(|f| surfaced_ids.contains(&f.id.to_string()))
            .collect())
    }

    /// Resolve granted feature sets to tools, applying session server overrides.
    pub async fn get_tools_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        self.get_invokable_tools_for_grants(space_id, feature_set_ids, session_id)
            .await
    }

    /// Resolve granted feature sets to prompts, applying session server overrides.
    pub async fn get_prompts_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        self.get_features_for_grants(
            space_id,
            feature_set_ids,
            session_id,
            Some(FeatureType::Prompt),
        )
        .await
    }

    /// Resolve granted feature sets to resources readable via search/read ACL.
    pub async fn get_readable_resources_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        self.get_resources_for_grants(space_id, feature_set_ids, session_id)
            .await
    }

    /// Resources promoted into client `resources/list` (surfaced only).
    pub async fn get_advertised_resources_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        if feature_set_ids.is_empty() {
            return Ok(Vec::new());
        }

        let readable = self
            .get_readable_resources_for_grants(space_id, feature_set_ids, session_id)
            .await?;
        let surfaced_ids = self
            .resolution
            .resolve_surfaced_feature_ids(feature_set_ids)
            .await?;

        Ok(readable
            .into_iter()
            .filter(|f| surfaced_ids.contains(&f.id.to_string()))
            .collect())
    }

    /// Resolve granted feature sets to prompts fetchable via search/fetch ACL.
    pub async fn get_fetchable_prompts_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        self.get_prompts_for_grants(space_id, feature_set_ids, session_id)
            .await
    }

    /// Prompts promoted into client `prompts/list` (surfaced only).
    pub async fn get_advertised_prompts_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        if feature_set_ids.is_empty() {
            return Ok(Vec::new());
        }

        let fetchable = self
            .get_fetchable_prompts_for_grants(space_id, feature_set_ids, session_id)
            .await?;
        let surfaced_ids = self
            .resolution
            .resolve_surfaced_feature_ids(feature_set_ids)
            .await?;

        Ok(fetchable
            .into_iter()
            .filter(|f| surfaced_ids.contains(&f.id.to_string()))
            .collect())
    }

    /// Resolve granted feature sets to resources, applying session server overrides.
    pub async fn get_resources_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
    ) -> Result<Vec<ServerFeature>> {
        self.get_features_for_grants(
            space_id,
            feature_set_ids,
            session_id,
            Some(FeatureType::Resource),
        )
        .await
    }

    /// Shared list materialization: binding FS resolution + session overrides.
    async fn get_features_for_grants(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        session_id: Option<&str>,
        filter_type: Option<FeatureType>,
    ) -> Result<Vec<ServerFeature>> {
        let binding_features = self
            .resolution
            .resolve_feature_sets(space_id, feature_set_ids, filter_type.clone())
            .await?;

        let Some(session_id) = session_id else {
            return Ok(binding_features);
        };

        let enabled = self.session_overrides.enabled_set(session_id);
        let disabled = self.session_overrides.disabled_set(session_id);

        if enabled.is_empty() && disabled.is_empty() {
            return Ok(binding_features);
        }

        let mut result: Vec<ServerFeature> = binding_features
            .into_iter()
            .filter(|f| !disabled.contains(&f.server_id))
            .collect();

        let binding_server_ids: HashSet<String> =
            result.iter().map(|f| f.server_id.clone()).collect();

        if feature_set_ids.is_empty() {
            let mut active_servers = binding_server_ids;
            active_servers.extend(
                enabled
                    .iter()
                    .filter(|server_id| !disabled.contains(*server_id))
                    .cloned(),
            );

            if active_servers.is_empty() {
                return Ok(Vec::new());
            }

            let all_features = self
                .resolution
                .get_all_features_for_space(space_id, filter_type)
                .await?;

            return Ok(all_features
                .into_iter()
                .filter(|f| f.is_available && active_servers.contains(&f.server_id))
                .collect());
        }

        let extra_enabled: HashSet<String> = enabled
            .iter()
            .filter(|server_id| {
                !disabled.contains(*server_id) && !binding_server_ids.contains(*server_id)
            })
            .cloned()
            .collect();

        if extra_enabled.is_empty() {
            return Ok(result);
        }

        let all_features = self
            .resolution
            .get_all_features_for_space(space_id, filter_type)
            .await?;

        result.extend(
            all_features
                .into_iter()
                .filter(|f| f.is_available && extra_enabled.contains(&f.server_id)),
        );

        Ok(result)
    }

    // Delegate to FeatureRoutingService (with type-specific helpers)
    pub async fn find_server_for_qualified_tool(
        &self,
        space_id: &str,
        qualified_name: &str,
    ) -> Result<Option<(String, String)>> {
        self.routing
            .find_server_for_qualified_feature(space_id, qualified_name, FeatureType::Tool)
            .await
    }

    pub async fn find_server_for_qualified_prompt(
        &self,
        space_id: &str,
        qualified_name: &str,
    ) -> Result<Option<(String, String)>> {
        self.routing
            .find_server_for_qualified_feature(space_id, qualified_name, FeatureType::Prompt)
            .await
    }

    /// Find server for a resource by its URI (not prefixed)
    ///
    /// Resources use URIs which are already namespaced. When clone servers share
    /// URIs with their parent, this Space-wide lookup is ambiguous — prefer
    /// [`Self::resolve_resource_server_from_grants`] on read paths.
    pub async fn find_server_for_resource(
        &self,
        space_id: &str,
        uri: &str,
    ) -> Result<Option<String>> {
        self.routing
            .find_server_for_resource_uri(space_id, uri)
            .await
    }

    /// Resolve the owning server for `uri` among grant-visible readable resources.
    ///
    /// Clone servers can expose the same URI as their parent; grant-scoped
    /// resolution ensures reads route to the bound clone, not an inactive parent.
    pub fn resolve_resource_server_from_grants(
        readable: &[ServerFeature],
        uri: &str,
    ) -> Option<String> {
        readable
            .iter()
            .find(|f| {
                f.feature_type == FeatureType::Resource && f.feature_name == uri && f.is_available
            })
            .map(|f| f.server_id.clone())
    }

    // === Helper methods for MCP handler ===

    /// Parse qualified tool name into (server_id, tool_name)
    pub async fn parse_qualified_tool_name(
        &self,
        space_id: &str,
        qualified_name: &str,
    ) -> Result<(String, String)> {
        self.routing
            .parse_qualified_tool_name(space_id, qualified_name)
            .await
    }

    /// Parse qualified prompt name into (server_id, prompt_name)
    pub async fn parse_qualified_prompt_name(
        &self,
        space_id: &str,
        qualified_name: &str,
    ) -> Result<(String, String)> {
        self.routing
            .parse_qualified_prompt_name(space_id, qualified_name)
            .await
    }
}
