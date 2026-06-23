# i18n — react-i18next Full String Extraction

**Last updated:** Jun 23, 2026
**Status:** Planning
**Branch:** `i18n` (off `dev`)
**Base branch:** `dev`
**Depends on:** Nothing — purely additive; no Rust changes
**Trigger:** Renaming `FeatureSets`→`Bundles`, `Workspaces`→`Projects`, `Discover`→`Search` required touching 7+ files (App.tsx, DashboardQuickLinks.tsx, DashboardStatCards.tsx, FeatureSetsPage.tsx, WorkspacesPage.tsx, ClientsPage.tsx, Sidebar.tsx)

---

## Problem

All ~250–500 user-visible strings in `apps/desktop/src/` are hardcoded inline. There is no centralized string registry. `App.tsx` and `DashboardQuickLinks.tsx` independently maintain the same nav labels — they already drifted during the `ui-cleanup` branch. Future renames are a grep-and-pray exercise across 50 TSX files.

Three ad-hoc patterns exist today but none are consistent:
- `CONTRIBUTE` const in [`lib/contribute.ts`](../../apps/desktop/src/lib/contribute.ts) — "update from one place" comment
- `QUICK_LINKS` array in [`DashboardQuickLinks.tsx`](../../apps/desktop/src/features/dashboard/DashboardQuickLinks.tsx) — nav labels registry that mirrors `App.tsx`
- `*.helpers.ts` string functions — colocated but one-off

---

## Decisions

| # | Decision | Choice | Rationale |
|---|----------|--------|-----------|
| 1 | Library | `react-i18next` + `i18next` | Industry standard, React 19 compatible, multi-locale ready when needed without re-architecture |
| 2 | Locale loading | Static imports in `i18n.ts` (not HTTP backend) | Tauri + Vite SPA — no async fetch needed; works identically in `pnpm dev:web` and Tauri modes; no Suspense boundary required |
| 3 | Scope | All ~250–500 user-visible strings | Nav labels were the trigger; full coverage is the goal so partial extraction doesn't recreate the problem |
| 4 | Structure | `apps/desktop/src/locales/en/<namespace>.json` — one file per feature area | Keeps individual files manageable; mirrors how `*.helpers.ts` files are colocated by feature without losing the single-directory guarantee |
| 5 | TypeScript safety | Augment `i18next` `CustomTypeOptions` in `src/i18n.types.ts` | Full key autocomplete and compile-time errors on missing keys; no stringly-typed `t()` calls |
| 6 | `packages/ui` | Stays prop-driven; no internal i18next dependency | UI package is presentational; English string defaults stay as caller-supplied props; only `apps/desktop` consumes i18next |
| 7 | E2E tests | Migrate to `data-testid` selectors; do not update to new display strings | Testids are already stable and comprehensive; copy-based selectors silently break on every rename |

---

## Scope

**In:**
- `i18next` + `react-i18next` installed in `apps/desktop/`
- `src/i18n.ts` init + `src/locales/en/<ns>.json` namespace files (10 namespaces)
- `src/i18n.types.ts` TypeScript `CustomTypeOptions` augmentation
- All hardcoded user-visible strings extracted from `apps/desktop/src/` — page headings, nav labels, toasts, confirm dialogs, aria-labels, button copy, stat card titles, helper string functions
- E2E text-based locators migrated to `data-testid`

**Out:**

| Item | Reason |
|------|--------|
| Second language / locale switching UI | YAGNI — English only for now; architecture supports adding `locales/fr/` etc. without changes to this setup |
| `packages/ui` internal i18n wiring | Package is prop-driven; stays that way; no coupling to app's i18next instance |
| Rust/backend string extraction | Server-side error messages returned over IPC are out of scope |
| Pluralization / date/number formatting beyond current usage | No current need beyond English; extend per-namespace when needed |
| Extracting strings from `tests/` | Test copy is intentional fixture data, not UI copy |

---

## Architecture

### Namespace map

