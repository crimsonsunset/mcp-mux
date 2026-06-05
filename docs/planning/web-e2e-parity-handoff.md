# Web E2E parity — thread handoff & grouped test index

**Status:** In progress — local fixes landed; full-suite re-run not completed after last edits  
**Last updated:** 2026-06-04  
**Branch:** `feat/meta-surface-lean-core` (fork)  
**PR:** [crimsonsunset/mcp-mux#4](https://github.com/crimsonsunset/mcp-mux/pull/4) → `dev`  
**CI job:** `e2e-web` in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) — `pnpm test:e2e:web` + `pnpm test:e2e:web:admin` (chromium)

---

## Thread handoff (read this first)

### What broke CI / local “web-only” runs

The Playwright suite in [`tests/e2e/playwright.config.ts`](../../tests/e2e/playwright.config.ts) runs the **web-admin SPA** (`VITE_ADMIN_WEB`, Vite `:1420` → proxy `/api` → AdminServer `:45819`). It is **not** a mocked-Tauri suite.

| Symptom | Root cause |
|--------|------------|
| Mass `networkidle` timeouts (~30s) | [`BasePage.waitForLoad()`](../../tests/e2e/pages/BasePage.ts) used `waitForLoadState('networkidle')`; admin **SSE** keeps HTTP active forever |
| 89 passed locally *without* backend, 0 passed *with* backend (parallel) | No `:45819` → proxy fails fast → network idles; with backend + many workers → SQLite/SSE contention |
| `ECONNREFUSED` spam in CI step 1 (old) | Job ran web specs with no AdminServer; Vite proxy retried forever |

### Fixes already in the working tree (uncommitted unless noted)

| Area | Change |
|------|--------|
| Load gate | `domcontentloaded` + [`waitForWebAppReady()`](../../tests/e2e/helpers/web-app-ready.helpers.ts) (space-switcher + `[useDataSync] Data sync complete`) |
| Diagnostics | `[e2e:web]` console / `/api/v1/*` response / requestfailed logs |
| Config | `workers: 1`, `maxFailures: 1` (config default), `expect.timeout: 15s`, `delete process.env.NO_COLOR` |
| Backend boot | `webServer` → [`scripts/admin-e2e-fixture.mjs`](../../scripts/admin-e2e-fixture.mjs); Linux CI `xvfb-run` for `tauri dev` |
| Smoke scripts | `pnpm test:e2e:web:wiring`, `pnpm test:e2e:web:smoke` |
| Tauri-only skips | `test.describe.skip('Software Updates')`, `test.skip` open-logs button |
| Locator refresh | Connections heading, `Connect a client`, `gateway-status-chip`, `stat-active-space-value`, dashboard copy scroll |

### Last full-suite numbers (before locator/skip pass)

**Run:** local, chromium, `--trace on --max-failures=0`, ~4.6m  
**Result:** **83 passed · 15 failed · 26 skipped** (124 tests) — log: terminal capture `769675.txt` / full run 2026-06-04

### Next agent actions

