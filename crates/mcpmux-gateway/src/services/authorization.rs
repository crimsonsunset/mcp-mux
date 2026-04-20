//! Authorization Service
//!
//! Delegates permission resolution to [`FeatureSetResolverService`] (pin >
//! workspace binding > space-active FS). The old per-client grants table is
//! no longer consulted — see migration 003 for its removal.
//!
//! This service remains as a thin adapter so existing callers that take a
//! `Vec<feature_set_id>` continue to work: the resolver yields at most one
//! FeatureSet, which we wrap in a Vec.

use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use super::feature_set_resolver::FeatureSetResolverService;

/// Authorization service for checking client permissions.
///
/// Backed by [`FeatureSetResolverService`]; no longer reads the legacy
/// `client_grants` table.
pub struct AuthorizationService {
    resolver: Arc<FeatureSetResolverService>,
}

impl AuthorizationService {
    pub fn new(resolver: Arc<FeatureSetResolverService>) -> Self {
        Self { resolver }
    }

    /// Resolve the active FeatureSet for a client+session and return it as a
    /// one-element Vec (or empty when the resolver denies).
    ///
    /// `session_id` is the client's `mcp-session-id` header; pass `None` for
    /// stateless callers (workspace-binding resolution will be skipped).
    pub async fn get_client_grants(
        &self,
        client_id: &str,
        _space_id: &Uuid,
        session_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let client_uuid = Uuid::parse_str(client_id)?;
        let resolved = self.resolver.resolve(&client_uuid, session_id).await?;
        Ok(resolved
            .feature_set_id
            .map(|fs| vec![fs.to_string()])
            .unwrap_or_default())
    }

    /// Check if a client has any grants in a space
    pub async fn has_access(
        &self,
        client_id: &str,
        space_id: &Uuid,
        session_id: Option<&str>,
    ) -> Result<bool> {
        let grants = self
            .get_client_grants(client_id, space_id, session_id)
            .await?;
        Ok(!grants.is_empty())
    }

    /// Check if a client has access to a specific feature set
    pub async fn has_feature_set_access(
        &self,
        client_id: &str,
        space_id: &Uuid,
        feature_set_id: &str,
        session_id: Option<&str>,
    ) -> Result<bool> {
        let grants = self
            .get_client_grants(client_id, space_id, session_id)
            .await?;
        Ok(grants.contains(&feature_set_id.to_string()))
    }
}