| Namespace | File | Covers |
|-----------|------|--------|
| `nav` | `nav.json` | Sidebar labels, section titles, tooltip values — shared between `App.tsx` and `DashboardQuickLinks.tsx` |
| `common` | `common.json` | Confirm, Cancel, Refresh, Save, Delete, shared empty/error states, loading copy |
| `dashboard` | `dashboard.json` | Stat card titles/descriptions, quick-link descriptions |
| `servers` | `servers.json` | ServersPage + ServerActionMenu (~43 strings) |
| `workspaces` | `workspaces.json` | WorkspacesPage + WorkspaceBindingSheet (~53 strings, heaviest file) |
| `featuresets` | `featuresets.json` | FeatureSetsPage + FeatureSetPanel |
| `clients` | `clients.json` | ClientsPage (~28 strings) |
| `settings` | `settings.json` | SettingsPage + ServerUpdatesSection + ServerPendingUpdatesList (~46 strings) |
| `spaces` | `spaces.json` | SpacesPage |
| `registry` | `registry.json` | RegistryPage + ServerCard + ServerDetailModal |

### Init shape (`src/i18n.ts`)

```ts
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import nav from './locales/en/nav.json';
import common from './locales/en/common.json';
// ... remaining namespaces

i18n.use(initReactI18next).init({
  lng: 'en',
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
  resources: {
    en: { nav, common, dashboard, servers, workspaces, featuresets, clients, settings, spaces, registry },
  },
});

export default i18n;
```

Imported at the top of [`main.tsx`](../../apps/desktop/src/main.tsx) before the React tree mounts. No `<Suspense>` needed.

### Key deduplication

`App.tsx` sidebar and `DashboardQuickLinks.tsx` both call `useTranslation('nav')` — one JSON key, zero drift between the two surfaces that caused the problem.

---

## Files to create

| Path | Purpose |
|------|---------|
| [`apps/desktop/src/i18n.ts`](../../apps/desktop/src/i18n.ts) | i18next init — static namespace imports, `initReactI18next` |
| [`apps/desktop/src/i18n.types.ts`](../../apps/desktop/src/i18n.types.ts) | `CustomTypeOptions` augmentation for all 10 namespaces |
| `apps/desktop/src/locales/en/nav.json` | Sidebar + nav copy |
| `apps/desktop/src/locales/en/common.json` | Shared chrome copy |
| `apps/desktop/src/locales/en/dashboard.json` | Dashboard copy |
| `apps/desktop/src/locales/en/servers.json` | Servers feature copy |
| `apps/desktop/src/locales/en/workspaces.json` | Workspaces/Projects copy |
| `apps/desktop/src/locales/en/featuresets.json` | Bundles/FeatureSets copy |
| `apps/desktop/src/locales/en/clients.json` | Clients copy |
| `apps/desktop/src/locales/en/settings.json` | Settings copy |
| `apps/desktop/src/locales/en/spaces.json` | Spaces copy |
| `apps/desktop/src/locales/en/registry.json` | Registry/Search copy |

## Files to modify

