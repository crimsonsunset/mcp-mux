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
use std::time::{Duration, Instant};

use dashmap::DashMap;
use mcpmux_core::normalize_workspace_root;
use tracing::{info, warn};

use super::tool_discovery::ToolIndex;
use super::window_identity::WindowKey;

/// Thread-safe registry mapping `mcp-session-id` to the caller's reported
/// workspace roots, plus the most recently resolved feature-set id so the
/// gateway can tell when a session's resolution flips and emit a per-peer
/// `list_changed` to that one session only.
#[derive(Debug, Default)]
pub struct SessionRootsRegistry {
    map: DashMap<String, Vec<String>>,
    /// `session_id -> last-resolved feature-set id` (or `None` for "deny").
    /// We compare each fresh resolution to this snapshot; a different value
    /// means the client's effective tools changed and we must notify it.
    last_resolution: DashMap<String, Option<String>>,
    /// `session_id -> declared MCP `roots` capability` (true when the peer's
    /// `initialize.params.capabilities.roots` was non-empty).
    ///
    /// Stamped during `on_initialized` regardless of whether roots have
    /// arrived yet. The resolver reads this to decide between
    /// `WorkspaceBinding` routing (capable) and the rootless `client_grants`
    /// fallback (not capable). Absence here means we never saw an
    /// `initialize` for that session — treated as "unknown" by the resolver
    /// and routed via grants.
    roots_capable: DashMap<String, bool>,
    /// `session_id -> Instant of the last on-demand `list_roots()` probe`.
    ///
    /// Used by [`Self::should_throttle_probe`] to avoid hammering a
    /// failing client when its previous probe already errored out
    /// recently. Only stamped after a probe attempt completes (success
    /// or failure), not on entry — so concurrent in-flight probes
    /// coordinate via [`Self::probe_lock`] instead of this throttle.
    last_probe: DashMap<String, Instant>,
    /// Per-session mutex guarding `peer.list_roots()` probe attempts.
    ///
    /// Single-flight semantics: when a burst of three list requests
    /// (`tools/list` + `prompts/list` + `resources/list`) hits a
    /// roots-pending session within milliseconds, only one upstream
    /// `list_roots()` call should be in flight. The other two block on
    /// the same lock; once the first attempt populates `map`, the
    /// followers re-check `map.get(sid)` and skip the upstream call
    /// entirely.
    ///
    /// Without this, a boolean "already tried" flag let the followers
    /// see `roots_pending` and return empty *before* the first probe's
    /// result landed — exactly the bug that left Claude Code's
    /// VS Code extension showing only the meta tools.
    probe_lock: DashMap<String, Arc<tokio::sync::Mutex<()>>>,
    /// `session_id -> Instant the resolver first saw this session with no
    /// roots yet`. Stamped lazily by [`Self::elapsed_since_first_seen`] so
    /// the resolver's `PendingRoots` tier can wait a grace window for a root
    /// to arrive before falling back to the Space default — preventing a
    /// roots-capable client from flashing the default FeatureSet and then
    /// flipping to its mapped one the instant its root lands.
    first_seen: DashMap<String, Instant>,
    /// `session_id -> explicit workspace root pinned via the
    /// `X-Mcpmux-Workspace` HTTP header`.
    ///
    /// McpMux's per-workspace client configs inject that header with the
    /// folder's path, so a connection routes to its workspace binding even
    /// when the client never reports MCP `roots` or reports a stale one (e.g.
    /// Cursor sharing a single MCP host across windows, with
    /// `roots.listChanged = false`). A pinned root is **authoritative**: it
    /// shadows the probed [`Self::map`] roots in [`Self::get`], so the
    /// resolver, the on-demand probe skip, and the prompt-root derivation all
    /// honor the header with no special-casing. Already normalized on insert.
    pinned: DashMap<String, String>,
    /// `session_id -> which claim produced the current [`Self::pinned`] entry.
    ///
    /// Read by [`Self::promote_pin_to_window`]. A header proves the root for
    /// that request, not that the `mcp-remote` PID serves one window — Cursor
    /// shares that process across every open window. Only a one-member
    /// candidate set (or a `SingleCandidate` pin, which is that set) is
    /// allowed to become a process-scoped window pin.
    pinned_source: DashMap<String, PinSource>,
    /// `client_id -> workspace path` held when `X-Mcpmux-Workspace` arrived
    /// before `mcp-session-id` (initialize). Applied on the first request
    /// that has a session id, or when initialize's response issues one.
    pending_by_client: DashMap<String, String>,
    /// `session_id -> the full set of folders open in the calling window`,
    /// sourced from the `X-Mcpmux-Workspace-Set` header (Cursor's
    /// `WORKSPACE_FOLDER_PATHS`, expanded by `mcp-remote`).
    ///
    /// This is a **constraint, never a resolver**. Measurement across 212
    /// multi-folder spawns found the active folder is always a member of this
    /// set, but its position within the set identifies the active folder only
    /// 70% of the time — so no ordering heuristic is safe for a routing
    /// decision that gates credentials. Two things it is good for:
    /// collapsing a one-member set to a pin (unambiguous), and rejecting a
    /// [`set_workspace_root`](super::meta_tools) pin that names a folder the
    /// calling window doesn't even have open.
    candidates: DashMap<String, Vec<String>>,
    /// `client_id -> candidate set` held when `X-Mcpmux-Workspace-Set` arrived
    /// before `mcp-session-id`, mirroring [`Self::pending_by_client`].
    pending_candidates_by_client: DashMap<String, Vec<String>>,
    /// Per-session active search index keyed by `(feature_set_ids fingerprint, index)`.
    /// Shared with [`MetaToolContext`](crate::services::meta_tools::MetaToolContext)
    /// so `mcpmux_search_tools` can reuse a session's resolved tool index.
    search_cache: Arc<DashMap<String, (u64, Arc<ToolIndex>)>>,
    /// `window_key -> explicit workspace root` — survives session churn for
    /// the life of the owning `mcp-remote` process. Only ever written from
    /// an explicit claim (header pin or `set_workspace_root`), never from
    /// probed roots or a deduction.
    window_pins: DashMap<WindowKey, String>,
    /// `session_id -> window_key` so session teardown and pin promotion can
    /// both find the window without redoing the socket lookup.
    session_window: DashMap<String, WindowKey>,
    /// Sessions whose current request sent an empty `X-Mcpmux-Workspace`.
    /// `get` / `window_pin_for_session` skip the remembered window pin so a
    /// sibling window on the shared `mcp-session-id` cannot keep A's folder.
    inherit_suppressed: DashMap<String, ()>,
}

