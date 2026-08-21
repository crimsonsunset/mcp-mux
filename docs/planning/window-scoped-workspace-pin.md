# Window-Scoped Workspace Pin

**Last Updated:** Aug 20, 2026
**Status:** Complete (Phases 1–4). Chosen from a six-option brainstorm as the highest value-per-line fix for the empty-`${workspaceFolder}` residual. Phase 1 load-bearing check passed in-process: unprivileged `netstat2` socket→PID lookup resolves a loopback peer to this process.
**Branch:** `root-resolution`
**Depends on:** [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) Phases 1–3 (shipped) — this reuses the bridge's `X-Mcpmux-Workspace` / `X-Mcpmux-Workspace-Set` headers and the `set_workspace_root` escape hatch rather than replacing any of them
**Unblocks:** One workspace answer per Cursor window instead of one per MCP session — the residual cost that [`resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) item 1 calls inherent

---

## Problem

The ~16% residual is real and this doc does not claim to fix it. What it fixes is the *price* of the residual, which turns out to be a keying bug rather than an ambiguity.

Tonight's gateway log, one machine, roughly six Cursor windows:

| Signal | Count |
| ------ | ----- |
| Distinct `session_id` values | 106 |
| Distinct `client_id` values | 2 |
| Empty `X-Mcpmux-Workspace` (fallback trigger) | 764 |
| `mcpmux_set_workspace_root` calls | 759 |

759 manual pins for six windows is not 759 ambiguities. It is the same handful of answers re-supplied every time a session churns, because [`SessionRootsRegistry`](../../crates/mcpmux-gateway/src/services/session_roots.rs) keys `pinned` by `session_id` and nothing else. The only coarser key available today is `pending_by_client`, keyed by `client_id` — and `client_id` is per-API-key, not per-window, which is why it collapses six windows into two values. So a pin has exactly one lifetime available to it: the session. Every reconnect, every auth bounce, every `mcp-remote` respawn throws the answer away and asks again.

### The dead end that isn't

[`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) Scope/Out records gateway-side process introspection as: *"Dead end — the gateway sees a TCP connection over streamable HTTP, not a spawned child process; there's no PID to walk."*

That premise is wrong, and it's the premise that closed this door. The bridge connects over **loopback**, so the connection's source port maps to the connecting process. Measured on Gondor with four windows live:

```text
node 75164  ->  127.0.0.1:60490, 60491   (mcp-mux window)
node 75278  ->  127.0.0.1:60513, 60535
node 75353  ->  127.0.0.1:60543, 60562
node 75651  ->  127.0.0.1:60587, 60593
```

