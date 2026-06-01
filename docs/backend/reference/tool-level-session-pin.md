# Tool-Level Session Pin Meta Tool (`mcpmux_pin_this_session`)

**Last Updated:** May 25, 2026
**Status:** Deferred — superseded by [`meta-gateway-invoke.md`](./meta-gateway-invoke.md) for token budget. May revive as Phase F (invoke ACL only) if search + invoke proves insufficient.
**Branch:** TBD — branch off `main` once [`dynamic-mcp-toggle-meta-tools`](./dynamic-mcp-toggle-meta-tools.md) merges
**Base branch:** `main` (depends on the meta-tools infrastructure shipped via the dynamic-toggle PR being live)
**Issue:** TBD — file after planning review
**Depends on:** [`dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) — provides `SessionOverrideRegistry`, `MetaToolContext`, per-peer `tools/list_changed` plumbing, and the `gateway.session_overrides_require_approval` setting reused here
**Supersedes:** Out-of-Scope row #1 in [`dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) — _"Tool-level granularity… no real evidence yet that tool-level matters more than server-level for token budget"_
**Unblocks:** Per-chat tool-budget control for high-tool-count installs (Google Workspace 120 tools, cloned ×2 = 240) — see [`server-account-clones.md`](./server-account-clones.md)

---

## Problem

The May 23 `dynamic-mcp-toggle-meta-tools` planning doc deferred tool-level granularity on the grounds that _server-level enable/disable covered the user's stated use case_ and there was _no real evidence yet that tool-level mattered more than server-level for token budget_. That evidence now exists.

[`server-account-clones.md`](./server-account-clones.md) shipped Phase 1–4 in mid-May, and the first heavy real-world install — Google Workspace cloned for a Personal + work account split — exposed the gap:

| Symptom                                                                 | Concrete number |
| ----------------------------------------------------------------------- | --------------- |
| Tools per Google Workspace install                                      | 120             |
| Installs after the account clone                                        | 2               |
| Tools surfaced to a Cursor session that needs both accounts             | 240             |
| Approximate system-prompt tokens consumed by those 240 tool definitions | ~30–50k         |

Server-level overrides do not help here: both clones are wanted in the session, so neither can be disabled. `mcpmux_create_feature_set` + `mcpmux_bind_current_workspace` can carve a persistent subset, but persisting a tool list per workspace is the wrong shape for _"for the next 10 minutes I only need Gmail send + read + 8 calendar tools, drop the other 230"_ — the LLM needs an ephemeral knob it can flip per task, not a binding.

This doc fills that gap with **one new write tool plus a small clear tool**, both session-scoped (with an opt-in workspace-scope path that reuses the existing FeatureSet + binding plumbing). The killer detail: the plumbing for this is already wired — `SessionOverrideRegistry` exists, `FeatureService::get_tools_for_grants` is already the materialization chokepoint, per-peer `tools/list_changed` already fires after meta-tool writes. The change is additive: one new field on the registry, one new filter step at the bottom of the existing composition rule, two new `MetaTool` impls.

A note on naming: `mcpmux_pin_this_session` is referenced verbatim in the meta-tools module docstring, in `SettingsPage.tsx`, in the approval-broker tests, and in the WDIO meta-tools spec — leftover from an early draft when the tool was intended to ship with the original meta-tools work. Reusing the name keeps every existing reference truthful instead of forcing a docs-only renaming pass.

---

## Decisions