/// Split an `X-Mcpmux-Workspace-Set` header into normalized folder paths,
/// sorted and deduped.
///
/// ponytail: splits on `,` because that is the delimiter Cursor uses for
/// `WORKSPACE_FOLDER_PATHS`, so a folder whose own name contains a comma will
/// shatter into bogus entries. The ceiling is acceptable because the set is
/// only ever a constraint: a shattered entry can't match a real root, so the
/// worst case is a pin rejection that falls back to today's behavior rather
/// than a misroute. Upgrade path is a length-prefixed or JSON-array header if
/// Cursor ever offers one.
fn parse_candidate_set(raw_set: &str) -> Vec<String> {
    let mut parsed: Vec<String> = raw_set
        .split(',')
        .filter(|entry| !is_unexpanded_variable(entry))
        .map(normalize_workspace_root)
        .filter(|path| !path.is_empty())
        .collect();
    parsed.sort();
    parsed.dedup();
    parsed
}

/// Whether a header value still carries a `${...}` template, meaning neither
/// the editor nor `mcp-remote` expanded it.
///
/// Such a value must never become a candidate: it can't match any real folder,
/// so it would turn the membership check into a blanket rejection of every
/// legitimate root the caller might declare.
fn is_unexpanded_variable(value: &str) -> bool {
    value.contains("${")
}

/// Which explicit claim produced a pin, carried purely for log attribution.
///
/// The registry treats all three identically, but a field trace cannot: a
/// header pin means Cursor's `${workspaceFolder}` substitution worked, while a
/// meta-tool pin means it failed and an agent recovered by hand. Reading one as
/// the other turns "did the bridge work?" into guesswork.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinSource {
    /// The `X-Mcpmux-Workspace` request header, including a value held across
    /// `initialize` until an `mcp-session-id` existed.
    WorkspaceHeader,
    /// A single-member `X-Mcpmux-Workspace-Set`, which names the active folder
    /// unambiguously.
    SingleCandidate,
    /// The `mcpmux_set_workspace_root` meta tool.
    MetaTool,
}

impl PinSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::WorkspaceHeader => "X-Mcpmux-Workspace",
            Self::SingleCandidate => "X-Mcpmux-Workspace-Set(single)",
            Self::MetaTool => "mcpmux_set_workspace_root",
        }
    }
}