Four windows, four distinct long-lived `mcp-remote` PIDs, each holding the POST + GET SSE pair. `ConnectInfo<SocketAddr>` is **already** wired on the listener (`into_make_service_with_connect_info::<SocketAddr>()` in [`server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs), already read by `restrict_management_to_loopback`), so the peer port reaches [`oauth_middleware`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) with zero new plumbing.

What the PID does **not** give us is the folder. Two doors stay closed and this doc does not reopen them:

- **Child env is unreadable.** `ps eww <pid>` against a live `mcp-remote` returns no environment on macOS — SIP blocks cross-process env reads even same-user. So the gateway cannot recover `WORKSPACE_FOLDER_PATHS` (or anything else) from the child it just identified.
- **PPID is app-level, as previously measured.** All four `npm exec` parents above share PPID 47899. That matches the `VSCODE_PID` finding in [`resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) item 1 — the extension host is shared across windows, so walking *up* the tree collapses the very distinction we need.

The PID is therefore useful for exactly one thing: **a stable per-window identity that outlives the session.** That is enough to make one answer stick.

### Field evidence (Aug 20, 10:27 PM auth bounce)

The daemon was never the problem during this window. `127.0.0.1:45818/health` returned 200 throughout, `mcpmux` PID 55792 had been up since 6:56 PM, and `/mcp` answered in ~2 ms. The dead part was the Cursor `mcp-remote` pipe, and the failure mode is directly relevant to this design:

- After the bounce, every client respawned with a **literal** `Authorization:Bearer ${MCPMUX_API_KEY}` on its argv. The gateway correctly 401s (`invalid_token`), and Cursor sits on `Timed out waiting for connection to user-mcpmux::mcpScope:...` until it gives up. Recovery is a Reload MCP (or reopening the window) so `mcp-remote` comes back with a real bearer.
- The same recycle left `X-Mcpmux-Workspace-Set:${WORKSPACE_FOLDER_PATHS}` unexpanded on **every** window. That path is already handled — `is_unexpanded_variable()` drops the literal so the session stays unconstrained rather than blanket-denied — but it means the constraint that Option 2-style deduction would depend on evaporates in exactly the moments things are worst.
- The catalog kept looking "ready" the whole time, because a tool catalog is local descriptors, not a live session.

Two things follow. First, the substitution flake is not scoped to `${workspaceFolder}` — it hits `${MCPMUX_API_KEY}` and `${WORKSPACE_FOLDER_PATHS}` through the same `mcp-remote` `${ENV}` pass, so any design that assumes "at least one header survives" is unsafe. Second, a mass respawn is precisely when session-keyed state is most expensive: every window loses its pin simultaneously, and every window has to be re-answered. A window-scoped pin survives the respawn if the process survives, and where the process doesn't survive, it at least collapses N sessions of re-asking into one.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Window identity source | Peer socket port → owning PID, resolved in-process | The port is already available via `ConnectInfo<SocketAddr>`, and the mapping is the only per-window signal that exists after env vars, PPID, and `roots/list` were each ruled out. Corrects the "no PID to walk" entry in the bridge doc's Out table. |
| 2 | How the PID lookup happens | [`netstat2`](https://crates.io/crates/netstat2) crate (one new dep, cross-platform socket→PID) | The alternatives are worse: shelling out to `lsof` spawns a child per new session (and the repo's child-process rules exist for good reason — `configure_child_process_platform()`), and hand-rolling libproc/`/proc`/`GetExtendedTcpTable` means three platform implementations with the CI blind spot documented in `AGENTS.md`. Phase 1 verifies it works unprivileged for own-user sockets before anything is built on it. |
| 3 | Window key shape | `pid` plus a liveness re-check on read, **not** `pid` + process start time | Avoids a second dependency for start-time lookup. A stale entry requires the process to die *and* its PID to be reused *by another `mcp-remote` connected to this gateway*. `ponytail:` ceiling — narrow but not impossible; the upgrade path is adding start time from the same crate family if a misroute is ever observed. Mitigated by decision 5. |
| 4 | What becomes durable | Only **explicit claims** — a substituted `X-Mcpmux-Workspace` header, or a `set_workspace_root` call | Probed `roots/list` values are already suspect (`listChanged: false`, stale across windows) and deductions are not proof. Promoting either to window scope would give a wrong answer a longer life, which is strictly worse than asking again. |
| 5 | Applying a window pin | Re-validate against the session's own candidate set when the set is present; skip validation when the set is absent or unexpanded | Keeps the invariant `set_workspace_root` already enforces — a session can only claim a folder its window actually has open. When the set header didn't survive (see Field evidence), there is nothing to validate against, and refusing would reintroduce the very friction this doc removes. |
| 6 | Precedence | Explicit header for *this* session > window pin > probed roots > `PendingRoots` | A live explicit claim must always beat remembered state, so a genuine window switch is never overridden by a stale pin. This is a new tier inserted below `pinned`, not a change to any existing tier's behavior. |
| 7 | Transport scope | Loopback peers only; remote/tunnel clients get no window pin | A tunnelled client has no local PID to resolve, so there's nothing to key on. Correct outcome, not a gap — those clients keep today's per-session behavior. |
| 8 | Where the state lives | New maps inside `SessionRootsRegistry`, not a new service | Every reader of workspace state already goes through that registry, and `get()`/`get_pinned()` are the natural insertion points. A separate service would need the same locking and the same lifecycle for no benefit. |

---

## Scope

**In:**

- Deriving a window key (owning PID) from the peer socket for loopback MCP requests, logged before it changes any behavior
- Persisting explicit workspace claims at window scope for the life of the `mcp-remote` process
- Applying a remembered window pin to a fresh session that has no explicit claim of its own, gated by the candidate-set check
- Eviction when the owning process goes away, plus tests covering precedence, eviction, and the set-mismatch refusal

**Out:**

| Item | Reason / Deferral |
| ---- | ------------------ |
| Elimination / constraint propagation across live connections (brainstorm Option 2) | Genuinely promising and composes with this work, but independent of it. Deferred until this ships, since durability changes how often deduction is even needed. |
| MCP elicitation folder picker (brainstorm Option 3) | Net-new protocol surface. Worth doing only after durability lands, otherwise it converts ~16% of spawns into a dialog. Cursor does declare the capability (`elicitation: Some(ElicitationCapability { form: … })`), so the door is open. |
| Opportunistic pinning from tool-call path arguments (Option 4) | Can't help at `tools/list` time, and coverage depends on which backends a workspace uses. Passive supplement at best. |
| First-party bridge binary replacing `npx mcp-remote` (Option 5) | Creates no signal that doesn't already exist. Still parked per bridge-doc Decision 1. |
| Reading Cursor's own `workspaceStorage` / `globalStorage` state files | A focus-recency heuristic, so it can never gate credentials. Only viable as picker *ordering* for the deferred elicitation work. |
| Tilde expansion for non-loopback clients | **Separate bug, needs its own fix.** `normalize_workspace_root()` expands `~` against *this machine's* home on the documented assumption that gateway and client share a filesystem. A Home Assistant OS client sending `X-Mcpmux-Workspace:~/helm` breaks that assumption — the value refers to the HA OS root, and expanding it locally produces a path that means something entirely different. Observed live tonight. Should be gated on peer locality (the same `ConnectInfo` signal decision 1 adds) and refused for remote peers. |
| Making the per-repo static install the recommended path | Blocked on the `.gitignore` gap in [`workspace_install.rs`](../../apps/desktop/src-tauri/src/commands/workspace_install.rs) — it writes a bearer token into a repo-local config with nothing stopping `git add .` from committing it. Tracked in `resilience-routing-leftovers.md` item 1. |
| Reporting the `${MCPMUX_API_KEY}` respawn flake upstream to Cursor | Worth filing alongside the 282-spawn `${workspaceFolder}` dataset, but it's a forum post, not code. |

---

## Architecture

### Pin lifetime (before → after)

```text
Before — pin dies with the session:
  window A ──spawn──> mcp-remote (pid 75278) ──session s1──> pin(s1) = /repo/a
                                             ──session s2──>  (nothing; ask again)
                                             ──session s3──>  (nothing; ask again)
                                                              ... 106 sessions tonight

After — pin lives as long as the window's bridge process:
  window A ──spawn──> mcp-remote (pid 75278) ──session s1──> pin(s1) = /repo/a
                                                             └─> window_pin(75278) = /repo/a
                                             ──session s2──> inherits /repo/a
                                             ──session s3──> inherits /repo/a
                       (process exits) ────────────────────> window_pin evicted
```

### Resolution ladder

The new tier slots in below the existing pin and above probed roots. Nothing above it changes.

```text
1. pinned[session]            explicit header or set_workspace_root, this session   (unchanged, authoritative)
2. window_pin[window_key]     remembered explicit claim from the same bridge process  (NEW)
3. map[session]               probed roots/list                                     (unchanged)
4. PendingRoots               ambiguous or absent — today's behavior                (unchanged)
```

### State added to `SessionRootsRegistry`

```rust
/// `window_key -> explicit workspace root` — survives session churn for the
/// life of the owning `mcp-remote` process. Only ever written from an
/// explicit claim (header pin or `set_workspace_root`), never from probed
/// roots or a deduction.
window_pins: DashMap<WindowKey, String>,
/// `session_id -> window_key` so session teardown and pin promotion can
/// both find the window without redoing the socket lookup.
session_window: DashMap<String, WindowKey>,
```

`WindowKey` starts as the owning PID (decision 3). The socket→PID lookup is memoized per session on first sight, so it runs once per session rather than once per request.

---

## Files to create / modify

| Area | File | Action |
| ---- | ---- | ------ |
| Gateway | [`crates/mcpmux-gateway/src/services/window_identity.rs`](../../crates/mcpmux-gateway/src/services/window_identity.rs) | Create — `resolve_window_key(peer: SocketAddr) -> Option<WindowKey>`; loopback guard, `netstat2` lookup, liveness re-check |
| Gateway | [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | Modify — add `window_pins` / `session_window`, `promote_pin_to_window()`, `inherit_window_pin()`, evict in `remove()`; `get()`/`get_pinned()` consult tier 2 |
| Gateway | [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Modify — read `ConnectInfo<SocketAddr>`, memoize the window key per session, promote on explicit header pin, attempt inheritance when the header is empty |
| Gateway | [`crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs) | Modify — a successful manual pin also promotes to window scope; description mentions the pin now persists for the window |
| Gateway | [`crates/mcpmux-gateway/Cargo.toml`](../../crates/mcpmux-gateway/Cargo.toml) | Modify — add `netstat2` |
| Docs | [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) | Modify — document the window pin, its lifetime, and how to observe it in logs |
| Docs | [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) | Modify — correct the "no PID to walk" Out-table entry with the loopback measurement |

---

## Phases

### Phase 1 — Window identity, observability only (~half day)

Proves the load-bearing assumption before any behavior depends on it. No routing changes.

- `window_identity.rs`: loopback check, `netstat2` socket→PID lookup, `WindowKey`
- Memoize the key per session in `oauth_middleware`; log it alongside the existing `session_id` / `workspace_header` info line
- Verify unprivileged operation on macOS for own-user sockets, and confirm the Linux path compiles and resolves (CI is Linux; see the cross-platform note in `AGENTS.md`)

**Outcome:** With four windows open and normal churn, the log shows a small stable set of window keys spanning many `session_id`s — concretely, one PID covering the same window's sessions across a Reload MCP. If instead every session gets its own key, or the lookup returns nothing unprivileged, the design is dead and this doc gets amended before Phase 2. Contingency if `netstat2` can't see the socket: a once-per-session `lsof` shell-out through `configure_child_process_platform()`, accepted as a heavier fallback.

---

### Phase 2 — Durable pin and inheritance (~1 day)

Makes one answer stick for the window.

- `window_pins` / `session_window` on the registry, with `promote_pin_to_window()` called from both explicit-claim paths (header pin in the middleware, `set_workspace_root` on success)
- `inherit_window_pin()` consulted when a session has no explicit claim, gated by the candidate-set check from decision 5
- Wire tier 2 into `get()` / `get_pinned()` so the resolver, the probe skip, and prompt-root derivation all honor it with no special-casing — the same property the session pin already has
- Log inheritance distinctly from a fresh pin so the two are separable in the field

**Outcome:** Pin a multi-folder window once with `mcpmux_set_workspace_root`, then Reload MCP in Cursor. The new session resolves to the same folder with no second pin call, and the log shows an inheritance line rather than another empty-header warn. `set_workspace_root` call volume for a steady-state session drops from per-session to per-window.

---

### Phase 3 — Eviction and safety (~half day)

Closes the ways a remembered pin could outlive its truth.

- Evict `window_pins` when the owning process no longer holds a connection to the gateway; drop `session_window` in `remove()`
- Assert precedence: a live explicit header always overrides an inherited pin (a genuine window switch must win)
- Refuse inheritance when the session's candidate set is present and the remembered root isn't in it, with a warn naming both
- Tests: precedence ordering, eviction on process exit, set-mismatch refusal, and inheritance skipped for non-loopback peers

**Outcome:** Closing a Cursor window drops its pin (a later window reusing that PID inherits nothing), and a window that switches folders re-pins immediately instead of serving the previous answer. `pnpm test:rust` covers all four cases.

---

### Phase 4 — Docs (~2 hours)

- `cursor-workspace-bridge.md`: what the window pin is, how long it lives, which log lines to grep, and the fact that a Reload MCP no longer costs a re-pin
- `cursor-workspace-routing-bridge.md`: correct the process-introspection Out entry — the loopback port→PID path is real; what stays dead is env reads (SIP) and PPID (app-level)
- `resilience-routing-leftovers.md` item 1: note that the residual's *cost* is addressed here even though the residual itself is unchanged

**Outcome:** The next person reading the bridge doc doesn't re-derive that process introspection is impossible, and can tell from the logs alone whether a given session was pinned, inherited, or genuinely unresolvable.

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | `pinned` is session-keyed; `pending_by_client` is client-keyed (2 values for 6 windows). The gap this doc fills sits between them |
| [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Header hold-then-pin, the empty-header warn, and the set-header constraint all live here — as will the window-key memoization |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | `into_make_service_with_connect_info::<SocketAddr>()` is already wired; `restrict_management_to_loopback` is prior art for trusting the peer socket over a spoofable header |
| [`crates/mcpmux-core/src/domain/workspace_binding.rs`](../../crates/mcpmux-core/src/domain/workspace_binding.rs) | `normalize_workspace_root()` / `expand_home_tilde()` — the shared-filesystem assumption that the HA OS `~/helm` case violates |
| [`crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs) | The manual escape hatch whose 759 nightly calls motivate this work; also the set-membership refusal reused by decision 5 |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | Owns the `PendingRoots` tier the new tier sits above |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs) | `peer.list_roots()` probe and the `[FeatureSetResolver] resolved` line that will show inherited roots |
| [`apps/desktop/src-tauri/src/commands/workspace_install.rs`](../../apps/desktop/src-tauri/src/commands/workspace_install.rs) | The immune per-repo path, still blocked on its `.gitignore` gap |
| [`scripts/cursor-env-probe.mjs`](../../scripts/cursor-env-probe.mjs) | Re-measure substitution failure if the rate needs rechecking after any Cursor update |

---

## Related documentation

- [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — the bridge this builds on; its Out table's process-introspection entry is corrected by Phase 4
- [`docs/planning/resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) — item 1 is the residual whose cost this addresses; also records the ruled-out env vars and the MCP `2026-07-28` Roots deprecation
- [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) — user-facing bridge setup and the re-measure recipe
- [`docs/manual/workspace-header-routing.md`](../manual/workspace-header-routing.md) — the original Cursor `roots`-reporting bug behind the header design
- [`docs/planning/per-device-machine-header.md`](./per-device-machine-header.md) — prior art for a routing signal keyed to something coarser than a session