| #   | Decision                            | Choice                                                                                                                                                                                                               | Rationale                                                                                                                                                                                                                                                                                               |
| --- | ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | New tool vs extend existing         | **One new write tool (`mcpmux_pin_this_session`) + one new write tool (`mcpmux_clear_session_pin`)** — do not overload `enable_server`/`disable_server`                                                              | Server-level and tool-level have different composition semantics (additive overlay vs exclusive replacement). Conflating them would force every call site to disambiguate. Two narrowly-scoped tools keep the surface clean.                                                                            |
| 2   | Composition with existing overrides | **Exclusive replacement applied AFTER server-level composition.** Effective = `(binding_servers ∪ session_on) − session_off`, materialize tools, then if `pinned_tools` non-empty: filter to `tools ∩ pinned_tools`. | Pin semantically means "lock to this set." Layering it on top means a pin to `["github_create_issue"]` works whether `github` is enabled via binding, via session-enable, or both — the user expresses intent (which tools they want) without caring about how those tools got into the candidate pool. |
| 3   | Empty list semantics                | **Empty `tool_qualified_names` rejected as invalid argument** — clearing is done via `mcpmux_clear_session_pin`                                                                                                      | Avoids two paths for the same operation. Reject-on-empty matches `mcpmux_create_feature_set` which already rejects empty `tool_qualified_names`.                                                                                                                                                        |
| 4   | Explicit clear vs implicit          | **Dedicated `mcpmux_clear_session_pin` tool** — no args, operates on caller's session                                                                                                                                | Forces an intentional unpin, audited like any other write. Letting "pin with full list" act as clear would require the LLM to call `list_all_tools` first just to undo — wasteful and ambiguous.                                                                                                        |
| 5   | Naming                              | **`mcpmux_pin_this_session`**                                                                                                                                                                                        | Matches every existing stale reference (mod.rs docstring, SettingsPage copy, approval tests, WDIO spec). The "this_session" suffix makes scope intuitive — ergonomic mirror to the existing `mcpmux_bind_current_workspace`.                                                                            |
| 6   | Scope support                       | **`scope: "session"` (default) + `scope: "workspace"`** mirroring `mcpmux_enable_server`                                                                                                                             | Session is the ephemeral knob. Workspace persists the same pin set via a new auto-named custom FeatureSet bound to the caller's first reported root — single approval-gated atomic op instead of two-step `create_feature_set` + `bind_current_workspace`.                                              |
| 7   | Workspace-scope FeatureSet naming   | **Auto-named `"Pinned: {root_basename} {YYYY-MM-DD HH:MM}"`** unless caller supplies optional `name` arg                                                                                                             | Discoverable in the Workspaces UI after creation. Optional override lets the LLM name it meaningfully (`"Gmail send-only"`) when the user prompt makes intent clear.                                                                                                                                    |
| 8   | Approval default                    | **Session: auto-allow, gated by existing `gateway.session_overrides_require_approval`. Workspace: always required via `ApprovalBroker`.**                                                                            | Reuses the existing setting — no new toggle. Session pin is ephemeral (dies with the MCP session) so the same risk profile as `enable_server`/`disable_server` applies. Workspace persists state, so it always shows the approval dialog with the full tool list in the diff.                           |
| 9   | Override store layout               | **Extend `SessionOverrideRegistry` with `pinned_tools: DashMap<SessionId, HashSet<QualifiedName>>`** — do not introduce a sibling registry                                                                           | Same lifecycle (process-only, dies with session reap), same factory, same GC hook in `MCPNotifier`. Keeping it co-located concentrates session-scoped override state in one struct so `reap_dead_sessions` only needs to know about one type.                                                           |
| 10  | Validation timing                   | **Validate every qualified name exists in caller's resolved Space at call time** (look up via `server_feature_repo::list_for_space`)                                                                                 | Rejects typos and stale tool names up-front with a clear error. Storing unresolved names would let the pin silently shrink to nothing if a server becomes unavailable.                                                                                                                                  |
| 11  | Audit decision string               | **`"session_pin"` for session-scope writes, `"allow_once"` for workspace-scope approvals**                                                                                                                           | Distinct from `"session_override"` used by enable/disable so the audit log + Workspaces UI can render pin-specific rows ("pinned 12 tools" vs "enabled github").                                                                                                                                        |

---

## The Model

### Override store extension

```text
SessionOverrideRegistry {
    enabled       : DashMap<SessionId, HashSet<ServerId>>,        // existing
    disabled      : DashMap<SessionId, HashSet<ServerId>>,        // existing
    pinned_tools  : DashMap<SessionId, HashSet<QualifiedName>>,   // NEW
}
```

`pinned_tools[sid]` stores fully qualified tool names (e.g. `"github_create_issue"`, `"google-workspace-mcp-uvx_send_email"`) — the same format `mcpmux_list_all_tools` returns and `mcpmux_create_feature_set` accepts. Empty/missing = no pin = pass-through.

GC: `SessionOverrideRegistry::remove(session_id)` extended to drop `pinned_tools[sid]` alongside the existing two sets. `MCPNotifier::reap_dead_sessions` already calls `remove` per reaped session — no notifier change needed.

