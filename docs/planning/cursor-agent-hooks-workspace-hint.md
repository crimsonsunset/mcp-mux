# Cursor Agent Hooks as a Workspace Signal

**Last Updated:** Aug 21, 2026
**Status:** Proposed, not spiked. Written the same night [`window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) shipped its decision-4b patch — that patch stops a wrong pin from becoming permanent on the shared global bridge, but it does not give the shared session a *right* answer. This doc is the first design that could.
**Branch:** `root-resolution`
**Depends on:** [`window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) (reuses `SessionRootsRegistry`'s tiered resolution and the loopback-trust model `window_identity.rs` established) and [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) (extends the global bridge's config-generator UI rather than replacing it)
**Unblocks:** The ~16% residual both docs above call inherent — a session on the shared global bridge that today needs a manual `mcpmux_set_workspace_root` call every single time, because nothing about that process is unique to one window

---

## Problem

Decision 4b closed a real leak: a `set_workspace_root` pin on the shared `mcp-remote` process no longer promotes to `window_pins` without independent proof of single-window intent. That is correct and it is not enough — it converts a *wrong, durable* answer back into *no* answer. The session still has to call `mcpmux_set_workspace_root` on every reconnect, because every signal the gateway can see from that connection (`mcp-session-id`, the peer PID, `roots/list`) is shared by every window that has the global bridge open. `window-scoped-workspace-pin.md`'s own field evidence ("759 manual pins for six windows") is the size of this cost when the design was still leaking; decision 4b removes the leak but leaves the 759 calls.

Every fix considered so far has tried to extract more signal from the *transport* — the loopback socket, the child's env, `roots/list`, `WORKSPACE_FOLDER_PATHS`. All of them are downstream of the same fact: Cursor spawns one `mcp-remote` child for the global bridge entry regardless of window count, so nothing arriving over that one TCP connection can be window-scoped no matter how it's read.

Cursor has a second, unrelated channel that doesn't go through `mcp-remote` at all: **hooks**. `hooks.json` (project, user, team, or enterprise scoped) lets Cursor spawn a fresh process per agent lifecycle event — `sessionStart`, `beforeSubmitPrompt`, `preToolUse`, `postToolUse`, `afterFileEdit`, `afterAgentResponse`, and others — and every one of those events carries a `workspace_roots` array that Cursor itself fills in for the exact window that triggered it, plus a `CURSOR_PROJECT_DIR` env var on the spawned process. There is no shared-process problem here because there is no shared process: each hook firing is its own spawn, the same way each `mcp-remote` child would be if Cursor gave the global bridge one per window (it doesn't).

