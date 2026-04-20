//! Client entity - AI clients that connect to McpMux

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Connection mode determines how a client resolves which Space to use
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectionMode {
    /// Client is locked to a specific Space
    Locked { space_id: Uuid },

    /// Client follows the currently active Space
    #[default]
    FollowActive,

    /// Prompt user when context suggests a different Space
    AskOnChange { triggers: Vec<ContextTrigger> },
}

/// Triggers for auto-suggesting Space changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContextTrigger {
    /// Match git remote URL
    GitRemote { pattern: String, space_id: Uuid },

    /// Match working directory
    Directory { pattern: String, space_id: Uuid },

    /// Match time of day
    TimeSchedule { cron: String, space_id: Uuid },
}

/// Client represents an AI client (Cursor, VS Code, Claude Desktop)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    /// Unique identifier
    pub id: Uuid,

    /// Human-readable name
    pub name: String,

    /// Client type (cursor, vscode, claude, etc.)
    pub client_type: String,

    /// How this client resolves Spaces
    #[serde(default)]
    pub connection_mode: ConnectionMode,

    /// FeatureSet grants per Space: space_id -> [feature_set_ids]
    ///
    /// Legacy field — superseded by `pinned_feature_set_id` + WorkspaceBinding.
    /// Kept while the FeatureSetResolver runs in shadow mode.
    #[serde(default)]
    pub grants: HashMap<Uuid, Vec<Uuid>>,

    /// Space this access key belongs to (chosen at approval time).
    ///
    /// Replaces the `Locked` variant of `ConnectionMode`. `None` means
    /// "follow the active Space" for legacy clients that haven't been
    /// migrated yet; new approvals always populate this.
    #[serde(default)]
    pub pinned_space_id: Option<Uuid>,

    /// FeatureSet this access key is pinned to (chosen at approval time).
    ///
    /// When `Some`, the resolver uses this FS directly. When `None`, the
    /// resolver falls through to workspace-root binding and then the
    /// Space's active FS.
    #[serde(default)]
    pub pinned_feature_set_id: Option<Uuid>,

    /// Access key for authentication (local only, never synced)
    #[serde(skip)]
    pub access_key: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Last seen timestamp
    pub last_seen: Option<DateTime<Utc>>,
}

impl Client {
    /// Create a new client
    pub fn new(name: impl Into<String>, client_type: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            client_type: client_type.into(),
            connection_mode: ConnectionMode::default(),
            grants: HashMap::new(),
            pinned_space_id: None,
            pinned_feature_set_id: None,
            access_key: None,
            created_at: now,
            updated_at: now,
            last_seen: None,
        }
    }

    /// Create a Cursor client
    pub fn cursor() -> Self {
        Self::new("Cursor", "cursor")
    }

    /// Create a VS Code client
    pub fn vscode() -> Self {
        Self::new("VS Code", "vscode")
    }

    /// Create a Claude Desktop client
    pub fn claude_desktop() -> Self {
        Self::new("Claude Desktop", "claude")
    }

    /// Set connection mode
    pub fn with_mode(mut self, mode: ConnectionMode) -> Self {
        self.connection_mode = mode;
        self
    }

    /// Grant FeatureSets for a Space
    pub fn grant(mut self, space_id: Uuid, feature_sets: Vec<Uuid>) -> Self {
        self.grants.insert(space_id, feature_sets);
        self
    }

    /// Check if client has any grants for a Space
    pub fn has_access_to(&self, space_id: &Uuid) -> bool {
        self.grants
            .get(space_id)
            .map(|g| !g.is_empty())
            .unwrap_or(false)
    }

    /// Generate a new access key
    pub fn generate_access_key(&mut self) {
        self.access_key = Some(format!("mcp_{}", Uuid::new_v4().simple()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = Client::cursor();
        assert_eq!(client.name, "Cursor");
        assert_eq!(client.client_type, "cursor");
        assert!(matches!(
            client.connection_mode,
            ConnectionMode::FollowActive
        ));
    }

    #[test]
    fn test_grants() {
        let space_id = Uuid::new_v4();
        let fs_id = Uuid::new_v4();

        let client = Client::cursor().grant(space_id, vec![fs_id]);

        assert!(client.has_access_to(&space_id));
        assert!(!client.has_access_to(&Uuid::new_v4()));
    }
}
