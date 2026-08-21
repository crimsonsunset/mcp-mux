# Cursor Workspace Routing via Global `mcp-remote` Bridge

**Last Updated:** Aug 20, 2026
**Status:** Complete (Phases 1–3) — Agents Window spike **done**; multi-root ambiguity gate covers resolver + bind + list_servers note. Non-empty header without session id is **held then pinned** (`dcc2977`). Empty `${workspaceFolder}` **measured and bounded** (`efabe48`); the residual ~16% is inherent, not a gap. Re-measure via `pnpm probe:cursor-env` / `pnpm probe:cursor-env:summary`.
**Branch:** `dev-rebased`

### Resolved question (Aug 20, 2026) — supersedes the Aug 14 open question

The Aug 14 entry asked when and why Cursor spawns an `mcp-remote` child without
resolving `${workspaceFolder}`, and assumed the Agents window was responsible. A
282-spawn env-probe wrapper answered it, and the assumption was wrong. Re-run
via [`scripts/cursor-env-probe.mjs`](../../scripts/cursor-env-probe.mjs) and
[`scripts/cursor-env-probe-summary.mjs`](../../scripts/cursor-env-probe-summary.mjs)
(`pnpm probe:cursor-env` / `pnpm probe:cursor-env:summary`; recipe in
[`cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) How to
re-measure):

- **Failure is ~21% overall and worse in editor windows** (29%) than Agents
  windows (4%). It happens at every folder count from zero to five. This is a
  flaky substitution, not a surface-specific behavior. The doc's earlier claim
  that "editor windows substitute the variable at spawn time" is false as an
  absolute.
- **Cursor emits the literal; `mcp-remote` strips it.** The unresolved
  `${workspaceFolder}` reaches the child, `mcp-remote`'s own `${ENV}` pass finds
  no matching variable, and rewrites it to an empty string. That answers the
  "literal or stripped" half of the question.
- **No active-folder fallback signal exists.** All 22 Cursor/VS Code child env
  vars were checked; details and the ruled-out candidates are in
  [`resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) item 1.
- **`WORKSPACE_FOLDER_PATHS` is sound as a constraint, useless as a selector.**
  Active folder present in the set 212/212; position correct only 70%.

**Shipped (`efabe48`):** the set rides along as `X-Mcpmux-Workspace-Set` and is
used only to collapse one-member sets, bound what `mcpmux_set_workspace_root`
may declare, and name candidates in refusals. Nothing infers a folder from it.

**Residual:** ~16% of spawns (multi-folder window plus failed substitution) cost
one `set_workspace_root` call per session. Closing that needs Cursor to either
make substitution reliable or export the active folder as an env var. The
per-repo static header install remains the only fully immune path.

### Phase 1 spike results (Jul 20, 2026)

- **mcp-remote:** `0.1.38` via `npx`; supports `--allow-http` and `--header` (no space after `:`).
- **Gateway:** `localhost:45818` up (`0.5.0`).
- **Auth + connect:** `phase1-spike-bridge` client reached gateway; machine-naming dialog appeared and was approved.
- **Remaining manual QA:** two-window `${workspaceFolder}` routing not yet verified in real Cursor; transport/auth path is confirmed.

**Depends on:** `docs/manual/workspace-header-routing.md` (existing per-repo header fix this supersedes as the recommended path), `upstream-client-mapping-reconciliation.md` Phase 1 (`mcpk_` API-key auth — this feature's auth mechanism)
**Unblocks:** Zero-maintenance Cursor workspace routing — no per-repo files, no agent cooperation required

### Agents Window multi-workspace spike (Jul 24, 2026)

**Hypothesis:** Cursor Agents Window groups agents by workspace in the UI, but may share one MCP session / mis-resolve `${workspaceFolder}` across workspaces, so mux cannot pin the correct root→FeatureSet binding. Gondor-local + global bridge config is already the intended Editor path; this spike proves what Agents Window actually sends.

**Observability (shipped):** gateway logs now include:

| Signal | Where | Level |
| ------ | ----- | ----- |
| `session_id` + `workspace_header` on every MCP POST | `oauth_middleware` `→ MCP` | info |
| Workspace header without `mcp-session-id` (held, then pinned) | `oauth_middleware` | info (`held until mcp-session-id exists`) |
| Workspace header present but empty (pin skipped) | `oauth_middleware` | warn |
| First pin / same-session root clobber | `session_roots.set_pinned` | info / warn |
| `workspace_root` on resolve | `handler` `[FeatureSetResolver] resolved` | info |
| `x-mcpmux-workspace` / `x-mcpmux-machine-id` in DEBUG request headers | `logging_middleware` | debug |

**Repro (Gondor):**

1. Rebuild/restart the desktop gateway so the new logs are live.
2. In Agents Window, start one agent under workspace A and one under workspace B (both already machine-bound on Gondor, e.g. `mcp-mux` vs `sync2hire-platform`).
3. From each agent, call any `mcpmux_*` tool (e.g. `mcpmux_list_servers` or `mcpmux_search_tools`).
4. Grep gateway logs: `SessionRoots`, `workspace_header`, `pin clobber`, `→ MCP`, `[FeatureSetResolver] resolved`.

**Pass:** distinct `session_id` values; each `workspace_header` / resolved `workspace_root` matches that agent's workspace; no `pin clobber` warn.

**Fail:** shared `session_id` with `pin clobber` (previous ≠ new), or `workspace_header=<absent>`, or empty header (Agents window). A non-empty header on initialize (no `mcp-session-id` yet) is **not** a fail — it is held, then pinned.

**Next if fail:** prefer per-repo static `.cursor/mcp.json` header for Agents Window, and/or treat as Cursor Agents Window MCP binding gap (not a new per-agent identity axis).

**Spike results (Jul 24–27, 2026):** Session isolation works — Agents Window agents in separate workspaces get distinct `session_id`s and (when present) distinct `X-Mcpmux-Workspace` headers; no pin-clobber. Identical tool answers in the repro were overlapping FeatureSets, not shared-session clobber. Real bug found: some sessions arrive with an empty/absent workspace header, so `SessionRootsRegistry::get()` returns the full multi-folder `roots/list` and the resolver used to first-match-wins across that list. **Fix shipped:** resolver returns `PendingRoots` whenever `reported_roots.len() > 1` (no pinned header); escape hatch is `mcpmux_set_workspace_root` / a correct header pin. Complements the bridge's header injection as a server-side safety net.

**Follow-up (Jul 27–28, 2026):** An unpinned dual-root session still let `mcpmux_bind_current_workspace` first-root-wins and offered to append `bundle:gait` onto `sync2hire-platform` (approval dialog). **Bind now shares the multi-root gate:** refuses with an `isError` listing the reported roots and instructing `mcpmux_set_workspace_root` with exactly one path before retry. `mcpmux_list_servers` adds a `note` with the same candidate list when resolution is `PendingRoots`. Pre-approval bind logs include `session_id` / `chosen_root` / `feature_set_id`.

---

## Problem

Cursor doesn't reliably report the MCP `roots` capability — it can report a stale or wrong workspace folder (e.g. a different open window's folder), so the resolver's path-based `WorkspaceBinding` lookup gets the wrong root and two folders mapped to different FeatureSets can cross-contaminate (`docs/manual/workspace-header-routing.md`).

The existing fix (`apps/desktop/src-tauri/src/commands/workspace_install.rs`) writes a project-local `.cursor/mcp.json` per repo with an `X-Mcpmux-Workspace` header baked in, because a *global* Cursor config can only hold one static header value — it can't vary per project. This works, but it's a real, standing maintenance burden: every new repo needs a manual "Install into 1 app" click plus a `.gitignore` entry, forever.

The other obvious escape hatch — the `mcpmux_set_workspace_root` meta tool, which lets an agent self-report its root — trades the per-repo file for a dependency on the LLM actually calling it every session. Not deterministic enough to rely on as the primary mechanism.

Cursor's own docs claim `${workspaceFolder}` resolves in the `command`/`args`/`env` fields of a stdio server entry — even one declared in the *global* `~/.cursor/mcp.json` — because Cursor spawns a stdio child fresh per workspace window and substitutes variables at spawn time rather than file-parse time. The documented interpolation flakiness (Cursor forum bug reports) is specific to the `headers` field on a native `url`-type (remote) entry. That gap is what this design exploits: route Cursor through a stdio bridge and pass the workspace header through the bridge's `args`.

**Measured correction (Aug 20, 2026):** `args` interpolation is *more* reliable than the `headers` field, but not reliable. Across 282 real spawns it failed 21% of the time (29% in editor windows, 4% in Agents windows). The design premise holds directionally and the bridge is still the right default, but "reliable" overstated it, and everything downstream has to assume the header can arrive empty. See the resolved question at the top of this doc.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Bridge implementation | **`mcp-remote`** (existing npm package, `npx mcp-remote`), not a first-party McpMux binary | Already solves stdio↔remote-HTTP bridging with a `--header` flag that supports arbitrary custom headers. Building our own binary duplicates it for no gain unless `mcp-remote` proves unreliable in practice (Phase 1 spike decides this). |
| 2 | Workspace signal | `${workspaceFolder}` passed inline inside a `--header` value in the bridge's `args`, e.g. `--header X-Mcpmux-Workspace:${workspaceFolder}` | The better of the two interpolation paths (`args`/`command`), versus the worse one (`headers` on a native remote entry). No space around the `:` to dodge Cursor's known arg-escaping bug with `npx`. **Measured at 21% failure** (Aug 20), so it can't be the only signal — see decision 6. |
| 6 | Fallback signal (added Aug 20, 2026) | Also send `${WORKSPACE_FOLDER_PATHS}` as `X-Mcpmux-Workspace-Set`, and treat it purely as a **constraint** | Not a Cursor variable, so it survives to `mcp-remote`, which expands it from the child env — it arrives even when `${workspaceFolder}` didn't. The active folder is always a member (212/212) but its position is right only 70% of the time, so the set may bound and disambiguate, never select. Rejected alternatives: first-entry heuristic (30% credential misroute), `CURSOR_WORKSPACE_LABEL` (stale), per-window process identity (`VSCODE_PID`/`VSCODE_IPC_HOOK` are app-level), FeatureSet union across roots (defeats isolation). |
| 3 | Auth | Static `mcpk_` API-key header (`Authorization: Bearer mcpk_...`) via a second `--header` flag, not OAuth-through-the-bridge | `mcp-remote` does its own OAuth dance if no static header is given, which is one more auth surface to reason about. The API-key auth path shipped in `upstream-client-mapping-reconciliation.md` Phase 1 exists for exactly this kind of headless/remote-client case. |
| 4 | Relationship to existing per-repo install | **Keep both** — the global bridge is the recommended setup for single-folder windows; the per-repo `.cursor/mcp.json` header install (`workspace_install.rs`) is the recommended setup for multi-root windows, not merely a fallback | Original rationale (don't replace a tested mechanism with an unverified one) held up, and the Aug 20 measurement sharpened the split: the per-repo install writes a literal path with no variable to substitute, so it is the *only* path immune to the 21% flake. Revisit whether the UI should say so. Blocker first: it writes the bearer token into a repo-local file with no `.gitignore` entry. |
| 5 | Scope of client support | Cursor only — no changes for VS Code, Claude Code, or other clients | Those clients already route correctly via standard `roots` reporting (confirmed in `docs/manual/workspace-header-routing.md`: "VS Code / Claude Code are good controls — they already route correctly via roots"). This is a Cursor-specific spec-compliance gap, not a general McpMux limitation. |

---

## Scope

**In:**

- Manual spike confirming `${workspaceFolder}` resolves per-window correctly through a global `mcp-remote` entry in real Cursor (not just per docs)
- A generated global bridge config snippet, surfaced in the desktop app, that mints an `mcpk_` API key and emits ready-to-paste JSON for `~/.cursor/mcp.json`
- Docs update recommending the global bridge as the primary Cursor setup path, with the existing per-repo header install documented as the fallback

**Out:**

| Item | Reason / Deferral |
| ---- | ------------------ |
| First-party McpMux bridge binary (replacing `mcp-remote`) | Decision 1 — only worth building if the Phase 1 spike finds `mcp-remote` unreliable or insufficient. Not blocking this feature. |
| Cursor/VS Code extension reading `vscode.workspace.workspaceFolders` directly | Disproportionate effort (a whole editor extension) for a gap that's isolated to one client's `roots` implementation. Revisit only if this class of bug recurs across other clients. |
| Deprecating/removing the per-repo `.cursor/mcp.json` install panel | Decision 4 — stays as a supported fallback indefinitely, not a transitional shim to delete later. |
| Gateway-side process-tree introspection to infer workspace without any client config | Dead end — the gateway sees a TCP connection over streamable HTTP, not a spawned child process; there's no PID to walk. Not pursued. |

---

## Architecture

### Connection shape (before → after)

```text
Before:
  Cursor  --url: http://localhost:45818/mcp-->  McpMux Gateway
          (roots/list unreliable, or per-repo .cursor/mcp.json header)

After (global, zero per-repo files):
  Cursor  --spawns per window-->  npx mcp-remote (stdio child)
                                       |
                                       | --header X-Mcpmux-Workspace:<resolved per window>
                                       | --header Authorization:Bearer mcpk_...
                                       v
                                  McpMux Gateway (http://localhost:45818/mcp)
```

Cursor resolves `${workspaceFolder}` to the active window's project root *before* spawning `npx`, so each window's `mcp-remote` child process carries a different, correct header value — from one global config entry, with no per-repo file and no agent involvement.

### Global config shape

```jsonc
// ~/.cursor/mcp.json
{
  "mcpServers": {
    "mcpmux": {
      "command": "npx",
      "args": [
        "-y", "mcp-remote",
        "http://localhost:45818/mcp",
        "--allow-http",
        "--header", "X-Mcpmux-Workspace:${workspaceFolder}",
        "--header", "Authorization:Bearer ${MCPMUX_API_KEY}"
      ],
      "env": { "MCPMUX_API_KEY": "mcpk_..." }
    }
  }
}
```

`--allow-http` is required since the gateway binds plain HTTP on loopback (`127.0.0.1:45818`), not HTTPS — `mcp-remote` otherwise assumes a TLS remote endpoint.

### Interaction with existing resolver tiers

The bridge is a transport-layer trick to get `X-Mcpmux-Workspace` populated correctly — the gateway already treats that header as authoritative and pins it ahead of probed `roots` (`session_roots.rs`, `SessionRootsRegistry`). When the header is absent and `roots/list` returns multiple folders, `feature_set_resolver.rs` holds at `PendingRoots`, and `mcpmux_bind_current_workspace` refuses with a recoverable error listing candidates (multi-root ambiguity gate, Jul 27–28) — the server-side safety net for the empty-header path the Agents Window spike found.

---

## Files to create / modify

| Area | File cluster | Action |
| ---- | ------------- | ------ |
| Desktop UI | `apps/desktop/src/features/clients/CursorBridgeSection.tsx` (or fold into `ClientsPage.tsx`) | Create — "Global Cursor setup (no per-repo files)" panel: mints an `mcpk_` key via the existing Phase 1 API-key commands, renders the ready-to-paste `~/.cursor/mcp.json` snippet, one-click copy |
| Tauri | `apps/desktop/src-tauri/src/commands/oauth.rs` | Modify (if needed) — reuse `create_client_api_key`/`register_api_key_client` from `upstream-client-mapping-reconciliation.md` Phase 1; no new command expected unless the UI needs a combined "register + mint key + render snippet" convenience call |
| Docs | `docs/manual/workspace-header-routing.md` | Modify — add a section presenting the global bridge as the recommended path, existing per-repo install as fallback |
| Docs | `docs/guide/remote-access.mdx` | Modify — mention the bridge option alongside existing tunneled-client config guidance, if applicable |
| Manual test | `docs/manual/cursor-workspace-bridge.md` | Create — step-by-step verification doc for Phase 1's spike (two windows, two folders, confirm correct routing per window) |

---

## Phases

### Phase 1 — Manual spike, no code (~1 hour)

Confirms the core assumption before any UI work is built on top of it.

- Manually write a global `~/.cursor/mcp.json` per the shape above, using a manually-minted `mcpk_` key (via existing Clients page UI from Phase 1 of the client-mapping reconciliation work)
- Open two real folders in two separate Cursor windows, each already mapped to a distinct FeatureSet via existing Workspace bindings
- Confirm via gateway logs that each window's `mcp-remote` child sends a different, correct `X-Mcpmux-Workspace` value, and that each window's agent sees only its own bound FeatureSet's tools
- Confirm `--allow-http` and the no-space `--header` syntax are both necessary/sufficient (verify against the actual installed `mcp-remote` version, not just its README)

**Outcome:** Either the bridge works exactly as designed (two Cursor windows on two folders, zero per-repo files, correct tool sets in each) — in which case Phase 2 proceeds — or it surfaces a real gap (e.g. `${workspaceFolder}` doesn't resolve for a global-scope entry the way the docs imply), in which case this doc gets amended before any UI is built.

---

### Phase 2 — Desktop UI generator (~1 day)

Removes the "hand-assemble JSON" friction so the bridge is actually usable by someone who isn't reading this planning doc.

- `CursorBridgeSection.tsx` (or equivalent): a panel that, on click, mints a new `mcpk_` API key scoped to a client named something like `cursor-global-bridge`, and renders the full `~/.cursor/mcp.json` snippet with the key already substituted in
- One-click copy of the snippet; a short inline note explaining it replaces the need for per-repo `.cursor/mcp.json` files
- No changes to the per-repo install panel — both paths coexist as documented alternatives (Decision 4)

**Outcome:** A user can go from "never configured this" to a working global bridge in under a minute, without touching a terminal or writing JSON by hand.

---

### Phase 3 — Docs consolidation (~half day)

- `docs/manual/workspace-header-routing.md`: add the global-bridge path as the recommended Cursor setup, explicitly keep the per-repo install documented as the supported fallback (not deprecated)
- New `docs/manual/cursor-workspace-bridge.md`: manual verification steps mirroring Phase 1's spike, so this stays a repeatable regression check rather than a one-time investigation
- Cross-link from `docs/guide/remote-access.mdx` if the tunneled/remote-gateway story overlaps

**Outcome:** Someone new to the repo can find and follow the recommended Cursor setup without reading this planning doc or the original brainstorm conversation.

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`apps/desktop/src-tauri/src/commands/workspace_install.rs`](../../apps/desktop/src-tauri/src/commands/workspace_install.rs) | The existing per-repo header install this feature supplements, not replaces |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | `X-Mcpmux-Workspace` pin is authoritative; Agents Window spike adds pin/clobber info+warn logs. Holds the `X-Mcpmux-Workspace-Set` candidate list, collapses one-member sets to a pin, and audits the "active folder is in the set" invariant on every change |
| [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | `→ MCP` logs `session_id` + `workspace_header`; non-empty header without sid is held then pinned; empty header still warn-skips. Reads the set header with the same hold-then-apply, and warns when either header arrives as an unexpanded `${…}` template |
| [`crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/set_workspace_root.rs) | Refuses a declared root that isn't in the caller's folder set — closes the self-service grant where any approved client could name any path |
| [`apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts`](../../apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts) | Emits `X-Mcpmux-Workspace-Set:${WORKSPACE_FOLDER_PATHS}` alongside the active-folder header |
| [`scripts/cursor-env-probe.mjs`](../../scripts/cursor-env-probe.mjs) | Drop-in Node wrapper that logs argv/env/pwd then execs `mcp-remote`; `pnpm probe:cursor-env` prints the mcp.json swap |
| [`scripts/cursor-env-probe-summary.mjs`](../../scripts/cursor-env-probe-summary.mjs) | Reprints the Aug 20 study cuts from `$HOME/Desktop/mcpmux-env-probe.log` |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs) | Resolver resolved log includes `workspace_root` |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | Multi-root ambiguity → `PendingRoots` when `get()` returns >1 root (no pin) |
| [`crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/bind_workspace.rs) | Same multi-root gate on bind; fat recoverable error + pre-approval info log |
| [`crates/mcpmux-gateway/src/services/meta_tools/list_servers.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/list_servers.rs) | `PendingRoots` `note` lists candidate roots for agent self-heal |
| [`docs/manual/workspace-header-routing.md`](../manual/workspace-header-routing.md) | Documents the underlying Cursor `roots`-reporting bug this bridge works around |
| [`docs/planning/upstream-client-mapping-reconciliation.md`](./upstream-client-mapping-reconciliation.md) | Phase 1 — `mcpk_` API-key auth, reused here as the bridge's auth mechanism |
| [`apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx`](../../apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx) | Existing API-key minting UI this feature's Phase 2 panel is modeled on |

---

## Related documentation

- [`docs/manual/workspace-header-routing.md`](../manual/workspace-header-routing.md) — the Cursor `roots`-reporting bug and the per-repo header fix
- [`docs/planning/upstream-client-mapping-reconciliation.md`](./upstream-client-mapping-reconciliation.md) — `mcpk_` API-key auth this feature depends on
- [`docs/planning/per-device-machine-header.md`](./per-device-machine-header.md) — prior art for a header-based routing signal (`X-Mcpmux-Machine-Id`), same pattern applied to a different axis
- [`docs/planning/pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md) — hold-then-pin for non-empty `X-Mcpmux-Workspace` without session id (`dcc2977`)
- [`docs/planning/resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) — the empty-header/Agents-window open question here is item 1 in that doc's next-work list
