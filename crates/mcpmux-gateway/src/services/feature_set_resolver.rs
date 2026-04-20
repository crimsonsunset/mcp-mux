//! FeatureSet Resolver Service (V2 — pin > workspace > space active).
//!
//! Replaces the per-client-grants lookup in [`super::AuthorizationService`]
//! with a single deterministic resolution:
//!
//! ```text
//! resolve(client, session_id):
//!     if client.pinned_feature_set_id:       source = Pin
//!     else if workspace binding matches:     source = WorkspaceBinding
//!     else:                                  source = SpaceActive (may be None = Deny)
//! ```
//!
//! The service runs alongside `AuthorizationService` in **shadow mode**: the
//! gateway still honours the legacy grants path for now, but the resolver's
//! decision is logged on every call so we can verify divergence before
//! flipping the switch.

use std::sync::Arc;

use anyhow::Result;
use mcpmux_core::{InboundMcpClientRepository, SpaceRepository, WorkspaceBindingRepository};
use serde::Serialize;
use tracing::{debug, warn};
use uuid::Uuid;

use super::session_roots::SessionRootsRegistry;

/// Why the resolver picked the FS it picked (or didn't pick one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionSource {
    /// Client's `pinned_feature_set_id` was set.
    Pin,
    /// A [`WorkspaceBinding`](mcpmux_core::WorkspaceBinding) matched one of
    /// the session's reported MCP roots.
    WorkspaceBinding,
    /// No pin and no workspace match; fell through to `Space.active_feature_set_id`.
    SpaceActive,
    /// No pin, no workspace match, and the Space has no active FS — deny.
    Deny,
}

/// Output of [`FeatureSetResolverService::resolve`].
#[derive(Debug, Clone)]
pub struct ResolvedFeatureSet {
    /// Chosen FeatureSet id, or `None` when `source == Deny`.
    pub feature_set_id: Option<Uuid>,
    pub source: ResolutionSource,
}

/// Resolves which FeatureSet applies for a given (client, session).
///
/// Cheap to clone via `Arc`; inject one instance into the gateway's service
/// container and reuse across requests.
pub struct FeatureSetResolverService {
    client_repo: Arc<dyn InboundMcpClientRepository>,
    space_repo: Arc<dyn SpaceRepository>,
    binding_repo: Arc<dyn WorkspaceBindingRepository>,
    session_roots: Arc<SessionRootsRegistry>,
}

impl FeatureSetResolverService {
    pub fn new(
        client_repo: Arc<dyn InboundMcpClientRepository>,
        space_repo: Arc<dyn SpaceRepository>,
        binding_repo: Arc<dyn WorkspaceBindingRepository>,
        session_roots: Arc<SessionRootsRegistry>,
    ) -> Self {
        Self {
            client_repo,
            space_repo,
            binding_repo,
            session_roots,
        }
    }

    /// Read the client's pin + the caller's reported roots + the Space's
    /// active FS, and return the winning FeatureSet with its source.
    ///
    /// `session_id` is the client's `mcp-session-id` header (or `None` for
    /// stateless callers) — used to look up reported MCP roots.
    pub async fn resolve(
        &self,
        client_id: &Uuid,
        session_id: Option<&str>,
    ) -> Result<ResolvedFeatureSet> {
        let Some(client) = self.client_repo.get(client_id).await? else {
            warn!(%client_id, "[FeatureSetResolver] client not found");
            return Ok(ResolvedFeatureSet {
                feature_set_id: None,
                source: ResolutionSource::Deny,
            });
        };

        // 1. Pin wins outright.
        if let Some(fs) = client.pinned_feature_set_id {
            debug!(%client_id, feature_set = %fs, "[FeatureSetResolver] resolved via Pin");
            return Ok(ResolvedFeatureSet {
                feature_set_id: Some(fs),
                source: ResolutionSource::Pin,
            });
        }

        // Determine which Space the caller belongs to. We prefer the explicit
        // pinned_space_id; if it's missing (legacy client pre-migration),
        // fall back to the active/default Space.
        let space_id = match client.pinned_space_id {
            Some(id) => id,
            None => match self.space_repo.get_default().await? {
                Some(s) => s.id,
                None => {
                    warn!(%client_id, "[FeatureSetResolver] no pinned_space_id and no default space");
                    return Ok(ResolvedFeatureSet {
                        feature_set_id: None,
                        source: ResolutionSource::Deny,
                    });
                }
            },
        };

        // 2. Workspace-root match, only when the session reported roots.
        if let Some(sid) = session_id {
            if let Some(roots) = self.session_roots.get(sid) {
                if !roots.is_empty() {
                    if let Some(binding) = self
                        .binding_repo
                        .find_longest_prefix_match(&space_id, &roots)
                        .await?
                    {
                        debug!(
                            %client_id,
                            session_id = sid,
                            feature_set = %binding.feature_set_id,
                            workspace_root = binding.workspace_root,
                            "[FeatureSetResolver] resolved via WorkspaceBinding",
                        );
                        return Ok(ResolvedFeatureSet {
                            feature_set_id: Some(binding.feature_set_id),
                            source: ResolutionSource::WorkspaceBinding,
                        });
                    }
                }
            }
        }

        // 3. Space active FS is the fallback.
        let space = self.space_repo.get(&space_id).await?;
        match space.and_then(|s| s.active_feature_set_id) {
            Some(fs) => {
                debug!(
                    %client_id,
                    %space_id,
                    feature_set = %fs,
                    "[FeatureSetResolver] resolved via SpaceActive",
                );
                Ok(ResolvedFeatureSet {
                    feature_set_id: Some(fs),
                    source: ResolutionSource::SpaceActive,
                })
            }
            None => {
                debug!(
                    %client_id,
                    %space_id,
                    "[FeatureSetResolver] no pin / no binding / no active FS — deny",
                );
                Ok(ResolvedFeatureSet {
                    feature_set_id: None,
                    source: ResolutionSource::Deny,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Resolver decision-table tests live in the integration test crate
    //! (`tests/rust/tests/integration/feature_set_resolver.rs`) so they can
    //! share the mock repositories with the other gateway tests.
}
