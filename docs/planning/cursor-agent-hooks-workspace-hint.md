# Cursor `preToolUse` as an Exact Per-Call Workspace Signal

**Last Updated:** Aug 21, 2026
**Status:** Phases 2–4 implemented Aug 21 (gateway extract/resolve, meta-tool threading, desktop installer). Phase 5 live concurrent-agent verification still open. The original `beforeSubmitPrompt` FIFO/TTL proposal is rejected and superseded by this plan. A real `preToolUse` hook added `_mcpmux_context` to `mcpmux_list_servers`; the gateway received the exact `generAIt` root and Cursor `tool_use_id` on that same `tools/call` despite the shared session reporting four roots.
**Branch:** `root-resolution`
**Depends on:** [`window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) (reuses `SessionRootsRegistry`'s tiered resolution and the loopback-trust model `window_identity.rs` established) and [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) (extends the global bridge's config-generator UI rather than replacing it)
**Unblocks:** Exact workspace routing for hundreds of concurrent Cursor agents whose calls share one global `mcp-remote` session, without timing heuristics or persistent cross-agent state

---

## Problem

Decision 4b closed a real leak: a `set_workspace_root` pin on the shared `mcp-remote` process no longer promotes to `window_pins` without independent proof of single-window intent. That is correct and it is not enough — it converts a _wrong, durable_ answer back into _no_ answer. The session still has to call `mcpmux_set_workspace_root` on every reconnect, because every signal the gateway can see from that connection (`mcp-session-id`, the peer PID, `roots/list`) is shared by every window that has the global bridge open. `window-scoped-workspace-pin.md`'s own field evidence ("759 manual pins for six windows") is the size of this cost when the design was still leaking; decision 4b removes the leak but leaves the 759 calls.

Every fix considered so far has tried to extract more signal from the _transport_ — the loopback socket, the child's env, `roots/list`, `WORKSPACE_FOLDER_PATHS`. All of them are downstream of the same fact: Cursor spawns one `mcp-remote` child for the global bridge entry regardless of window count, so nothing arriving over that one TCP connection can be window-scoped no matter how it's read.

Cursor has a second channel that doesn't go through `mcp-remote`: **hooks**. The first spike used `beforeSubmitPrompt`, which carried the right root but had no identifier in common with the later MCP request. That killed correlation by observation, not by assumption: `conversation_id` / `generation_id` stayed inside Cursor, while rmcp exposed an unrelated `mcp-session-id` and sequential JSON-RPC id.

The second spike changed the question. Cursor's `preToolUse` hook runs on the exact MCP call and may return `updated_input`. Instead of correlating two unrelated events, the hook can put the workspace identity inside the call itself. A live call proved Cursor accepts an undeclared reserved argument and rmcp delivers it unchanged:

```json
{
  "_mcpmux_context": {
    "workspace_root": "/Users/joe/Desktop/Repos/Contracts/generAIt",
    "tool_use_id": "4f43d3cc-7af4-4fa7-852e-2f0025c1b9ff"
  }
}
```

The gateway saw that object on JSON-RPC request 17 for `mcpmux_list_servers`. There is no queue to race, and no session mutation that another agent can inherit.

---

## Decisions

| #   | Decision                  | Choice                                                                                      | Rationale                                                                                                                                                                                                                                    |
| --- | ------------------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Hook event                | `preToolUse`, matched to `MCP:mcpmux_*`                                                     | It runs on the exact call and supports `updated_input`. The matcher avoids spawning the hook for shell, file, and unrelated MCP tools; the throwaway unfiltered spike proved that global noise would otherwise be substantial.               |
| 2   | Correlation               | Create it: inject `_mcpmux_context` into the exact tool input                               | The first spike proved no existing id crosses both channels. The second proved a reserved input object does. This removes correlation state instead of improving a heuristic.                                                                |
| 3   | Root source               | Inject only when `workspace_roots` has exactly one entry                                    | Real MCP hook payloads had no `cwd`; relying on it would silently fail. A multi-root payload still does not identify the active folder, so it falls through to today's safe `PendingRoots` behavior.                                         |
| 4   | Gateway state             | Per-call only; never write `pinned`, `window_pins`, or a hint store                         | Shared-session writes recreate the cross-agent leak. The explicit root is borrowed through resolution and discarded after the call.                                                                                                          |
| 5   | Precedence                | Valid `_mcpmux_context.workspace_root` is authoritative for that `call_tool` only           | It is more precise than every session-level signal because Cursor attached it to this exact invocation. List operations remain unchanged.                                                                                                    |
| 6   | Validation                | Normalize, require a non-empty root, and require candidate-set membership when a set exists | The candidate set cannot select a root, but it can reject a hook claim outside the folders the shared bridge reported. If the set is absent, the existing approved-client root trust model applies.                                          |
| 7   | Argument handling         | Strip `_mcpmux_context` before meta-tool parsing or backend forwarding                      | The field is transport metadata, not part of any backend tool contract. No backend sees it and strict schemas stay valid.                                                                                                                    |
| 8   | Ambiguous-session catalog | Keep the six core meta tools only; do not union surfaced tools                              | `tools/list` has no hook context and therefore cannot vary per agent on one session. Search → schema → invoke already exists for this exact token-efficient shape. Per-repo installs remain the strict option for a per-root direct catalog. |
| 9   | Meta-tool propagation     | Add the explicit root to `MetaToolCall`; every resolver helper honors it                    | `mcpmux_list_servers`, search, schema, invoke, bind, resource, and prompt tools must agree. Resolving only in the outer handler would still let inner helpers fall back to the shared session.                                               |
| 10  | Installation              | One-click safe merge plus a managed script under `~/.cursor/hooks/`                         | Back up plain JSON, preserve all unrelated entries (including WakaTime), and add one short idempotent hook command. Refuse JSONC rather than clobber it and show a manual snippet.                                                           |
| 11  | Script form               | Managed Node script, not an inline one-liner or a new native binary                         | The bridge already requires Node through `npx`; a readable script is easier to inspect and update. A native helper is an upgrade only if profiling shows process startup matters.                                                            |
| 12  | Scope                     | Cursor global bridge only                                                                   | Other clients route correctly via roots, and project-local Cursor installs already carry a literal authoritative header.                                                                                                                     |

---

## Scope

**In:**

- A managed `preToolUse` script that adds `_mcpmux_context` to `MCP:mcpmux_*` calls when Cursor reports one exact workspace root
- Gateway extraction, validation, logging, and removal of that reserved argument
- A non-persistent explicit-root resolver path used by ordinary tool calls and every meta-tool helper
- Core-meta-only catalog behavior for ambiguous shared sessions
- A one-click desktop installer that safely merges the hook into `~/.cursor/hooks.json`, with backup and manual fallback
- Live concurrent-agent verification across distinct roots sharing one `mcp-session-id`

**Out:**

| Item                                                         | Reason / Deferral                                                                                                                                                                           |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Per-agent `tools/list` on the shared session                 | Impossible with the available signal: hooks run on tool use, not list requests. Core meta tools remain the safe shared catalog; project-local config remains the exact direct-catalog path. |
| Guessing inside a multi-root hook payload                    | `workspace_roots` names candidates, not the active folder. No ordering or recency rule may gate credentials.                                                                                |
| FIFO, TTL, candidate-gated queues, or hook-to-request timing | Rejected after scale review. At high concurrency they misroute by construction, even if candidate filtering reduces the collision set.                                                      |
| Hook-side loopback POST endpoint                             | The exact root already rides inside `updated_input`; another request and temporary map add no identity.                                                                                     |
| Native helper binary                                         | Deferred until managed-script startup cost is measured under realistic load. Node is already required by the global bridge.                                                                 |
| Project-scoped `.cursor/hooks.json`                          | Per-repo MCP installs already provide a literal static header and exact catalog. A hook there duplicates a solved path.                                                                     |
| Any non-Cursor client                                        | Cursor hooks are client-specific; VS Code and Claude Code already route through standard roots.                                                                                             |
| Replacing decision 4b's durable-pin gate                     | Exact call context is additive and non-persistent. Existing session/window pin safety remains unchanged.                                                                                    |
| Automatically rewriting JSONC                                | The installer refuses non-plain JSON and presents the managed entry for manual merge. Preserving comments safely needs a JSONC-aware writer and is not required for the first build.        |
| Cloud Agents                                                 | Different runtime. User-level hooks and `mcp-remote` never load. Cloud identity stays on the shipped rootless / `set_workspace_root` path. See [Cloud Agents](#cloud-agents-researched-aug-21). |

---

## Architecture

### Exact call flow

```text
Cursor agent in /repo/a
  |
  +-- preToolUse(MCP:mcpmux_search_tools)
        input.workspace_roots = ["/repo/a"]
        output.updated_input += _mcpmux_context{workspace_root:"/repo/a", tool_use_id:"t1"}
  |
  v
shared mcp-remote process / shared mcp-session-id
  |
  v
gateway call_tool
  1. remove + validate _mcpmux_context
  2. resolve binding directly from /repo/a
  3. execute meta/backend call with /repo/a's Space + FeatureSet
  4. forward original arguments only
  5. discard context
```

Agent B can execute the same sequence for `/repo/b` concurrently. The calls share transport state but no routing state.

### Call argument

```json
{
  "query": "jira",
  "_mcpmux_context": {
    "workspace_root": "/repo/a",
    "tool_use_id": "cursor-tool-use-id"
  }
}
```

`tool_use_id` is observability only. Routing uses the root carried on the same request; it never performs a second lookup keyed by that id.

### Call-time precedence

```text
call_tool only:
1. valid _mcpmux_context.workspace_root   exact call identity       (NEW)
2. pinned[session]                        existing explicit state
3. window_pin[window_key]                 existing durable state
4. map[session]                           existing roots/list
5. PendingRoots

tools/list / prompts/list / resources/list:
1. pinned[session]
2. window_pin[window_key]
3. map[session]
4. PendingRoots
```

The two ladders intentionally differ. A shared list request has no agent identity and returns core meta tools; an exact call has identity and routes through the right binding.

### Managed hook entry

```jsonc
{
  "version": 1,
  "hooks": {
    "preToolUse": [
      {
        "command": "node /absolute/path/to/mcpmux-workspace-context.js",
        "matcher": "MCP:mcpmux_.*",
        "timeout": 5,
      },
    ],
  },
}
```

The installer merges only the McpMux entry, preserves sibling hooks, writes a backup before changing an existing file, and treats repeated installation as an update rather than a duplicate.

---

## Files to create / modify

| Area            | File                                                                                                                                           | Action                                                                                                             |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Gateway         | [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs)                                                   | Modify — parse, validate, log, and strip `_mcpmux_context`; route `call_tool` through the exact root               |
| Gateway         | [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs)               | Modify — add a non-persistent `resolve_for_workspace_root` entry point sharing the existing binding logic          |
| Gateway         | [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs)                 | Modify — thread `workspace_root` through `MetaToolCall` and registry dispatch                                      |
| Gateway         | `crates/mcpmux-gateway/src/services/meta_tools/meta_tool_common.rs`                                                                            | Modify — make caller resolution and Space lookup prefer the per-call root                                          |
| Gateway         | `crates/mcpmux-gateway/src/services/meta_tools/*.rs`                                                                                           | Audit — replace any direct session-root read that bypasses `caller_resolution`, especially bind and set-root paths |
| Desktop command | `apps/desktop/src-tauri/src/commands/cursor_hook_install.rs`                                                                                   | Create — managed script writer, hooks JSON merge, backup, idempotence, JSONC refusal, uninstall support            |
| Desktop command | `apps/desktop/src-tauri/src/lib.rs`                                                                                                            | Modify — register the hook install/status/uninstall commands                                                       |
| Desktop UI      | [`apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts`](../../apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts) | Modify — build the manual fallback entry and installer status text                                                 |
| Desktop UI      | [`apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx`](../../apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx)     | Modify — add Install hook, installed/error state, backup path, and manual fallback alongside the bridge config     |
| Frontend API    | `apps/desktop/src/lib/api/cursorHooks.ts`                                                                                                      | Create — typed wrappers for install/status/uninstall                                                               |
| i18n            | `apps/desktop/src/locales/*/clients.json`                                                                                                      | Modify — Cursor hook installation and fallback copy                                                                |
| Docs            | [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md)                                                               | Modify — exact-call behavior, installer, fallback, logs, and concurrency verification                              |

---

## Phases

### Phase 1 — Exact input spike, no production behavior — ✅ DONE Aug 21

The first attempt proved there is no existing correlation id. The replacement spike proved the missing identity can be created inside the exact call.

- Installed a real `preToolUse` hook and returned `updated_input`
- Injected `_mcpmux_context.workspace_root` and Cursor's `tool_use_id`
- Called `mcpmux_list_servers` from the `generAIt` agent while the shared session reported four roots
- Captured the exact injected object in gateway `CallToolRequestParams.arguments`

**Outcome:** Exact per-call identity is available. FIFO/TTL and all queue variants are deleted from the design.

**Spike notes (throwaway, not landed):**

- MCP `preToolUse` payloads had `cwd=undefined`; `workspace_roots` is the usable field.
- The tested agent payload had exactly one root even though the downstream shared session reported four.
- Cursor accepted the extra input property despite it not appearing in `mcpmux_list_servers`' schema.
- The global unfiltered hook fired for unrelated Read/Write/Shell actions across other agents. Production must use the `MCP:mcpmux_.*` matcher.
- `handler.rs` instrumentation, the temporary user hook, scratch script, and `/tmp/mcpmux-hook-correlation.log` remain pending cleanup.

---

### Phase 2 — Exact gateway resolution (~1 day)

- Parse `_mcpmux_context` before resolution and remove it from `params.arguments`.
- Normalize the root; reject malformed objects and candidate-set mismatches with an explicit MCP error.
- Add `resolve_for_workspace_root` without mutating `SessionRootsRegistry`.
- Use the result for ordinary backend permission checks and routing.
- Log `workspace_root`, `tool_use_id`, session id, and `source=cursor_pre_tool_use` without logging arbitrary tool arguments.

**Outcome:** Two simultaneous calls carrying different roots on one session resolve to different bindings, and neither changes the next call's answer.

---

### Phase 3 — Meta-tool propagation and shared catalog (~1 day)

- Add the exact root to `MetaToolCall` and registry dispatch.
- Make `caller_resolution` / `caller_space_id` use it.
- Audit all meta tools for direct session-root reads; bind actions must target the exact call root.
- Keep ambiguous shared `tools/list` restricted to core meta tools.
- Confirm search → schema → invoke stays inside one binding across the full chain.

**Outcome:** `mcpmux_list_servers`, search, schema, invoke, bind, resource, and prompt operations all report and act on the hook's root, while direct surfaced tools remain intentionally absent from the ambiguous shared catalog.

---

### Phase 4 — Managed Cursor hook installation (~1 day)

- Write the versioned script under `~/.cursor/hooks/`.
- Safely merge one idempotent `preToolUse` entry into plain `~/.cursor/hooks.json`.
- Back up before modifying; preserve WakaTime and every unrelated hook.
- Refuse JSONC/non-object shapes and show a copyable manual entry.
- Add install status and uninstall to the existing Cursor bridge result UI.

**Outcome:** One click installs or updates the hook without duplicating it or changing existing hooks; unsupported config shapes remain untouched and receive actionable fallback output.

---

### Phase 5 — Scale verification, docs, and spike cleanup (~half day)

- Fire concurrent calls from several existing Cursor agents rooted in different repositories.
- Verify exact root, Space, and FeatureSet in gateway logs for every call.
- Exercise missing context, malformed context, multi-root payload, absent candidate set, and candidate mismatch.
- Run Rust format/check/Clippy plus frontend lint/typecheck.
- Update the manual bridge doc and cross-link the window-pin and routing-bridge plans.
- Remove scratch logging, the temporary hook entry/script, and the temporary log.

**Outcome:** Concurrent agents sharing one transport route independently, the production installer is documented, validation is clean, and no spike artifact remains.

---

## Key files referenced

| File                                                                                                                                           | Note                                                                                                                             |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs)                                                   | `call_tool` is the only request path where `preToolUse.updated_input` arrives; the spike captured `_mcpmux_context` here         |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs)               | Existing binding-canonical resolution logic must be reused with an explicit root, not copied or fed through shared session state |
| [`crates/mcpmux-gateway/src/services/session_roots.rs`](../../crates/mcpmux-gateway/src/services/session_roots.rs)                             | Candidate membership validation comes from here; exact call context never writes its pin maps                                    |
| `crates/mcpmux-gateway/src/services/meta_tools/registry.rs`                                                                                    | `MetaToolCall` is the per-request context seam for propagating the exact root through every built-in tool                        |
| `crates/mcpmux-gateway/src/services/meta_tools/meta_tool_common.rs`                                                                            | Central resolver and Space helpers used by search, invoke, list, resource, and prompt tools                                      |
| [`apps/desktop/src-tauri/src/commands/workspace_install.rs`](../../apps/desktop/src-tauri/src/commands/workspace_install.rs)                   | Existing plain-JSON merge, backup, preservation, and refusal behavior to copy for `hooks.json`                                   |
| [`apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx`](../../apps/desktop/src/features/clients/RegisterApiKeyClientModal.tsx)     | Existing global Cursor bridge setup surface where the managed-hook installer belongs                                             |
| [`apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts`](../../apps/desktop/src/features/clients/cursor-bridge-config.helpers.ts) | Existing bridge generator and home for the manual fallback hook entry                                                            |
| [`ryanhiizy/cursor-agent-wakatime`](https://github.com/ryanhiizy/cursor-agent-wakatime)                                                        | Existing user hook that the installer must preserve byte-for-byte semantically                                                   |
| [Hooks — Cursor Docs](https://cursor.com/docs/hooks.md)                                                                                        | `preToolUse` input, MCP matcher shape, and `updated_input` output contract                                                       |

---

## Cloud Agents (researched Aug 21)

The hook path is dead for [Cloud Agents](https://cursor.com/docs/cloud-agent). It does not fix them. Treating a cloud-injected filesystem root as call-authoritative can make them worse.

This plan is scoped to the local global bridge: one `mcp-remote` child, `~/.cursor/hooks.json`, inject `_mcpmux_context` on `MCP:mcpmux_*`. Cloud Agents share none of that stack.

| Constraint | What Cursor actually does | Consequence here |
| --- | --- | --- |
| Hook source | Cloud VMs do not load user-level `~/.cursor/hooks.json`. They run committed [`.cursor/hooks.json`](https://cursor.com/docs/hooks.md#cloud-agent-support), plus Enterprise team/enterprise hooks. | The Phase 4 installer never fires for a cloud run. Decision 12 already parks project hooks. |
| Transport | [Capabilities](https://cursor.com/docs/cloud-agent/capabilities.md): SSE and `mcp-remote` are not supported. MCP comes from the [dashboard / `cursor.com/agents` dropdown](https://cursor.com/docs/mcp), not `~/.cursor/mcp.json`. HTTP is proxied through Cursor's backend. | The VM never talks to `127.0.0.1:45818`. Even [My Machines](https://forum.cursor.com/t/my-machines-cloud-agent-worker-does-not-load-mcp-servers-from-the-host-s-local-cursor-mcp-config/160956) workers ignore host `mcp.json`. |
| `preToolUse` | Supported from repo hooks once the VM is writable. [`beforeMCPExecution`](https://cursor.com/docs/hooks.md#cloud-agent-support) is deferred. | A repo hook that did inject would send the **clone path** (`/workspace/...`), not `/Users/joe/Desktop/Repos/...`. |
| Identity today | Rootless, no `roots/list`, no per-repo header on the dashboard entry. | Shipped answer is `mcpmux_set_workspace_root` plus repo-name matching and machine-scoped bindings. Hundreds of local agents sharing one `mcp-session-id` is a local-bridge bug. Cloud agents are a separate HTTP client with no roots broadcast. |

Exact-path match on a VM root is the trap already recorded in [`rootless-declare-root-gate.md`](./rootless-declare-root-gate.md): Tier 1b hard-returns `Unbound` unless the session is `roots_capable == false` and falls through to basename / grant.

**Does shipping this regress cloud?** Not if missing `_mcpmux_context` keeps today's ladder. Cloud calls then look like they do now.

It *does* regress if:

- a committed project/team hook starts injecting VM paths
- `resolve_for_workspace_root` treats that as call-authoritative without the rootless fallthrough
- candidate-set-absent trust accepts a `/workspace/...` claim as any binding

Decision 4 stays the right default for cloud too: hook context must never write `pinned` or `window_pins`.

**If cloud is wanted later, it is a different feature.** Keep `set_workspace_root` as the declare path, or ship a committed `.cursor/hooks.json` that injects a **repo name / git remote**, not a filesystem path, and resolve that through the existing basename gate. Do not reuse the local absolute-root `_mcpmux_context` object as-is.

Residual: proxy preservation of undeclared `_mcpmux_context` through Cursor's HTTP hop is untested. Irrelevant while the hook stays user-level.

Sources: [Cloud Agents](https://cursor.com/docs/cloud-agent), [Hooks: Cloud agent support](https://cursor.com/docs/hooks.md#cloud-agent-support), [Cloud Agent capabilities](https://cursor.com/docs/cloud-agent/capabilities.md), [MCP](https://cursor.com/docs/mcp), [Do Cloud Agents run hooks?](https://cursor.com/help/ai-features/cloud-agents), [My Machines MCP](https://forum.cursor.com/t/my-machines-cloud-agent-worker-does-not-load-mcp-servers-from-the-host-s-local-cursor-mcp-config/160956), [self-hosted MCP](https://forum.cursor.com/t/how-to-use-mcps-with-self-hosted-agents/157677), plus [`rootless-declare-root-gate.md`](./rootless-declare-root-gate.md) and [`workspace-machine-binding.md`](./workspace-machine-binding.md).

---

## Related documentation

- [`docs/planning/window-scoped-workspace-pin.md`](./window-scoped-workspace-pin.md) — durable session/window state and the field-confirmed shared-process leak this plan must never recreate
- [`docs/planning/cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md) — global `mcp-remote` bridge, core-meta catalog model, and unreliable `${workspaceFolder}` signal
- [`docs/planning/resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) — measured substitution residual and ruled-out active-folder signals
- [`docs/planning/rootless-declare-root-gate.md`](./rootless-declare-root-gate.md) — Cloud Agent / rootless identity; why a VM filesystem path must not go through exact-path Tier 1b
- [`docs/planning/workspace-machine-binding.md`](./workspace-machine-binding.md) — machine-scoped bindings for cloud / remote callers
- [`docs/manual/cursor-workspace-bridge.md`](../manual/cursor-workspace-bridge.md) — user-facing setup and live verification procedure updated in Phase 5