### Composition rule

The existing composition in `FeatureService::get_tools_for_grants` runs steps 1–6; this doc adds steps 7–8:

```text
1. (space, feature_set_ids) ← FeatureSetResolverService::resolve(...)
2. binding_servers          ← FeatureService::servers_for(space, feature_set_ids)
3. session_on               ← SessionOverrideRegistry.enabled[session_id]
4. session_off              ← SessionOverrideRegistry.disabled[session_id]
5. effective_servers        ← (binding_servers ∪ session_on) − session_off
6. base_tools               ← every Tool feature whose server_id ∈ effective_servers AND is_available

7. pinned ← SessionOverrideRegistry.pinned_tools[session_id]
8. if pinned is non-empty:
       tools ← base_tools.filter(qualified_name ∈ pinned)
   else:
       tools ← base_tools
```

The pin filter applies only to **tools**. Prompts and resources are unaffected — pinning is a tool-budget concept, not a capability-restriction concept. `get_prompts_for_grants` and `get_resources_for_grants` skip steps 7–8.

A pin that resolves to zero matches (every name filtered out by `is_available`) returns an empty tool list. The caller's `tools/list` will show no tools; calling `mcpmux_clear_session_pin` is the recovery path. This is intentional — silent fall-through to `base_tools` on empty intersection would mask user error.

### Tool surface

Two new tools registered in `build_default_registry`:

| Tool                       | Type  | Approval (default)                                                                                       | Purpose                                                                                                                                               |
| -------------------------- | ----- | -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mcpmux_pin_this_session`  | write | session: auto-allow (configurable via `gateway.session_overrides_require_approval`); workspace: required | Replace the caller's session tool list with the explicit qualified-name set. Optional `scope: "workspace"` persists as a custom FeatureSet + binding. |
| `mcpmux_clear_session_pin` | write | auto-allow                                                                                               | Drop the caller's pin; next `tools/list` returns the full default-routed set.                                                                         |

Both fire `MCPNotifier::notify_session_lists_changed(session_id)` after a successful write — the calling LLM sees the new (or restored) tool list on its next `tools/list` poll.

### Workspace-scope variant flow

`mcpmux_pin_this_session({ tool_qualified_names: [...], scope: "workspace", name?: "..." })` is sugar for the two-call sequence the user would otherwise run:

```text
1. Validate every qualified_name exists in caller's Space
2. Resolve caller's first reported workspace root (require session_roots; reject if missing)
3. Open approval dialog with diff: { root, new FS name, full tool list }
4. On allow:
   a. Create custom FeatureSet with the matched ServerFeature ids (using add_feature_member + MemberMode::Include)
   b. Create WorkspaceBinding(root, space_id, feature_set_id)
   c. Emit FeatureSetMembersChanged and WorkspaceBindingChanged so the resolver picks it up
5. Return { ok: true, feature_set_id, binding_id, scope: "workspace" }
```

The same path `create_feature_set` + `bind_current_workspace` exercises, run atomically as one approval. Failure between (4a) and (4b) leaves a custom FeatureSet without a binding — the existing Workspaces UI already handles unbound custom FSes (they appear in the list and can be bound or deleted), so no compensating cleanup is needed.

### What McpMux stores

| Item                | Storage                                                                         | Persistence                              |
| ------------------- | ------------------------------------------------------------------------------- | ---------------------------------------- |
| `pinned_tools` set  | `SessionOverrideRegistry` (in-memory `DashMap`)                                 | Process-lifetime; dies with session reap |
| Workspace-scope pin | new custom `FeatureSet` row + new `workspace_bindings` row                      | Persistent (uses existing schema)        |
| Audit trail         | `DomainEvent::MetaToolInvoked` with `decision: "session_pin"` or `"allow_once"` | Persistent via existing audit log        |

No new tables, columns, or migrations.

---

## Architecture

```
                ┌──────────────────────────────────────────────┐
                │  FeatureService::get_tools_for_grants        │
                │                                              │
                │  1–6. Server composition (existing)          │
                │       effective_servers =                    │
                │         (binding ∪ session_on) − session_off │
                │       base_tools = tools for servers         │
                │                                              │
                │  7–8. NEW: tool-pin filter                   │
                │       if pinned_tools[sid] non-empty:        │
                │         tools = base_tools ∩ pinned_tools    │
                │       else:                                  │
                │         tools = base_tools                   │
                └──────────────────────────────────────────────┘
                                    ▲
                                    │
                    ┌───────────────┴───────────────┐
                    │                               │
                    ▼                               ▼
        ┌──────────────────────┐    ┌──────────────────────────────┐
        │ SessionOverride-     │    │ Meta tool writes mutate this │
        │ Registry (extended)  │    │ registry directly.           │
        │                      │    │                              │
        │ enabled              │    │ mcpmux_pin_this_session      │
        │ disabled             │    │ mcpmux_clear_session_pin     │
        │ pinned_tools  ← NEW  │    │                              │
        └──────────────────────┘    └──────────────────────────────┘
