# Feature Sets as the Capability Consent Unit

**Last Updated:** May 28, 2026
**Status:** Planning — scoped via `dig-and-ask` + `propose-opts-brainstorm`; not yet started
**Branch:** TBD
**Depends on:** the shipped meta-tools + workspace-binding infrastructure — [`dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) (session overrides, `mcpmux_enable_server`), [`meta-gateway-invoke.md`](./meta-gateway-invoke.md) (search → schema → invoke, surfaced tools)
**Supersedes:** the session-override model from [`dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) — this doc removes the ephemeral path it added.

---

## Problem

An agent in this workspace needed a Jira tool and couldn't get to it. Three failures stacked:

1. **Inactive capability is invisible.** `mcpmux_search_tools` searches *invokable* tools only. The Jira server wasn't in the resolved feature set, so search returned zero matches — the agent had no signal the capability even existed. The only way to see a dormant server is `mcpmux_list_servers`, which the agent didn't think to call.
2. **The agent reached for the wrong activation path.** It session-enabled Jira (`mcpmux_enable_server`, session scope) — ephemeral, dies with the session — so it had to redo the dance every new session, instead of binding the capability persistently to the workspace once.
3. **The "security" story doesn't hold under the current model.** Session enable defaults to **auto-allow** (`gateway.session_overrides_require_approval = false`) and grants the **whole server** (`facade.rs` pulls all of a server's features, ignoring feature-set ACL granularity). So an autonomous agent can self-grant any installed server's entire tool surface with no human in the loop, at a *coarser* grain than the operator's own feature sets.

The throughline: McpMux already has a fine-grained, persistent, human-curated capability primitive — the **FeatureSet** bound to a **WorkspaceBinding** — but the agent-facing flow routes *around* it via the ephemeral, coarse, ACL-blind session-override escape hatch.

This doc makes the FeatureSet the single unit of **discovery, consent, and persistence**, and removes the ephemeral path entirely. The model becomes: a human authors bundles; an agent discovers them (active or not) and binds an existing bundle to the workspace through one approval; every future agent in that folder inherits it.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Capability unit | **The FeatureSet is the only unit.** "Enable one tool" = a FeatureSet of one. No separate tool-toggle axis. | Feature sets already express tool-level include/exclude. A second granularity axis (server vs tool vs FS) is redundant complexity. Finer grain is *more* least-privilege, not less — there was never a security reason to forbid tool-at-a-time. |
| 2 | Who authors bundles | **Humans only.** The agent cannot create FeatureSets. It may only *bind an existing* one. | The single approval dialog reviews bundle *contents*, but the human chose `human_only`: agents don't author ACLs. Removes `mcpmux_create_feature_set` (and the server-all FS minting in workspace enable) from the agent surface. |
| 3 | Activation lifetime | **No ephemeral path.** All activation is a persistent WorkspaceBinding. Session-scoped enable/disable is removed. | Bindings already give consent-once + persistent + granular + survives new agents. The ephemeral path is the worst of both worlds (coarse *and* dies with the session) and is the source of the security hole. Remove it cleanly — no shim. |
| 4 | Consent granularity | **At the bundle.** Approve the FeatureSet bind once; every tool in it (reads and destructive writes alike) is usable thereafter. No per-tool sensitivity tiering. | User decision: "if you approve it as part of the bundle it's safe to use." Kills the dependency on MCP tool annotations / a sensitivity classifier. The approval dialog shows the tool list; the human owns that gate at bind time. |
| 5 | Discovery | **Never gated, and must surface inactive capability.** `search_tools` / `list_feature_sets` / `list_servers` return inactive servers and feature sets, flagged, with the `feature_set_id` needed to bind. | Seeing that a capability exists is not exposure. The original Jira-returns-zero failure is a discovery bug. Discovery must point the agent at the bind it should request. |
| 6 | Binding composition | **Layer, don't clobber.** `mcpmux_bind_current_workspace` appends/unions the FeatureSet onto the binding. | The binding model and `workspace_binding.rs` doc explicitly intend layering (`Read Only` + project + Jira, unioned). `bind_current_workspace` currently *replaces* (`feature_set_ids = vec![fs_id]`) — a bug relative to `enable_workspace_server`, which appends. |
| 7 | Approval surface | **Desktop + web.** Approvals must render in the web admin client, not only the Tauri desktop dialog. | Cloud/headless agents connect without a desktop UI to answer `ApprovalBroker`. A dedicated phase designs how bind approvals surface and are answered in the browser. |

