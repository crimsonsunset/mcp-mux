# New Workspace Onboarding

**Last Updated:** Jun 19, 2026

Step-by-step operator flow for routing a **new folder** (or a folder that has never been bound) through McpMux. Read [`../technical/consent-and-binding.md`](../technical/consent-and-binding.md) for the consent model; this guide is the practical checklist.

---

## Two Different Things

| What | How it happens | Persisted? |
| ---- | -------------- | ---------- |
| **Live root detection** | Cursor (or another roots-capable client) connects to `:45818`, completes OAuth + `initialize`, gateway probes `roots/list` | In-memory only (`SessionRootsRegistry`) — wiped on gateway restart |
| **Workspace binding** | You save a mapping: `workspace_root` → Space + FeatureSet(s) | SQLite (`workspace_bindings`) — survives restarts |

Cursor **does not** create bindings for you. It only reports folder paths. You (or the `WorkspaceNeedsBinding` UI prompt) persist the binding.

---

## Prerequisites (one-time per machine)

1. McpMux desktop running; gateway listening on `http://localhost:45818`.
2. Target MCP servers installed and enabled in the default Space (Servers page).
3. A **FeatureSet** that contains the tools you want (e.g. `bundle:s2h`). Author bundles in the desktop UI — agents cannot create them.
4. Cursor configured with the McpMux MCP endpoint and OAuth completed at least once.

---

## Standard Flow — New Folder

### 1. Open the folder in its own Cursor window

Workspace routing keys on the **filesystem root the MCP session reports**, not on your user identity. One OAuth client (`mcp_36740f70` for Cursor) is shared across windows; **the reported root** is what differentiates routing.

Prefer a dedicated Cursor window rooted at the project you care about. If Cursor reports multiple roots in one session, the resolver picks the **longest-prefix binding match** across all reported roots — which may not be the folder you intended.

### 2. Connect / re-auth MCP in that window

After a gateway rebuild or `pnpm dev:restart`, stale MCP sessions are gone. Cursor may keep a TCP socket but not complete OAuth.

- **Cursor → Settings → MCP** → toggle user-mcpmux off/on, or reconnect.
- Approve OAuth in the McpMux consent dialog if prompted.

Until `initialize` completes, the Workspaces page will show no **live** roots for that folder.

### 3. Confirm roots landed (optional but fast)

Gateway log (`~/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.YYYY-MM-DD.log` on macOS):

```text
create new session session_id="..."
[FeatureSetResolver] fetched MCP roots ... roots=["/absolute/path/to/your/project"]
```

If you see `roots=[]`, the client connected but reported no folder — common on background/agent-only sessions. Retry from the window that has the repo open.

If you see **multiple paths** in `roots=[...]`, expect routing to follow longest-prefix match, not necessarily the shortest path.

### 4. Bind on the Workspaces page

**Workspaces** (desktop or web admin) unions live reported roots with persisted bindings:

| Badge | Meaning |
| ----- | ------- |
| **Live + Unmapped** (amber) | Root reported; no binding yet — agents get **deny** for that path |
| **Live + Mapped** (emerald) | Root reported and binding exists |
| **Offline** (neutral) | Binding exists; no current session reporting that root |

For a new folder:

1. Refresh Workspaces (or wait for `session-roots-changed`).
2. Click the **amber** card for your folder (or use the **WorkspaceNeedsBinding** popup if it appears).
3. Pick **Space** + **FeatureSet(s)** (e.g. `bundle:s2h` + `bundle:core` + `bundle:browser`).
4. Save.

Binding is **additive**: multiple FeatureSets union into one allow-list.

### 5. Verify routing

From an agent in **that same window** (after `list_changed` or MCP reload):

- `mcpmux_list_servers` — target servers should show `readiness: ready` (not `bindable`) when bound and connected. Entries with pre-configured invoke defaults (e.g. Atlassian `cloudId`) show **`prefilled_params`** so agents skip manual discovery for those keys.
- `mcpmux_search_tools` — hits include `display_name`, `required_params` (with `"prefilled": true` for pre-configured keys), and `server_readiness`.
- Workspaces inspector → **Effective Features** — should list tools from your chosen bundle(s).

For servers that need sticky per-call args (Atlassian `cloudId`, default `projectKey`), set **`default_params`** in **Servers → Configure → Default Tool Parameters** before expecting one-shot invokes. See [`server-config-lanes.md`](./server-config-lanes.md#default_params).

Gateway log should show:

```text
[FeatureSetResolver] resolved via WorkspaceBinding workspace_root=/your/normalized/path ...
```

---

## After Gateway Restart

Bindings survive; live sessions do not.

1. Gateway comes back (`pnpm dev`, `dev:restart`, or app relaunch).
2. **Re-auth MCP in each Cursor window** that should route (step 2 above).
3. Roots reappear on Workspaces as **Live**; existing bindings show **Live + Mapped** without re-saving.
4. If tools look stale, **Cursor → MCP → Reload tools** (see [`dev-workflow.md`](./dev-workflow.md#cursor-mcp-reload)).

---

## Agent Self-Service (optional)

Agents can append a FeatureSet to the current workspace binding via `mcpmux_bind_current_workspace`, but:

- Requires the session to have reported MCP roots.
- Requires desktop approval (ApprovalBroker).
- Is **not** in `CORE_META_TOOLS` by default (hidden from Cursor's startup tool list) — reachable via error hints or by adding it to core tools in code.

Preferred operator path for new workspaces: **Workspaces UI** (step 4).

---

## Common Pitfalls

| Symptom | Likely cause |
| ------- | ------------ |
| Folder not on Workspaces at all | No MCP session has reported that root yet — reconnect from the correct Cursor window |
| Live but tools are `bindable` | Binding missing or resolver matched a **different** root/binding (check `fetched MCP roots` + longest-prefix winner) |
| Everything OFFLINE after restart | Expected until Cursor reconnects; bindings are still in DB |
| Wrong project's tools | Session reports another window's root (e.g. `generAIt` + `mcp-mux` in one `roots=[...]` list) |
| Deleted binding by mistake | Bindings are not recreated automatically — save again from Workspaces |
| Stale binding at wrong path | e.g. `/Repos/sync2hire-platform` vs `/Repos/Sync2Hire/sync2hire-platform` — delete the dead path only; keep the canonical normalized root |

---

## Quick DB Sanity Check

```bash
DB="$HOME/Library/Application Support/com.mcpmux.desktop/mcpmux.db"

sqlite3 "$DB" "
SELECT wb.workspace_root, wb.label, fs.name
FROM workspace_bindings wb
LEFT JOIN workspace_binding_feature_sets wbfs ON wb.id = wbfs.binding_id
LEFT JOIN feature_sets fs ON wbfs.feature_set_id = fs.id
ORDER BY wb.workspace_root;"
```

Log grep for a session:

```bash
rg 'fetched MCP roots|resolved via WorkspaceBinding' \
  "$HOME/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log"
```

---

## Related docs

- [`../technical/consent-and-binding.md`](../technical/consent-and-binding.md) — FeatureSet consent, resolution tiers, agent bind flow
- [`../technical/data-model.md`](../technical/data-model.md) — `WorkspaceBinding` entity
- [`dev-workflow.md`](./dev-workflow.md) — ports, rebuild, Cursor MCP reload, log paths
- [`../reference/agent-mcp-session-readiness.md`](../reference/agent-mcp-session-readiness.md) — roots timing, cold start, `PendingRoots`