```

- `SessionOverrideRegistry` lives where it already does (`crates/mcpmux-gateway/src/services/session_overrides.rs`). One new field, three new methods (`pin`, `clear_pin`, `pinned_set`). Existing `remove(session_id)` extended to clear the new map.
- `FeatureService::get_tools_for_grants` gains steps 7–8. The sibling `get_prompts_for_grants` / `get_resources_for_grants` are intentionally untouched — pin is tool-only.
- The two new `MetaTool` impls land in `meta_tools/tools.rs` alongside the existing five write tools. Same `with_approval()` template, same `caller_space_id` / `validate_server_in_space` helpers — workspace variant calls into a new `workspace_pin::pin_workspace` helper module mirroring `workspace_server.rs`.

---

## Files to create

| File                                                                                                                                     | Purpose                                                                                                                                                          |
| ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/services/meta_tools/workspace_pin.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/workspace_pin.rs) | `pin_workspace(call, space_id, qualified_names, name) -> CallToolResult` — atomic create-FS + bind-workspace under one approval, mirroring `workspace_server.rs` |
| [`tests/rust/tests/integration/tool_pin.rs`](../../tests/rust/tests/integration/tool_pin.rs)                                             | E2E composition + meta-tool tests for pin / clear / workspace-scope                                                                                              |

## Files to modify

| File                                                                                                                           | Change                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/services/session_overrides.rs`](../../crates/mcpmux-gateway/src/services/session_overrides.rs)     | Add `pinned_tools: DashMap<String, HashSet<String>>` field. Methods: `pin(session_id, names)`, `clear_pin(session_id)`, `pinned_set(session_id) -> HashSet<String>`. Extend `remove()` to drop the entry. Extend `list_all()` to surface pinned counts for UI                                   |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs)                 | `get_tools_for_grants` applies the pin filter as the final step. `get_prompts_for_grants` / `get_resources_for_grants` unchanged                                                                                                                                                                |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs)       | Implement `PinThisSessionTool` and `ClearSessionPinTool`. Session path mirrors `EnableServerTool` (optional approval). Workspace path delegates to `workspace_pin::pin_workspace`. Validation via existing `server_feature_repo::list_for_space`                                                |
| [`crates/mcpmux-gateway/src/services/meta_tools/mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs)           | `pub mod workspace_pin;` + `registry.register(Box::new(tools::PinThisSessionTool))` and `registry.register(Box::new(tools::ClearSessionPinTool))` in `build_default_registry`. Update module-level docstring example (currently references `mcpmux_pin_this_session` aspirationally — now real) |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | Document `"session_pin"` as a valid `decision` value in the audit-emission block; no other change                                                                                                                                                                                               |
| [`apps/desktop/src/features/settings/SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx)             | Update meta-tools description from `"mcpmux_list_all_tools, mcpmux_pin_this_session, and 6 others"` to `"mcpmux_list_all_tools, mcpmux_pin_this_session, and 7 others"` (count goes from 7 → 9 registered tools)                                                                                |
| [`apps/desktop/src/features/workspaces/WorkspacesPage.tsx`](../../apps/desktop/src/features/workspaces/WorkspacesPage.tsx)     | Extend "Active session overrides" panel with a "Pinned tools" row per session — shows count + expandable list, plus a "Clear pin" button calling a new Tauri command                                                                                                                            |
| [`apps/desktop/src-tauri/src/commands/session_overrides.rs`](../../apps/desktop/src-tauri/src/commands/session_overrides.rs)   | Extend `list_session_overrides` return shape with `pinned_tools: string[]`. Add `clear_session_pin(session_id)` Tauri command                                                                                                                                                                   |
| [`apps/desktop/src/lib/api/sessionOverrides.ts`](../../apps/desktop/src/lib/api/sessionOverrides.ts)                           | TS wrapper update: extend return type with `pinned_tools` field; add `clearSessionPin(sessionId)`                                                                                                                                                                                               |
| [`README.md`](../../README.md)                                                                                                 | Self-management meta tools section gains the pin tool — keep the section format the existing dynamic-toggle work established                                                                                                                                                                    |