---

## The Model

### Canonical agent flow (after this work)

```text
1. Agent needs a capability it can't see.
2. mcpmux_search_tools("jira issue")
     → returns matches even when the containing FeatureSet/server is INACTIVE,
       each annotated { status: "inactive", bindable_feature_set_id: "<uuid>" }.
3. Agent calls mcpmux_bind_current_workspace(feature_set_id)
     → ApprovalBroker prompt (desktop OR web) showing the bundle's tool list.
     → on approve: FeatureSet is APPENDED to the workspace binding (persistent).
4. tools/list_changed fires; the bundle's tools are now invokable.
5. Every FUTURE agent that reports this workspace root resolves the binding
   automatically — no re-approval, no enable call.
```

If **no** FeatureSet contains the needed tool, the agent cannot proceed on its own (Decision 2) — it surfaces a message asking the user to author a bundle in the desktop/web UI. That is the intended dead-end, not a gap.

### What is removed

| Removed | Why |
| ------- | --- |
| `mcpmux_enable_server` / `mcpmux_disable_server` (both scopes) | Session scope is the ephemeral path (Decision 3). Workspace scope minted server-all FeatureSets on the agent's behalf (Decision 2). Bind-existing replaces both. |
| `mcpmux_create_feature_set` | Agents don't author bundles (Decision 2). |
| `SessionOverrideRegistry` + facade composition | No ephemeral overrides remain. |
| `gateway.session_overrides_require_approval` setting + admin routes | The setting it gated no longer exists. |
| "Active session overrides" UI panel + `session_overrides` Tauri commands | Nothing to display. |

### What remains / changes

| Item | State |
| ---- | ----- |
| `WorkspaceBinding` (root → space + `feature_set_ids[]`) | Unchanged storage; `bind_current_workspace` write path fixed to layer (Decision 6). |
| FeatureSet ACL resolution (`resolve_feature_sets`) | Unchanged — already the granular, ACL-respecting path. |
| `mcpmux_bind_current_workspace` | Stays; the *only* agent write tool. Fixed to append + approval (desktop/web). |
| Discovery tools (`search_tools`, `list_feature_sets`, `list_servers`, `get_tool_schema`) | Extended to include + flag inactive capability with bind affordance. |
| `ApprovalBroker` | Extended with a web-renderable surface (Phase 5). |

---

## Files to modify

