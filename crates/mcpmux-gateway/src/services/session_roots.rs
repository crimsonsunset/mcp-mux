//! Session-scoped registry of MCP workspace roots.
//!
//! When a client declares the `roots` capability on `initialize`, the gateway
//! calls `roots/list` via the peer and stashes the result here keyed by the
//! client's `mcp-session-id`. The `FeatureSetResolverService` consults this
//! registry to pick a workspace binding.
//!
//! Roots are stored already-normalized (via
//! [`mcpmux_core::normalize_workspace_root`]) so the resolver doesn't need to
//! re-normalize on every lookup.

use std::sync::Arc;

use dashmap::DashMap;
use mcpmux_core::normalize_workspace_root;

/// Thread-safe registry mapping `mcp-session-id` to the caller's reported
/// workspace roots.
#[derive(Debug, Default)]
pub struct SessionRootsRegistry {
    map: DashMap<String, Vec<String>>,
}

impl SessionRootsRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            map: DashMap::new(),
        })
    }

    /// Store the reported roots for a session. `roots` should already be
    /// absolute paths or `file://` URIs — we normalize them before storing.
    pub fn set<I, S>(&self, session_id: impl Into<String>, roots: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let normalized: Vec<String> = roots
            .into_iter()
            .map(|r| normalize_workspace_root(r.as_ref()))
            .filter(|r| !r.is_empty())
            .collect();
        self.map.insert(session_id.into(), normalized);
    }

    /// Retrieve the (already-normalized) roots for a session, if any.
    pub fn get(&self, session_id: &str) -> Option<Vec<String>> {
        self.map.get(session_id).map(|v| v.clone())
    }

    /// Drop a session's roots — call on client disconnect.
    pub fn remove(&self, session_id: &str) {
        self.map.remove(session_id);
    }

    /// Current number of tracked sessions. Test helper; cheap to call but
    /// not useful in hot paths.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_normalizes_and_filters_empty() {
        let reg = SessionRootsRegistry::default();
        reg.set(
            "sess-1",
            [
                #[cfg(windows)]
                "file:///D:/proj/",
                #[cfg(not(windows))]
                "file:///home/user/proj/",
                "",
            ],
        );
        let roots = reg.get("sess-1").unwrap();
        assert_eq!(roots.len(), 1);
        #[cfg(windows)]
        assert_eq!(roots[0], "d:\\proj");
        #[cfg(not(windows))]
        assert_eq!(roots[0], "/home/user/proj");
    }

    #[test]
    fn test_remove() {
        let reg = SessionRootsRegistry::default();
        reg.set("sess-1", ["/a"]);
        assert_eq!(reg.len(), 1);
        reg.remove("sess-1");
        assert_eq!(reg.len(), 0);
    }
}