---

## Phasing

### Phase 1 — Registry extension + composition wiring

**Effort:** 1 evening

- [ ] Add `pinned_tools: DashMap<String, HashSet<String>>` to `SessionOverrideRegistry`
- [ ] Methods: `pin(session_id, names)` (replaces any existing pin), `clear_pin(session_id)`, `pinned_set(session_id) -> HashSet<String>`
- [ ] Extend `remove(session_id)` to drop `pinned_tools` alongside `enabled` / `disabled`
- [ ] Extend `list_all()` snapshot to include pinned counts so the UI panel can render without extra calls
- [ ] `FeatureService::get_tools_for_grants(session_id)` applies the pin filter as the final step (steps 7–8 in the composition rule above)
- [ ] Unit tests on the registry; composition test in `tests/rust/tests/integration/tool_pin.rs` exercising: no pin → full list, pin with subset → filtered, pin with non-matching names → empty, clear → restored

**Outcome:** Direct registry mutation (`registry.pin("sess-1", ["github_create_issue"])`) causes the next `get_tools_for_grants("sess-1", grants)` to return exactly one tool, regardless of the binding's normal output. Meta tools and UI unchanged.

### Phase 2 — `mcpmux_pin_this_session` (session scope only)

**Effort:** 1 evening

- [ ] `PinThisSessionTool` in `meta_tools/tools.rs` — args `{ tool_qualified_names: string[], scope?: "session" | "workspace", name?: string }`
- [ ] Validate `tool_qualified_names` non-empty (else `InvalidArgument`)
- [ ] Validate every qualified name resolves to an available `Tool` feature in the caller's resolved Space — collect the mismatches and surface them in the error message so the LLM can recover without guessing
- [ ] Session path: optional approval via `gateway.session_overrides_require_approval` → `registry.pin(sid, names)` → set audit decision to `"session_pin"` → emit per-peer `tools/list_changed` via `MCPNotifier::notify_session_lists_changed`
- [ ] Workspace path: return `InvalidArgument("workspace scope ships in Phase 4")` for now
- [ ] Register in `build_default_registry`
- [ ] Integration tests: pin reduces tool list, `list_changed` fires, invalid name returns descriptive error, approval-required setting routes through `ApprovalBroker`

**Outcome:** LLM calls `mcpmux_pin_this_session({ tool_qualified_names: ["google-workspace-mcp-uvx_send_email", "google-workspace-mcp-uvx_list_calendar_events"] })` and the next `tools/list` returns exactly those two tools instead of 240. Cursor's UI updates without restarting the session.

### Phase 3 — `mcpmux_clear_session_pin`

**Effort:** 1 hour

- [ ] `ClearSessionPinTool` — no args; operates on caller's `session_id`
- [ ] Calls `registry.clear_pin(sid)`; idempotent (clearing an unpinned session is `ok: true` with `was_pinned: false`)
- [ ] No approval gate even when `session_overrides_require_approval` is on — clear is always safe (broadens scope)
- [ ] Audit decision `"session_pin_cleared"`
- [ ] Emit per-peer `tools/list_changed`
- [ ] Register in `build_default_registry`
- [ ] Integration test: pin then clear restores full set

**Outcome:** LLM can recover from a bad pin (e.g. pinned the wrong tool name and now has 0 visible tools) without restarting the session.

### Phase 4 — Workspace-scope variant

**Effort:** 1 day

