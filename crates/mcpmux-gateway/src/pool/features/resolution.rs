//! Feature Resolution Service - SRP: Feature set resolution & permissions

use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::debug;

use crate::services::PrefixCacheService;
use mcpmux_core::{
    FeatureSet, FeatureSetRepository, FeatureType, MemberMode, MemberType, ServerFeature,
    ServerFeatureRepository,
};

/// A catalog tool visible in discovery but not invokable until its FeatureSet is bound.
#[derive(Debug, Clone)]
pub struct InactiveDiscoveryEntry {
    pub feature: ServerFeature,
    pub bindable_feature_set_id: String,
}

/// Helper to apply include/exclude mode (DRY)
fn apply_mode_to_set(
    mode: MemberMode,
    feature_ids: impl Iterator<Item = String>,
    allowed: &mut HashSet<String>,
    excluded: &mut HashSet<String>,
) {
    match mode {
        MemberMode::Include => allowed.extend(feature_ids),
        MemberMode::Exclude => excluded.extend(feature_ids),
    }
}

/// Handles feature set resolution and permission evaluation
pub struct FeatureResolutionService {
    feature_repo: Arc<dyn ServerFeatureRepository>,
    feature_set_repo: Arc<dyn FeatureSetRepository>,
    prefix_cache: Arc<PrefixCacheService>,
}

impl FeatureResolutionService {
    pub fn new(
        feature_repo: Arc<dyn ServerFeatureRepository>,
        feature_set_repo: Arc<dyn FeatureSetRepository>,
        prefix_cache: Arc<PrefixCacheService>,
    ) -> Self {
        Self {
            feature_repo,
            feature_set_repo,
            prefix_cache,
        }
    }

    /// Get all available features for a space (optionally filtered by type)
    pub async fn get_all_features_for_space(
        &self,
        space_id: &str,
        filter_type: Option<FeatureType>,
    ) -> Result<Vec<ServerFeature>> {
        let all_features = self.feature_repo.list_for_space(space_id).await?;

        let mut result: Vec<ServerFeature> = all_features
            .into_iter()
            .filter(|f| f.is_available)
            .collect();

        if let Some(feature_type) = filter_type {
            result.retain(|f| f.feature_type == feature_type);
        }

        // Enrich with prefixes
        for feature in &mut result {
            let prefix = self
                .prefix_cache
                .get_prefix_for_server(space_id, &feature.server_id)
                .await;
            feature.server_alias = Some(prefix);
        }

        Ok(result)
    }

    /// Resolve feature set IDs to actual features (with optional type filter)
    pub async fn resolve_feature_sets(
        &self,
        space_id: &str,
        feature_set_ids: &[String],
        filter_type: Option<FeatureType>,
    ) -> Result<Vec<ServerFeature>> {
        let mut allowed_feature_ids: HashSet<String> = HashSet::new();
        let mut excluded_feature_ids: HashSet<String> = HashSet::new();

        let all_features = self.feature_repo.list_for_space(space_id).await?;

        debug!(
            "[FeatureResolution] Resolving {} feature sets for space {}",
            feature_set_ids.len(),
            space_id
        );

        for fs_id in feature_set_ids {
            let feature_set = match self.feature_set_repo.get_with_members(fs_id).await? {
                Some(fs) => {
                    debug!(
                        "[FeatureResolution] Found feature set: id={}, type={:?}, server_id={:?}",
                        fs.id, fs.feature_set_type, fs.server_id
                    );
                    fs
                }
                None => {
                    debug!("[FeatureResolution] FeatureSet {} not found", fs_id);
                    continue;
                }
            };

            // Both Default and Custom sets use explicit members; the
            // resolution is identical — walk the members and build up
            // allow/exclude sets.
            self.resolve_members(
                &feature_set,
                &all_features,
                &mut allowed_feature_ids,
                &mut excluded_feature_ids,
            )
            .await?;
        }

        debug!(
            "[FeatureResolution] Filtering: all_features={}, allowed_ids={}, excluded_ids={}",
            all_features.len(),
            allowed_feature_ids.len(),
            excluded_feature_ids.len()
        );

        let mut result: Vec<ServerFeature> = all_features
            .into_iter()
            .filter(|f| {
                let in_allowed = allowed_feature_ids.contains(&f.id.to_string());
                let in_excluded = excluded_feature_ids.contains(&f.id.to_string());
                let passes = f.is_available && in_allowed && !in_excluded;
                if !passes && in_allowed {
                    debug!(
                        "[FeatureResolution] Feature {} (server={}) filtered out: is_available={}, in_allowed={}, in_excluded={}",
                        f.feature_name, f.server_id, f.is_available, in_allowed, in_excluded
                    );
                }
                passes
            })
            .collect();

        debug!(
            "[FeatureResolution] After filter: {} features",
            result.len()
        );

        // Apply type filter if specified (OCP)
        if let Some(feature_type) = filter_type {
            result.retain(|f| f.feature_type == feature_type);
        }

        // Enrich with prefixes
        for feature in &mut result {
            let prefix = self
                .prefix_cache
                .get_prefix_for_server(space_id, &feature.server_id)
                .await;
            feature.server_alias = Some(prefix);
        }

        Ok(result)
    }