| File | Change |
| ---- | ------ |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | Extend search/list tools to include inactive servers + feature sets with `bindable_feature_set_id`. Fix `BindCurrentWorkspaceTool` to **append/union** instead of replace. Delete `EnableServerTool`, `DisableServerTool`, `CreateFeatureSetTool`. |
| [`crates/mcpmux-gateway/src/services/meta_tools/mod.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/mod.rs) | Drop the three deleted tools from `build_default_registry`. Remove `session_overrides` from `MetaToolContext`. |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | Remove `SESSION_OVERRIDES_REQUIRE_APPROVAL_KEY` + the `"session_override"` decision string. |
| [`crates/mcpmux-gateway/src/services/meta_tools/workspace_server.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/workspace_server.rs) | Delete — server-all FS minting is removed (Decision 2). |
| [`crates/mcpmux-gateway/src/services/session_overrides.rs`](../../crates/mcpmux-gateway/src/services/session_overrides.rs) | Delete. |
| [`crates/mcpmux-gateway/src/services/mod.rs`](../../crates/mcpmux-gateway/src/services/mod.rs) | Remove `session_overrides` module + re-export. |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs) | Remove session-override composition from `get_*_for_grants`; signatures drop `session_id`. Add an inactive-capability listing path for discovery. |
| [`crates/mcpmux-gateway/src/mcp/handler.rs`](../../crates/mcpmux-gateway/src/mcp/handler.rs) | Stop threading `session_id` into facade. Update the direct-call redirect copy to point at bind, not enable. |
| [`crates/mcpmux-gateway/src/server/service_container.rs`](../../crates/mcpmux-gateway/src/server/service_container.rs) | Drop `SessionOverrideRegistry` construction + wiring. |
| [`crates/mcpmux-gateway/src/consumers/mcp_notifier.rs`](../../crates/mcpmux-gateway/src/consumers/mcp_notifier.rs) | Drop the session-override reap. |
| [`apps/desktop/src/features/workspaces/WorkspacesPage.tsx`](../../apps/desktop/src/features/workspaces/WorkspacesPage.tsx) | Remove the "Active session overrides" panel. |
| [`apps/desktop/src-tauri/src/commands/session_overrides.rs`](../../apps/desktop/src-tauri/src/commands/session_overrides.rs) | Delete. |
| [`apps/desktop/src-tauri/src/commands/settings.rs`](../../apps/desktop/src-tauri/src/commands/settings.rs) | Remove `get/set_session_overrides_require_approval`. |
| `crates/mcpmux-gateway/src/admin/` (router + read/write handlers) | Remove `session-overrides-require-approval` routes. |
| [`README.md`](../../README.md) | Replace the session-override / self-management section with the bind-driven consent model. |
| Web admin approval surface (paths TBD in Phase 5 spike) | New approval rendering + answer transport for browser-connected agents. |

---

## Phasing

### Phase 1 — Discovery surfaces inactive capability

**Effort:** ~1 day

- Extend `mcpmux_search_tools` to match tools in **inactive** servers/feature sets, annotated `{ status: "inactive", bindable_feature_set_id }`.
- Extend `mcpmux_list_feature_sets` / `mcpmux_list_servers` to clearly distinguish bound vs available-but-inactive, each carrying the id needed to bind.
- Tighten `search_tools` zero-result copy to steer toward `list_feature_sets` → `bind_current_workspace`.

**Outcome:** an agent searching `jira issue` in a workspace with **nothing** Jira active gets results pointing at the inactive Jira FeatureSet and the `feature_set_id` to bind. The original zero-match failure is impossible.

### Phase 2 — Bind becomes the canonical activation path; fix layering

**Effort:** ~1 day

- Fix `BindCurrentWorkspaceTool` to append/union the FeatureSet onto the binding's `feature_set_ids` (dedupe), matching `enable_workspace_server`'s prior append semantics and the binding-doc layering intent.
- Update tool descriptions so binding is the obvious next step after discovery.
- Integration tests: bind layers onto an existing binding without dropping prior sets; re-bind is idempotent; a second session inherits the bound bundle.

**Outcome:** an agent binds the Jira bundle via one approval; it layers on top of existing bindings; every future session in that folder resolves it with no further prompts.

### Phase 3 — Remove the ephemeral session-override path

**Effort:** ~1 day

- Delete `SessionOverrideRegistry`, the facade composition, `mcpmux_enable_server` / `mcpmux_disable_server`, `gateway.session_overrides_require_approval` (+ admin routes), the Tauri `session_overrides` commands, and the Workspaces UI panel.
- Drop `session_id` from the `get_*_for_grants` signatures and their callsites.

**Outcome:** the only way to activate capability is a persistent, approved binding. No code path lets an agent self-grant ephemeral or whole-server access. `pnpm validate` + `pnpm test:rust` green after the removal.

### Phase 4 — Lock authoring to humans

**Effort:** ~half a day

