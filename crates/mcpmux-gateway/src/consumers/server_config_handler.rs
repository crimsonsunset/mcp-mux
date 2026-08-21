//! Server Config Event Handler - Reconnects pool instances on config changes
//!
//! Listens for `ServerConfigUpdated` and runs `reconnect_fresh` for
//! enabled servers so the next call uses transport rebuilt from DB.

use mcpmux_core::{DomainEvent, InstalledServerRepository};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::pool::transport::resolution::resolve_auto_connection_context;
use crate::pool::{ConnectionResult, PoolService};

/// Evicts and reconnects pooled server instances when configuration changes.
pub struct ServerConfigUpdatedHandler {
    installed_server_repo: Arc<dyn InstalledServerRepository + Send + Sync>,
    pool_service: Arc<PoolService>,
    state_dir: Option<PathBuf>,
}

impl ServerConfigUpdatedHandler {
    /// Create a handler wired to the installed-server repo, connection pool,
    /// and optional MCP state dir used when re-resolving transport.
    pub fn new(
        installed_server_repo: Arc<dyn InstalledServerRepository + Send + Sync>,
        pool_service: Arc<PoolService>,
        state_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            installed_server_repo,
            pool_service,
            state_dir,
        }
    }

    /// Start listening to domain events on a background task.
    pub fn start(self: Arc<Self>, mut event_rx: broadcast::Receiver<DomainEvent>) {
        tokio::spawn(async move {
            info!("[ServerConfigHandler] Started listening for ServerConfigUpdated events");

            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        if let Err(error) = self.handle_event(event).await {
                            warn!("[ServerConfigHandler] Failed to handle event: {error}");
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("[ServerConfigHandler] Lagged behind, skipped {skipped} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("[ServerConfigHandler] Event channel closed");
                        break;
                    }
                }
            }
        });
    }

    /// Handle one domain event, reconnecting the pool instance when applicable.
    async fn handle_event(&self, event: DomainEvent) -> anyhow::Result<()> {
        let DomainEvent::ServerConfigUpdated {
            space_id,
            server_id,
        } = event
        else {
            return Ok(());
        };

        self.handle_config_updated(space_id, &server_id).await
    }

    /// Reconnect an enabled server from current DB config after a write.
    ///
    /// Resolve failure falls back to evict-only so a stale instance cannot
    /// keep serving the pre-save transport.
    async fn handle_config_updated(&self, space_id: Uuid, server_id: &str) -> anyhow::Result<()> {
        let space_id_str = space_id.to_string();
        let Some(installed) = self
            .installed_server_repo
            .get_by_server_id(&space_id_str, server_id)
            .await?
        else {
            debug!(
                "[ServerConfigHandler] Server {space_id}/{server_id} not found, skipping eviction"
            );
            return Ok(());
        };

        if !installed.enabled {
            debug!(
                "[ServerConfigHandler] Skipping eviction for disabled server {space_id}/{server_id}"
            );
            return Ok(());
        }

        let started = Instant::now();
        self.drop_stale_features(space_id, server_id).await;
        match resolve_auto_connection_context(
            self.installed_server_repo.as_ref(),
            self.state_dir.as_deref(),
            space_id,
            server_id,
        )
        .await
        {
            Ok(ctx) => {
                let result = self.pool_service.reconnect_fresh(&ctx).await;
                info!(
                    server_id = %server_id,
                    space_id = %space_id,
                    ok = result.is_connected(),
                    duration_ms = started.elapsed().as_millis(),
                    "[ServerConfigHandler] reconnect_fresh after config update"
                );
                if let ConnectionResult::Failed { error } = result {
                    warn!(
                        server_id = %server_id,
                        space_id = %space_id,
                        error = %error,
                        "[ServerConfigHandler] reconnect_fresh failed after config update"
                    );
                }
            }
            Err(error) => {
                warn!(
                    server_id = %server_id,
                    space_id = %space_id,
                    error = %error,
                    "[ServerConfigHandler] re-resolve failed, evicting only"
                );
                self.pool_service.remove_instance(space_id, server_id);
            }
        }
        Ok(())
    }

    /// Flip cached features off and drop the resolution cache before reconnect.
    ///
    /// A successful discover writes `is_available` back. Without this, a failed
    /// `reconnect_fresh` evicts the instance while `tools/list` still advertises
    /// the pre-save rows.
    async fn drop_stale_features(&self, space_id: Uuid, server_id: &str) {
        if let Err(error) = self
            .pool_service
            .feature_service()
            .mark_unavailable(&space_id.to_string(), server_id)
            .await
        {
            warn!(
                server_id = %server_id,
                space_id = %space_id,
                error = %error,
                "[ServerConfigHandler] mark_unavailable failed before reconnect"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::PoolService;
    use async_trait::async_trait;
    use chrono::Utc;
    use mcpmux_core::{InstalledServer, ServerDefinition, TransportConfig};
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MapInstalledRepo {
        servers: Mutex<HashMap<(String, String), InstalledServer>>,
    }

    impl MapInstalledRepo {
        /// Store one installed-server row keyed by `(space_id, server_id)`.
        fn with(server: InstalledServer) -> Arc<Self> {
            let mut servers = HashMap::new();
            servers.insert((server.space_id.clone(), server.server_id.clone()), server);
            Arc::new(Self {
                servers: Mutex::new(servers),
            })
        }
    }

    #[async_trait]
    impl InstalledServerRepository for MapInstalledRepo {
        async fn list(&self) -> mcpmux_core::repository::RepoResult<Vec<InstalledServer>> {
            Ok(self.servers.lock().unwrap().values().cloned().collect())
        }
        async fn list_for_space(
            &self,
            _space_id: &str,
        ) -> mcpmux_core::repository::RepoResult<Vec<InstalledServer>> {
            Ok(vec![])
        }
        async fn list_by_source_file(
            &self,
            _file_path: &std::path::Path,
        ) -> mcpmux_core::repository::RepoResult<Vec<InstalledServer>> {
            Ok(vec![])
        }
        async fn get(
            &self,
            _id: &Uuid,
        ) -> mcpmux_core::repository::RepoResult<Option<InstalledServer>> {
            Ok(None)
        }
        async fn get_by_server_id(
            &self,
            space_id: &str,
            server_id: &str,
        ) -> mcpmux_core::repository::RepoResult<Option<InstalledServer>> {
            Ok(self
                .servers
                .lock()
                .unwrap()
                .get(&(space_id.to_string(), server_id.to_string()))
                .cloned())
        }
        async fn install(
            &self,
            _server: &InstalledServer,
        ) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
        async fn update(
            &self,
            _server: &InstalledServer,
        ) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
        async fn uninstall(&self, _id: &Uuid) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
        async fn list_enabled(
            &self,
            _space_id: &str,
        ) -> mcpmux_core::repository::RepoResult<Vec<InstalledServer>> {
            Ok(vec![])
        }
        async fn list_enabled_all(
            &self,
        ) -> mcpmux_core::repository::RepoResult<Vec<InstalledServer>> {
            Ok(vec![])
        }
        async fn set_enabled(
            &self,
            _id: &Uuid,
            _enabled: bool,
        ) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
        async fn set_oauth_connected(
            &self,
            _id: &Uuid,
            _connected: bool,
        ) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
        async fn update_inputs(
            &self,
            _id: &Uuid,
            _input_values: HashMap<String, String>,
        ) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
        async fn update_cached_definition(
            &self,
            _id: &Uuid,
            _server_name: Option<String>,
            _cached_definition: Option<String>,
        ) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
        async fn set_display_name_override(
            &self,
            _id: &Uuid,
            _value: Option<String>,
        ) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
        async fn update_version_cache(
            &self,
            _id: &Uuid,
            _latest_available_version: Option<String>,
            _current_version: Option<String>,
            _version_checked_at: chrono::DateTime<Utc>,
        ) -> mcpmux_core::repository::RepoResult<()> {
            Ok(())
        }
    }

    /// Minimal stdio definition whose command does not exist on disk.
    fn stdio_definition(server_id: &str) -> ServerDefinition {
        ServerDefinition {
            id: server_id.to_string(),
            name: server_id.to_string(),
            description: None,
            alias: None,
            auth: None,
            icon: None,
            transport: TransportConfig::Stdio {
                command: "/nonexistent/mcpmux-config-handler-test".into(),
                args: vec![],
                env: HashMap::new(),
                metadata: Default::default(),
            },
            categories: vec![],
            publisher: None,
            source: Default::default(),
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

    #[tokio::test]
    async fn enabled_server_reconnects_and_evicts_stale_instance() {
        let space_id = Uuid::new_v4();
        let server_id = "cfg-enabled";
        let installed = InstalledServer::new(space_id.to_string(), server_id)
            .with_definition(&stdio_definition(server_id))
            .with_enabled(true);
        let repo = MapInstalledRepo::with(installed);
        let pool = Arc::new(PoolService::new_test_with_repo(repo.clone()));
        pool.insert_test_instance(space_id, server_id);
        assert_eq!(pool.stats().total_instances, 1);

        let handler = ServerConfigUpdatedHandler::new(repo, pool.clone(), None);
        handler
            .handle_config_updated(space_id, server_id)
            .await
            .expect("handler");

        assert!(
            pool.get_instance(space_id, server_id).is_none(),
            "stale instance must be gone after reconnect_fresh"
        );
    }

    #[tokio::test]
    async fn disabled_server_is_left_in_the_pool() {
        let space_id = Uuid::new_v4();
        let server_id = "cfg-disabled";
        let installed = InstalledServer::new(space_id.to_string(), server_id)
            .with_definition(&stdio_definition(server_id))
            .with_enabled(false);
        let repo = MapInstalledRepo::with(installed);
        let pool = Arc::new(PoolService::new_test_with_repo(repo.clone()));
        pool.insert_test_instance(space_id, server_id);

        let handler = ServerConfigUpdatedHandler::new(repo, pool.clone(), None);
        handler
            .handle_config_updated(space_id, server_id)
            .await
            .expect("handler");

        assert!(pool.get_instance(space_id, server_id).is_some());
    }
}
