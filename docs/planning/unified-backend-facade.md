# Unified Backend Facade (Option 4A)

**Last Updated:** May 26, 2026  
**Status:** Planning — **partial prep on `feat/web-ui`** (facade tree not started; route split + shell behaviors landed pre-merge)  
**Branch:** TBD — suggest `feat/backend-facade` off **`dev`** (after [`feat/web-ui`](./web-admin-remote-access.md) merges or rebases)
**Base branch:** `dev`
**Issue:** TBD
**Depends on:** [Web Admin Mode](./web-admin-remote-access.md) — transport + `command_bridge` + admin server landed on `feat/web-ui`; this doc hardens the **frontend boundary** so web/desktop regressions stop
**Related brainstorm:** Option 4 from session (May 26, 2026) — thin facade, not namespaced rewrite

---

## Problem

Web admin introduced a correct split: **domain commands** go through `apiCall()` (`invoke` in Tauri, `fetch` to `/api/v1` in browser), while **events** and **OS integrations** still use Tauri JS APIs directly.

That produced real bugs in the browser:

- `listen()` / `transformCallback` crashes when components or dual hooks touched `@tauri-apps/api/event` outside Tauri
- Scattered `isTauri()` guards in pages, hooks, and `App.tsx` instead of one enforceable boundary
- Stragglers still calling `invoke()` in `lib/api` (`configExport`, `registry.set_server_enabled`, mixed `oauth` paths, `settings` admin commands)
- Dev confusion: `:1420` (Vite) vs `:45819` (admin) — data path works only when admin is on + proxy configured

**This is not “we need a second API.”** Data is ~already unified via `apiCall()`. The gap is **discipline and surface area**: UI must not import `@tauri-apps/*`; events and shell need explicit modules with documented web behavior.

### Pre-facade work on `feat/web-ui` (May 26, 2026)

Post-review commits did **not** create `lib/backend/` yet, but they reduce Phase 1/3/4 scope:

| Item | Status | Notes |
| ---- | ------ | ----- |
| Split `fetch-api.ts` route map | ✅ Done | `lib/api/fetch-api.routes/*` + helpers/types; transport core ~180 lines |
| `open_url` parity | ✅ Done | REST endpoint removed; `gateway.ts` branches on `isTauri()` |
| Admin settings web UX | ✅ Partial | `SettingsPage` hides admin card when `!isTauri()` — still invoke-only |
| Live gateway integration test | ✅ Done | `LiveGatewayRuntime` + `admin_api_live_gateway.rs` |
| ESLint `@tauri-apps` boundary | ⬜ Not started | Phase 1 deliverable |
| `lib/backend/` scaffold | ⬜ Not started | Phase 1 deliverable |

**Estimated remaining effort after merge:** ~2–2.5 days (down from ~3 days).

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Approach | **Option 4A — thin facade** | Re-export + relocate; **no** `backend.spaces.list()` rename across 40+ files |
| 2 | Not chosen | **Option 2 — always HTTP in Tauri** | Admin would be mandatory; extra latency; fights optional admin + desktop IPC consent model |
| 3 | Not chosen | **Option 5 — Tauri webview loads only from admin origin** | Large product/ dev-HMR change; defer |
| 4 | Command transport | **Keep `apiCall()` in `lib/backend/transport.ts`** (move from `lib/api/transport.ts`) | Single runtime branch (`invoke` vs `fetch`); unchanged semantics |
| 5 | Events | **Single export: `backend.events`** — one hook surface, Tauri + SSE adapters inside | Eliminates dual-hook pattern that called `listen()` in browser |
| 6 | OS / desktop shell | **Single export: `backend.shell`** — invoke-only helpers with typed “unsupported on web” no-ops or hidden UI | File dialogs, updater, `convertFileSrc`, client install, deep links cannot be HTTP |
| 7 | Page imports | **ESLint: no `@tauri-apps/*` outside `apps/desktop/src/lib/backend/**`** | Enforcement beats documentation |
| 8 | `lib/api/*` migration | **Re-export through `lib/backend/index.ts` first**; keep `@/lib/api/*` paths as re-exports during transition (optional deprecation comment) | Zero forced churn in feature pages on day one |
| 9 | Admin settings commands | **`backend.shell` or `backend.admin`** — `get_admin_web_settings` / `update_admin_web_settings` stay invoke-only | Control plane for starting HTTP server; not exposed over HTTP to remote browser |
| 10 | Timing | **After `feat/web-ui` stabilizes** | Avoid import wars while HTTP parity is still moving |

---

## What this is / is not