| Path | Change |
|------|--------|
| [`apps/desktop/package.json`](../../apps/desktop/package.json) | Add `i18next`, `react-i18next` |
| [`apps/desktop/src/main.tsx`](../../apps/desktop/src/main.tsx) | Import `./i18n` before app mount |
| [`apps/desktop/src/App.tsx`](../../apps/desktop/src/App.tsx) | `useTranslation('nav')` for sidebar |
| [`apps/desktop/src/features/dashboard/DashboardQuickLinks.tsx`](../../apps/desktop/src/features/dashboard/DashboardQuickLinks.tsx) | `useTranslation('nav')` — deduplicates with App.tsx |
| [`apps/desktop/src/features/dashboard/DashboardStatCards.tsx`](../../apps/desktop/src/features/dashboard/DashboardStatCards.tsx) | `useTranslation('dashboard')` |
| [`apps/desktop/src/features/featuresets/FeatureSetsPage.tsx`](../../apps/desktop/src/features/featuresets/FeatureSetsPage.tsx) | `useTranslation('featuresets')` |
| [`apps/desktop/src/features/featuresets/FeatureSetPanel.tsx`](../../apps/desktop/src/features/featuresets/FeatureSetPanel.tsx) | `useTranslation('featuresets')` |
| [`apps/desktop/src/features/workspaces/WorkspacesPage.tsx`](../../apps/desktop/src/features/workspaces/WorkspacesPage.tsx) | `useTranslation('workspaces')` |
| [`apps/desktop/src/features/workspaces/WorkspaceBindingSheet.tsx`](../../apps/desktop/src/features/workspaces/WorkspaceBindingSheet.tsx) | `useTranslation('workspaces')` |
| [`apps/desktop/src/features/clients/ClientsPage.tsx`](../../apps/desktop/src/features/clients/ClientsPage.tsx) | `useTranslation('clients')` |
| [`apps/desktop/src/features/servers/ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) | `useTranslation('servers')` |
| [`apps/desktop/src/features/servers/ServerActionMenu.tsx`](../../apps/desktop/src/features/servers/ServerActionMenu.tsx) | `useTranslation('servers')` |
| [`apps/desktop/src/features/settings/SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx) | `useTranslation('settings')` |
| [`apps/desktop/src/features/settings/ServerUpdatesSection.tsx`](../../apps/desktop/src/features/settings/ServerUpdatesSection.tsx) | `useTranslation('settings')` |
| [`apps/desktop/src/features/settings/ServerPendingUpdatesList.tsx`](../../apps/desktop/src/features/settings/ServerPendingUpdatesList.tsx) | `useTranslation('settings')` |
| [`apps/desktop/src/features/spaces/SpacesPage.tsx`](../../apps/desktop/src/features/spaces/SpacesPage.tsx) | `useTranslation('spaces')` |
| [`apps/desktop/src/features/registry/RegistryPage.tsx`](../../apps/desktop/src/features/registry/RegistryPage.tsx) | `useTranslation('registry')` |
| [`apps/desktop/src/features/registry/ServerCard.tsx`](../../apps/desktop/src/features/registry/ServerCard.tsx) | `useTranslation('registry')` |
| [`apps/desktop/src/features/registry/ServerDetailModal.tsx`](../../apps/desktop/src/features/registry/ServerDetailModal.tsx) | `useTranslation('registry')` |
| [`apps/desktop/src/components/ConnectIDEs.tsx`](../../apps/desktop/src/components/ConnectIDEs.tsx) | `useTranslation('common')` |
| [`apps/desktop/src/components/source-badge.helpers.ts`](../../apps/desktop/src/components/source-badge.helpers.ts) | Move string literals to `common.json`; accept `t` as param |
| [`apps/desktop/src/features/servers/server-update-policy.helpers.ts`](../../apps/desktop/src/features/servers/server-update-policy.helpers.ts) | Move string literals to `servers.json`; accept `t` as param |
| `tests/e2e/specs/*.spec.ts` + `tests/e2e/pages/*.ts` | Replace text-based locators with `data-testid` |

---

## Phases

### Phase 1 — Install, init, and nav/chrome layer

- `pnpm add i18next react-i18next` in `apps/desktop/`
- Create `src/i18n.ts` with all 10 namespaces wired (stub JSONs initially)
- Create `src/i18n.types.ts` with `CustomTypeOptions` augmenting all namespaces
- Create all 10 `locales/en/*.json` files
- Import `./i18n` in `main.tsx`
- Populate `nav.json` + `common.json` + `dashboard.json`
- Update `App.tsx` → `useTranslation('nav')` for all sidebar labels
- Update `DashboardQuickLinks.tsx` → `useTranslation('nav')` (deduplicated)
- Update `DashboardStatCards.tsx` → `useTranslation('dashboard')`
- `pnpm typecheck` + `pnpm lint` — zero new errors

**Outcome:** App boots normally with i18next active. The nav rename that previously touched 7 files now requires editing `nav.json` only — verifiable by changing a label in the JSON and confirming both the sidebar and the quick-links card update. TypeScript rejects any unknown key passed to `t()`.

---

### Phase 2 — Three heaviest feature pages

- [`WorkspacesPage.tsx`](../../apps/desktop/src/features/workspaces/WorkspacesPage.tsx) + [`WorkspaceBindingSheet.tsx`](../../apps/desktop/src/features/workspaces/WorkspaceBindingSheet.tsx) → `workspaces.json`
- [`SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx) + [`ServerUpdatesSection.tsx`](../../apps/desktop/src/features/settings/ServerUpdatesSection.tsx) + [`ServerPendingUpdatesList.tsx`](../../apps/desktop/src/features/settings/ServerPendingUpdatesList.tsx) → `settings.json`
- [`ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) + [`ServerActionMenu.tsx`](../../apps/desktop/src/features/servers/ServerActionMenu.tsx) → `servers.json`