1. Re-run full suite (command in [Verification](#verification)) and update failure table below.
2. Add `package.json` group scripts from [Group commands](#group-commands-copy-paste) if still useful.
3. Push + babysit fork PR `e2e-web` (needs live backend + xvfb on Linux).
4. Optional: `test:e2e:web:admin` is separate config — do not mix with web-only groups.

**Prereqs local:** McpMux on `:45819` (or let fixture start it in CI only); repo `.env` with `MCPMUX_CF_ACCESS_*` when CF trust is on.

---

## Overview

Bring **`pnpm test:e2e:web`** (CI `e2e-web` step 1) to a stable green state against a **real** AdminServer, without pretending the suite is Tauri-mocked.

**In scope:** Playwright web-only specs under `tests/e2e/specs/*.spec.ts` (excluding `admin/**`), page objects, helpers, `playwright.config.ts`, fixture, CI job labels.

**Out of scope:**

| Item | Reason |
|------|--------|
| Admin parity suite (`test:e2e:web:admin`) | Separate config + fixture; track in admin E2E work |
| WDIO desktop E2E | Tauri shell; different transport |
| Mocking all `/api/v1/*` in web suite | Large fixture; deferred unless CI cannot run `pnpm dev` |
| Meta-tool / gateway Rust changes | PR #4 core; only touch if e2e exposes real regressions |

---

## Decisions

| # | Decision | Choice | Rationale |
|---|----------|--------|-----------|
| 1 | App ready gate | **`waitForWebAppReady`**, not `networkidle` | SSE + polling never satisfy networkidle |
| 2 | CI parallelism | **`workers: 1`** | One AdminServer + SQLite |
| 3 | Backend in CI | **Reuse `admin-e2e-fixture.mjs`** for web config | Same as local “pnpm dev + CF headers” wiring that passed smoke |
| 4 | Tauri-only UI | **`describe.skip` / `test.skip`**, not fake DOM | `UpdateChecker`, `open-logs-btn` gated on `isTauri()` in [`SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx) |
| 5 | Stale copy | **Update locators** to current UI (`Connections`, `Connect a client`, etc.) | Renames in ConnectionCard / dashboard, not product bugs |
| 6 | Fail-fast default | **`maxFailures: 1`** in config; **`--max-failures=0`** for full audit runs | Smokes stay fast; full runs enumerate all reds |

---

## Failure catalog (15) — last full run index

Playwright list indices are **1-based per run order** (alphabetical spec files). Use **`file:line`** for stable grouped runs — indices shift if files are added.

| Group | Idx (last run) | Spec locator | Title | Failure mode | Resolution |
|-------|----------------|--------------|-------|--------------|------------|
| **G1** | 82–84 | `settings.spec.ts:54` | Software Updates › should display update checker section | `update-checker` missing | **Skip** — `describe.skip` entire block |
| **G1** | | `settings.spec.ts:66` | Software Updates › should display current version | `current-version` missing | **Skip** (same block) |
| **G1** | | `settings.spec.ts:77` | Software Updates › should have check for updates button | `check-updates-btn` missing | **Skip** (same block) |
| **G1** | 89 | `settings.spec.ts:164` | Logs › should have open logs folder button | `open-logs-btn` missing | **Skip** — Tauri-only render |
| **G1** | 91 | `settings.spec.ts:186` | Page Layout › should display all sections in order | Expected `Software Updates` | **Fix** — drop Tauri section from list |
| **G2** | 15 | `confirm-dialog.spec.ts:78` | ConfirmDialog – Clients › Remove Client | `Connected Clients` h1 | **Fix** — `Connections` heading |
| **G3** | 18 | `dashboard.spec.ts:25` | Dashboard › should display connect IDEs section | `Connect Your IDEs` | **Fix** — `Connect a client` + `client-grid` |
| **G3** | 19 | `dashboard.spec.ts:34` | Dashboard › should copy config via JSON button | copy popover | **Fix** — scroll + `.first()` on icon |
| **G3** | 118 | `user-flows.spec.ts:123` | Dashboard Interactions › connect IDEs section | same as dashboard:25 | **Fix** (same locators) |
| **G4** | 35 | `navigation.spec.ts:21` | Navigation › should navigate to all main pages | `Connected Clients` h1 | **Fix** — `Connections` |
| **G4** | 114 | `user-flows.spec.ts:32` | Complete User Flows › navigate all main sections | same | **Fix** (same) |
| **G5** | 103 | `spaces.spec.ts:49` | Space Switcher › current space name on dashboard | `Currently viewing` | **Fix** — `stat-active-space-value` |
| **G6** | 69 | `servers.spec.ts:16` | My Servers › gateway status banner | text `Gateway Running` | **Fix** — `gateway-status-chip` |
| **G7** | 45 | `registry.spec.ts:34` | Registry › should filter servers when searching | count assertion (~1.2s) | **Fix** — wait for `serverCount`; verify registry data |
| **G8** | 97 | `settings.spec.ts:282` | Startup › disable start minimized when auto-launch off | `toBeDisabled` | **Fix** — wait `settings-startup-section`; may be env-conditional |

**Not failures (context):** 26 tests marked `-` skipped in suite (toast/mutation/desktop-only tests already `test.skip`).

---

## Group commands (copy-paste)

All use chromium, web config, full logs (no `tee`). Add `--max-failures=0` for audit; omit for fail-fast.

```bash
# G0 — wiring only (~1s)
pnpm test:e2e:web:wiring

# G1 — Tauri-only settings (should show as skipped, not failed)
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  tests/e2e/specs/settings.spec.ts:54 \
  tests/e2e/specs/settings.spec.ts:66 \
  tests/e2e/specs/settings.spec.ts:77 \
  tests/e2e/specs/settings.spec.ts:164 \
  tests/e2e/specs/settings.spec.ts:186

# G2 — confirm dialog + clients page shell
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  tests/e2e/specs/confirm-dialog.spec.ts:78

# G3 — dashboard connection card + copy config
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  tests/e2e/specs/dashboard.spec.ts:5 \
  tests/e2e/specs/dashboard.spec.ts:25 \
  tests/e2e/specs/dashboard.spec.ts:34 \
  tests/e2e/specs/user-flows.spec.ts:123

# G4 — navigation / multi-page flows (Connections rename)
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  tests/e2e/specs/navigation.spec.ts:21 \
  tests/e2e/specs/user-flows.spec.ts:32

# G5 — spaces dashboard stat card
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  tests/e2e/specs/spaces.spec.ts:49 \
  tests/e2e/specs/spaces.spec.ts:57

# G6 — registry discover filter
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  tests/e2e/specs/registry.spec.ts:34

# G7 — settings startup tray
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  tests/e2e/specs/settings.spec.ts:282

# G8 — curated smoke (post-fix)
pnpm test:e2e:web:smoke

# Full suite — debug (trace retained on failure)
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  --trace on --reporter=list --max-failures=0
```

### By spec file (grep title)

```bash
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  -g "Software Updates"   # G1 — expect skipped

pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  -g "Connections|Connect a client|gateway-status"

pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium \
  tests/e2e/specs/settings.spec.ts
```

---

## Files to create / modify

| File | Purpose |
|------|---------|
| [`tests/e2e/helpers/web-app-ready.helpers.ts`](../../tests/e2e/helpers/web-app-ready.helpers.ts) | App-ready gate + diagnostics (**new**) |
| [`tests/e2e/helpers/desktop-only.helpers.ts`](../../tests/e2e/helpers/desktop-only.helpers.ts) | Shared skip reason string (**new**) |
| [`tests/e2e/specs/web-wiring.spec.ts`](../../tests/e2e/specs/web-wiring.spec.ts) | Minimal wiring smoke (**new**) |
| [`tests/e2e/pages/BasePage.ts`](../../tests/e2e/pages/BasePage.ts) | Remove `networkidle`; call `waitForWebAppReady` |
| [`tests/e2e/pages/DashboardPage.ts`](../../tests/e2e/pages/DashboardPage.ts) | ConnectionCard testids; dedupe `navigate()` |
| [`tests/e2e/playwright.config.ts`](../../tests/e2e/playwright.config.ts) | Fixture, CF headers, workers, timeouts, NO_COLOR |
| [`scripts/admin-e2e-fixture.mjs`](../../scripts/admin-e2e-fixture.mjs) | `xvfb-run` on Linux CI |
| [`package.json`](../../package.json) | `test:e2e:web:wiring`, `test:e2e:web:smoke` |
| `tests/e2e/specs/{settings,dashboard,navigation,user-flows,spaces,servers,registry,confirm-dialog}.spec.ts` | Skips + locator updates |
| [`docs/planning/web-e2e-parity-handoff.md`](web-e2e-parity-handoff.md) | This doc |

**Suggested follow-up (not done):** add `test:e2e:web:group-*` scripts mirroring G0–G7 above.

---

## Phasing

### Phase 1 — Unblock load gate (~2h) ✅ done locally

- Replace `networkidle` with `waitForWebAppReady` in `BasePage`
- Add `web-wiring.spec.ts` + `pnpm test:e2e:web:wiring`
- `workers: 1`, NO_COLOR fix, admin fixture on `webServer`

**Outcome:** Wiring smoke passes in &lt;2s with `[e2e:web] waitForWebAppReady:done` and health/spaces 200 in logs.

### Phase 2 — Classify failures (~1h) ✅ done

- Full run with `--max-failures=0`; catalog 15 failures into groups G1–G8 (table above)
- Skip Tauri-only settings block + open logs

**Outcome:** Every previous failure has a group id and resolution type (skip vs locator vs env).

### Phase 3 — Locator + settings parity (~2h) 🟡 in working tree

- Apply G2–G7 locator fixes (Connections, Connect a client, chips, registry wait, layout list)
- Re-run `pnpm test:e2e:web:smoke` then full suite

**Outcome:** `pnpm test:e2e:web:smoke` green; full suite **0 failed** (skipped count may increase for intentional Tauri tests).

### Phase 4 — CI proof (~1h + Actions time)

- Commit + push to fork PR #4
- Confirm `e2e-web` step 1 on Ubuntu: fixture boots app, chromium suite green
- Step 2 `test:e2e:web:admin` validated separately (CF secrets **not** required for step 1)

**Outcome:** `gh pr checks 4` shows `e2e-web` success on `crimsonsunset/mcp-mux`.

### Phase 5 — Hygiene (optional)

- Add `test:e2e:web:group-*` npm scripts
- Document in [`tests/e2e/README.md`](../../tests/e2e/README.md)
- Tag remaining `test.skip` tests with `DESKTOP_TAURI_ONLY` reason in title

**Outcome:** Future agents can run a single group without re-deriving indexes.

---

## Pre-existing skips (intentional, not failures)

These already use `test.skip` in web specs — expect `-` in list reporter:

| Spec file | Skipped tests (title substring) |
|-----------|----------------------------------|
| `clients.spec.ts` | toast on display-name save; toast on revoke |
| `featuresets.spec.ts` | toast create/delete/save/JSON errors |
| `post-action-guidance.spec.ts` | install toast; navigate toast; approval state |
| `servers.spec.ts` | toast enable; clear logs; copy log path |
| `spaces.spec.ts` | toast space create/delete |
| `registry.spec.ts` | toast install/uninstall |
| `settings.spec.ts` | update check flows; logs path; startup toast; manual toast dismiss |
| `dashboard.spec.ts` | copy config when `browserName !== 'chromium'` |

After Phase 2, also skipped: entire **Software Updates** describe; **open logs folder** button.

---

## Architecture (test runtime)

```text
playwright.config.ts
  webServer → admin-e2e-fixture.mjs → pnpm dev (MCPMUX_DEV_ADMIN=1)
  baseURL :1420 (Vite, VITE_ADMIN_WEB)
  extraHTTPHeaders ← CF service token from repo .env
  workers: 1

Browser → :1420/* → Vite proxy /api → :45819 AdminServer
BasePage.goto → attachWebPageDiagnostics → waitForWebAppReady
```

---

## Verification

```bash
# From repo root; reuse running pnpm dev or let fixture start (CI)
pnpm test:e2e:web:wiring
pnpm test:e2e:web:smoke
pnpm exec playwright test -c tests/e2e/playwright.config.ts --project=chromium --max-failures=0
```

Traces: `test-results/**/trace.zip` → `pnpm exec playwright show-trace <path>`

---

## Key files referenced

| File | Note |
|------|------|
| [`tests/e2e/playwright.config.ts`](../../tests/e2e/playwright.config.ts) | Web-only Playwright entry |
| [`tests/e2e/playwright.admin.config.ts`](../../tests/e2e/playwright.admin.config.ts) | Admin parity; `:45819` baseURL |
| [`apps/desktop/src/components/ConnectionCard.tsx`](../../apps/desktop/src/components/ConnectionCard.tsx) | `Connect a client`, gateway testids |
| [`apps/desktop/src/features/settings/SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx) | `isTauri()` gates UpdateChecker / open-logs |
| [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) | `e2e-web` job |
| [`docs/planning/fork-pr-ci.md`](fork-pr-ci.md) | Why fork PR runs CI on `dev` |

---

## Related documentation

- [`docs/planning/fork-pr-ci.md`](fork-pr-ci.md) — fork CI triggers
- [`docs/planning/pre-web-admin-desktop-cleanup.md`](pre-web-admin-desktop-cleanup.md) — IPC cleanup before web admin
- [`tests/e2e/README.md`](../../tests/e2e/README.md) — E2E conventions (update when Phase 5 done)
- PR [#4](https://github.com/crimsonsunset/mcp-mux/pull/4) — meta-tool surface + e2e-web job changes