| In scope | Out of scope |
| -------- | ------------- |
| Consolidate Tauri touchpoints under `lib/backend/` | Rename every `listSpaces()` call site to namespaced API |
| Collapse event hooks to one public surface | Merge MCP gateway (`:45818`) and admin (`:45819`) into one server |
| ESLint boundary + fix ~10–20 direct `@tauri-apps` call sites | Always-on admin server for desktop (Option 5) |
| Finish `apiCall` stragglers in moved `lib/api` modules | New HTTP routes for file picker / native dialogs |
| Document web behavior for `shell.*` (hide vs no-op vs error toast) | Dual compile-time bundles (Option 3) unless needed later |

---

## Architecture

### Target import graph

```text
features/, components/, hooks/ (non-backend)
        │
        ▼
  lib/backend/index.ts          ← only public entry (preferred)
        │
        ├── data/               ← today’s lib/api/* (all use apiCall)
        │     transport.ts      ← isTauri() + invoke vs fetchApi
        │     fetch-api.ts      ← HTTP transport (CSRF, retry)
        │     fetch-api.routes/ ← per-resource routeFor switches
        │     spaces.ts, gateway.ts, …
        │
        ├── events/             ← useDomainEvents (+ web SSE) unified export
        │     subscribe.ts
        │
        └── shell/              ← desktop-only; never imported for data
              dialogs.ts
              updater.ts
              icons.ts          ← convertFileSrc wrapper
              client-install.ts
              admin-settings.ts
```

### Three channels (unchanged wires, clearer ownership)

| Channel | Desktop | Web admin | Facade module |
| ------- | ------- | --------- | ------------- |
| Commands | `invoke` → Tauri → `command_bridge` | `fetch` → admin → `command_bridge` | `backend.data.*` via `apiCall` |
| Live updates | `listen` | SSE `/api/v1/events` | `backend.events` |
| OS / control plane | `invoke` only | N/A (hide or message) | `backend.shell` |

---

## Files to create

| File | Purpose |
| ---- | ------- |
| [`apps/desktop/src/lib/backend/index.ts`](../../apps/desktop/src/lib/backend/index.ts) | Public facade: re-export `data`, `events`, `shell` |
| [`apps/desktop/src/lib/backend/transport.ts`](../../apps/desktop/src/lib/backend/transport.ts) | Move from `lib/api/transport.ts` |
| [`apps/desktop/src/lib/backend/events/index.ts`](../../apps/desktop/src/lib/backend/events/index.ts) | Single `useBackendEvents()` (or keep `useDomainEvents` name as alias) |
| [`apps/desktop/src/lib/backend/shell/index.ts`](../../apps/desktop/src/lib/backend/shell/index.ts) | Barrel for shell helpers |
| [`apps/desktop/src/lib/backend/shell/dialogs.ts`](../../apps/desktop/src/lib/backend/shell/dialogs.ts) | `@tauri-apps/plugin-dialog` wrappers |
| [`apps/desktop/src/lib/backend/shell/updater.ts`](../../apps/desktop/src/lib/backend/shell/updater.ts) | Updater check + relaunch (from `App.tsx`, `UpdateChecker`) |
| [`apps/desktop/src/lib/backend/shell/icons.ts`](../../apps/desktop/src/lib/backend/shell/icons.ts) | `convertFileSrc` + `resolveWorkspaceIconPath` for web fallback |
| [`apps/desktop/src/lib/backend/shell/client-install.ts`](../../apps/desktop/src/lib/backend/shell/client-install.ts) | Move from `lib/api/clientInstall.ts` |
| [`apps/desktop/src/lib/backend/shell/admin-settings.ts`](../../apps/desktop/src/lib/backend/shell/admin-settings.ts) | `get_admin_web_settings` / `update_admin_web_settings` |
| [`apps/desktop/eslint.config.js`](../../apps/desktop/eslint.config.js) or root ESLint | Rule: restrict `@tauri-apps/*` imports to `lib/backend/**` |

## Files to modify (direct `@tauri-apps` today — must move or gate)

