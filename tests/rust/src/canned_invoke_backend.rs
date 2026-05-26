//! Canned invoke backend for integration tests.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use mcpmux_gateway::{InvokeToolBackend, ToolCallResult};
use serde_json::Value;
use uuid::Uuid;

/// Returns predetermined tool results keyed by qualified tool name.
pub struct CannedInvokeBackend {
    responses: HashMap<String, ToolCallResult>,
}

impl CannedInvokeBackend {
    /// Create an empty canned backend.
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    /// Register a response for a qualified tool name.
    pub fn with_response(
        mut self,
        qualified_name: impl Into<String>,
        result: ToolCallResult,
    ) -> Self {
        self.responses.insert(qualified_name.into(), result);
        self
    }

    /// Wrap as a trait object for registry wiring.
    pub fn into_arc(self) -> Arc<dyn InvokeToolBackend> {
        Arc::new(self)
    }
}

impl Default for CannedInvokeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InvokeToolBackend for CannedInvokeBackend {
    async fn call_tool(
        &self,
        _space_id: Uuid,
        _feature_set_ids: &[String],
        _session_id: Option<&str>,
        qualified_name: &str,
        _arguments: Value,
    ) -> Result<ToolCallResult> {
        self.responses
            .get(qualified_name)
            .cloned()
            .ok_or_else(|| anyhow!("no canned response for {qualified_name}"))
    }
}
