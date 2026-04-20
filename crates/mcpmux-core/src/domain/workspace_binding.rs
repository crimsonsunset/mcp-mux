//! WorkspaceBinding entity — maps a workspace root on disk to a FeatureSet.
//!
//! Bindings are the middle tier of FeatureSet resolution:
//! pinned_feature_set_id (on Client) > WorkspaceBinding > Space.active_feature_set_id.
//!
//! When a connected client declares MCP `roots` capability, the gateway calls
//! `roots/list` and matches each reported `file://` root against the bindings
//! for the client's Space using longest-prefix-wins.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A binding between a normalized workspace root path and a FeatureSet.
///
/// Uniqueness is `(space_id, workspace_root)` — the same on-disk directory
/// can bind different FeatureSets in different Spaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceBinding {
    /// Unique identifier
    pub id: Uuid,

    /// Space this binding belongs to
    pub space_id: Uuid,

    /// Normalized absolute path.
    ///
    /// Normalization rules (applied before insert/compare):
    ///   * resolve symlinks / junctions (`std::fs::canonicalize`)
    ///   * Windows: lowercase drive letter, use backslashes
    ///   * strip trailing path separator
    ///   * drop the `file://` scheme if the caller provided a URI
    pub workspace_root: String,

    /// FeatureSet to apply when this binding matches
    pub feature_set_id: Uuid,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceBinding {
    /// Create a new binding. Caller is responsible for passing an already-normalized path.
    pub fn new(space_id: Uuid, workspace_root: impl Into<String>, feature_set_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            space_id,
            workspace_root: workspace_root.into(),
            feature_set_id,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Normalize an absolute filesystem path or `file://` URI into the canonical
/// form used for binding comparisons.
///
/// This is the single source of truth for path comparisons — always route
/// through here before calling any repository method that takes `workspace_root`.
pub fn normalize_workspace_root(input: &str) -> String {
    // Empty in → empty out: callers filter on this to drop garbage roots
    // without needing to know about "/" vs "\" filesystem conventions.
    if input.is_empty() {
        return String::new();
    }

    // Strip file:// scheme if present; tolerate both "file:///abs/path" and
    // "file://host/abs/path" (we don't use host, it's always localhost).
    let without_scheme = if let Some(rest) = input.strip_prefix("file://") {
        // A leading triple-slash (file:///abs) leaves us with "/abs".
        // A double-slash host form (file://localhost/abs) leaves us with
        // "localhost/abs" — drop the host component before the first slash.
        match rest.find('/') {
            Some(0) => rest.to_string(),
            Some(n) => rest[n..].to_string(),
            None => rest.to_string(),
        }
    } else {
        input.to_string()
    };

    // URL-decode percent-escapes (e.g. %20 -> space) — MCP roots are URIs.
    let decoded = urlencoding::decode(&without_scheme)
        .map(|s| s.into_owned())
        .unwrap_or(without_scheme);

    // On Windows, "file:///D:/foo" decodes to "/D:/foo" — strip the leading
    // slash so callers see "D:\foo"-style paths before case folding.
    #[cfg(windows)]
    let stripped = {
        let trimmed = decoded
            .strip_prefix('/')
            .filter(|rest| {
                let bytes = rest.as_bytes();
                bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
            })
            .unwrap_or(&decoded);
        trimmed.replace('/', "\\")
    };
    #[cfg(not(windows))]
    let stripped = decoded;

    // Lowercase the drive letter on Windows so "D:\" and "d:\" compare equal.
    #[cfg(windows)]
    let cased = {
        let mut chars: Vec<char> = stripped.chars().collect();
        if chars.len() >= 2 && chars[0].is_ascii_alphabetic() && chars[1] == ':' {
            chars[0] = chars[0].to_ascii_lowercase();
        }
        chars.into_iter().collect::<String>()
    };
    #[cfg(not(windows))]
    let cased = stripped;

    // Strip trailing path separators (but keep a root like "/" or "d:\").
    let sep: &[char] = if cfg!(windows) { &['\\', '/'] } else { &['/'] };
    let trimmed = cased.trim_end_matches(sep);

    // Preserve root — if the trim removed everything, keep one separator.
    if trimmed.is_empty() {
        if cfg!(windows) {
            "\\".to_string()
        } else {
            "/".to_string()
        }
    } else if cfg!(windows) && trimmed.ends_with(':') {
        // "d:" → "d:\"
        format!("{}\\", trimmed)
    } else {
        trimmed.to_string()
    }
}

/// Returns the `workspace_root` in `candidates` whose path is the longest
/// prefix of `query`. Used by the resolver to pick which binding wins when
/// a client reports multiple roots.
///
/// Both `query` and every candidate MUST be already normalized via
/// [`normalize_workspace_root`] — this function does not re-normalize.
pub fn longest_prefix_match<'a, I>(query: &str, candidates: I) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut best: Option<&'a str> = None;
    for candidate in candidates {
        // Match only at a path-component boundary so "/workspaces/foo" does
        // not match a binding for "/workspaces/foo-bar".
        let matches = query == candidate
            || (query.starts_with(candidate)
                && query
                    .as_bytes()
                    .get(candidate.len())
                    .is_some_and(|b| *b == b'/' || (cfg!(windows) && *b == b'\\')));
        if matches && best.map(|b| candidate.len() > b.len()).unwrap_or(true) {
            best = Some(candidate);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_file_uri_unix() {
        let n = normalize_workspace_root("file:///home/user/proj");
        #[cfg(not(windows))]
        assert_eq!(n, "/home/user/proj");
        #[cfg(windows)]
        assert_eq!(n, "\\home\\user\\proj");
    }

    #[test]
    fn test_normalize_trailing_sep() {
        let sep = if cfg!(windows) { "\\" } else { "/" };
        let input = format!("/foo/bar{sep}");
        let n = normalize_workspace_root(&input);
        assert!(!n.ends_with(sep) || n.len() <= 3, "got {n}");
    }

    #[cfg(windows)]
    #[test]
    fn test_normalize_windows_drive_letter_case_insensitive() {
        assert_eq!(
            normalize_workspace_root("D:\\Projects\\Foo"),
            normalize_workspace_root("d:\\Projects\\Foo")
        );
        assert_eq!(normalize_workspace_root("D:"), "d:\\");
    }

    #[cfg(windows)]
    #[test]
    fn test_normalize_windows_file_uri() {
        assert_eq!(
            normalize_workspace_root("file:///D:/Projects/Foo"),
            "d:\\Projects\\Foo"
        );
    }

    #[test]
    fn test_percent_decoded() {
        let n = normalize_workspace_root("file:///home/user/my%20project");
        assert!(n.ends_with("my project"));
    }

    #[test]
    fn test_longest_prefix_match_exact() {
        let bindings = ["/a", "/a/b", "/a/b/c"];
        assert_eq!(longest_prefix_match("/a/b/c", bindings), Some("/a/b/c"));
        assert_eq!(longest_prefix_match("/a/b/c/d", bindings), Some("/a/b/c"));
        assert_eq!(longest_prefix_match("/a/b", bindings), Some("/a/b"));
    }

    #[test]
    fn test_longest_prefix_no_false_partial() {
        // "/a/b-extra" must NOT match binding "/a/b".
        let bindings = ["/a/b"];
        assert_eq!(longest_prefix_match("/a/b-extra", bindings), None);
    }

    #[test]
    fn test_longest_prefix_empty_candidates() {
        let bindings: [&str; 0] = [];
        assert_eq!(longest_prefix_match("/a", bindings), None);
    }
}
