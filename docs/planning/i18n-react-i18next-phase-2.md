# i18n — Phase 2: Remaining Desktop String Extraction

**Last updated:** Jun 23, 2026  
**Status:** Shipped (Phases 5–9 complete on `i18n`)  
**Branch:** `i18n` (off `dev`)  
**Base branch:** `dev`  
**Depends on:** [`i18n-react-i18next.md`](./i18n-react-i18next.md) Phases 1–4 (infra, major pages, shared components, E2E testids)  
**Estimated effort:** ~12–16 hours across 6 phases  

---

## Problem

Phase 1 landed i18next infra, all seven top-level `*Page.tsx` files, nav/chrome dedup, and a first wave of shared components (`SourceBadge`, `ConnectionCard`, `OAuthConsentModal`, `DashboardPage`, installed `ServerCard`). **~22 TSX files and 3 helper modules still render hardcoded English**, and several migrated files leak inline copy (notably `ServersPage` config-modal sections duplicating keys already in `servers.json`).

A repo sweep (Jun 23, 2026) found **~200+ distinct bare strings** still in JSX, confirm dialogs, toasts, tooltips, and helper-fed labels. The worst offenders are settings children mounted inside an already-migrated `SettingsPage`, registry CTAs via unmigrated `Contribute.tsx`, and gateway confirm copy in `useGatewayControl.tsx`.

Until Phase 2 completes, a nav rename is safe but **any modal-only or settings-subpanel copy change still requires grep across multiple files**.

---

## Decisions

| # | Decision | Choice | Rationale |
|---|----------|--------|-----------|
| 1 | Scope | **100% of `apps/desktop/src` user-visible strings** | Original plan goal; Phase 1 proved the pattern — finish it |
| 2 | New namespace | Add `metatools.json` for meta-tool approval/grants/audit UI | Keeps `settings.json` from growing further; meta-tool copy is a cohesive domain |
| 3 | Existing JSON reuse | **Wire before invent** — `Contribute.tsx` → `settings.contribute.*`; `ServerLogViewer` → `settings.logs.*`; `ServersPage` config tail → `servers.configModal.*` | Keys already exist from Phase 2/3 page work; avoid duplicate strings |
| 4 | Helper pattern | `*.helpers.ts` display functions accept `TFunction<'ns'>` (same as `source-badge.helpers.ts`) | Callers pass `t` from `useTranslation`; non-React sites use `i18n.getFixedT('ns')` |
| 5 | `lib/api/serverManager.ts` | Refactor `getConnectButtonLabel()` to accept `t` or return keys | Today returns English literals; any caller still using it bypasses i18n |
| 6 | `packages/ui` | **Still no i18next inside UI package** | Callers pass `cancelLabel` / `confirmLabel` (started in Phase 1); English defaults remain fallbacks only |
| 7 | Vitest | Extend `renderWithI18n` usage to any component test asserting display copy | Established in `tests/ts/render-with-i18n.helpers.tsx` |
| 8 | E2E | **No copy assertions** — run full `pnpm test:e2e:web` once at end | Testids already migrated in Phase 4; renames must not break suite |
| 9 | Locale switcher | Still deferred | English-only; architecture ready for `locales/fr/` later |
| 10 | Rust / IPC errors | Still deferred | User-facing errors from Tauri commands are a separate epic (gateway error localization) |
| 11 | `tests/` fixture copy | Still deferred | Intentional test data, not product UI |

---

## Scope

**In:**

- All unmigrated TSX under `apps/desktop/src/components/` and `apps/desktop/src/features/`
- Partially migrated files: `App.tsx`, `ServersPage.tsx`, `DashboardStatCards.tsx`
- Helper modules feeding UI: `servers-page.helpers.ts`, `dashboard.helpers.ts`, `getConnectButtonLabel` in `serverManager.ts`
- New locale file `metatools.json` + `i18n.ts` / `i18n.types.ts` registration
- Unit test updates for migrated components
- Final grep gate + `pnpm validate` + full web E2E