- [ ] `crates/mcpmux-gateway/src/services/meta_tools/workspace_pin.rs` — `pin_workspace(call, space_id, qualified_names, name)`
- [ ] Resolve caller's first reported root via `session_roots.get(sid)` — reject with descriptive error if no roots reported
- [ ] Auto-name FS as `"Pinned: {root_basename} {YYYY-MM-DD HH:MM}"` unless caller supplied `name`
- [ ] Build approval diff payload: `{ workspace_root, feature_set_name, added_tools: [first ~10 qualified names], total_count }`
- [ ] `with_approval` → on allow: create custom `FeatureSet` with matched `ServerFeature` ids (using `add_feature_member` + `MemberMode::Include`) → create `WorkspaceBinding` → emit `FeatureSetMembersChanged` and `WorkspaceBindingChanged`
- [ ] Audit decision `"allow_once"` (matches workspace writes)
- [ ] Return `{ ok: true, feature_set_id, binding_id, workspace_root, scope: "workspace", tool_count }`
- [ ] Integration test: workspace-scope pin survives simulated session restart (new session for the same root resolves through the new binding), unbound caller rejected, name override respected

**Outcome:** A single approval-gated tool call persists a custom tool subset for the caller's workspace cwd. Equivalent to `create_feature_set` + `bind_current_workspace` chained, with one approval and one audit row instead of two.

### Phase 5 — UI surface + doc cleanup

**Effort:** 0.5 day

- [ ] Workspaces page session inspector gains "Pinned tools (N)" row per session; expanding shows the full qualified-name list
- [ ] "Clear pin" button next to the existing per-session "Clear all overrides" button — calls `clearSessionPin` from `lib/api/sessionOverrides.ts`
- [ ] Settings copy update in `SettingsPage.tsx`: bump "and 6 others" to "and 7 others" so the displayed count matches the registry
- [ ] `meta_tools/mod.rs` module docstring no longer needs the aspirational `mcpmux_pin_this_session` reference qualifier — leave it as-is (the example is now accurate)
- [ ] README self-management meta-tools section gains `mcpmux_pin_this_session` + `mcpmux_clear_session_pin` entries; keep the table format the existing dynamic-toggle work established
- [ ] CHANGELOG: release-please handles it via conventional `feat(meta-tools): add tool-level session pin` commit — no manual edit

**Outcome:** A user viewing the Workspaces page can see "session abc123 is pinned to 12 tools" with the tool names expandable and a one-click clear. Settings copy is honest about the registered tool count. README and module docs no longer reference a tool that doesn't exist.

---

## Pre-PR validation

Do **not** open a PR until all automated checks pass and the production build is verified manually.

| Step             | Command                                                                                                                                                   | Purpose                                                                     |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Full validate    | `pnpm validate`                                                                                                                                           | fmt, clippy, check, eslint, typecheck                                       |
| Rust tests       | `pnpm test:rust`                                                                                                                                          | unit + integration (`tool_pin.rs`)                                          |
| TS tests         | `pnpm test:ts`                                                                                                                                            | vitest                                                                      |
| Production build | `pnpm build`                                                                                                                                              | Tauri build on current platform                                             |
| Manual smoke     | Run app, exercise pin + clear from a real MCP client (Cursor), verify Workspaces panel reflects state, verify approval dialog content for workspace scope | UX verification — the diff payload's tool-list rendering is easy to regress |

Optional (slow / env-dependent): `pnpm test:e2e`, `pnpm test:e2e:web`.

**PR target:** `main` (assumes `dynamic-mcp-toggle-meta-tools` is already merged).

---

## Out of scope

| Item                                                                                                  | Reason                                                                                                                                                                                                 |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Pin expiry / TTL                                                                                      | Sessions are already ephemeral. Explicit `mcpmux_clear_session_pin` covers the cleanup path. A TTL would be a different concept and isn't asked for.                                                   |
| Wildcard / glob support in pin lists (`google-workspace-mcp-uvx_*`)                                   | Explicit qualified names are unambiguous and the LLM can produce them by reading `mcpmux_list_all_tools` first. Wildcard support is additive — defer until a real use case shows up.                   |
| Multi-pin layering (stack multiple pin sets, additive)                                                | Single replacement set keeps semantics crisp. Layering would need ordering rules and conflict resolution — not justified by the current evidence.                                                      |
| Pin export to a portable template (across workspaces or users)                                        | Workspace-scope variant already creates a persisted custom FeatureSet; sharing those across workspaces is a separate "FeatureSet templates" feature not yet scoped.                                    |
| LLM-driven pin recommendations (host suggests "based on this prompt, here are the 10 tools you need") | That's a host-side concern (Cursor / Claude Code), not a gateway concern. Gateway exposes the mechanism; host decides when to use it.                                                                  |
| Pin applied to prompts / resources                                                                    | Pin is tool-budget-shaped. Prompts/resources are not the bottleneck. Keep the surface narrow.                                                                                                          |
| Settings UI toggle for `mcpmux_pin_this_session` independent of other session overrides               | Reuses the existing `gateway.session_overrides_require_approval` setting deliberately — adding per-tool toggles balloons the settings surface for no clear benefit. Revisit if user feedback diverges. |