    /// Collect feature IDs marked `surfaced: true` across the given FeatureSets.
    pub async fn resolve_surfaced_feature_ids(
        &self,
        feature_set_ids: &[String],
    ) -> Result<HashSet<String>> {
        let mut surfaced = HashSet::new();
        for fs_id in feature_set_ids {
            let Some(feature_set) = self.feature_set_repo.get_with_members(fs_id).await? else {
                continue;
            };
            self.collect_surfaced_members(&feature_set, &mut surfaced)
                .await?;
        }
        Ok(surfaced)
    }

    async fn collect_surfaced_members(
        &self,
        feature_set: &FeatureSet,
        surfaced: &mut HashSet<String>,
    ) -> Result<()> {
        for member in &feature_set.members {
            match member.member_type {
                MemberType::Feature => {
                    if member.mode == MemberMode::Include && member.surfaced {
                        surfaced.insert(member.member_id.clone());
                    }
                }
                MemberType::FeatureSet => {
                    if let Some(nested_fs) = self
                        .feature_set_repo
                        .get_with_members(&member.member_id)
                        .await?
                    {
                        Box::pin(self.collect_surfaced_members(&nested_fs, surfaced)).await?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Tools granted by some FeatureSet in the Space but not in `invokable_keys`.
    ///
    /// Used by meta-tool discovery (`include_inactive`); first matching FeatureSet
    /// wins when multiple bundles contain the same tool.
    pub async fn list_inactive_tools_for_discovery(
        &self,
        space_id: &str,
        invokable_keys: &HashSet<(String, String)>,
    ) -> Result<Vec<InactiveDiscoveryEntry>> {
        let mut by_key: std::collections::HashMap<(String, String), InactiveDiscoveryEntry> =
            std::collections::HashMap::new();

        let sets = self.feature_set_repo.list_by_space(space_id).await?;
        let mut sets: Vec<_> = sets.into_iter().filter(|fs| !fs.is_deleted).collect();
        // Prefer custom bundles over the auto-seeded Default when both grant the same tool.
        sets.sort_by(|a, b| {
            a.is_builtin
                .cmp(&b.is_builtin)
                .then_with(|| a.name.cmp(&b.name))
        });

        for fs in sets {
            let tools = self
                .resolve_feature_sets(
                    space_id,
                    std::slice::from_ref(&fs.id),
                    Some(FeatureType::Tool),
                )
                .await?;
            for feature in tools {
                let key = (feature.server_id.clone(), feature.feature_name.clone());
                if invokable_keys.contains(&key) {
                    continue;
                }
                by_key.entry(key).or_insert(InactiveDiscoveryEntry {
                    feature,
                    bindable_feature_set_id: fs.id.clone(),
                });
            }
        }

        let mut entries: Vec<_> = by_key.into_values().collect();
        entries.sort_by_key(|entry| entry.feature.qualified_name());
        Ok(entries)
    }

    async fn resolve_members(
        &self,
        feature_set: &FeatureSet,
        all_features: &[ServerFeature],
        allowed: &mut HashSet<String>,
        excluded: &mut HashSet<String>,
    ) -> Result<()> {
        for member in &feature_set.members {
            match member.member_type {
                MemberType::Feature => {
                    apply_mode_to_set(
                        member.mode,
                        std::iter::once(member.member_id.clone()),
                        allowed,
                        excluded,
                    );
                }
                MemberType::FeatureSet => {
                    // Composition: recurse into the nested FS, walking its
                    // members the same way. Both Default and Custom sets
                    // are purely member-driven now.
                    if let Some(nested_fs) = self
                        .feature_set_repo
                        .get_with_members(&member.member_id)
                        .await?
                    {
                        Box::pin(self.resolve_members(&nested_fs, all_features, allowed, excluded))
                            .await?;
                    }
                }
            }
        }
        Ok(())
    }
}