**Out:**

| Item | Reason |
|------|--------|
| Second language / locale picker UI | YAGNI — add `locales/<lang>/` folders only when product asks |
| `packages/ui` internal `useTranslation` | Prop-driven package; desktop owns all copy |
| Rust/Tauri command error strings | Different surface (IPC JSON errors); track as `i18n-backend-errors` if needed |
| `tests/e2e` / `tests/ts` assertion strings | Fixture copy, not UI registry |
| `lib/contribute.ts` URL constants | URLs stay centralized; only **display** labels move to JSON |

---

## Architecture

### Namespace additions

| Namespace | File | Covers |
|-----------|------|--------|
| `metatools` | `metatools.json` *(new)* | `MetaToolApprovalDialog`, `MetaToolGrantsPanel`, `MetaToolAuditLog` |
| `common` | `common.json` *(extend)* | `SpaceSwitcher`, `StaleBuildBanner`, shared gateway strings if not feature-specific |
| `dashboard` | `dashboard.json` *(extend)* | `DashboardServerHealth`, `DashboardRecentActivity`, `App.tsx` statusbar/update banner |
| `servers` | `servers.json` *(extend)* | Modals, filters, clone/uninstall dialogs, `servers-page.helpers.ts` labels |
| `settings` | `settings.json` *(extend)* | `UpdateChecker`, `AboutSection`, `BuildStampPanel` — mostly keys already present |
| `spaces` | `spaces.json` *(extend)* | `SpacePanel` CRUD + confirm copy |
| `clients` | `clients.json` *(extend)* | `useGatewayControl` port-conflict confirms (gateway is client-connection surface) |
| `registry` | `registry.json` *(extend)* | `Contribute.tsx` / `RequestServerCTA` if not reusing `settings.contribute` |

### Helper injection (canonical)

```ts
// servers-page.helpers.ts
export function formatServerCountSummary(
  t: TFunction<'servers'>,
  summary: ServerCountSummary,
): string {
  return t('countSummary.inline', summary);
}
```

```tsx
// ServersCountSummary.tsx
const { t } = useTranslation('servers');
const text = formatServerCountSummary(t, summary);
```

### Done criteria (global)

```bash
# From apps/desktop/src — should return only dynamic/computed values, not product copy
rg '>[^<{]*[A-Za-z]{4,}[^<{]*<' --glob '*.tsx' apps/desktop/src
rg 'title="[A-Z]' --glob '*.tsx' apps/desktop/src
rg 'placeholder="[A-Z]' --glob '*.tsx' apps/desktop/src
```

Manual review of hits; allowlist: brand names (`McpMux`), emoji, server names from API, Monaco/schema labels.

---

## Files to create

| Path | Purpose |
|------|---------|
| [`apps/desktop/src/locales/en/metatools.json`](../../apps/desktop/src/locales/en/metatools.json) | Meta-tool approval, grants, audit copy |
| [`apps/desktop/src/locales/en/install.json`](../../apps/desktop/src/locales/en/install.json) | *(optional)* `ServerInstallModal` if `registry.json` gets crowded — prefer `registry.installModal.*` first |

## Files to modify (by area)

### Global shell

| Path | Change |
|------|--------|
| [`apps/desktop/src/components/SpaceSwitcher.tsx`](../../apps/desktop/src/components/SpaceSwitcher.tsx) | `useTranslation('common')` or new `spaces` keys for switcher chrome |
| [`apps/desktop/src/App.tsx`](../../apps/desktop/src/App.tsx) | Status bar gateway text, theme toggle titles, update banner |
| [`apps/desktop/src/features/gateway/useGatewayControl.tsx`](../../apps/desktop/src/features/gateway/useGatewayControl.tsx) | Port-conflict confirm dialogs → `clients` or `common` |
| [`apps/desktop/src/components/StaleBuildBanner.tsx`](../../apps/desktop/src/components/StaleBuildBanner.tsx) | Dev/build warning copy |

### Shared modals & viewers

