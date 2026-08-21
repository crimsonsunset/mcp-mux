# Harness Workspace Adapters

**Last Updated:** Aug 21, 2026
**Status:** Planning. Decisions locked after the Cursor global-bridge hook + window-pin work shipped on `root-resolution`. No code yet.
**Branch:** not started — cut from `root-resolution` after that branch lands on fork `dev`
**Depends on:** [`cursor-agent-hooks-workspace-hint.md`](./cursor-agent-hooks-workspace-hint.md) (CallRoot via `_mcpmux_context`), [`window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) (SessionClaim + SurfaceKey), [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) (SurfaceConstraint header), [`mcp-2026-07-28-spec-impact.md`](./mcp-2026-07-28-spec-impact.md) (roots are a dying signal)
**Unblocks:** A second harness (Claude Code first) using the same routing ladder without copying Cursor hook JSON, Cursor installer paths, or the loopback-PID SurfaceKey

---

## Problem

The last three plans solved one product: Cursor's global `mcp-remote` bridge, where many agents share one TCP session and nothing on that session is window-scoped. The working answer is three *signals*, not one hook format:

| Signal | Meaning | Cursor today |
| ------ | ------- | ------------ |
| **CallRoot** | Exact folder for this `tools/call` only. Never writes pins. | `_mcpmux_context.workspace_root` from `preToolUse` |
| **SessionClaim** | Durable pin for this connection | `X-Mcpmux-Workspace` or `mcpmux_set_workspace_root` |
| **SurfaceConstraint** | Folders this surface has open. Constraint, never a guess. | `X-Mcpmux-Workspace-Set` from `WORKSPACE_FOLDER_PATHS` |

Plus a **SurfaceKey** (loopback peer → `mcp-remote` PID) so a SessionClaim can outlive `mcp-session-id` churn. That key is Cursor-global-bridge-specific. CLIs that are one process / one cwd / one session do not need it.

If the next harness is treated as "Cursor hooks, but for X," the repo grows a second `cursor_hook.rs` with a different JSON schema, matcher dialect, and install path, while the gateway keeps growing `if client == cursor` branches. The durable piece is the signal tuple. Each product only maps vendor payload → that tuple and, optionally, writes *that product's* hook or config.

This is not a framework-before-a-second-customer job. Cursor stays the first adapter. Claude Code is the first *new* one, because it already has a real `PreToolUse` and a `cwd` that follows `cd` and worktrees. Everything else waits until a live caller needs it.

The old "VS Code / Claude Code already route via `roots`" line in [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) decision 5 is still true on today's wire (`rmcp 1.5.0` / `2025-11-25`). It is the wrong long-term source: [SEP-2577](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2577-deprecate-roots-sampling-and-logging.md) deprecated roots in the `2026-07-28` spec. New work stays on headers + `_mcpmux_context` (or that harness's CallRoot equivalent). Do not add a new `roots/list` dependency.

---

## Decisions

| #   | Decision                         | Choice                                                                                         | Rationale                                                                                                                                                                                                 |
| --- | -------------------------------- | ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | What to abstract                 | Three signals + optional SurfaceKey, not hook JSON                                             | Cursor `preToolUse`, Claude Code `PreToolUse`, and "no hook" CLIs share a meaning, not a schema. Unifying `permission` vs `permissionDecision`, matchers, or file locations would be a compatibility tax with no routing gain. |
| 2   | Adapter shape                    | Thin `HarnessAdapter`: `id`, `read_connect`, `read_call`, `surface_key`, optional `install()`  | One place to name the tuple. `install()` is per product and may be a no-op. No plugin registry, no dyn-loaded crates, no shared hook runner.                                                              |
| 3   | First new product                | Claude Code, not a framework and not VS Code                                                   | Real rewrite hook + `cwd` that tracks the session. Next-highest value after Cursor. VS Code / Windsurf / Cline have MCP and weak or no rewrite hooks; they stay constraint-only until a caller needs more. |
| 4   | CallRoot persistence             | Never write `pinned` or `window_pins` from a call-scoped root                                  | Same as hook-plan decision 4. Shared-session writes recreate the cross-agent leak. A CLI that is 1:1 with cwd still must not promote CallRoot; SessionClaim is the durable slot.                          |
| 5   | Missing / lying vendor data      | Fail open: omit the signal, keep today's ladder                                                | A wrong folder is a credential-scope bug. An omitted CallRoot is `PendingRoots` or an existing pin. Adapters must not invent a root from recency, first-entry, or `roots/list`.                          |
| 6   | SurfaceConstraint                | Constraint only. Absent set stays permissive.                                                  | Already shipped for Cursor. One-folder set may pin (self-attesting). Multi-folder set may reject a claim outside the set; it must not pick. Same doctrine for every adapter.                              |
| 7   | SurfaceKey                       | Optional. Cursor global bridge only until a second shared-transport shows up                   | Loopback PID is how Cursor's one `mcp-remote` outlives Reload MCP. A local Claude Code process is already the session; inventing a PID key there is ceremony.                                             |
| 8   | Roots                            | Do not take new dependencies on `roots/list`                                                   | Soft-deprecated in `2026-07-28`, still the source of Cursor's mixed-window junk. Existing probe stays as a fallback on the current protocol version; new adapters do not add a roots reader.              |
| 9   | Installers                       | One `install()` per adapter. No universal writer.                                              | `~/.cursor/hooks.json` vs Claude Code settings vs a CLI rc file are different merge/backup/JSONC problems. The existing per-repo MCP writer in `workspace_install.rs` stays a client-config installer, not a hook installer. |
| 10  | Catalog on shared sessions       | Unchanged: core meta tools only when the session is ambiguous                                  | `tools/list` still has no CallRoot. Search → schema → invoke stays the token-efficient path. Per-repo installs stay the exact-catalog path.                                                               |

### Smallest adapter shape

```text
HarnessAdapter {
  id: "cursor" | "claude-code" | ...
  read_connect(env, headers, cwd) -> SessionClaim + SurfaceConstraint
  read_call(vendor_payload)       -> Option<CallRoot>
  surface_key(peer, pid, extra)   -> Option<SurfaceKey>
  install()                       -> writes that product's hook/config   // optional
}
```

Gateway resolution stays the ladder already shipped:

```text
call_tool:
1. CallRoot                         exact, discarded after the call
2. SessionClaim (pinned[session])
3. window_pin[SurfaceKey]           only if a key exists
4. map[session]                     existing roots/list, current protocol only
5. PendingRoots

tools/list / prompts/list / resources/list:
same minus CallRoot
```

---

## Scope

**In:**

- Name the three signals (and optional SurfaceKey) in gateway code so Cursor's headers / hook / PID path is visibly the first adapter, not a pile of Cursor-only types
- A Claude Code adapter: connect-time cwd (or equivalent header) as SessionClaim, `PreToolUse` `cwd` as CallRoot when present, no SurfaceKey
- A Claude Code `install()` that writes *Claude Code's* hook config, with the same backup / refuse-JSONC / preserve-siblings habits as the Cursor installer, not a shared JSON merger
- Fail-open tests: omitted CallRoot, cwd outside SurfaceConstraint, empty SessionClaim
- Docs that tell the next person "add an adapter, do not add a hook format"

**Out:**

| Item | Reason / Deferral |
| ---- | ----------------- |
| Universal hook JSON / matcher / `permissionDecision` schema | Decision 1. Each vendor owns its file. A shared schema would lag every product release. |
| Universal installer / "one click, any client" | Decision 9. Cursor installer stays in `cursor_hook.rs`. Claude Code gets its own writer. The per-repo MCP merge in `workspace_install.rs` is unrelated. |
| VS Code, Windsurf, Cline CallRoot | No rewrite hook worth depending on today. Constraint-only (headers or cwd) if a live caller needs it; not in the first cut. |
| Codex, Gemini CLI, Copilot CLI adapters | 1 process = 1 cwd = 1 session. SessionClaim = cwd is enough when someone actually connects one. Not before a caller. |
| Cursor CLI | Shell hooks only, no `preToolUse`. Different product from the IDE hook path. |
| Cursor Cloud Agents | Still a different runtime (no user-level hooks, no loopback `mcp-remote`). Recheck current Cloud hook docs before treating as permanently hook-less; do not reuse local absolute-path CallRoot. See the Cloud Agents section in [`cursor-agent-hooks-workspace-hint.md`](./cursor-agent-hooks-workspace-hint.md). |
| Framework, plugin crate, or dyn Adapter trait object | Decision 3. A `match harness_id` or a small static table is enough for two products. Upgrade if a third shared-transport (not a third CLI) appears. |
| New `roots/list` readers or roots-based "adapters" | Decision 8 / [SEP-2577](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2577-deprecate-roots-sampling-and-logging.md). |
| Changing Cursor hook behavior | This plan wraps what shipped. No new Cursor matcher, no `_mcpmux_context` shape change, no pin-write from CallRoot. |
| Per-agent `tools/list` on a shared session | Still impossible without a list-time identity. Same deferral as the hook plan. |

---

## Architecture

### What the gateway already has

```text
headers          → SessionClaim + SurfaceConstraint   (oauth_middleware)
loopback PID     → SurfaceKey                         (window_identity)
_mcpmux_context  → CallRoot                           (handler, then stripped)
SessionRootsRegistry                                  (pins, sets, never written by CallRoot)
resolve_for_workspace_root                            (binding without session mutation)
```

Phase 1 is a rename-and-boundary job on that stack. Phase 2 adds a second `read_call` / `read_connect` pair. The resolver does not grow a second ladder.

### Claude Code (first new adapter)

```text
claude -p /repo/a  (or a worktree / later `cd`)
  |
  +-- MCP Streamable HTTP to 127.0.0.1:45818
  |     read_connect: SessionClaim = process cwd (or a future header)
  |     SurfaceConstraint = {cwd} when only one folder is in play
  |     SurfaceKey = None
  |
  +-- PreToolUse on the exact MCP call
        payload.cwd = current folder (follows cd / worktrees)
        read_call → CallRoot = cwd
        inject _mcpmux_context.workspace_root  OR  a Claude-native equivalent
        gateway strips vendor metadata before backend forward
```

If Claude Code's hook cannot rewrite MCP tool input the way Cursor's `updated_input` does, CallRoot rides a header or a reserved argument the gateway already knows how to strip. Do not invent a side-channel POST. Measure that in Phase 2 before writing the installer.

One Claude Code process is one session. The shared-`mcp-remote` collision that forced CallRoot on Cursor is not the default here. CallRoot still earns its keep when the user `cd`s or the hook sees a worktree that the connect-time cwd missed. SessionClaim covers the no-hook path.

### Fail-open

```text
vendor omitted CallRoot     → today's session ladder
vendor sent a root          → normalize; if a SurfaceConstraint exists, require membership
vendor sent garbage         → drop CallRoot, log, do not pin
no SurfaceConstraint        → existing approved-client root trust (same as Cursor absent-set)
```

---

## Files to create / modify

| Area | File | Action |
| ---- | ---- | ------ |
| Gateway | new `crates/mcpmux-gateway/src/harness/` (or `services/harness/`) | Create — `CallRoot` / `SessionClaim` / `SurfaceConstraint` / `SurfaceKey` newtypes, `HarnessAdapter` trait or static table, Cursor adapter that wraps today's header + hook + PID reads |
| Gateway | [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | Modify — `read_connect` + optional `surface_key` instead of inline Cursor header logic where it is cheap to extract. Behavior unchanged in Phase 1. |
| Gateway | [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs) | Modify — `read_call` returns `Option<CallRoot>`; Cursor `_mcpmux_context` stays the first implementation |
| Gateway | [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | Modify only if signal types want a home next to the pin maps. No new pin-write path. |
| Gateway | [`crates/mcpmux-gateway/src/cursor_hook.rs`](../../crates/mcpmux-gateway/src/cursor_hook.rs) | Leave as the Cursor `install()`. Do not generalize this file into a multi-product writer. |
| Gateway | new `crates/mcpmux-gateway/src/claude_code_hook.rs` (name TBD) | Create in Phase 2 — Claude Code hook script + settings merge, backup, JSONC refusal |
| Gateway | [`crates/mcpmux-gateway/src/admin/router.rs`](../../crates/mcpmux-gateway/src/admin/router.rs) | Modify in Phase 2 — `GET\|POST /api/v1/claude-code-hook` mirroring `/api/v1/cursor-hook`, not a generic `/hooks/:id` |
| Desktop | Connections side panel + register-client result | Phase 2 — Claude Code installer card, copy the Cursor card's shape, do not merge the two components into a polymorphic "any hook" widget until a third installer exists |
| Desktop | [`apps/desktop/src-tauri/src/commands/workspace_install.rs`](../../apps/desktop/src-tauri/src/commands/workspace_install.rs) | Do not reuse for hooks. It already installs per-repo `.mcp.json` for Claude Code. That is SessionClaim via a literal URL config, not CallRoot. |
| Docs | [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) | Cross-link only. Cursor behavior does not change. |
| Docs | new `docs/manual/claude-code-workspace.md` | Create in Phase 2 — install, fail-open, how this differs from Cursor's shared-session story |
| Docs | [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) | Phase 4 — amend decision 5: roots still work on `2025-11-25`; new harness work does not depend on them |

---

## Phases

### Phase 1 — Name the signals, Cursor stays the only adapter (~half day)

No new product. Prove the boundary is real by making Cursor go through it with zero routing change.

- Introduce the four types and a Cursor adapter that reads today's headers, `_mcpmux_context`, and loopback PID
- Point `oauth_middleware` / `handler` at those functions; keep log field names (`source=cursor_pre_tool_use`, window key) so field greps still work
- Do not move `cursor_hook.rs` or the Connections installer
- `pnpm test:rust:unit` on session roots + handler extraction must stay green with no new fixtures that encode a second harness

**Outcome:** A Cursor two-window / two-root call on the shared bridge still routes from `_mcpmux_context` and still does not write pins. `git diff` on resolver behavior is empty aside from type names. A reviewer can point at `HarnessAdapter` and see Cursor as `id: "cursor"`.

### Phase 2 — Claude Code adapter (~1–2 days)

The first new `read_connect` / `read_call` / `install()`.

- Spike against a live `claude` MCP session: can `PreToolUse` rewrite the MCP tool input (Cursor-style), or is CallRoot a header / reserved arg? Record the answer in this doc before writing production install
- Map process cwd → SessionClaim; map hook `cwd` → CallRoot; `surface_key` returns `None`
- Fail-open on omit / mismatch; never pin from CallRoot
- Managed hook writer + admin HTTP + Connections card, copied in spirit from Cursor, not parameterized by a generic hook spec
- Tests: cwd change / worktree changes CallRoot on the next call; a bare call with no hook still uses SessionClaim; a CallRoot outside a one-folder constraint is rejected

**Outcome:** From a Claude Code session in `/repo/a`, `mcpmux_list_servers` / search / invoke resolve `/repo/a`'s Space. After `cd /repo/b` (or a worktree switch) the next hooked call follows `/repo/b` and leaves `/repo/a`'s session pin unpoisoned. One-click install writes Claude Code's file only.

### Phase 3 — Constraint-only note, no third adapter (~2 hours)

Only if Phase 2 lands and a VS Code or Windsurf caller is actually failing. Otherwise skip.

- Document how a no-hook harness uses SessionClaim + SurfaceConstraint (headers or cwd) with CallRoot = `None`
- If a live VS Code window is misrouting, add headers to that client's generated config — not a hook

**Outcome:** A no-hook client either routes from a literal header / cwd or stays on `PendingRoots` + `set_workspace_root`. No third `install()` exists unless that client needed a config snippet.

### Phase 4 — Cross-links and the roots amendment (~2 hours)

- New Claude Code manual (if Phase 2 shipped) or a short "adapter contract" section on the existing bridge manual (if only Phase 1 shipped)
- Amend [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) decision 5 and the hook-plan "any non-Cursor client" Out row: those clients are not "done forever via roots"; they are "no CallRoot yet, do not grow a roots dependency"
- Point [`mcp-2026-07-28-spec-impact.md`](./mcp-2026-07-28-spec-impact.md) at this doc as the concrete "avoid new roots hard-deps" follow-through

**Outcome:** The next person who wants Codex or VS Code reads this file, adds one adapter, and does not open a PR that parses `roots/list` or generalizes `cursor_hook.rs`.

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs) | CallRoot extraction and strip of `_mcpmux_context` |
| [`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`](../../crates/mcpmux-gateway/src/mcp/oauth_middleware.rs) | SessionClaim + SurfaceConstraint headers; empty-header vs one-folder-set pin rule |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs) | Pin maps CallRoot must never write; set membership is the only constraint |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | `resolve_for_workspace_root` is the CallRoot entry |
| [`crates/mcpmux-gateway/src/cursor_hook.rs`](../../crates/mcpmux-gateway/src/cursor_hook.rs) | Cursor `install()` — do not generalize |
| [`apps/desktop/src-tauri/src/commands/workspace_install.rs`](../../apps/desktop/src-tauri/src/commands/workspace_install.rs) | Already writes Claude Code `.mcp.json`. That is client MCP config, not a harness hook. |
| [`docs/planning/mcp-2026-07-28-spec-impact.md`](./mcp-2026-07-28-spec-impact.md) | Roots deprecated; sessions removed in the next protocol version McpMux does not speak yet |
| [Claude Code hooks](https://docs.anthropic.com/en/docs/claude-code/hooks) | Vendor `PreToolUse` + `cwd` — Phase 2 spike source |
| [SEP-2577](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2577-deprecate-roots-sampling-and-logging.md) | Why new adapters must not grow a `roots/list` reader |

---

## Related documentation

- [`docs/planning/cursor-agent-hooks-workspace-hint.md`](./cursor-agent-hooks-workspace-hint.md) — CallRoot on Cursor; Cloud Agents stay out
- [`docs/planning/window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) — SurfaceKey and the shared-process leak CallRoot exists to avoid writing back into
- [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — SurfaceConstraint header; decision 5 amended in Phase 4
- [`docs/planning/mcp-2026-07-28-spec-impact.md`](./mcp-2026-07-28-spec-impact.md) — spec-level reason to stay on headers + call context
- [`docs/planning/rootless-declare-root-gate.md`](./rootless-declare-root-gate.md) — remote / cloud identity; do not treat a VM path as CallRoot
- [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) — user-facing Cursor setup
- [`docs/manual/workspace-header-routing.md`](../manual/workspace-header-routing.md) — original roots-reporting bug; Claude Code listed as a roots control