| File | Change |
| ---- | ------ |
| [`apps/desktop/src/App.tsx`](../../apps/desktop/src/App.tsx) | Updater → `backend.shell.updater` |
| [`apps/desktop/src/main.tsx`](../../apps/desktop/src/main.tsx) | Test API expose only when `isTauri()` (already partial) |
| [`apps/desktop/src/components/ServerInstallModal.tsx`](../../apps/desktop/src/components/ServerInstallModal.tsx) | Deep link listen → `backend.events` or shell |
| [`apps/desktop/src/components/OAuthConsentModal.tsx`](../../apps/desktop/src/components/OAuthConsentModal.tsx) | Remaining `listen` → events facade |
| [`apps/desktop/src/components/ServerIcon.tsx`](../../apps/desktop/src/components/ServerIcon.tsx) | `convertFileSrc` → `backend.shell.icons` |
| [`apps/desktop/src/features/metaTools/MetaToolApprovalDialog.tsx`](../../apps/desktop/src/features/metaTools/MetaToolApprovalDialog.tsx) | `meta-tool-approval-request` listen → events/shell |
| [`apps/desktop/src/features/settings/UpdateChecker.tsx`](../../apps/desktop/src/features/settings/UpdateChecker.tsx) | Updater plugin → `backend.shell` |
| [`apps/desktop/src/features/servers/ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) | `plugin-dialog` → `backend.shell.dialogs` |
| [`apps/desktop/src/features/workspaces/WorkspacesPage.tsx`](../../apps/desktop/src/features/workspaces/WorkspacesPage.tsx) | `plugin-dialog` → `backend.shell.dialogs` |
| [`apps/desktop/src/lib/api/oauth.ts`](../../apps/desktop/src/lib/api/oauth.ts) | Remove dual `invoke`/`apiCall` branches where possible; `flush_pending_deep_link` → shell |
| [`apps/desktop/src/lib/api/configExport.ts`](../../apps/desktop/src/lib/api/configExport.ts) | `apiCall` for preview/export where HTTP exists; file path via shell |
| [`apps/desktop/src/lib/api/registry.ts`](../../apps/desktop/src/lib/api/registry.ts) | `set_server_enabled` → remove or `apiCall` |
| [`apps/desktop/src/lib/api/settings.ts`](../../apps/desktop/src/lib/api/settings.ts) | Admin settings → shell; rest stays data |
| [`apps/desktop/src/lib/api/gateway.ts`](../../apps/desktop/src/lib/api/gateway.ts) | `openUrl` already branches on `isTauri()` — move to `backend.shell` in Phase 3 |
| [`apps/desktop/src/hooks/useDomainEvents.ts`](../../apps/desktop/src/hooks/useDomainEvents.ts) | Move under `backend/events/`; ensure Tauri adapter no-ops when `!isTauri()` |
| [`apps/desktop/src/hooks/useWorkspaceEvents.ts`](../../apps/desktop/src/hooks/useWorkspaceEvents.ts) | Fold into events facade or re-export only from `backend/events` |
| [`apps/desktop/src/hooks/useOAuthClientEvents.ts`](../../apps/desktop/src/hooks/useOAuthClientEvents.ts) | Same |
| [`apps/desktop/src/hooks/useMetaToolEvents.ts`](../../apps/desktop/src/hooks/useMetaToolEvents.ts) | Same |
| [`apps/desktop/src/lib/tauri-events.ts`](../../apps/desktop/src/lib/tauri-events.ts) | Move to `backend/events/tauri.ts` (internal) |

## Files to modify (re-export shim — low risk)

| File | Change |
| ---- | ------ |
| All [`apps/desktop/src/lib/api/*.ts`](../../apps/desktop/src/lib/api/) | Update imports to `../backend/transport`; optional one-line re-export from `backend/data` |
| [`apps/desktop/src/hooks/index.ts`](../../apps/desktop/src/hooks/index.ts) | Export events from `backend/events` |
| [`apps/desktop/vite.config.ts`](../../apps/desktop/vite.config.ts) | Document proxy requirement in comment; optional `VITE_ADMIN_PORT` for proxy target |

**Estimated touch count:** ~**25–30 files** total; ~**10–12** behavioral moves off `@tauri-apps`; ~**15** import-path updates. **Not** 40+ page rewrites.

---

## Phasing

### Phase 1 — Scaffold + enforcement (~0.5 day)

**Work**

- [ ] Create `lib/backend/` tree and `index.ts` re-exporting existing `lib/api` modules
- [x] Split `fetch-api` route map into per-resource modules — **done on `feat/web-ui`; still at `lib/api/` until move**
- [ ] Move `transport.ts` + `fetch-api.ts` + `fetch-api.routes/` into `backend/`; shim `lib/api/transport.ts` re-exports
- [ ] Add ESLint `no-restricted-imports` for `@tauri-apps/*` outside `lib/backend/**`
- [ ] Document the three-channel model in this doc’s Architecture section (link from `AGENTS.md` one line)

**Outcome:** `pnpm lint` fails if a new component imports `@tauri-apps` directly. Existing app behavior unchanged.

---

### Phase 2 — Events facade (~1 day)

**Work**

- [ ] Add `backend/events/index.ts` — single hook used by app (`useDomainEvents` re-exported for compat)
- [ ] Merge `useDomainEventsWeb` / `useWorkspaceEventsWeb` / etc. as internal adapters only
- [ ] Remove duplicate `listen()` from `ClientsPage`, `WorkspaceBindingSheet`, `MetaToolApprovalDialog` (use facade)
- [ ] Delete or internalize `lib/tauri-events.ts` under `backend/events/`

**Outcome:** Open `localhost:1420` with admin enabled — **no** `transformCallback` errors; SSE subscriptions work via one code path.

---

### Phase 3 — Shell facade (~1 day)

**Work**

- [ ] Implement `backend.shell` modules (dialogs, updater, icons, client-install, admin-settings)
- [ ] Migrate `App.tsx`, `UpdateChecker`, `ServersPage`, `WorkspacesPage`, `ServerIcon`, `ServerInstallModal`
- [x] Web: hide admin settings card when `!isTauri()` — **done on `feat/web-ui`**
- [ ] Web: hide or disable remaining shell-only UI (Connect IDE install, open logs folder, updater banner)
- [ ] Move `openUrl` (`gateway.ts`) into `backend.shell`

**Outcome:** Grep `apps/desktop/src` for `@tauri-apps` — hits only under `lib/backend/**`.

---

### Phase 4 — Data stragglers (~0.5 day)

**Work**

- [ ] `configExport.ts` — `apiCall` for HTTP-backed ops; shell for native save path
- [ ] `registry.ts` — remove dead `set_server_enabled` invoke
- [ ] `oauth.ts` — single path per command via `apiCall`; shell for `flush_pending_deep_link`
- [ ] `settings.ts` — admin settings via shell only
- [x] Remove fake `open_url` REST route — **done on `feat/web-ui`**

**Outcome:** Every command in parity matrix that is “REST” uses `apiCall` only; desktop-only rows call `shell` only.

---

### Phase 5 — Optional cleanup (~0.5 day)

**Work**

- [ ] Deprecation comments on `@/lib/api/*` → prefer `@/lib/backend`
- [ ] Update [`web-admin-remote-access.md`](./web-admin-remote-access.md) frontend section to reference facade
- [ ] Narrow `useDataSync` / `AutoStartConflictResolver` imports to `backend.data`

**Outcome:** New code defaults to `backend` import; planning docs aligned with implementation.

---

## Pre-PR validation

| Step | Command |
| ---- | ------- |
| Lint | `pnpm lint` (ESLint rule active) |
| Typecheck | `pnpm typecheck` |
| Desktop smoke | `pnpm dev` — Tauri window: spaces, servers, gateway, settings |
| Web smoke | Enable admin → `pnpm build:web:admin` → `http://127.0.0.1:45819` + hard refresh `localhost:1420` |
| Console | Browser devtools: zero `transformCallback` / `invoke` errors on dashboard load |

---

## Comparison to “do nothing” (Option 1 only)

| | Option 1 (lint + fixes only) | Option 4A (this doc) |
| - | ----------------------------- | --------------------- |
| Effort | ~0.5 day | ~3 days |
| Regression risk | Medium — new `listen()` in components | Low — ESLint blocks it |
| Mental model | “Remember to use apiCall” | “Import backend; only three submodules” |
| Web admin long-term | Works if disciplined | Works by construction |

---

## Key files referenced

| File | Why |
| ---- | --- |
| [`apps/desktop/src/lib/api/transport.ts`](../../apps/desktop/src/lib/api/transport.ts) | Existing unified command switch |
| [`apps/desktop/src/lib/api/fetch-api.routes/`](../../apps/desktop/src/lib/api/fetch-api.routes/) | Per-resource route map (split pre-facade on `feat/web-ui`) |
| [`apps/desktop/src/hooks/useDomainEvents.ts`](../../apps/desktop/src/hooks/useDomainEvents.ts) | Dual Tauri/web hooks — primary consolidation target |
| [`docs/planning/pr-2-web-admin-code-review.md`](./pr-2-web-admin-code-review.md) | Post-review remediation status — what landed before facade work |
| [`docs/planning/web-admin-remote-access.md`](./web-admin-remote-access.md) | Parent feature — `command_bridge` + admin HTTP |
| [`docs/planning/web-admin-parity-matrix.md`](./web-admin-parity-matrix.md) | Desktop-only vs REST rows inform `shell` vs `data` |

---

## Related documentation

- [Web Admin Mode (Remote UI via HTTP)](./web-admin-remote-access.md)
- [Web Admin Parity Matrix](./web-admin-parity-matrix.md)
- [Pre–Web Admin Desktop Cleanup](./pre-web-admin-desktop-cleanup.md)
- [`AGENTS.md`](../../AGENTS.md) — add one-line pointer after implementation

---

## Reconciliation

Update **Status** and **Branch** when work starts. Do not start until `feat/web-ui` merge path is clear — this doc is frontend-only and should rebase on the branch that contains `apiCall` + admin server.

**May 26, 2026:** `feat/web-ui` post-review commits (`0c1a017`, `cc7bf54`, `558a319`) completed fetch-api route split, `open_url` shell parity, admin settings web hide, and live gateway tests. Facade Phase 1 ESLint + `lib/backend/` tree remain the first tasks on `feat/backend-facade`.