| Path | Change |
|------|--------|
| [`apps/desktop/src/components/ServerInstallModal.tsx`](../../apps/desktop/src/components/ServerInstallModal.tsx) | → `registry` namespace |
| [`apps/desktop/src/components/ConfigEditorModal.tsx`](../../apps/desktop/src/components/ConfigEditorModal.tsx) | → `servers` or `common` |
| [`apps/desktop/src/components/ServerLogViewer.tsx`](../../apps/desktop/src/components/ServerLogViewer.tsx) | → `settings.logs.*` |
| [`apps/desktop/src/components/ServerDefinitionModal.tsx`](../../apps/desktop/src/components/ServerDefinitionModal.tsx) | → `registry` or `servers` |
| [`apps/desktop/src/components/Contribute.tsx`](../../apps/desktop/src/components/Contribute.tsx) | → `settings.contribute.*` (keys exist) |

### Servers feature

| Path | Change |
|------|--------|
| [`apps/desktop/src/features/servers/ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) | Finish config modal — use existing `servers.configModal.*` |
| [`apps/desktop/src/features/servers/CloneAccountModal.tsx`](../../apps/desktop/src/features/servers/CloneAccountModal.tsx) | → `servers.cloneModal.*` |
| [`apps/desktop/src/features/servers/UninstallSourceWithClonesDialog.tsx`](../../apps/desktop/src/features/servers/UninstallSourceWithClonesDialog.tsx) | → `servers.uninstallClones.*` |
| [`apps/desktop/src/features/servers/AddServerMenu.tsx`](../../apps/desktop/src/features/servers/AddServerMenu.tsx) | → `servers.addMenu.*` |
| [`apps/desktop/src/features/servers/ServersFiltersPopover.tsx`](../../apps/desktop/src/features/servers/ServersFiltersPopover.tsx) | → `servers.filters.*` |
| [`apps/desktop/src/features/servers/ServerEnabledToggle.tsx`](../../apps/desktop/src/features/servers/ServerEnabledToggle.tsx) | → `servers.actions.*` |
| [`apps/desktop/src/features/servers/ServersCountSummary.tsx`](../../apps/desktop/src/features/servers/ServersCountSummary.tsx) | → helpers + `t` |
| [`apps/desktop/src/features/servers/servers-page.helpers.ts`](../../apps/desktop/src/features/servers/servers-page.helpers.ts) | Filter labels, count summary, tooltip lines |

### Dashboard

| Path | Change |
|------|--------|
| [`apps/desktop/src/features/dashboard/DashboardServerHealth.tsx`](../../apps/desktop/src/features/dashboard/DashboardServerHealth.tsx) | → `dashboard.health.*` |
| [`apps/desktop/src/features/dashboard/DashboardRecentActivity.tsx`](../../apps/desktop/src/features/dashboard/DashboardRecentActivity.tsx) | → `dashboard.activity.*` |
| [`apps/desktop/src/features/dashboard/dashboard.helpers.ts`](../../apps/desktop/src/features/dashboard/dashboard.helpers.ts) | Attention detail strings |
| [`apps/desktop/src/features/dashboard/DashboardStatCards.tsx`](../../apps/desktop/src/features/dashboard/DashboardStatCards.tsx) | Replace hardcoded `None` with `common.none` |

### Settings & spaces children

| Path | Change |
|------|--------|
| [`apps/desktop/src/features/settings/UpdateChecker.tsx`](../../apps/desktop/src/features/settings/UpdateChecker.tsx) | → `settings.updates.*` (extend JSON) |
| [`apps/desktop/src/features/settings/AboutSection.tsx`](../../apps/desktop/src/features/settings/AboutSection.tsx) | → `settings.about.*` |
| [`apps/desktop/src/features/settings/BuildStampPanel.tsx`](../../apps/desktop/src/features/settings/BuildStampPanel.tsx) | → `settings.buildStamp.*` |
| [`apps/desktop/src/features/spaces/SpacePanel.tsx`](../../apps/desktop/src/features/spaces/SpacePanel.tsx) | → `spaces.panel.*`; pass `cancelLabel` on confirm |

### Meta tools

| Path | Change |
|------|--------|
| [`apps/desktop/src/features/metaTools/MetaToolApprovalDialog.tsx`](../../apps/desktop/src/features/metaTools/MetaToolApprovalDialog.tsx) | → `metatools.approval.*` |
| [`apps/desktop/src/features/metaTools/MetaToolGrantsPanel.tsx`](../../apps/desktop/src/features/metaTools/MetaToolGrantsPanel.tsx) | → `metatools.grants.*` |
| [`apps/desktop/src/features/metaTools/MetaToolAuditLog.tsx`](../../apps/desktop/src/features/metaTools/MetaToolAuditLog.tsx) | → `metatools.audit.*` |

### Infra & lib

| Path | Change |
|------|--------|
| [`apps/desktop/src/i18n.ts`](../../apps/desktop/src/i18n.ts) | Register `metatools` namespace |
| [`apps/desktop/src/i18n.types.ts`](../../apps/desktop/src/i18n.types.ts) | Augment `CustomTypeOptions` |
| [`apps/desktop/src/lib/api/serverManager.ts`](../../apps/desktop/src/lib/api/serverManager.ts) | `getConnectButtonLabel(t, …)` |

### Tests

| Path | Change |
|------|--------|
| `tests/ts/components/*.test.tsx` | `renderWithI18n` for any spec touching newly migrated components |
| [`tests/ts/vitest.config.ts`](../../tests/ts/vitest.config.ts) | Already has i18n aliases — no change expected |

---

## Phases

### Phase 5 — Global shell & gateway confirms (~2–3h)

- Migrate `SpaceSwitcher.tsx`, `App.tsx` (status bar, theme toggle, update banner), `StaleBuildBanner.tsx`
- Migrate `useGatewayControl.tsx` confirm dialogs; pass `cancelLabel` via `useConfirm` options
- Extend `dashboard.json` / `common.json` / `clients.json` as needed
- `pnpm typecheck` + spot-check: change `nav.json` label still updates sidebar + quick-links

**Outcome:** Opening the app shows zero hardcoded strings in the title bar, status bar, space switcher, or stale-build banner. Gateway port-conflict confirms display translated copy and honor `cancelLabel`.

---

### Phase 6 — Shared modals & registry CTAs (~3–4h)

- Migrate `ServerInstallModal`, `ConfigEditorModal`, `ServerDefinitionModal`, `ServerLogViewer`
- Wire `Contribute.tsx` / `RequestServerCTA` to existing `settings.contribute.*` (or `registry.contribute.*` if registry-context labels differ)
- Add/extend `registry.json`, `servers.json`, reuse `settings.logs.*` for log viewer
- Update component unit tests with `renderWithI18n`

**Outcome:** Install-from-registry deep link, custom JSON editor, server definition viewer, log viewer, and registry empty-state CTAs render entirely from JSON. `pnpm test:ts` green for touched test files.

---

### Phase 7 — Settings, spaces, and servers tail (~3–4h)

- Migrate `UpdateChecker`, `AboutSection`, `BuildStampPanel`, `SpacePanel`
- Migrate server modals/menus: `CloneAccountModal`, `UninstallSourceWithClonesDialog`, `AddServerMenu`, `ServersFiltersPopover`, `ServerEnabledToggle`, `ServersCountSummary`
- Finish `ServersPage.tsx` config-modal sections using existing `servers.configModal.*` keys
- Refactor `servers-page.helpers.ts` to accept `t`; update all call sites
- Refactor `getConnectButtonLabel()` in `serverManager.ts`

**Outcome:** Settings page sub-panels, space edit panel, and entire My Servers toolbar/filter/modal surface externalized. Server count summary and filter tooltips update when `servers.json` changes. No duplicate English in `ServersPage` config modal vs `servers.configModal`.

---

### Phase 8 — Dashboard widgets & meta tools (~2–3h)

- Migrate `DashboardServerHealth`, `DashboardRecentActivity`; refactor `dashboard.helpers.ts`
- Fix `DashboardStatCards` `None` → `common.none`
- Create `metatools.json`; migrate approval dialog, grants panel, audit log
- Register namespace in `i18n.ts` / `i18n.types.ts`

**Outcome:** Dashboard health and activity cards fully localized. Meta-tool approval flow (the highest-risk security UX) uses `metatools` namespace. TypeScript rejects unknown meta-tool keys.

---

### Phase 9 — Audit, tests, and E2E gate (~1–2h)

- Run ripgrep done-criteria (see Architecture); triage and fix stragglers
- `pnpm validate` + `pnpm test:ts` full suite
- `pnpm test:e2e:web` full run (not smoke-only)
- Update [`i18n-react-i18next.md`](./i18n-react-i18next.md) status → **Complete**
- Optional: add `pnpm lint:i18n` script that fails on `title="[A-Z]` in `apps/desktop/src` (CI guardrail)

**Outcome:** Grep audit returns no product copy in TSX. CI `ts-check` and full web E2E pass. Changing any string in any namespace JSON updates the UI without touching TSX. Phase 2 doc marked shipped.

---

## Inventory reference (unmigrated as of Jun 23, 2026)

| Severity | Files |
|----------|-------|
| **High** | `SpaceSwitcher`, `useGatewayControl`, `ServerInstallModal`, `ConfigEditorModal`, `ServerLogViewer`, `CloneAccountModal`, `UninstallSourceWithClonesDialog`, `MetaToolApprovalDialog`, `SpacePanel`, `UpdateChecker` |
| **Medium** | `DashboardServerHealth`, `DashboardRecentActivity`, `Contribute`, `StaleBuildBanner`, `AboutSection`, `BuildStampPanel`, `AddServerMenu`, `ServersFiltersPopover`, `ServerDefinitionModal`, meta-tool grants/audit |
| **Low** | `ServerEnabledToggle`, `ServersCountSummary`, aria/tooltip-only strings |
| **Leaky (migrated)** | `App.tsx`, `ServersPage.tsx`, `DashboardStatCards.tsx` |
| **Helpers** | `servers-page.helpers.ts`, `dashboard.helpers.ts`, `serverManager.getConnectButtonLabel` |

---

## Key files referenced

| Path | Notes |
|------|-------|
| [`docs/planning/i18n-react-i18next.md`](./i18n-react-i18next.md) | Phase 1 plan — infra, namespaces, E2E strategy |
| [`apps/desktop/src/i18n.ts`](../../apps/desktop/src/i18n.ts) | Namespace registration point |
| [`tests/ts/render-with-i18n.helpers.tsx`](../../tests/ts/render-with-i18n.helpers.tsx) | Vitest harness from Phase 1 hardening |
| [`apps/desktop/src/locales/en/settings.json`](../../apps/desktop/src/locales/en/settings.json) | `contribute.*` and `logs.*` already defined — wire Contribute + LogViewer |
| [`apps/desktop/src/locales/en/servers.json`](../../apps/desktop/src/locales/en/servers.json) | `configModal.*` complete — ServersPage must consume it |
| [`packages/ui/src/components/common/ConfirmDialog.tsx`](../../packages/ui/src/components/common/ConfirmDialog.tsx) | `cancelLabel` prop — callers must pass `t('common:actions.cancel')` |
| [`docs/planning/web-e2e-parity-handoff.md`](./web-e2e-parity-handoff.md) | E2E uses testids; full suite gate for Phase 9 |

---

## Related documentation

- [`i18n-react-i18next.md`](./i18n-react-i18next.md) — Phase 1 implementation plan (shipped)
- [`web-e2e-parity-handoff.md`](./web-e2e-parity-handoff.md) — Playwright web suite setup and smoke vs full run
- PR [`i18n` branch](https://github.com/crimsonsunset/mcp-mux/tree/i18n) — active implementation
