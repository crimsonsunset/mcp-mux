//! Canned disclosure backend for integration tests.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use mcpmux_gateway::services::meta_tools::DisclosureBackend;
use serde_json::{Map, Value};
use uuid::Uuid;

/// Returns predetermined resource reads and prompt fetches keyed by server + name.
pub struct CannedDisclosureBackend {
    resource_reads: HashMap<(String, String), Vec<Value>>,
    prompt_fetches: HashMap<(String, String), Value>,
}

impl CannedDisclosureBackend {
    /// Create an empty canned disclosure backend.
    pub fn new() -> Self {
        Self {
            resource_reads: HashMap::new(),
            prompt_fetches: HashMap::new(),
        }
    }

    /// Register a canned resource read for `(server_id, uri)`.
    pub fn with_resource_read(
        mut self,
        server_id: impl Into<String>,
        uri: impl Into<String>,
        contents: Vec<Value>,
    ) -> Self {
        self.resource_reads
            .insert((server_id.into(), uri.into()), contents);
        self
    }

    /// Wrap as a trait object for registry wiring.
    pub fn into_arc(self) -> Arc<dyn DisclosureBackend> {
        Arc::new(self)
    }
}

impl Default for CannedDisclosureBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DisclosureBackend for CannedDisclosureBackend {
    async fn read_resource(
        &self,
        _space_id: Uuid,
        server_id: &str,
        uri: &str,
    ) -> Result<Vec<Value>> {
        self.resource_reads
            .get(&(server_id.to_string(), uri.to_string()))
            .cloned()
            .ok_or_else(|| anyhow!("no canned read for {server_id} {uri}"))
    }

    async fn fetch_prompt(
        &self,
        _space_id: Uuid,
        server_id: &str,
        prompt_name: &str,
        _arguments: Option<Map<String, Value>>,
    ) -> Result<Value> {
        self.prompt_fetches
            .get(&(server_id.to_string(), prompt_name.to_string()))
            .cloned()
            .ok_or_else(|| anyhow!("no canned prompt for {server_id} {prompt_name}"))
    }
}