---

## Key files referenced

| File                                                                                                                                           | Why                                                                                                                                                                                                              |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`crates/mcpmux-gateway/src/services/session_overrides.rs`](../../crates/mcpmux-gateway/src/services/session_overrides.rs)                     | Registry being extended. Existing `enabled` / `disabled` field shape and `remove()` GC contract are the template for the new `pinned_tools` field                                                                |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs)                       | Where the two new `MetaTool` impls land. `EnableServerTool` / `DisableServerTool` are the closest templates (session-scope short-circuit + workspace delegation pattern)                                         |
| [`crates/mcpmux-gateway/src/services/meta_tools/workspace_server.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/workspace_server.rs) | Pattern reference for `workspace_pin.rs` — shows how workspace-scope writes resolve roots, build approval payloads, and create/modify bindings atomically                                                        |
| [`crates/mcpmux-gateway/src/services/meta_tools/mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs)                           | `build_default_registry` factory — registration site for the new tools. Module-level docstring already mentions `mcpmux_pin_this_session` as the canonical example; this PR makes that mention accurate          |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs)                 | `MetaToolContext` + `MetaToolRegistry::call` dispatch. Audit-decision string list extended with `"session_pin"` / `"session_pin_cleared"`                                                                        |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs)                                 | `FeatureService::get_tools_for_grants` is the materialization chokepoint where the pin filter applies. Existing server composition logic stays untouched — the filter is one new conditional block at the bottom |
| [`crates/mcpmux-gateway/src/consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs)                             | Session-reap pass already calls `SessionOverrideRegistry::remove` per reaped session — automatically picks up the new `pinned_tools` field, no notifier change needed                                            |
| [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md)                                                         | Direct predecessor — defines the SessionOverrideRegistry pattern, approval flow conventions, and Out-of-Scope row #1 this doc supersedes                                                                         |
| [`docs/planning/server-account-clones.md`](./server-account-clones.md)                                                                         | Origin of the 240-tool context-bloat evidence that justifies revisiting tool-level granularity                                                                                                                   |

---

## Related work

- [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) — defines the meta-tools infrastructure and the session-override registry pattern. Supersedes its Out-of-Scope row #1.
- [`docs/planning/server-account-clones.md`](./server-account-clones.md) — clone feature that created the 240-tool context-bloat evidence. This doc is a natural follow-on for users who hit that ceiling.
- [MikkoParkkola/mcp-gateway](https://github.com/MikkoParkkola/mcp-gateway) and [abdullah1854/MCPGateway](https://github.com/abdullah1854/MCPGateway) — alternative architectural answer (search-then-invoke meta gateway, ~95% context reduction). Considered and rejected for this PR: their model hides all backend tools by default and requires the LLM to discover via `gateway_search_tools` on every call. The McpMux approach keeps the LLM's mental model of named tools intact and lets the user (via the LLM) opt in to budget reduction per session. Worth revisiting as a separate planning doc if the pin-based approach proves insufficient.
- [MCP spec — Tools `list_changed` notification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#list-changed-notification) — protocol mechanism that makes the post-pin tool-list refresh observable mid-conversation. Already wired by [`dynamic-mcp-toggle-meta-tools`](./dynamic-mcp-toggle-meta-tools.md).
- [modelcontextprotocol/servers#2173](https://github.com/modelcontextprotocol/servers/issues/2173) — upstream tracking issue for server-side multi-tenancy. Pin is a gateway-side workaround for the same underlying problem (tool surface scaling poorly with multi-account use).

---

## Reconciliation

This doc is the source of truth for what gets built. When implementation completes, update the **Status** field at the top and reconcile any deviations (extra files, dropped phases, scope changes) per [`update-planning-md`](~/.cursor/commands/update-planning-md.md).