- Remove `mcpmux_create_feature_set` from the agent registry; FeatureSet authoring lives only in the desktop/web UI.
- When discovery finds no FeatureSet containing a requested tool, return an actionable "no bundle contains this tool — create one in McpMux, then I can bind it" result.

**Outcome:** an agent cannot mint an ACL. Binding a tool that lives in no FeatureSet yields a clear instruction to author a bundle, not a silent grant.

### Phase 5 — Approval rendering in the web client

**Effort:** ~1–2 days (includes a design spike)

- Spike: how the `ApprovalBroker` request reaches a browser-connected client (SSE event → web modal → answer routed back to the broker) and how it coexists with the desktop dialog.
- Implement bind-approval rendering + answering in the web admin SPA.
- Tests: a bind initiated by a web-admin-connected agent surfaces an answerable approval; approve writes the binding, deny is a clean no-op.

**Outcome:** a bind request from a web/headless agent renders an approval the user can answer in the browser, with the same one-time-consent semantics as desktop.

---

## Out of scope

| Item | Reason |
| ---- | ------ |
| Per-tool sensitivity tiering / MCP annotation trust | Decision 4 — consent is at the bundle; destructive and read tools are treated identically once the bundle is approved. |
| Auto-enable / auto-bind on invoke | Removing the ephemeral path is the point; silent activation reintroduces the consent hole and defeats the audit trail. |
| Agent-authored bundles | Decision 2 — explicitly `human_only`. Could revisit if a real need appears, but not now. |
| Cross-client / cross-workspace binding sharing | Bindings are per normalized root by design; out of this scope. |
| Migration of existing `session_overrides_require_approval` settings rows | Setting is deleted; no user-facing data to preserve (ephemeral by definition). Drop on read. |

---

## Key files referenced

| File | Why |
| ---- | --- |
| [`crates/mcpmux-gateway/src/services/meta_tools/tools.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/tools.rs) | Search/list/bind tool impls; deletion site for enable/disable/create. `BindCurrentWorkspaceTool` (replace→layer) lives here. |
| [`crates/mcpmux-gateway/src/services/meta_tools/workspace_server.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/workspace_server.rs) | The server-all FS minting + the append pattern bind should adopt; deleted under Decision 2. |
| [`crates/mcpmux-gateway/src/pool/features/facade.rs`](../../crates/mcpmux-gateway/src/pool/features/facade.rs) | `get_features_for_grants` — where session-override composition is ripped out and inactive-listing for discovery is added. |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | Tier-1 binding resolution — already the persistent, granular path the model leans on. Unchanged. |
| [`crates/mcpmux-core/src/domain/workspace_binding.rs`](../../crates/mcpmux-core/src/domain/workspace_binding.rs) | Binding entity + the documented multi-FS layering intent that justifies Decision 6. |
| [`crates/mcpmux-gateway/src/services/meta_tools/registry.rs`](../../crates/mcpmux-gateway/src/services/meta_tools/registry.rs) | `MetaToolContext` + the approval-setting const and decision string being removed. |
| [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) | The model this doc supersedes — defines what's being removed. |

---

## Related work

- [`docs/planning/meta-gateway-invoke.md`](./meta-gateway-invoke.md) — search → schema → invoke surface that discovery (Phase 1) extends.
- [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) — the session-override feature being removed in Phase 3.
- [`docs/planning/tool-level-session-pin.md`](./tool-level-session-pin.md) — prior deferral; subsumed by Decision 1 (FeatureSet-of-one is the tool-level unit).
- Stale `/start-ticket` command (user-level, `~/.cursor/commands/start-ticket.md`) — references a pre-McpMux `user-mcp-jira` server; the local-config half of the original incident, fixed outside this repo.

---

## Reconciliation

This doc is the source of truth for what gets built. When implementation completes, update the **Status** field at the top and reconcile deviations (extra files, dropped phases, scope changes) per [`update-planning-md`](~/.cursor/commands/update-planning-md.md).
