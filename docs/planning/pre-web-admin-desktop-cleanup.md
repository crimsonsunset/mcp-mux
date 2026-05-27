# Pre–Web Admin Desktop Cleanup (IPC, API, Events)

**Last Updated:** May 25, 2026
**Status:** Complete
**Branch:** `fix/pre-web-admin-cleanup` (off **`dev`**)
**Base branch:** `dev` (fork)
**Issue:** TBD — file after planning review
**Depends on:** None
**Blocks:** [Web Admin Mode (Remote UI via HTTP)](../guide/gateway.mdx) — do not start admin HTTP work until this doc is complete
**Source audit:** [web-admin-parity-matrix.md](./web-admin-parity-matrix.md) (May 25, 2026 invoke + event scan)

---

## Problem

The parity matrix scan for web admin surfaced real bugs and structural drift in the desktop IPC layer — not theoretical cleanup. Today:

- **2 FE `invoke()` calls target commands that do not exist** (`export_config`, `list_registry_categories`).
- **2 server lifecycle stacks** coexist (`set_server_enabled` vs `enable_server_v2`; `disconnect_server` vs `disconnect_server_v2`).
- **`invoke()` is scattered** across components (`SettingsPage`, `OAuthConsentModal`, `MetaToolApprovalDialog`) outside `lib/api/`.
- **Event channels are inconsistent** — `useDomainEvents` documents `grants-changed` but the backend emits `client-grant-changed`; WorkspacesPage listens to channels nothing emits (`workspace-appearance-changed`, `server-status`).
- **~7 additional Tauri channels** are used by the UI but absent from `useDomainEvents` — web admin SSE would miss them if we only bridge the hook’s 10 channels.

