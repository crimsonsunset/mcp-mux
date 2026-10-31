//! Serializable approval payload types for meta-tool write dialogs.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Payload delivered to the desktop UI so it can render a meaningful dialog.
///
/// Keep this narrow and JSON-serializable — it crosses the Tauri boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPayload {
    pub tool_name: String,
    /// Human summary the dialog puts above the diff. e.g.
    /// "Pin this connection to FeatureSet 'android-dev' (12 tools)".
    pub summary: String,
    /// Name of the Space this write targets, surfaced as a labeled chip so the
    /// user can see (and reject) a change aimed at a Space other than the one
    /// they expect. `None` for writes with no single target Space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_name: Option<String>,
    /// Id of the Space this write targets. "Always allow" grants are scoped
    /// to it, so approving writes to one Space never approves another.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_id: Option<Uuid>,
    /// Tool-list diff the dialog shows to make the change concrete.
    /// Optional because some writes (e.g. create_feature_set without
    /// activation) don't shift the caller's resolved toolset.
    pub diff: Option<serde_json::Value>,
    /// The tool's `action` argument (e.g. create / update / delete for
    /// `mcpmux_manage_feature_set`). "Always allow" grants are scoped to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Whether the dialog may offer "Always allow". Set by the broker: never
    /// for workspace binding, deletes, or tokenless connections.
    #[serde(default)]
    pub allow_always: bool,
    /// Raw arguments the LLM supplied; shown verbatim for auditability.
    ///
    /// TODO(redaction): no write meta tool accepts credential-bearing args
    /// today, so rendering this unredacted in the dialog / SSE is safe. If a
    /// future write tool takes secrets, add a per-tool redaction allowlist
    /// before this payload crosses the Tauri / admin-SSE boundary.
    pub raw_args: serde_json::Value,
    /// Does this change affect clients other than the caller? Dictates
    /// whether the dialog shows the "also affects other connections" warning.
    pub affects_other_clients: bool,
}

/// Data the broker hands to whoever listens for approval requests.
#[derive(Debug, Clone, Serialize)]
pub struct ApprovalRequest {
    pub request_id: String,
    pub client_id: String,
    pub payload: ApprovalPayload,
    /// UNIX seconds at which this request will time out if no response.
    pub expires_at_unix_secs: u64,
}