impl SessionRootsRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            map: DashMap::new(),
            last_resolution: DashMap::new(),
            roots_capable: DashMap::new(),
            last_probe: DashMap::new(),
            probe_lock: DashMap::new(),
            first_seen: DashMap::new(),
            pinned: DashMap::new(),
            pinned_source: DashMap::new(),
            pending_by_client: DashMap::new(),
            candidates: DashMap::new(),
            pending_candidates_by_client: DashMap::new(),
            search_cache: Arc::new(DashMap::new()),
            window_pins: DashMap::new(),
            session_window: DashMap::new(),
            inherit_suppressed: DashMap::new(),
        })
    }

    /// Shared per-session `search_tools` active index cache.
    pub fn search_cache(&self) -> Arc<DashMap<String, (u64, Arc<ToolIndex>)>> {
        self.search_cache.clone()
    }

    /// Evict cached active indexes for sessions reporting `workspace_root`.
    pub fn evict_search_cache_for_workspace_root(&self, workspace_root: &str) {
        let normalized = normalize_workspace_root(workspace_root);
        let session_ids: Vec<String> = self
            .map
            .iter()
            .filter(|entry| entry.value().iter().any(|root| root == &normalized))
            .map(|entry| entry.key().clone())
            .collect();
        for session_id in session_ids {
            self.search_cache.remove(&session_id);
        }
    }

    /// Elapsed time since this session was first observed without roots,
    /// stamping "now" on the first call. The resolver uses this to bound the
    /// `PendingRoots` wait: while the result is below the grace window it
    /// keeps waiting for a root; past it, it settles on the Space default.
    /// Idempotent — the timestamp is only set once per session and cleared by
    /// [`Self::remove`].
    pub fn elapsed_since_first_seen(&self, session_id: &str) -> Duration {
        let first = *self
            .first_seen
            .entry(session_id.to_string())
            .or_insert_with(Instant::now);
        first.elapsed()
    }

    /// Get (or create) the per-session probe lock. The returned Arc is
    /// what the handler awaits to serialize concurrent probes — see
    /// [`Self::probe_lock`] for the rationale.
    pub fn probe_lock(&self, session_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.probe_lock
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Should we skip an on-demand probe because the previous attempt
    /// completed (success or failure) within the last `throttle`?
    ///
    /// Distinct from `probe_lock`: the lock serializes *concurrent*
    /// probes; this rate-limit prevents *sequential* probes from
    /// hammering a peer whose previous attempt errored.
    pub fn should_throttle_probe(&self, session_id: &str, throttle: Duration) -> bool {
        let Some(last) = self.last_probe.get(session_id) else {
            return false;
        };
        Instant::now().duration_since(*last) < throttle
    }

    /// Stamp the completion of an on-demand probe so the next caller
    /// observes the throttle. Called after the probe returns (regardless
    /// of success or failure) so successive probes back off only when
    /// the previous one actually finished.
    pub fn mark_probe_completed(&self, session_id: &str) {
        self.last_probe
            .insert(session_id.to_string(), Instant::now());
    }

    /// Record whether a session declared the MCP `roots` capability on
    /// `initialize`. Idempotent — called once per session lifecycle.
    pub fn set_roots_capable(&self, session_id: impl Into<String>, capable: bool) {
        self.roots_capable.insert(session_id.into(), capable);
    }

    /// `Some(true)` when the session declared `roots`, `Some(false)` when it
    /// explicitly didn't, `None` when no `initialize` has been observed
    /// (callers without a session id, or pre-init requests).
    pub fn is_roots_capable(&self, session_id: &str) -> Option<bool> {
        self.roots_capable.get(session_id).map(|v| *v)
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
    ///
    /// Precedence: session pin (`X-Mcpmux-Workspace` / `set_workspace_root`)
    /// then a remembered window pin, then probed MCP roots. The session pin
    /// always wins so a genuine folder switch is never overridden by a
    /// leftover window answer. That single seam is what makes an explicit
    /// claim authoritative everywhere `get` is consulted (resolver Tier 1,
    /// the probe early-return, prompt-root derivation) without threading the
    /// header through any of those call paths.
    pub fn get(&self, session_id: &str) -> Option<Vec<String>> {
        if let Some(pinned) = self.pinned.get(session_id) {
            return Some(vec![pinned.clone()]);
        }
        if let Some(inherited) = self.window_pin_for_session(session_id) {
            return Some(vec![inherited]);
        }
        self.map.get(session_id).map(|v| v.clone())
    }

    /// Pin an explicit workspace root for a session, sourced from the
    /// `X-Mcpmux-Workspace` HTTP header. `raw_root` is a filesystem path or
    /// `file://` URI; it's normalized like every other root before storage.
    /// A value that normalizes to empty is ignored (no pin), so a malformed
    /// header falls back to the client's reported roots rather than denying.
    /// Cheap to call on the request hot path: redundant writes (same
    /// normalized value already pinned) are skipped to avoid shard churn.
    ///
    /// Logs at info on first pin, warn when the same session is re-pinned to a
    /// different root (Agents Window / shared-session cross-workspace clobber).
    /// Both lines name `source` so a trace can separate a working header from a
    /// manual recovery. Also copies the pin onto the session's window when one
    /// is attached, so later sessions from the same `mcp-remote` process
    /// inherit it.
    pub fn set_pinned(&self, session_id: &str, raw_root: &str, source: PinSource) {
        let normalized = normalize_workspace_root(raw_root);
        if normalized.is_empty() {
            return;
        }
        if let Some(previous) = self.pinned.get(session_id) {
            if *previous == normalized {
                return;
            }
            warn!(
                %session_id,
                previous = %previous.as_str(),
                new = %normalized,
                source = source.as_str(),
                "[SessionRoots] pin clobber — same session, different root",
            );
        } else {
            info!(
                %session_id,
                workspace_root = %normalized,
                source = source.as_str(),
                "[SessionRoots] pinned explicit workspace root",
            );
        }
        self.last_resolution.remove(session_id);
        self.search_cache.remove(session_id);
        self.inherit_suppressed.remove(session_id);
        self.pinned.insert(session_id.to_string(), normalized);
        self.pinned_source.insert(session_id.to_string(), source);
        self.promote_pin_to_window(session_id);
    }

    /// Drop this session's pin and skip window-pin inheritance.
    ///
    /// An empty `X-Mcpmux-Workspace` means this request has no workspace
    /// claim. Clearing only the session pin is not enough: [`Self::get`]
    /// would still return a remembered window pin. Suppression lasts until
    /// the next explicit [`Self::set_pinned`].
    pub fn forget_empty_header_claim(&self, session_id: &str) {
        self.pinned.remove(session_id);
        self.pinned_source.remove(session_id);
        self.last_resolution.remove(session_id);
        self.search_cache.remove(session_id);
        self.inherit_suppressed.insert(session_id.to_string(), ());
    }

    /// Hold a workspace path for `client_id` until a session id exists.
    ///
    /// Empty/whitespace values are ignored (same as [`Self::set_pinned`]).
    pub fn remember_pending_workspace(&self, client_id: &str, raw_root: &str) {
        let normalized = normalize_workspace_root(raw_root);
        if normalized.is_empty() {
            return;
        }
        self.pending_by_client
            .insert(client_id.to_string(), normalized);
    }

    /// Pin a session from a previously remembered client header, if any.
    ///
    /// Returns `true` when a pending path was applied. The pending entry is
    /// kept so a later session from the same client can reuse it.
    ///
    /// The pending slot is keyed by `client_id`, and every Cursor window shares
    /// one access key — so the parked path may belong to a *different* window
    /// than the session now claiming it. `reported_set` is the claiming
    /// request's own `X-Mcpmux-Workspace-Set`; together with any set already
    /// stored for the session it decides membership, and a non-member is
    /// refused rather than pinned onto the wrong workspace.
    pub fn apply_pending_workspace(
        &self,
        client_id: &str,
        session_id: &str,
        reported_set: Option<&str>,
    ) -> bool {
        let Some(path) = self
            .pending_by_client
            .get(client_id)
            .map(|value| value.clone())
        else {
            return false;
        };
        let reported = reported_set.map(parse_candidate_set).unwrap_or_default();
        let outside_reported = !reported.is_empty() && !reported.contains(&path);
        if outside_reported || !self.is_candidate(session_id, &path) {
            warn!(
                %session_id,
                pending_root = %path,
                reported_candidates = ?reported,
                session_candidates = ?self.get_candidates(session_id),
                "[SessionRoots] held X-Mcpmux-Workspace names a folder this window does \
                 not have open — pin skipped (the value was parked by another window \
                 sharing this access key)",
            );
            return false;
        }
        self.set_pinned(session_id, &path, PinSource::WorkspaceHeader);
        true
    }

    /// The explicit workspace root for a session, if any (already normalized).
    ///
    /// Includes a remembered window pin when this session has no header /
    /// `set_workspace_root` claim of its own, so callers that treat "pinned"
    /// as "we know the folder" (probe skip, candidate audit) honor durability
    /// without a second code path.
    pub fn get_pinned(&self, session_id: &str) -> Option<String> {
        if let Some(pinned) = self.pinned.get(session_id) {
            return Some(pinned.clone());
        }
        self.window_pin_for_session(session_id)
    }

    /// Session-only pin, excluding a remembered window pin.
    ///
    /// Used by the middleware to decide "this request already has an explicit
    /// claim" before attempting inheritance.
    pub fn session_pin(&self, session_id: &str) -> Option<String> {
        self.pinned.get(session_id).map(|v| v.clone())
    }

    /// Associate `session_id` with a window so later pins and inherits can
    /// find it without another socket lookup.
    pub fn attach_window(&self, session_id: &str, key: WindowKey) {
        self.session_window.insert(session_id.to_string(), key);
    }

    /// The window key attached to this session, if any.
    pub fn window_key_for(&self, session_id: &str) -> Option<WindowKey> {
        self.session_window.get(session_id).map(|key| *key)
    }

    /// Copy this session's explicit pin onto its window, if both exist and
    /// the pin is attributable to that one window.
    ///
    /// No-op when the session has no pin or no window — initialize often
    /// pins before the peer socket has been mapped, and the next request
    /// retries.
    ///
    /// A header names the root for this request. It does not prove the PID
    /// serves one window (Cursor shares one `mcp-remote` across every window;
    /// see `docs/planning/window-scoped-workspace-pin.md`). `SingleCandidate`
    /// is the set itself. Every other source promotes only when the session's
    /// candidate set independently narrows to one folder.
    ///
    /// Called from the request hot path (every request carrying a session pin
    /// re-promotes), so an unchanged value skips both the write and the log.
    /// Without that guard the durability this feature exists to provide would
    /// only be observable as one log line per request.
    pub fn promote_pin_to_window(&self, session_id: &str) {
        let Some(key) = self.window_key_for(session_id) else {
            return;
        };
        let Some(root) = self.pinned.get(session_id).map(|value| value.clone()) else {
            return;
        };
        let source = self
            .pinned_source
            .get(session_id)
            .map(|value| *value)
            .unwrap_or(PinSource::MetaTool);
        if source != PinSource::SingleCandidate {
            let candidates = self.get_candidates(session_id);
            let single_folder_proven = matches!(candidates.as_deref(), Some([_]));
            if !single_folder_proven {
                warn!(
                    %session_id,
                    window_key = %key,
                    workspace_root = %root,
                    source = source.as_str(),
                    candidates = ?candidates,
                    "[SessionRoots] window pin skipped — no single-folder candidate set to \
                     prove this mcp-remote process serves one window; this session keeps \
                     the pin, but it will not survive a Reload MCP or session churn",
                );
                return;
            }
        }
        // Owned bool so the read guard drops before the insert below — see
        // `record_resolution` for the self-deadlock this avoids.
        let unchanged = self
            .window_pins
            .get(&key)
            .is_some_and(|existing| *existing == root);
        if unchanged {
            return;
        }
        info!(
            %session_id,
            window_key = %key,
            workspace_root = %root,
            "[SessionRoots] window pin stored — later sessions from this process inherit it",
        );
        self.window_pins.insert(key, root);
    }

    /// Apply a remembered window pin to a session that has no explicit claim.
    ///
    /// Returns the inherited root when the window is live and the root passes
    /// the candidate-set check (or no set was reported). Logs distinctly from
    /// [`Self::set_pinned`] so field traces can tell a fresh pin from a reuse.
    pub fn inherit_window_pin(&self, session_id: &str) -> Option<String> {
        if self.pinned.contains_key(session_id) {
            return None;
        }
        let root = self.window_pin_for_session(session_id)?;
        info!(
            %session_id,
            workspace_root = %root,
            window_key = ?self.window_key_for(session_id),
            "[SessionRoots] inherited workspace pin from window",
        );
        self.last_resolution.remove(session_id);
        self.search_cache.remove(session_id);
        Some(root)
    }

    /// Window-scoped pin for `session_id`, or [`None`] if the window is dead,
    /// the pin is missing, or the session's candidate set rejects it.
    fn window_pin_for_session(&self, session_id: &str) -> Option<String> {
        if self.inherit_suppressed.contains_key(session_id) {
            return None;
        }
        let key = self.window_key_for(session_id)?;
        if !key.is_live() {
            self.window_pins.remove(&key);
            return None;
        }
        let root = self.window_pins.get(&key).map(|value| value.clone())?;
        if !self.is_candidate(session_id, &root) {
            warn!(
                %session_id,
                remembered_root = %root,
                candidates = ?self.get_candidates(session_id),
                "[SessionRoots] window pin is absent from X-Mcpmux-Workspace-Set — \
                 inheritance skipped",
            );
            return None;
        }
        Some(root)
    }

    /// Record the calling window's full folder set from the
    /// `X-Mcpmux-Workspace-Set` header.
    ///
    /// A one-member set is unambiguous, so it pins directly — that is the only
    /// case where this header decides a route. Larger sets are stored as a
    /// constraint for [`Self::is_candidate`] and for naming candidates in
    /// refusals; they never select a folder on their own.
    pub fn set_candidates(&self, session_id: &str, raw_set: &str) {
        self.store_candidates(session_id, parse_candidate_set(raw_set));
    }

    fn store_candidates(&self, session_id: &str, parsed: Vec<String>) {
        if parsed.is_empty() {
            return;
        }
        // The set header rides every request, so skip the audit and the write
        // when nothing changed. Read the comparison into an owned bool so the
        // DashMap guard drops before the insert below (see `record_resolution`
        // for the self-deadlock this avoids).
        let unchanged = self
            .candidates
            .get(session_id)
            .is_some_and(|existing| *existing == parsed);
        if unchanged {
            return;
        }

        if let [only] = parsed.as_slice() {
            // The active folder is always a member of the set, so a set of one
            // names it outright — no guessing involved. `set_pinned` logs the
            // pin itself with `source=X-Mcpmux-Workspace-Set(single)`.
            self.set_pinned(session_id, only, PinSource::SingleCandidate);
        }

        match self.get_pinned(session_id) {
            // Audits the invariant the whole design rests on: the active
            // folder was a member of the reported set in 212 of 212 sampled
            // multi-folder spawns. If this ever fires, membership is no longer
            // safe as a constraint and `is_candidate` will start refusing
            // legitimate roots — treat it as a design regression, not noise.
            Some(pinned) if !parsed.iter().any(|c| c == &pinned) => {
                warn!(
                    %session_id,
                    pinned_root = %pinned,
                    candidates = ?parsed,
                    "[SessionRoots] pinned root is absent from X-Mcpmux-Workspace-Set — \
                     membership invariant violated; set_workspace_root may now reject \
                     valid roots for this session",
                );
            }
            Some(_) => {}
            // The active-folder header failed and more than one folder is
            // open, so this session cannot be routed without the caller
            // declaring which folder it is in. Logged at warn because it is
            // the case that costs the user an extra round trip.
            None if parsed.len() > 1 => {
                warn!(
                    %session_id,
                    candidate_count = parsed.len(),
                    candidates = ?parsed,
                    "[SessionRoots] no pinned root and multiple folders open — session must \
                     call mcpmux_set_workspace_root to disambiguate (a per-repo static \
                     header install avoids this entirely)",
                );
            }
            None => {}
        }

        self.candidates.insert(session_id.to_string(), parsed);
    }

    /// The calling window's folder set, if the header supplied one.
    pub fn get_candidates(&self, session_id: &str) -> Option<Vec<String>> {
        self.candidates.get(session_id).map(|v| v.clone())
    }

    /// Whether `root` is one of the folders the calling window has open.
    ///
    /// `true` when no set was reported — an absent constraint must not block
    /// clients that don't send the header (every non-Cursor client, and Cursor
    /// before the bridge config is reinstalled).
    pub fn is_candidate(&self, session_id: &str, root: &str) -> bool {
        let Some(candidates) = self.candidates.get(session_id) else {
            return true;
        };
        let normalized = normalize_workspace_root(root);
        candidates.iter().any(|c| c == &normalized)
    }

    /// Hold a candidate set for `client_id` until a session id exists,
    /// mirroring [`Self::remember_pending_workspace`].
    pub fn remember_pending_candidates(&self, client_id: &str, raw_set: &str) {
        let parsed = parse_candidate_set(raw_set);
        if parsed.is_empty() {
            return;
        }
        self.pending_candidates_by_client
            .insert(client_id.to_string(), parsed);
    }

    /// Apply a previously remembered candidate set to a session. Returns
    /// `true` when one was applied.
    pub fn apply_pending_candidates(&self, client_id: &str, session_id: &str) -> bool {
        let Some(candidates) = self
            .pending_candidates_by_client
            .get(client_id)
            .map(|value| value.clone())
        else {
            return false;
        };
        self.store_candidates(session_id, candidates);
        true
    }

    /// Drop a session's roots — call on client disconnect.
    pub fn remove(&self, session_id: &str) {
        self.map.remove(session_id);
        self.last_resolution.remove(session_id);
        self.roots_capable.remove(session_id);
        self.last_probe.remove(session_id);
        self.probe_lock.remove(session_id);
        self.first_seen.remove(session_id);
        self.pinned.remove(session_id);
        self.pinned_source.remove(session_id);
        self.candidates.remove(session_id);
        self.search_cache.remove(session_id);
        self.inherit_suppressed.remove(session_id);
        if let Some((_, key)) = self.session_window.remove(session_id) {
            let still_used = self
                .session_window
                .iter()
                .any(|entry| *entry.value() == key);
            if !still_used && !key.is_live() {
                self.window_pins.remove(&key);
            }
        }
    }

    /// Compare-and-set the session's resolved feature-set id. Returns `true`
    /// when the value actually changed (caller should fire `list_changed`),
    /// `false` when it's the same as before.
    pub fn record_resolution(&self, session_id: &str, fs_id: Option<&str>) -> bool {
        let new_val: Option<String> = fs_id.map(|s| s.to_string());
        // IMPORTANT: read the prior value into an owned `bool` and let the
        // `get()` read guard drop at the end of THIS statement. Holding a
        // DashMap `Ref` across the `insert()` below would request a write lock
        // on the same shard while still holding its read lock — a self-deadlock
        // that fires exactly when a session's resolution changes from one
        // Some(..) to a different Some(..) (the common "binding changed" path).
        let unchanged = self
            .last_resolution
            .get(session_id)
            .is_some_and(|prev| *prev == new_val);
        if unchanged {
            return false;
        }
        self.last_resolution.insert(session_id.to_string(), new_val);
        true
    }

    /// Returns every reported root across every active session, de-duplicated
    /// and sorted for stable presentation. Used by the UI's "Detected
    /// workspaces" panel so the user can act on folders that clients have
    /// surfaced but haven't been bound yet.
    pub fn list_all_roots(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .map
            .iter()
            .flat_map(|entry| entry.value().clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Drop a single root from the registry regardless of binding status.
    /// Removes it from every session that holds it; sessions left empty are
    /// evicted (same as [`Self::forget_unmapped_roots`]). Returns `true` when
    /// the root was found and removed from at least one session.
    pub fn forget_root(&self, root: &str) -> bool {
        let normalized = normalize_workspace_root(root);
        !self
            .forget_unmapped_roots(|r| r != normalized.as_str())
            .is_empty()
    }

    /// Forget every reported root that is **not** currently mapped, so the
    /// Workspaces tab's "Unmapped" list clears and the gateway re-offers the
    /// "map this folder?" prompt the next time those sessions report a root.
    ///
    /// `is_mapped(root)` returns `true` for roots that have a binding — those
    /// are kept. For each tracked session the unmapped roots are dropped; a
    /// session left with no roots is removed from the registry entirely (along
    /// with its last-resolution snapshot and probe throttle) so its next
    /// `tools/list` re-probes the peer and the resolver fires
    /// `WorkspaceNeedsBinding` again. Sessions that still hold a mapped root
    /// keep their entry untouched (they route via their binding and never
    /// prompt). Returns the dropped roots (sorted, deduped) for logging.
    pub fn forget_unmapped_roots<F>(&self, is_mapped: F) -> Vec<String>
    where
        F: Fn(&str) -> bool,
    {
        let mut dropped: Vec<String> = Vec::new();
        let mut emptied: Vec<String> = Vec::new();

        for mut entry in self.map.iter_mut() {
            let mut removed_any = false;
            entry.value_mut().retain(|root| {
                if is_mapped(root) {
                    true
                } else {
                    dropped.push(root.clone());
                    removed_any = true;
                    false
                }
            });
            if removed_any && entry.value().is_empty() {
                emptied.push(entry.key().clone());
            }
        }

        // Remove emptied sessions AFTER the iterator above is released — a
        // `map.remove()` while iterating would request a write lock on a shard
        // the iterator still read-locks (self-deadlock). Dropping the roots
        // entry (rather than leaving an empty Vec) is what makes the next
        // request re-probe: `ensure_roots_probed` early-returns while
        // `get(sid)` is `Some(_)`, even for an empty Vec.
        for sid in emptied {
            self.map.remove(&sid);
            self.last_resolution.remove(&sid);
            self.last_probe.remove(&sid);
            // Reset the grace clock too, so the re-probed session waits afresh
            // for its root to re-arrive instead of immediately defaulting on a
            // stale first-seen timestamp.
            self.first_seen.remove(&sid);
        }

        dropped.sort();
        dropped.dedup();
        dropped
    }

    /// Current number of tracked sessions. Test helper; cheap to call but
    /// not useful in hot paths.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether no sessions are tracked. Paired with [`Self::len`] — clippy
    /// requires this when `len` is present.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
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

    #[test]
    fn test_pinned_root_shadows_reported_roots() {
        let reg = SessionRootsRegistry::default();
        #[cfg(windows)]
        let (reported, pin_in, pin_norm) = (
            "file:///D:/reported/",
            "D:\\Pinned\\Path",
            "d:\\pinned\\path",
        );
        #[cfg(not(windows))]
        let (reported, pin_in, pin_norm) = (
            "file:///home/u/reported/",
            "/home/u/Pinned",
            "/home/u/Pinned",
        );

        reg.set("sess-1", [reported]);
        reg.set_pinned("sess-1", pin_in, PinSource::WorkspaceHeader);

        // The pinned (header) root entirely shadows the probed root.
        assert_eq!(reg.get("sess-1"), Some(vec![pin_norm.to_string()]));
        assert_eq!(reg.get_pinned("sess-1"), Some(pin_norm.to_string()));
    }

    #[test]
    fn pending_header_pins_when_session_id_arrives() {
        let reg = SessionRootsRegistry::default();
        #[cfg(windows)]
        let (pin_in, pin_norm) = ("D:\\Pinned\\Path", "d:\\pinned\\path");
        #[cfg(not(windows))]
        let (pin_in, pin_norm) = (
            "/Users/joe/Desktop/Repos/Personal/mcp-mux",
            "/Users/joe/Desktop/Repos/Personal/mcp-mux",
        );

        reg.remember_pending_workspace("client-1", pin_in);
        assert!(reg.get_pinned("sess-new").is_none());
        assert!(reg.apply_pending_workspace("client-1", "sess-new", None));
        assert_eq!(reg.get("sess-new"), Some(vec![pin_norm.to_string()]));

        reg.set(
            "sess-new",
            ["/a", "/b", "/c", "/d", "/e", "/f"]
                .into_iter()
                .map(String::from),
        );
        assert_eq!(
            reg.get("sess-new"),
            Some(vec![pin_norm.to_string()]),
            "six-way roots/list must stay shadowed by the pending pin"
        );
    }

    #[test]
    fn empty_pending_header_does_not_pin() {
        let reg = SessionRootsRegistry::default();
        reg.remember_pending_workspace("client-1", "   ");
        assert!(!reg.apply_pending_workspace("client-1", "sess-1", None));
        assert!(reg.get_pinned("sess-1").is_none());
    }

    #[test]
    fn pending_header_from_another_window_does_not_pin() {
        // Observed live: every Cursor window shares one access key, so window A
        // parked its own folder in the client-keyed pending slot and window B's
        // brand-new session claimed it — pinning B onto A's workspace even
        // though B never had that folder open.
        let reg = SessionRootsRegistry::default();
        #[cfg(windows)]
        let other_window = "d:\\delta";
        #[cfg(not(windows))]
        let other_window = "/repos/delta";
        reg.remember_pending_workspace("client-1", other_window);

        // Refused against the claiming request's own reported set.
        let reported = CANDIDATES.join(",");
        assert!(!reg.apply_pending_workspace("client-1", "sess-b", Some(&reported)));
        assert!(reg.get_pinned("sess-b").is_none());

        // Refused against a set already stored for the session.
        reg.set_candidates("sess-c", &reported);
        assert!(!reg.apply_pending_workspace("client-1", "sess-c", None));
        assert!(reg.get_pinned("sess-c").is_none());

        // A member of the set still pins — the gate is membership, not a block.
        reg.remember_pending_workspace("client-1", CANDIDATES[0]);
        assert!(reg.apply_pending_workspace("client-1", "sess-c", Some(&reported)));
        assert_eq!(reg.get_pinned("sess-c").as_deref(), Some(CANDIDATES[0]));
    }

    #[test]
    fn set_pinned_clears_last_resolution() {
        let reg = SessionRootsRegistry::default();
        assert!(reg.record_resolution("sess-1", Some("fs-pending")));
        assert!(!reg.record_resolution("sess-1", Some("fs-pending")));
        reg.set_pinned("sess-1", "/p", PinSource::WorkspaceHeader);
        assert!(
            reg.record_resolution("sess-1", Some("fs-bound")),
            "pin must drop the cached PendingRoots resolution"
        );
    }

    #[test]
    fn test_set_pinned_ignores_empty_and_normalizes() {
        let reg = SessionRootsRegistry::default();
        // Whitespace/garbage that normalizes to empty leaves no pin, so a
        // malformed header falls back to reported roots rather than denying.
        reg.set_pinned("sess-1", "   ", PinSource::WorkspaceHeader);
        assert!(reg.get_pinned("sess-1").is_none());

        #[cfg(windows)]
        let (pin_in, pin_norm) = ("file:///D:/Foo/", "d:\\foo");
        #[cfg(not(windows))]
        let (pin_in, pin_norm) = ("file:///home/u/Foo/", "/home/u/Foo");
        reg.set_pinned("sess-1", pin_in, PinSource::WorkspaceHeader);
        assert_eq!(reg.get_pinned("sess-1"), Some(pin_norm.to_string()));
    }

    #[test]
    fn test_remove_clears_pinned() {
        let reg = SessionRootsRegistry::default();
        reg.set_pinned("sess-1", "/p", PinSource::WorkspaceHeader);
        assert!(reg.get_pinned("sess-1").is_some());
        reg.remove("sess-1");
        assert!(reg.get_pinned("sess-1").is_none());
        assert!(reg.get("sess-1").is_none());
    }

    #[test]
    fn test_record_resolution_flips_on_change() {
        let reg = SessionRootsRegistry::default();
        // First sighting always counts as a change so the caller emits the
        // initial list_changed for whoever subscribed late.
        assert!(reg.record_resolution("sess-1", Some("fs-fallback")));
        // Same value → no change.
        assert!(!reg.record_resolution("sess-1", Some("fs-fallback")));
        // Different value → change.
        assert!(reg.record_resolution("sess-1", Some("fs-bound")));
        // None ↔ Some both count.
        assert!(reg.record_resolution("sess-1", None));
        assert!(!reg.record_resolution("sess-1", None));
    }

    #[test]
    fn test_forget_unmapped_roots_clears_unmapped_sessions() {
        let reg = SessionRootsRegistry::default();
        #[cfg(windows)]
        let (mapped_in, unmapped_in) = ("file:///D:/mapped/", "file:///D:/unmapped/");
        #[cfg(not(windows))]
        let (mapped_in, unmapped_in) = ("file:///home/u/mapped/", "file:///home/u/unmapped/");

        reg.set("sess-mapped", [mapped_in]);
        reg.set("sess-unmapped", [unmapped_in]);
        reg.record_resolution("sess-unmapped", Some("fs-x"));

        // Treat only the first session's (normalized) root as mapped.
        let mapped_norm = reg.get("sess-mapped").unwrap()[0].clone();
        let dropped = reg.forget_unmapped_roots(|root| root == mapped_norm);

        // Exactly the unmapped root was dropped.
        assert_eq!(dropped.len(), 1);
        assert_ne!(dropped[0], mapped_norm);
        // The mapped session is untouched.
        assert_eq!(reg.get("sess-mapped"), Some(vec![mapped_norm]));
        // The unmapped session is removed entirely so the next request
        // re-probes the peer and the binding prompt fires again.
        assert!(reg.get("sess-unmapped").is_none());
        // ...and its resolution snapshot was cleared (fresh = counts as change).
        assert!(reg.record_resolution("sess-unmapped", Some("fs-x")));
    }

    #[test]
    fn test_forget_unmapped_roots_keeps_mixed_session() {
        let reg = SessionRootsRegistry::default();
        #[cfg(windows)]
        let (mapped_in, unmapped_in) = ("file:///D:/keep/", "file:///D:/drop/");
        #[cfg(not(windows))]
        let (mapped_in, unmapped_in) = ("file:///home/u/keep/", "file:///home/u/drop/");

        reg.set("sess-mixed", [mapped_in, unmapped_in]);
        let roots = reg.get("sess-mixed").unwrap();
        let keep = roots[0].clone();

        let dropped = reg.forget_unmapped_roots(|root| root == keep);

        // The unmapped root went; the session survives with its mapped root.
        assert_eq!(dropped.len(), 1);
        assert_eq!(reg.get("sess-mixed"), Some(vec![keep]));
    }

    /// Paths that survive `normalize_workspace_root` unchanged on this
    /// platform, so the candidate assertions below don't fight normalization.
    #[cfg(windows)]
    const CANDIDATES: [&str; 3] = ["d:\\alpha", "d:\\beta", "d:\\gamma"];
    #[cfg(not(windows))]
    const CANDIDATES: [&str; 3] = ["/repos/alpha", "/repos/beta", "/repos/gamma"];

    #[test]
    fn single_candidate_pins_but_multiple_only_constrain() {
        let reg = SessionRootsRegistry::default();

        // One folder open is unambiguous — the set is allowed to decide.
        reg.set_candidates("sess-one", CANDIDATES[0]);
        assert_eq!(reg.get_pinned("sess-one"), Some(CANDIDATES[0].to_string()));

        // Several folders open must never auto-select, however tempting the
        // ordering looks: position predicts the active folder only ~70% of
        // the time, which is a misroute, not a fallback.
        let many = CANDIDATES.join(",");
        reg.set_candidates("sess-many", &many);
        assert!(
            reg.get_pinned("sess-many").is_none(),
            "a multi-folder set must not pin any of its members"
        );
        assert_eq!(reg.get_candidates("sess-many").unwrap().len(), 3);
    }

    #[test]
    fn candidate_set_constrains_declarable_roots() {
        let reg = SessionRootsRegistry::default();
        reg.set_candidates("sess-1", &CANDIDATES.join(","));

        assert!(reg.is_candidate("sess-1", CANDIDATES[1]));
        assert!(
            !reg.is_candidate("sess-1", "/repos/not-open"),
            "a folder the window doesn't have open must be refusable"
        );

        // No set reported (every client that doesn't send the header) stays
        // permissive — the constraint must not become a new denial path.
        assert!(reg.is_candidate("sess-unknown", "/repos/anything"));
    }

    #[test]
    fn parse_candidate_set_dedupes_and_drops_blanks() {
        let raw = format!("{a},,{a},  ,{b}", a = CANDIDATES[0], b = CANDIDATES[1]);
        assert_eq!(
            parse_candidate_set(&raw),
            vec![CANDIDATES[0].to_string(), CANDIDATES[1].to_string()]
        );
        assert!(parse_candidate_set("  ,  ").is_empty());
    }

    #[test]
    fn unexpanded_template_never_becomes_a_candidate() {
        // If mcp-remote stops expanding the variable, the literal must be
        // dropped. Keeping it would match no real folder and so turn the
        // membership check into a blanket refusal.
        assert!(parse_candidate_set("${WORKSPACE_FOLDER_PATHS}").is_empty());

        let reg = SessionRootsRegistry::default();
        reg.set_candidates("sess-1", "${WORKSPACE_FOLDER_PATHS}");
        assert!(reg.get_candidates("sess-1").is_none());
        assert!(
            reg.is_candidate("sess-1", CANDIDATES[0]),
            "an unexpanded set must leave the session unconstrained, not deny it"
        );

        // A partially expanded value keeps the real folders and drops the rest.
        let mixed = format!("{},${{WORKSPACE_FOLDER_PATHS}}", CANDIDATES[0]);
        assert_eq!(parse_candidate_set(&mixed), vec![CANDIDATES[0].to_string()]);
    }

    #[test]
    fn pending_candidates_apply_when_session_id_arrives() {
        let reg = SessionRootsRegistry::default();
        reg.remember_pending_candidates("client-1", CANDIDATES[0]);
        assert!(reg.get_candidates("sess-new").is_none());
        assert!(reg.apply_pending_candidates("client-1", "sess-new"));
        // A held single-folder set still pins once the session id lands.
        assert_eq!(reg.get_pinned("sess-new"), Some(CANDIDATES[0].to_string()));
    }

    #[test]
    fn test_remove_clears_resolution_too() {
        let reg = SessionRootsRegistry::default();
        reg.record_resolution("sess-1", Some("fs-a"));
        reg.remove("sess-1");
        // After remove, recording the same value should be considered a
        // change (no prior entry).
        assert!(reg.record_resolution("sess-1", Some("fs-a")));
    }

    #[test]
    fn window_pin_survives_session_churn_for_live_process() {
        let reg = SessionRootsRegistry::default();
        let key = WindowKey::from_pid(std::process::id());
        reg.attach_window("sess-1", key);
        reg.set_candidates("sess-1", CANDIDATES[0]);
        reg.set_pinned("sess-1", CANDIDATES[0], PinSource::WorkspaceHeader);

        reg.attach_window("sess-2", key);
        assert_eq!(
            reg.inherit_window_pin("sess-2"),
            Some(CANDIDATES[0].to_string())
        );
        assert_eq!(reg.get("sess-2"), Some(vec![CANDIDATES[0].to_string()]));
        assert!(
            reg.session_pin("sess-2").is_none(),
            "inherit must not copy into the session pin — a later header must still win"
        );
    }

    #[test]
    fn repinning_a_session_moves_the_window_pin() {
        let reg = SessionRootsRegistry::default();
        let key = WindowKey::from_pid(std::process::id());
        reg.attach_window("sess-1", key);
        reg.set_candidates("sess-1", CANDIDATES[0]);
        reg.set_pinned("sess-1", CANDIDATES[0], PinSource::WorkspaceHeader);
        // A MetaTool pin only promotes with single-folder proof — see
        // `meta_tool_pin_promotes_only_with_single_folder_proof` for the
        // no-proof case this test deliberately avoids.
        reg.set_candidates("sess-1", CANDIDATES[1]);
        reg.set_pinned("sess-1", CANDIDATES[1], PinSource::MetaTool);

        reg.attach_window("sess-2", key);
        assert_eq!(
            reg.inherit_window_pin("sess-2"),
            Some(CANDIDATES[1].to_string()),
            "the window must remember the newest explicit claim, not the first"
        );
    }

    #[test]
    fn header_pin_promotes_only_with_single_folder_proof() {
        let reg = SessionRootsRegistry::default();
        let key = WindowKey::from_pid(std::process::id());
        reg.attach_window("sess-a", key);
        reg.set_pinned("sess-a", CANDIDATES[0], PinSource::WorkspaceHeader);
        assert_eq!(
            reg.get("sess-a"),
            Some(vec![CANDIDATES[0].to_string()]),
            "the calling session still gets its own header"
        );
        reg.attach_window("sess-b", key);
        assert!(
            reg.inherit_window_pin("sess-b").is_none(),
            "a header without a one-folder set must not become a process pin"
        );
    }

    #[test]
    fn empty_header_clears_session_pin_and_skips_window_inherit() {
        let reg = SessionRootsRegistry::default();
        let key = WindowKey::from_pid(std::process::id());
        reg.attach_window("sess-shared", key);
        reg.set_candidates("sess-shared", CANDIDATES[0]);
        reg.set_pinned("sess-shared", CANDIDATES[0], PinSource::WorkspaceHeader);
        assert_eq!(
            reg.get("sess-shared"),
            Some(vec![CANDIDATES[0].to_string()])
        );

        reg.forget_empty_header_claim("sess-shared");
        assert!(reg.session_pin("sess-shared").is_none());
        assert!(
            reg.get("sess-shared").is_none(),
            "empty header must not fall through to the remembered window pin"
        );
        assert!(reg.inherit_window_pin("sess-shared").is_none());

        reg.set_pinned("sess-shared", CANDIDATES[0], PinSource::WorkspaceHeader);
        assert_eq!(
            reg.get("sess-shared"),
            Some(vec![CANDIDATES[0].to_string()]),
            "the next real header must lift the suppress"
        );
    }

    /// Regression test for the field-confirmed cross-window leak (see
    /// `docs/planning/window-scoped-workspace-pin.md`, "second incident"):
    /// Cursor's global bridge shares one `mcp-remote` process, and therefore
    /// one `mcp-session-id`, across every open window. A `set_workspace_root`
    /// call on that shared session names no window, so it must not become
    /// the answer every other window inherits after this session ends.
    #[test]
    fn meta_tool_pin_promotes_only_with_single_folder_proof() {
        let reg = SessionRootsRegistry::default();
        let key = WindowKey::from_pid(std::process::id());

        // No candidate set at all (the leak's actual trigger: an unexpanded
        // X-Mcpmux-Workspace-Set never got stored) — must not promote.
        reg.attach_window("sess-shared-a", key);
        reg.set_pinned("sess-shared-a", CANDIDATES[0], PinSource::MetaTool);
        assert_eq!(
            reg.get("sess-shared-a"),
            Some(vec![CANDIDATES[0].to_string()]),
            "the calling session still gets its own answer"
        );

        reg.attach_window("sess-shared-b", key);
        assert!(
            reg.inherit_window_pin("sess-shared-b").is_none(),
            "an unproven meta-tool claim must not leak to a sibling session \
             on the same shared process"
        );

        // A multi-folder candidate set is equally insufficient — it proves
        // the window is one of several, not that it is exactly one.
        reg.attach_window("sess-shared-c", key);
        reg.set_candidates("sess-shared-c", &CANDIDATES.join(","));
        reg.set_pinned("sess-shared-c", CANDIDATES[0], PinSource::MetaTool);
        reg.attach_window("sess-shared-d", key);
        assert!(
            reg.inherit_window_pin("sess-shared-d").is_none(),
            "a multi-folder candidate set is not proof of single-window intent"
        );

        // A one-member candidate set IS proof — the window really has only
        // one folder open right now. `set_candidates` self-pins in that case
        // (decision 4b's SingleCandidate path), so the MetaTool call below is
        // redundant in practice; it's here to confirm a MetaTool claim on an
        // already-proven session is at worst a no-op, never a regression.
        reg.attach_window("sess-solo-a", key);
        reg.set_candidates("sess-solo-a", CANDIDATES[0]);
        reg.set_pinned("sess-solo-a", CANDIDATES[0], PinSource::MetaTool);
        reg.attach_window("sess-solo-b", key);
        assert_eq!(
            reg.inherit_window_pin("sess-solo-b"),
            Some(CANDIDATES[0].to_string()),
            "single-folder proof must still let the window pin work"
        );
    }

    #[test]
    fn explicit_session_pin_beats_window_pin() {
        let reg = SessionRootsRegistry::default();
        let key = WindowKey::from_pid(std::process::id());
        reg.attach_window("sess-1", key);
        reg.set_pinned("sess-1", CANDIDATES[0], PinSource::WorkspaceHeader);
        reg.attach_window("sess-2", key);
        reg.set_pinned("sess-2", CANDIDATES[1], PinSource::WorkspaceHeader);
        assert_eq!(reg.get("sess-2"), Some(vec![CANDIDATES[1].to_string()]));
        assert_eq!(
            reg.session_pin("sess-2"),
            Some(CANDIDATES[1].to_string()),
            "live header must win over the remembered window answer"
        );
    }

    #[test]
    fn inherit_refuses_when_remembered_root_not_in_candidate_set() {
        let reg = SessionRootsRegistry::default();
        let key = WindowKey::from_pid(std::process::id());
        reg.attach_window("sess-1", key);
        reg.set_candidates("sess-1", CANDIDATES[0]);
        reg.set_pinned("sess-1", CANDIDATES[0], PinSource::WorkspaceHeader);
        reg.attach_window("sess-2", key);
        let set = format!("{},{}", CANDIDATES[1], CANDIDATES[2]);
        reg.set_candidates("sess-2", &set);
        assert!(reg.inherit_window_pin("sess-2").is_none());
        assert!(reg.get_pinned("sess-2").is_none());
    }

    #[test]
    fn inherit_skipped_without_window_key() {
        let reg = SessionRootsRegistry::default();
        assert!(reg.inherit_window_pin("sess-1").is_none());
        assert!(reg.window_key_for("sess-1").is_none());
    }

    #[test]
    fn dead_window_pin_is_evicted_on_read_and_remove() {
        let reg = SessionRootsRegistry::default();
        let dead = WindowKey::from_pid(u32::MAX);
        reg.attach_window("sess-1", dead);
        reg.set_candidates("sess-1", CANDIDATES[0]);
        reg.set_pinned("sess-1", CANDIDATES[0], PinSource::WorkspaceHeader);
        reg.attach_window("sess-2", dead);
        assert!(
            reg.inherit_window_pin("sess-2").is_none(),
            "dead PID must not inherit"
        );
        reg.remove("sess-1");
        reg.remove("sess-2");
        reg.attach_window("sess-3", dead);
        assert!(reg.inherit_window_pin("sess-3").is_none());
    }
}
