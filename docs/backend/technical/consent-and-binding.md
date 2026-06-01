# Consent & Binding

**Synthesizes:** [`feature-set-consent-model.md`](../reference/feature-set-consent-model.md), [`dynamic-mcp-toggle-meta-tools.md`](../reference/dynamic-mcp-toggle-meta-tools.md)

**Last Updated:** Jun 1, 2026

---

## The Model in One Paragraph

A **FeatureSet** is the single unit of capability consent in McpMux. It is a named, human-authored bundle of tools (and resources/prompts) drawn from one or more installed servers. A **WorkspaceBinding** maps a normalized folder root to a Space and a list of FeatureSet IDs. When an agent opens a session in a workspace root, `FeatureSetResolverService` resolves which FeatureSets are active. Those sets define everything the agent can search, schema-inspect, and invoke — nothing else is reachable. Bundles are written by humans; agents can only bind pre-existing ones (with approval).

---

## FeatureSet

A FeatureSet contains zero or more `FeatureSetMember` entries. Each member references a specific `ServerFeature` (tool, resource, or prompt) with two flags:

- **included** — the feature is in the invokable/searchable set for this bundle.
- **surfaced** — the feature is _also_ promoted directly into `tools/list` / `resources/list` / `prompts/list` for one-hop access. Default: false everywhere, including built-in bundles.

Surfacing is opt-in and deliberately rare — it re-introduces the context bloat the whole design was built to avoid. The normal path is search → schema → invoke.

`FeatureSetType` values: `ServerAll` (all features of a server), `Custom` (hand-curated subset). `ServerAll` sets are only ever created by the desktop/web UI, never by an agent.

---

## WorkspaceBinding

```
WorkspaceBinding {
    workspace_root: String,   // normalized absolute path
    space_id: Uuid,
    feature_set_ids: Vec<Uuid>,
}
```

Multiple FeatureSets layer additively — the effective invokable set is the union of all bound FeatureSets' included members, intersected with currently available (connected) server features. Binding an additional FeatureSet **appends** to `feature_set_ids`; it never replaces existing entries.

---

## Resolution Tiers

`FeatureSetResolverService` evaluates in priority order:

| Tier | Source | Notes |
| ---- | ------ | ----- |
| 1 | `WorkspaceBinding` | Persistent; primary path for all workspace-root-reporting clients |
| 2 | `ClientGrant` | Per-client grant (admin-configured) |
| 3 | Deny | No roots / no binding → empty grant |

The session-override tier that existed in `dynamic-mcp-toggle-meta-tools.md` (ephemeral `enabled`/`disabled` sets) is **removed** in the consent-model work. There is no ephemeral path. Every capability activation is a persistent, approved binding.

---

## Canonical Agent Activation Flow

```
1. Agent needs a capability it cannot find.
2. search_tools("jira issue", include_inactive: true)
      → returns inactive matches, each with { status: "inactive", bindable_feature_set_id }
3. Agent calls bind_current_workspace(feature_set_id)
      → ApprovalBroker surfaces dialog (desktop dialog or web modal)
      → on approve: FeatureSet ID is APPENDED to the binding (deduplicated)
      → tools/list_changed fires; the bundle's included tools become invokable
4. Every future agent session at this workspace root resolves the binding
   automatically — no re-approval, no enable call.
```

If no FeatureSet contains the needed tool, the agent cannot proceed — it should surface a message asking the user to author a bundle in the desktop or web UI. This is the intended dead-end.

---

## Discovery Must Surface Inactive Capability

Search and list operations always include inactive servers and feature sets (flagged) so an agent can find a capability even if nothing is currently active. The original failure mode — `search_tools("jira issue")` returning zero results because the Jira server had no active binding — is fixed by including inactive matches annotated with `bindable_feature_set_id`.

Key rules:

- Seeing that a capability exists is **not** exposure — inactive tools cannot be invoked.
- `mcpmux_search_tools` defaults to the active scope (fast). Pass `include_inactive: true` to widen.
- On zero/thin active results the tool response tells the agent to widen or call `list_feature_sets → bind_current_workspace`.
- `mcpmux_list_feature_sets` and `mcpmux_list_servers` clearly distinguish bound-and-active from available-but-inactive, each carrying the ID needed to bind.

---

## Removed: Session-Override Path

The `dynamic-mcp-toggle-meta-tools.md` design introduced an ephemeral `SessionOverrideRegistry` (in-memory `DashMap<SessionId, HashSet<ServerId>>`) so agents could enable/disable servers for a single session without writing a binding. This path is **removed** in the consent model:

| Removed | Reason |
| ------- | ------ |
| `mcpmux_enable_server` / `mcpmux_disable_server` | Session scope was ephemeral and coarse; workspace scope minted server-all FeatureSets on the agent's behalf |
| `mcpmux_create_feature_set` | Agents don't author bundles |
| `mcpmux_list_all_tools` | Catalog firehose; superseded by `search_tools` + `list_servers` + `list_feature_sets` |
| `SessionOverrideRegistry` | No ephemeral overrides remain |
| `gateway.session_overrides_require_approval` setting | The setting it gated no longer exists |
| "Active session overrides" UI panel | Nothing to display |

`mcpmux_bind_current_workspace` is the **only** agent write tool. It always appends, always requires approval.

---

## ApprovalBroker

The `ApprovalBroker` surfaces a consent dialog at bind time showing the bundle's tool list. On approve, the binding write proceeds; on deny, it is a clean no-op. The broker supports:

- **Desktop dialog** — Tauri native dialog for locally-connected clients.
- **Web modal** — SSE event → browser modal → answer routed back to broker (planned Phase 5 of consent-model work; see reference doc).

---

## Key Source Locations

| Path | Role |
| ---- | ---- |
| `crates/mcpmux-core/src/domain/workspace_binding.rs` | `WorkspaceBinding` entity + layering semantics |
| `crates/mcpmux-gateway/src/services/feature_set_resolver.rs` | Tier 1/2/3 resolution; `PendingRoots` path |
| `crates/mcpmux-gateway/src/pool/features/facade.rs` | `get_invokable_tools_for_grants` vs `get_advertised_tools_for_grants` split |
| `crates/mcpmux-gateway/src/services/meta_tools/tools.rs` | `BindCurrentWorkspaceTool`, `SearchToolsTool`, `ListFeatureSetsTool` |
| `crates/mcpmux-gateway/src/services/meta_tools/approval.rs` | `ApprovalBroker` |