**Outcome:** The three files that together account for ~55% of all hardcoded strings are fully externalized. `pnpm typecheck` stays green. Opening Workspaces, Settings, or Servers in the running app shows no regressions (every string renders from JSON).

---

### Phase 3 — Remaining feature pages + string helpers

- [`ClientsPage.tsx`](../../apps/desktop/src/features/clients/ClientsPage.tsx) → `clients.json`
- [`FeatureSetsPage.tsx`](../../apps/desktop/src/features/featuresets/FeatureSetsPage.tsx) + [`FeatureSetPanel.tsx`](../../apps/desktop/src/features/featuresets/FeatureSetPanel.tsx) → `featuresets.json`
- [`SpacesPage.tsx`](../../apps/desktop/src/features/spaces/SpacesPage.tsx) → `spaces.json`
- [`RegistryPage.tsx`](../../apps/desktop/src/features/registry/RegistryPage.tsx) + [`ServerCard.tsx`](../../apps/desktop/src/features/registry/ServerCard.tsx) + [`ServerDetailModal.tsx`](../../apps/desktop/src/features/registry/ServerDetailModal.tsx) → `registry.json`
- [`ConnectIDEs.tsx`](../../apps/desktop/src/components/ConnectIDEs.tsx) → `common.json`
- [`source-badge.helpers.ts`](../../apps/desktop/src/components/source-badge.helpers.ts) + [`server-update-policy.helpers.ts`](../../apps/desktop/src/features/servers/server-update-policy.helpers.ts) — move string literals to namespace JSONs; pass `t` as a parameter

**Outcome:** Zero hardcoded user-visible strings remain anywhere in `apps/desktop/src/`. A global grep for JSX text-node literals or `aria-label="` string literals returns only dynamic/computed values. Any copy change anywhere = one JSON edit.

---

### Phase 4 — E2E migration to testids

- Audit all `getByText`, `getByRole` text matches, and inline string assertions in `tests/e2e/specs/*.spec.ts` and `tests/e2e/pages/*.ts`
- Replace each with the corresponding stable `data-testid` locator (all critical UI already has testids per the existing suite)
- Verify `pnpm test:e2e:web` stays green

**Outcome:** E2E suite makes zero assertions on display copy. A full nav rename (e.g. changing `nav.json` keys) produces no test failures. `pnpm test:e2e:web` passes clean.

---

## Key files referenced

| Path | Notes |
|------|-------|
| [`apps/desktop/src/App.tsx`](../../apps/desktop/src/App.tsx) | Primary nav label source; highest-friction rename file |
| [`apps/desktop/src/features/dashboard/DashboardQuickLinks.tsx`](../../apps/desktop/src/features/dashboard/DashboardQuickLinks.tsx) | Mirrors nav labels independently — root cause of drift |
| [`apps/desktop/src/lib/contribute.ts`](../../apps/desktop/src/lib/contribute.ts) | Existing "single source" const precedent |
| [`apps/desktop/src/stores/types.ts`](../../apps/desktop/src/stores/types.ts) | Internal `NavItem` keys stay unchanged; only display labels move to JSON |
| [`apps/desktop/src/main.tsx`](../../apps/desktop/src/main.tsx) | i18n init import goes here |
| [`apps/desktop/package.json`](../../apps/desktop/package.json) | Dep install target |
| [`packages/ui/src/components/layout/Sidebar.tsx`](../../packages/ui/src/components/layout/Sidebar.tsx) | Presentational; stays prop-driven; no i18next dependency |
| [`tests/e2e/specs/navigation.spec.ts`](../../tests/e2e/specs/navigation.spec.ts) | Representative E2E file with text-based locators to migrate |
| [`docs/planning/web-e2e-parity-handoff.md`](./web-e2e-parity-handoff.md) | Prior E2E locator refresh — same testid-over-text pattern |

---

## Related documentation

- [`docs/planning/web-e2e-parity-handoff.md`](./web-e2e-parity-handoff.md) — prior work migrating stale copy locators; same testid rationale
- [`ui-cleanup` branch](https://github.com/crimsonsunset/mcp-mux/tree/ui-cleanup) — the 7-file rename that triggered this work