Building `command_bridge` + REST on top of this duplicates bugs and guarantees parity drift. Fix the desktop surface first so web admin mirrors one clean contract.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Gate web admin | **This cleanup is a hard prerequisite** for [gateway guide](../guide/gateway.mdx) | One IPC contract → one bridge → one HTTP map. |
| 2 | Server enable/disable | **Standardize on `*_server_v2` + ServerManager** for UI-driven connect/disconnect | `enable_server_v2` is what ServersPage / `useServerManager` use; `set_server_enabled` path is legacy and unused in live UI. |
| 3 | Server disconnect semantics | **Keep both commands, document roles** — `disconnect_server` (logout / token clear) vs `disconnect_server_v2` (pause, preserve creds) | Different UX on ServersPage; do not merge into one ambiguous endpoint. Wire dead `disconnectServerV2` or remove wrapper. |
| 4 | Config export | **Wire FE to `preview_config_export` + `export_config_to_file`** — delete `export_config` invoke | Backend already has the correct API; `gateway.ts` export helper is stale. |
| 5 | Registry categories | **Remove `listCategories` / `list_registry_categories`** unless product needs it | No Tauri handler; never called. Categories come from registry bundle UI config today. |
| 6 | Feature members API | **Single command family** — UI uses `feature_set` batch APIs (`set_feature_set_members`); delete unused `featureMembers.ts` unless a screen needs granular `add_feature_to_set` | Two backend modules + dead FE module = parity trap. |
| 7 | Component invokes | **All Tauri calls go through `lib/api/*`** | Required for later `transport.ts` swap; grep-able surface. |
| 8 | Event channels | **One canonical channel list** — align `gateway.rs` emitter, `useDomainEvents`, and direct `listen()` calls | SSE bridges Tauri channel names, not an abbreviated subset. |
| 9 | `grants-changed` naming | **Rename hook channel to `client-grant-changed`** to match backend (or alias emit — prefer rename) | `useClientEvents` currently subscribes to a channel nothing emits. |
| 10 | Dead backend commands | **Defer wiring, do not delete** — see [Audit checklist](#audit-checklist-may-25-2026) § Deferred backend | Not web-admin blockers; track in parity matrix. No scope creep in this doc. |
| 11 | Testing | **Desktop E2E regression is the gate** — no new test frameworks | Fixes must not break existing WDIO specs; add narrow tests only where bugs are reproducible. |
| 12 | Dead FE wrappers | **Remove unused exports** — `connectServer`, `exportConfig`, `listCategories`, `disconnectServerV2` (if still uncalled) | Dead API surface becomes accidental web-admin routes. |
| 13 | `approve_oauth_client` | **Keep as E2E-only** — document in parity matrix as Desktop/E2E-only; no `lib/api` wrapper | Returns error outside E2E test mode; not operator UI. |
| 14 | Backend `feature_members` commands | **Keep registered, no FE exposure** — UI uses `feature_set` batch APIs only; matrix marks `add_feature_to_set` / `get_feature_set_members` as Deferred | Avoid two HTTP families for the same UX; Rust stays for future granular editor if needed. |
| 15 | Duplicate settings invokes | **`set_log_retention_days` only via `lib/api/logs.ts`** — SettingsPage imports helper, no second invoke site | Same command, two call paths today. |
| 16 | Dual event emitters | **Document two Rust paths; unify FE listeners only** — EventBus→`gateway.rs` bridge vs direct `app.emit` in `oauth.rs` / `session_overrides.rs`; merging emitters deferred to web-admin SSE work | Desktop cleanup fixes TS/Rust channel *names*; SSE bridge must handle both sources later. |
| 17 | `flush_pending_deep_link` | **Wrap in `lib/api/oauth.ts`** — desktop-only (deep link handler); matrix N/A for web | Consent modal must not invoke directly post–Phase 3. |

---

## The Model

### Target IPC shape (post-cleanup)

```text
React UI / hooks / stores
        │
        ▼
  lib/api/*.ts  (sole invoke surface — no component-level invoke)
        │
        ▼
  Tauri commands (thin — no duplicated business logic)
        │
        ├── ServerManager path  → enable/disable/auth/disconnect_v2
        ├── Gateway path        → gateway lifecycle, disconnect+logout (not connectServer — removed if unused)
        └── Domain services     → spaces, registry, feature sets, …
```

### Canonical event channels (post-audit target)

Channels the UI may subscribe to (Tauri today; SSE tomorrow):

| Channel | Source | Primary consumers |
| ------- | ------ | ----------------- |
| `space-changed` | EventBus → gateway bridge | `useDomainEvents` |
| `server-changed` | EventBus → gateway bridge | `useDomainEvents`, ServersPage reload |
| `server-status-changed` | EventBus → gateway bridge | `useServerManager`, `serverManager.ts` |
| `server-auth-progress` | EventBus → gateway bridge | `useServerManager` |
| `server-features-refreshed` | EventBus → gateway bridge | `useServerManager` |
| `feature-set-changed` | EventBus → gateway bridge | `useDomainEvents` |
| `client-changed` | EventBus → gateway bridge | `useDomainEvents` |
| `client-grant-changed` | EventBus → gateway bridge | Clients grants UI (**rename from `grants-changed` in hook**) |
| `gateway-changed` | EventBus → gateway bridge | `useGatewayEvents` |
| `mcp-notification` | EventBus → gateway bridge | `useDomainEvents` |
| `session-roots-changed` | EventBus → gateway bridge | WorkspacesPage |
| `workspace-binding-changed` | EventBus → gateway bridge | WorkspacesPage (includes appearance updates today) |
| `workspace-needs-binding` | EventBus → gateway bridge | Workspaces binding prompt |
| `session-overrides-changed` | Direct emit (`session_overrides.rs`) | WorkspacesPage session overrides panel |
| `oauth-client-changed` | Direct emit (`oauth.rs`) | ClientsPage OAuth client list |
| `meta-tool-invoked` | EventBus → gateway bridge | MetaToolAuditLog, Workspaces connection log |

**Remove listeners for:** `workspace-appearance-changed`, `server-status`, `grants-changed` (after rename/fix).

### Event emission paths (do not merge in this doc)

| Path | Rust location | Channels |
| ---- | ------------- | -------- |
| EventBus → Tauri bridge | [`gateway.rs`](../../apps/desktop/src-tauri/src/commands/gateway.rs) `domain_event_to_tauri` | Most rows in table above |
| Direct `app.emit` | [`oauth.rs`](../../apps/desktop/src-tauri/src/commands/oauth.rs) | `oauth-client-changed` |
| Direct `app.emit` | [`session_overrides.rs`](../../apps/desktop/src-tauri/src/commands/session_overrides.rs) | `session-overrides-changed` |

Web admin SSE must subscribe to **both** paths (or fan-in at emit time in a follow-up). This cleanup only ensures FE channel names match what Rust actually emits.

### Target event hooks (replace page-level `listen()`)

| Hook (new or extended) | Channels | Replaces direct `listen` in |
| ---------------------- | -------- | --------------------------- |
| `useDomainEvents` (+ extended union) | space, server, feature-set, client, gateway, mcp-notification, client-grant-changed | Various via existing hook |
| `useServerManager` / `serverManager.ts` | server-status-changed, server-auth-progress, server-features-refreshed | Already centralized — keep |
| `useWorkspaceEvents` (new) | session-roots-changed, workspace-binding-changed, workspace-needs-binding, session-overrides-changed | `WorkspacesPage.tsx` |
| `useOAuthClientEvents` (new) or extend clients hook | oauth-client-changed | `ClientsPage.tsx` |
| `useMetaToolEvents` (new) | meta-tool-invoked | `MetaToolAuditLog.tsx`, Workspaces connection log |

---

## Scope

### In scope

- Fix or remove broken `invoke()` targets
- Consolidate server lifecycle call sites onto documented paths
- Move component-level invokes into `lib/api/`
- Split or reorganize `gateway.ts` OAuth/export concerns into focused API modules
- Delete dead FE modules / store actions (`featureMembers.ts`, `registryStore.toggleServer`, unused wrappers — see Decision 12)
- Deduplicate invoke call sites (`set_log_retention_days`, `respond_to_meta_tool_approval`)
- Event channel audit + hook consolidation (no orphan `listen()` in pages)
- Refresh [web-admin-parity-matrix.md](./web-admin-parity-matrix.md) after cleanup

### Out of scope

| Item | Reason |
| ---- | ------ |
| Admin HTTP / `command_bridge` / SSE implementation | Blocked on this doc — [gateway guide](../guide/gateway.mdx) |
| Wire `search_servers` (registry uses client-side filter in `registryStore`) | Deferred — parity matrix |
| Wire full config-export UI (`preview_config_export`, `get_config_paths`, `check_config_exists`, `backup_existing_config`) | Deferred — `configExport.ts` module only in Phase 1 |
| Wire `seed_server_features`, `generate_gateway_config` | Deferred — no FE consumer |
| `approve_oauth_client` in operator UI | E2E-test-only command — parity matrix Desktop/E2E-only |
| Expose backend `feature_members` commands over future HTTP | Deferred — batch `feature_set` APIs are the UI contract |
| Merge EventBus + direct `app.emit` into single Rust emitter | Deferred — web-admin SSE fan-in |
| `window.__TAURI_TEST_API__` gating in `main.tsx` | E2E harness; optional hardening later |
| Playwright `.spec.ts` / WDIO parity strategy | [gateway guide](../guide/gateway.mdx) — not desktop cleanup |
| Multi-tenant / web auth | Web admin concern |
| Rewriting ServerManager architecture | Consolidate call sites only |

---

## Files to create

| File | Purpose |
| ---- | ------- |
| `apps/desktop/src/lib/api/settings.ts` | Settings / startup / gateway port / app logs path (from SettingsPage invokes) |
| `apps/desktop/src/lib/api/oauth.ts` | OAuth consent, pending consent, flush deep link, OAuth **client** CRUD (from modal + ex-`gateway.ts`) |
| `apps/desktop/src/lib/api/app.ts` | `get_version`, `get_bundle_version` (from App / UpdateChecker) |
| `apps/desktop/src/lib/api/configExport.ts` | `preview_config_export`, `export_config_to_file`, `get_config_paths`, `check_config_exists`, `backup_existing_config` |
| `apps/desktop/src/hooks/useWorkspaceEvents.ts` | Workspace + session-override Tauri channels |
| `apps/desktop/src/hooks/useOAuthClientEvents.ts` | `oauth-client-changed` |
| `apps/desktop/src/hooks/useMetaToolEvents.ts` | `meta-tool-invoked` |

## Files to modify

| File | Change |
| ---- | ------ |
| [`apps/desktop/src/lib/api/gateway.ts`](../../apps/desktop/src/lib/api/gateway.ts) | Remove `exportConfig`, dead `connectServer` (unused); move OAuth client CRUD to `oauth.ts`; keep gateway lifecycle + `disconnectServer` only |
| [`apps/desktop/src/lib/api/registry.ts`](../../apps/desktop/src/lib/api/registry.ts) | Remove `listCategories` / ghost invoke |
| [`apps/desktop/src/lib/api/index.ts`](../../apps/desktop/src/lib/api/index.ts) | Barrel-export all API modules (`logs`, `sessionOverrides`, `serverClone`, `configExport`, new modules) |
| [`apps/desktop/src/lib/api/serverManager.ts`](../../apps/desktop/src/lib/api/serverManager.ts) | Document disconnect variants; remove `disconnectServerV2` if still uncalled after UI audit |
| [`apps/desktop/src/lib/api/metaTools.ts`](../../apps/desktop/src/lib/api/metaTools.ts) | Sole home for `respond_to_meta_tool_approval` (dialog imports this) |
| [`apps/desktop/src/lib/api/logs.ts`](../../apps/desktop/src/lib/api/logs.ts) | Sole home for `set_log_retention_days` / `get_log_retention_days` |
| [`apps/desktop/src/features/settings/SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx) | Use `lib/api/settings` + `lib/api/logs` — no direct invoke; remove duplicate log-retention invoke |
| [`apps/desktop/src/features/settings/UpdateChecker.tsx`](../../apps/desktop/src/features/settings/UpdateChecker.tsx) | Use `lib/api/app` |
| [`apps/desktop/src/components/OAuthConsentModal.tsx`](../../apps/desktop/src/components/OAuthConsentModal.tsx) | Use `lib/api/oauth` (`get_pending_consent`, `approve_oauth_consent`, `flush_pending_deep_link`) |
| [`apps/desktop/src/features/metaTools/MetaToolApprovalDialog.tsx`](../../apps/desktop/src/features/metaTools/MetaToolApprovalDialog.tsx) | Use `lib/api/metaTools.respondToMetaToolApproval` only — delete inline invoke |
| [`apps/desktop/src/features/metaTools/MetaToolAuditLog.tsx`](../../apps/desktop/src/features/metaTools/MetaToolAuditLog.tsx) | Use `useMetaToolEvents` hook — no direct `listen('meta-tool-invoked')` |
| [`apps/desktop/src/features/clients/ClientsPage.tsx`](../../apps/desktop/src/features/clients/ClientsPage.tsx) | Use `useOAuthClientEvents` — no direct `listen('oauth-client-changed')` |
| [`apps/desktop/src/App.tsx`](../../apps/desktop/src/App.tsx) | Use `lib/api/app.getVersion` |
| [`apps/desktop/src/stores/registryStore.ts`](../../apps/desktop/src/stores/registryStore.ts) | Remove dead `toggleServer` / `set_server_enabled` path |
| [`apps/desktop/src/hooks/useDomainEvents.ts`](../../apps/desktop/src/hooks/useDomainEvents.ts) | Rename `grants-changed` → `client-grant-changed`; fix `useClientEvents` |
| [`apps/desktop/src/hooks/useWorkspaceEvents.ts`](../../apps/desktop/src/hooks/useWorkspaceEvents.ts) | **New** — workspace/session-override channels |
| [`apps/desktop/src/hooks/useOAuthClientEvents.ts`](../../apps/desktop/src/hooks/useOAuthClientEvents.ts) | **New** — `oauth-client-changed` |
| [`apps/desktop/src/hooks/useMetaToolEvents.ts`](../../apps/desktop/src/hooks/useMetaToolEvents.ts) | **New** — `meta-tool-invoked` |
| [`apps/desktop/src/features/workspaces/WorkspacesPage.tsx`](../../apps/desktop/src/features/workspaces/WorkspacesPage.tsx) | Remove dead listeners; use `useWorkspaceEvents` |
| [`apps/desktop/src-tauri/src/commands/feature_members.rs`](../../apps/desktop/src-tauri/src/commands/feature_members.rs) | Module doc: deferred — no FE wrapper; batch APIs preferred |
| [`docs/guide/gateway.mdx`](../guide/gateway.mdx) | Depends on this doc; SSE channel count → 16 |
| [`docs/planning/web-admin-parity-matrix.md`](./web-admin-parity-matrix.md) | Re-scan invokes + channels; mark E2E-only / deferred rows |

## Files to delete

| File | Condition |
| ---- | --------- |
| [`apps/desktop/src/lib/api/featureMembers.ts`](../../apps/desktop/src/lib/api/featureMembers.ts) | After confirming zero imports (currently true) |

---

## Phasing

Five phases. Each phase ends with **`pnpm validate`** green and relevant WDIO smoke unchanged unless a spec is updated for intentional behavior change.

### Phase 1 — Broken invokes + dead exports

**Effort:** ~0.5 day

**Work**

- [ ] Remove `listCategories()` and `list_registry_categories` invoke from `registry.ts` (never called)
- [ ] Remove `exportConfig()` / `export_config` from `gateway.ts` (never imported; wrong command name)
- [ ] Remove `connectServer()` from `gateway.ts` if grep confirms zero call sites (dead wrapper; `connect_server` stays registered for E2E/internal use)
- [ ] Add `configExport.ts` with correct backend commands — no UI wiring unless export UI exists
- [ ] Grep confirm no remaining references to removed symbols
- [ ] Invoke audit script: FE command list ⊆ `lib.rs` handler list

**Outcome:** Every `invoke('…')` in `apps/desktop/src` targets a registered Tauri command (verified by script matching FE invokes to `lib.rs` handler list). Parity matrix **Fix mismatch** rows drop to zero.

---

### Phase 2 — Server lifecycle call-site consolidation

**Effort:** ~1 day

**Work**

- [ ] Document in code (module-level comment in `serverManager.ts` + `gateway.ts`) when to use:
  - `enable_server_v2` / `disable_server_v2` — primary UI toggle + connection attempt
  - `disconnect_server` — logout / credential clear (ServersPage today)
  - `disconnect_server_v2` — pause without logout — **wire to UI or delete FE wrapper**
- [ ] Remove `disconnectServerV2` from `serverManager.ts` if no UI action maps to pause-without-logout; otherwise wire to explicit menu action
- [ ] Remove dead `connectServer` export (Phase 1) — ServersPage uses v2 enable + gateway `disconnectServer` only
- [ ] Remove `registryStore.toggleServer` + `set_server_enabled` FE path
- [ ] Audit `ServersPage` disconnect/logout flows — single import path per semantic
- [ ] Manual smoke: enable → connect → disconnect → logout on one OAuth server

**Outcome:** One documented path per user action on My Servers. No orphaned v1/v2 wrappers in `lib/api/`. Parity matrix server-manager rows have unambiguous HTTP semantics for web admin.

---

### Phase 3 — API layer consolidation (all invokes through `lib/api/`)

**Effort:** ~1.5 days

**Work**

- [ ] Add `settings.ts`, `oauth.ts`, `app.ts`, `configExport.ts` modules
- [ ] Refactor `SettingsPage` — use `settings.ts` + `logs.ts` only; **remove duplicate `set_log_retention_days` invoke**
- [ ] Refactor `UpdateChecker`, `OAuthConsentModal`, `MetaToolApprovalDialog`, `App.tsx` — no direct invoke
- [ ] `MetaToolApprovalDialog` → `metaTools.respondToMetaToolApproval` (eliminate duplicate invoke path)
- [ ] Split OAuth client methods from `gateway.ts` → `oauth.ts`
- [ ] Update `lib/api/index.ts` barrel exports (include `logs`, `sessionOverrides`, `serverClone`, all new modules)
- [ ] Delete `featureMembers.ts`; add Rust module doc on `feature_members.rs` (Deferred, no FE)
- [ ] Optional CI grep: fail on `invoke(` outside `lib/api/**` and `main.tsx`

**Outcome:** `rg 'invoke\\(' apps/desktop/src` hits only `lib/api/**` and `main.tsx`. Web admin transport refactor becomes a mechanical `apiCall` swap in one directory tree.

---

### Phase 4 — Event channel audit + hook alignment

**Effort:** ~1 day

**Work**

- [ ] Inventory every `listen('…')` in `apps/desktop/src` — map to Rust emitter (EventBus bridge vs direct emit)
- [ ] Fix `useDomainEvents`: `grants-changed` → `client-grant-changed`; fix `GrantsChangedPayload` / `useClientEvents`
- [ ] Add `useWorkspaceEvents`, `useOAuthClientEvents`, `useMetaToolEvents` (see Target event hooks table)
- [ ] Migrate `WorkspacesPage`, `ClientsPage`, `MetaToolAuditLog` off direct `listen()`
- [ ] Remove dead WorkspacesPage listeners: `workspace-appearance-changed`, `server-status`
- [ ] Verify appearance edits still refresh via `workspace-binding-changed`
- [ ] Document dual emit paths in [`gateway.rs`](../../apps/desktop/src-tauri/src/commands/gateway.rs) module comment (SSE fan-in note)
- [ ] Update web-admin Phase 5 SSE list → **16 channels** + note dual Rust emit sources
- [ ] Update parity matrix SSE + deferred rows

**Outcome:** Subscribing via hooks covers every live UI refresh path. No documented channel name mismatches between Rust emit and TS listen. Manual smoke: edit workspace binding, session override, OAuth client grant — each updates UI without navigation.

---

### Phase 5 — Verification + web admin unblock

**Effort:** ~0.5 day

**Work**

- [x] Re-run invoke + `listen` scans → update parity matrix (all audit rows ✅ or explicitly Deferred)
- [x] Complete [Audit checklist](#audit-checklist-may-25-2026) sign-off column
- [x] `pnpm validate` — Rust fmt/clippy/check + desktop lint pass; `@mcpmux/ui` HoverTooltip pre-existing failure documented
- [ ] WDIO smoke: `spaces`, `server-lifecycle`, `workspaces`, `clients`, `gateway` specs — skipped (`MCPMUX_REGISTRY_URL` unset)
- [x] Update [gateway guide](../guide/gateway.mdx): **Depends on** this doc complete; Phase 1 matrix scaffolding can start
- [x] Mark this doc **Status: Complete** with branch + date

**Outcome:** Web admin implementation can start on `feat/web-admin` with a trustworthy IPC/event contract. Parity matrix has zero fix-mismatch rows and complete SSE channel list.

---

## Pre-PR validation

| Step | Command | When |
| ---- | ------- | ---- |
| Format + lint + types | `pnpm validate` | Every phase |
| Rust tests | `pnpm test:rust` | Phases 2–4 if Rust touched |
| TS tests | `pnpm test:ts` | Phase 3+ if API modules affect vitest targets |
| Desktop E2E smoke | `pnpm test:e2e:grep -- "space\|server\|workspace\|client\|gateway"` | Phase 5 (or subset per phase) |
| Invoke audit | `rg --no-filename -o "invoke(?:<[^>]*>)?\\(\\s*['\\\"]([a-z0-9_]+)['\\\"]" apps/desktop/src \| sort -u` vs `lib.rs` handlers | Phase 1 + Phase 5 |

---

## Key files referenced

| File | Why |
| ---- | --- |
| [`docs/planning/web-admin-parity-matrix.md`](./web-admin-parity-matrix.md) | Invoke inventory + anomaly list that triggered this doc |
| [`apps/desktop/src/lib/api/gateway.ts`](../../apps/desktop/src/lib/api/gateway.ts) | Broken export + OAuth/gateway mix |
| [`apps/desktop/src/lib/api/registry.ts`](../../apps/desktop/src/lib/api/registry.ts) | Ghost `list_registry_categories` |
| [`apps/desktop/src/hooks/useDomainEvents.ts`](../../apps/desktop/src/hooks/useDomainEvents.ts) | `grants-changed` mismatch |
| [`apps/desktop/src-tauri/src/commands/gateway.rs`](../../apps/desktop/src-tauri/src/commands/gateway.rs) | DomainEvent → Tauri channel mapping (source of truth) |
| [`apps/desktop/src/features/workspaces/WorkspacesPage.tsx`](../../apps/desktop/src/features/workspaces/WorkspacesPage.tsx) | Dead event listeners |
| [`apps/desktop/src/stores/registryStore.ts`](../../apps/desktop/src/stores/registryStore.ts) | Legacy `set_server_enabled` / dead `toggleServer` |
| [`apps/desktop/src/features/clients/ClientsPage.tsx`](../../apps/desktop/src/features/clients/ClientsPage.tsx) | Direct `oauth-client-changed` listener |
| [`apps/desktop/src/features/metaTools/MetaToolAuditLog.tsx`](../../apps/desktop/src/features/metaTools/MetaToolAuditLog.tsx) | Direct `meta-tool-invoked` listener |
| [`apps/desktop/src/lib/api/serverManager.ts`](../../apps/desktop/src/lib/api/serverManager.ts) | Server event listeners (keep; reference pattern) |
| [`apps/desktop/src/main.tsx`](../../apps/desktop/src/main.tsx) | `__TAURI_TEST_API__` test harness |

---

## Related documentation

- [Web Admin Mode (Remote UI via HTTP)](../guide/gateway.mdx) — blocked until this cleanup ships
- [web-admin-parity-matrix.md](./web-admin-parity-matrix.md) — re-scan after cleanup
- [workspace-binding-icons.md](./workspace-binding-icons.md) — documents `workspace-binding-changed` reuse for appearance events

---

## Audit checklist (May 25, 2026)

Complete sign-off in Phase 5. Every row must be **Fixed**, **Removed**, or **Deferred** (with matrix note) before web admin starts.

### Bugs — broken or no-op today

| # | Finding | Resolution | Phase | Sign-off |
| - | ------- | ---------- | ----- | -------- |
| B1 | `list_registry_categories` — FE invoke, no Tauri handler | Remove `listCategories()` from `registry.ts` | 1 | ✅ Removed |
| B2 | `export_config` — FE invoke, backend has `export_config_to_file` | Remove `exportConfig()`; add `configExport.ts` with correct commands | 1 | ✅ Fixed |
| B3 | `grants-changed` — hook channel never emitted | Rename to `client-grant-changed` in `useDomainEvents` | 4 | ✅ Fixed |
| B4 | `workspace-appearance-changed` — listener, never emitted | Remove listener; rely on `workspace-binding-changed` | 4 | ✅ Fixed |
| B5 | `server-status` — listener, never emitted (real: `server-status-changed`) | Remove listener from WorkspacesPage | 4 | ✅ Fixed |

### Dead code — FE exports / paths never used

| # | Finding | Resolution | Phase | Sign-off |
| - | ------- | ---------- | ----- | -------- |
| D1 | `exportConfig()` — exported, never imported | Remove (same as B2) | 1 | ✅ Removed |
| D2 | `listCategories()` — exported, never called | Remove (same as B1) | 1 | ✅ Removed |
| D3 | `connectServer()` — exported, zero UI call sites | Remove from `gateway.ts` | 1 | ✅ Removed |
| D4 | `disconnectServerV2()` — exported, zero UI call sites | Remove or wire to explicit UI action | 2 | ✅ Removed |
| D5 | `registryStore.toggleServer` → `set_server_enabled` — never called from RegistryPage | Remove store action | 2 | ✅ Removed |
| D6 | `featureMembers.ts` — zero imports; overlaps `featureSets.ts` | Delete file | 3 | ✅ Removed |

### Anti-patterns — structural drift

| # | Finding | Resolution | Phase | Sign-off |
| - | ------- | ---------- | ----- | -------- |
| A1 | Dual enable: `set_server_enabled` vs `enable_server_v2` | UI standardizes on v2; remove legacy FE path (D5) | 2 | ✅ Fixed |
| A2 | Dual disconnect: `disconnect_server` vs `disconnect_server_v2` | Document both; FE uses gateway logout + optional v2 pause | 2 | ✅ Fixed |
| A3 | `gateway.ts` mixes gateway + OAuth client + export | Split OAuth → `oauth.ts`; export → `configExport.ts` | 3 | ✅ Fixed |
| A4 | Component-level `invoke()` (Settings, OAuth modal, MetaTool dialog, App) | Move to `lib/api/*` | 3 | ✅ Fixed |
| A5 | Duplicate `respond_to_meta_tool_approval` (dialog + `metaTools.ts`) | Dialog uses `metaTools` only | 3 | ✅ Fixed |
| A6 | Duplicate `set_log_retention_days` (SettingsPage + `logs.ts`) | SettingsPage imports `logs.ts` helper | 3 | ✅ Fixed |
| A7 | Duplicate `get_version` (App + UpdateChecker) | Both use `app.ts` | 3 | ✅ Fixed |
| A8 | Incomplete `lib/api/index.ts` barrel | Export all API modules | 3 | ✅ Fixed |
| A9 | Two backend feature-member command families (`feature_set` vs `feature_members`) | FE uses batch APIs only; Rust `feature_members` Deferred | 3 | ✅ Deferred |
| A10 | Page-level `listen()` bypassing hooks (Workspaces, Clients, MetaTool audit) | New event hooks; migrate pages | 4 | ✅ Fixed |
| A11 | `useDomainEvents` lists 10 channels; UI needs 16 | Extend hooks + channel registry table | 4 | ✅ Fixed |
| A12 | Dual Rust emit paths (EventBus bridge vs direct `app.emit`) | Document; SSE fan-in deferred to web admin | 4 / defer | ✅ Deferred |

### Deferred backend — registered, no FE invoke (parity matrix only)

| Command | Why deferred | Web admin note |
| ------- | ------------ | -------------- |
| `search_servers` | Registry search is client-side in `registryStore` | Add HTTP when server-side search is product-required |
| `preview_config_export`, `export_config_to_file`, `get_config_paths`, `check_config_exists`, `backup_existing_config` | No export UI wired | `configExport.ts` ready in Phase 1 |
| `generate_gateway_config` | No FE consumer | Gateway internal / future |
| `seed_server_features` | No FE consumer | Test/setup only |
| `add_feature_to_set`, `remove_feature_from_set`, `get_feature_set_members` | No FE consumer (batch APIs used) | Do not expose until granular editor exists |
| `approve_oauth_client` | E2E test mode only | Matrix: Desktop/E2E-only |
| `connect_server` | May remain for E2E after FE `connectServer` removed | Keep Tauri command; optional internal use |

### Explicitly out of scope (tracked elsewhere)

| # | Finding | Where tracked |
| - | ------- | ------------- |
| O1 | `window.__TAURI_TEST_API__` always exposed in `main.tsx` | Out of scope table |
| O2 | Playwright mocks Tauri — not real parity | [gateway guide](../guide/gateway.mdx) |
| O3 | WDIO as behavioral catalog for admin E2E | [gateway guide](../guide/gateway.mdx) Phase 8 |
| O4 | Merge EventBus + direct emit in Rust | Web admin SSE implementation |
| O5 | Desktop-only invokes (`open_space_config_file`, `add_to_vscode`, `flush_pending_deep_link`, …) | Parity matrix N/A rows — wrap in `lib/api` where applicable (Phase 3) |

### Phase 5 sign-off

| Check | Command / action | Status |
| ----- | ---------------- | ------ |
| All B* rows Fixed | Manual review | ✅ |
| All D* rows Removed or wired | `rg` dead symbol check | ✅ |
| All A* rows addressed | Manual review | ✅ |
| Deferred commands documented | Parity matrix updated | ✅ |
| Invoke audit clean | FE invokes ⊆ `lib.rs` handlers (115/115) | ✅ |
| Listen audit clean | `listen(` in hooks + `serverManager.ts` only | ✅ |
| CI green | `pnpm validate` | ⚠️ pre-existing `@mcpmux/ui` HoverTooltip lint (out of scope) |
| WDIO smoke | `pnpm test:e2e:grep -- "space\|server\|workspace\|client\|gateway"` | ⏭ skipped — `MCPMUX_REGISTRY_URL` unset |

---

## Reconciliation

This doc is the gate for web admin work. **Status: Complete** on branch `fix/pre-web-admin-cleanup` (May 25, 2026). [gateway guide](../guide/gateway.mdx) **Depends on** updated to allow Phase 1 matrix scaffolding.

**Decision record (May 25, 2026):** Parity matrix scan found broken invokes, dual server stacks, scattered IPC, and incomplete event channel documentation. User chose to fix desktop contract before any admin HTTP implementation — avoids building REST on top of latent bugs.

**Decision record (May 25, 2026 — audit appendix):** Added full audit checklist (B1–B5 bugs, D1–D6 dead code, A1–A12 anti-patterns, deferred backend table, O1–O5 out-of-scope) so every scan finding maps to a phase or explicit deferral — 100% coverage gate for Phase 5.