The npm package [`cursor-agent-wakatime`](https://github.com/ryanhiizy/cursor-agent-wakatime) is existing proof this channel works for exactly this kind of per-window attribution problem: it installs `afterAgentResponse` / `afterFileEdit` / `postToolUse` hooks to attribute AI coding time to the right project when a developer has several Cursor windows open, which is the same ambiguity this doc is trying to resolve for MCP routing instead of time tracking.

**What hooks don't give for free:** a hook fires in Cursor's own process, out-of-band from the MCP transport. Nothing today ties a hook invocation to the specific HTTP request that later arrives at the gateway through the shared `mcp-remote` child. Closing that gap is this doc's actual technical risk, and Phase 1 exists to measure it rather than assume it.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Hook event | `beforeSubmitPrompt` (fires once per user turn), not `preToolUse` (fires once per tool call) | A turn's tool calls all belong to the same window, so one hint per turn covers them all. `preToolUse` is more precise but far chattier — every `mcpmux_*` call would spawn a hook process. Start with the cheaper signal; the per-call event is the documented upgrade path if the turn-level heuristic (decision 3) proves too coarse. |
| 2 | Delivery mechanism | The hook script does one `POST` to a new loopback-only gateway endpoint with `{workspace_roots}` from its own stdin payload | The gateway already trusts loopback for desktop-only surfaces (`restrict_management_to_loopback` in [`server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs)); a hook process spawned by the user's own Cursor is exactly that trust level. No new auth surface, no key to embed in a hooks file. |
| 3 | What the gateway does with a hint | Hold it as a short-TTL (proposed 5s), FIFO, best-effort hint — **never** written to `pinned` or `window_pins` | The hook doesn't know the `mcp-session-id` its own turn's tool calls will arrive on, so there's nothing to key a durable write to. Treating it as durable would risk the exact class of bug decision 4b just closed. `ponytail:` ceiling — two turns started in two different windows within the same ~5s window can still grab each other's hint; the upgrade path is decision 4b below turning this into an exact match instead of a queue. |
| 3b | How this differs from `window_pins` | Applied per-request from a queue, not per-window from a table | `window_pins` answers "which folder does this *process* always mean," which is false on the shared bridge. This tier answers "which folder did the *most recent nearby prompt* mean," which degrades gracefully (a miss falls through to `PendingRoots`, same as today) instead of failing durably wrong. |
| 4 | Correlation precision (Phase 1 question) | Measure whether the hook's `tool_call_id` / `generation_id` shows up anywhere on the corresponding `tools/call` JSON-RPC request the gateway receives | If Cursor's outbound MCP request carries a matching id, decision 3's FIFO queue becomes exact matching and the race in 3's ceiling disappears entirely. If not, the FIFO/TTL design ships as the ceiling-carrying fallback it's written as. This is exactly the kind of load-bearing assumption `window-scoped-workspace-pin.md` Phase 1 proved before Phase 2 was built, applied here to a different unknown. |
| 5 | Resolution tier placement | New tier sits **below** `window_pins`, **above** probed `roots/list` | A window pin is still stronger proof than a turn-level hint when one exists (e.g. a per-repo install, or a single-candidate promotion). The hint only matters for exactly the case nothing else resolves: the shared global bridge with an ambiguous candidate set. |
| 6 | Scope | Cursor only; global (`~/.cursor/hooks.json`) bridge only, not per-repo installs | Same client-scoping precedent as decision 5 in `cursor-workspace-routing-bridge.md` — other clients already route correctly via `roots`. Per-repo installs (`workspace_install.rs`) are already fully deterministic via a static header; adding a hook there solves an already-solved case. |
| 7 | Hook script shape | An inline one-liner `command`/`args` in `hooks.json` (e.g. `node -e "..."`), not a shipped/versioned script file or npm package | Fewest files, nothing to version-sync between the desktop app and a published package. The one-liner reads stdin, does one `fetch`, and passes stdin through as stdout (hooks expect a response). |
| 8 | Installation surface | Extend the existing global-bridge config generator in [`RegisterApiKeyClientModal.tsx`](../../apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx) to also render a `~/.cursor/hooks.json` snippet alongside the `~/.cursor/mcp.json` one it already builds | One panel, one "copy" action per file, same flow the user already knows from setting up the bridge. Not a new feature surface. |

---

## Scope

**In:**

- A new loopback-only gateway endpoint that accepts `{workspace_roots: string[]}` and stores it as a short-TTL hint
- A new resolution tier consulted by `SessionRootsRegistry::get()` / `get_pinned()`, slotted between `window_pins` and probed `roots/list`
- A `buildCursorBridgeHooksJson()` helper alongside the existing `buildCursorBridgeMcpJson()`, and a second snippet block in the Cursor tab of `RegisterApiKeyClientModal.tsx`
- Phase 1: a real measurement of whether hook payload ids correlate with anything visible on the resulting MCP request, before any of the above is built on an assumption

**Out:**

| Item | Reason / Deferral |
| ---- | ------------------ |
| `preToolUse` as the primary hook event | Deferred per decision 1 — only revisited if the turn-level hint proves too coarse in the field (races, or turns whose tool calls span more than ~5s of latency before the first `mcpmux_*` call) |
| Project-scoped `.cursor/hooks.json` | Decision 6 — per-repo installs already have a fully deterministic path; a hook there would duplicate `workspace_install.rs`, not improve on it |
| Any non-Cursor client | Hooks are a Cursor-specific capability. VS Code / Claude Code already route correctly via `roots` (same finding `cursor-workspace-routing-bridge.md` decision 5 relied on) |
| Replacing or removing decision 4b's gate | This tier is additive and ranks below `window_pins` (decision 5) — the gate that stops a wrong pin from becoming durable stays exactly as-is |
| Exact-match correlation via `tool_call_id` as the *initial* build | Decision 4 — this is Phase 1's open question, not a decision made in advance of measuring it |
| Filing the hooks gap upstream with Cursor | Not applicable — hooks already carry everything needed; there's no upstream bug here, unlike the `${workspaceFolder}` substitution flake |

---

## Architecture

### Why this sidesteps the shared-process problem instead of gating it further

```text
Existing signals, all downstream of the same TCP connection:
  mcp-remote (pid 62753, ONE process for every window with the global bridge)
    |
    +-- mcp-session-id       <- shared, decision 4b already handles this
    +-- peer socket -> PID   <- shared (it's the same PID for every window)
    +-- roots/list           <- shared, stale, listChanged:false

Hooks, an entirely separate channel:
  Cursor's own process
    +-- beforeSubmitPrompt hook, spawned fresh, THIS window's workspace_roots
            |
            v (loopback POST, out of band from mcp-remote)
      Gateway hint queue (TTL, best-effort)
```

The hook signal isn't a better way to read the same shared connection — it never touches that connection. That's what makes it different from every prior attempt in `window-scoped-workspace-pin.md`, all of which tried to extract more from the transport `mcp-remote` already shares.

### Resolution ladder (extends `window-scoped-workspace-pin.md`'s ladder)

```text
1. pinned[session]        explicit header or set_workspace_root, this session   (unchanged, authoritative)
2. window_pin[window_key] remembered explicit claim from the same bridge process (unchanged)
3. hook_hint[]             most recent unexpired beforeSubmitPrompt hint         (NEW — best-effort)
4. map[session]            probed roots/list                                    (unchanged)
5. PendingRoots            ambiguous or absent — today's behavior                (unchanged)
```

### New state

```rust
/// A workspace claim from a Cursor hook, not tied to any session id at
/// write time — the hook fires in Cursor's own process, out of band from
/// the `mcp-remote` connection its turn's tool calls will arrive on.
///
/// ponytail: FIFO with a short TTL, not exact correlation. Two windows
/// starting turns within the same ~5s window can grab each other's hint.
/// The upgrade path is decision 4's `tool_call_id` correlation, if Phase 1
/// finds it's available.
struct HookHint {
    workspace_roots: Vec<String>,
    received_at: Instant,
}

/// FIFO queue of recent hints, drained (not just peeked) on each consult so
/// a stale hint from an earlier turn can't outlive its own TTL by sitting
/// behind a fresher one.
hook_hints: Mutex<VecDeque<HookHint>>,
```

### New endpoint

```text
POST /internal/cursor-hook/workspace-hint     (loopback-gated, same trust model as /oauth/clients)
Body: { "workspace_roots": ["/Users/joe/Desktop/Repos/Personal/mcp-mux"] }
```

### `hooks.json` shape (global, user-scoped)

```jsonc
// ~/.cursor/hooks.json
{
  "hooks": {
    "beforeSubmitPrompt": [
      {
        "command": "node",
        "args": [
          "-e",
          "let d='';process.stdin.on('data',c=>d+=c);process.stdin.on('end',()=>{const p=JSON.parse(d);fetch('http://127.0.0.1:45818/internal/cursor-hook/workspace-hint',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({workspace_roots:p.workspace_roots})}).catch(()=>{}).finally(()=>process.stdout.write(d));});"
        ]
      }
    ]
  }
}
```

(Illustrative — Phase 2 renders the exact snippet from the desktop app, matching the copy-paste pattern `buildCursorBridgeMcpJson` already uses for `~/.cursor/mcp.json`.)

---

## Files to create / modify

| Area | File | Action |
| ---- | ---- | ------ |
| Gateway | `crates/mcpmux-gateway/src/services/hook_hints.rs` | Create — `HookHint`, the TTL queue, `push()` / `take_fresh()` |
| Gateway | [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | Modify — `get()` / `get_pinned()` consult the new tier between `window_pins` and probed roots |
| Gateway | [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | Modify — new route, added to `is_management_path()` so it shares the existing loopback gate |
| Gateway | `crates/mcpmux-gateway/src/mcp/handlers.rs` (or a new sibling) | Create/modify — handler for the new endpoint |
| Desktop UI | [`apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts`](../../apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts) | Modify — add `buildCursorBridgeHooksJson()` alongside `buildCursorBridgeMcpJson()` |
| Desktop UI | [`apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx`](../../apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx) | Modify — Cursor tab renders the hooks.json snippet alongside the existing mcp.json one |
| Docs | [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) | Modify — document the hook, what it sends, and how to see a hint applied vs. missed in the logs |

---

## Phases

### Phase 1 — Correlation spike, no gateway changes (~half day)

Proves or kills decision 4 before anything is built on either branch of it.

- Hand-write the `hooks.json` one-liner from the Architecture section, pointed at a throwaway `nc`/logging endpoint instead of the real gateway
- Trigger a real turn that calls an `mcpmux_*` tool; capture both the hook's payload and the raw MCP `tools/call` request the gateway receives for that same turn
- Compare every id field on each side (`tool_call_id`, `generation_id`, `conversation_id`, the JSON-RPC `id`) looking for any overlap

**Outcome:** Either a correlating id exists (decision 4 resolves to "exact match," Phase 2 builds a keyed lookup instead of a FIFO queue) or it doesn't (decision 3's FIFO/TTL design ships as designed, with its ceiling documented and accepted). Either answer unblocks Phase 2 — this phase's job is to pick which design, not to stall on uncertainty.

---

### Phase 2 — Gateway endpoint and resolver tier (~1 day)

- `hook_hints.rs`: the queue (or keyed map, per Phase 1's answer), a bounded size/TTL, and `take_fresh()` semantics that drain rather than peek
- New loopback-gated route; extend `is_management_path()`
- Wire the new tier into `SessionRootsRegistry::get()` / `get_pinned()`, below `window_pins`
- Log a hint's application distinctly from a window-pin inheritance, so field traces can tell which tier actually resolved a given session

**Outcome:** With the hand-written `hooks.json` from Phase 1 now pointed at the real gateway, a session on the shared bridge with an ambiguous candidate set resolves correctly without a `mcpmux_set_workspace_root` call, and the log names the hint as the source.

---

### Phase 3 — Desktop UI and docs (~half day)

- `buildCursorBridgeHooksJson()` + the second snippet block in `RegisterApiKeyClientModal.tsx`'s Cursor tab
- `cursor-workspace-bridge.md`: what the hook does, the exact log lines for "hint applied" vs. "hint missed / expired," and the FIFO ceiling from decision 3 stated plainly so a future racing-windows report doesn't reopen the investigation
- `window-scoped-workspace-pin.md`: note that this tier addresses the residual its own Scope/Out table left standing

**Outcome:** A user can copy both snippets from the same modal in one sitting, and the residual `set_workspace_root` call count on a steady-state multi-window setup drops toward zero without them having done anything beyond the one-time hooks.json install.

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | `get()` / `get_pinned()` are the two insertion points; `window_pins` / `PinSource` are the tier this one sits directly below |
| [`crates/mcpmux-gateway/src/services/window_identity.rs`](../../crates/mcpmux-gateway/src/services/window_identity.rs) | Not modified, but the reason this doc exists — its own doc comment says the PID "is a window key, not a folder," and on the shared bridge it isn't even that |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | `is_management_path()` / `restrict_management_to_loopback` — the exact trust model the new endpoint reuses rather than inventing auth |
| [`apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx`](../../apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx) | Where the Cursor tab already renders `buildCursorBridgeMcpJson()`; the hooks snippet joins it here |
| [`apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts`](../../apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts) | `buildCursorBridgeMcpJson()` is the direct model for the new `buildCursorBridgeHooksJson()` |
| [`ryanhiizy/cursor-agent-wakatime`](https://github.com/ryanhiizy/cursor-agent-wakatime) | External prior art — hooks into `afterAgentResponse` / `afterFileEdit` / `postToolUse` to attribute AI activity per-window; proves the hook channel carries enough signal for this class of problem |
| [Hooks — Cursor Docs](https://cursor.com/docs/hooks.md) | Payload schema: `workspace_roots`, `conversation_id`, `generation_id`, `tool_call_id`, `CURSOR_PROJECT_DIR` env var; project/user/team/enterprise scoping |

---

## Related documentation

- [`docs/planning/window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) — the durable-pin design and decision 4b's leak fix; this doc's Scope/Out table is exactly the residual left standing after that patch
- [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — the global bridge this extends; Decision 5's Cursor-only scoping precedent carries over unchanged
- [`docs/planning/resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) — item 1, the residual this doc's Unblocks line refers to
- [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) — user-facing bridge setup; gets the hooks section in Phase 3
