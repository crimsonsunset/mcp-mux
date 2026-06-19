# Web E2E parity — thread handoff & grouped test index

**Status:** Phase 3 complete locally; committed + pushed (`f926c31`) — awaiting CI proof (Phase 4)  
**Last updated:** 2026-06-04  
**Branch:** `feat/meta-surface-lean-core` (fork)  
**PR:** [crimsonsunset/mcp-mux#4](https://github.com/crimsonsunset/mcp-mux/pull/4) → `dev`  
**CI job:** `e2e-web` in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml), `MCPMUX_ADMIN_TEST=1` (chromium). Gated by a `changes` paths filter; **PRs run `test:e2e:web:smoke`**, **pushes (post-merge) run full `test:e2e:web` + `test:e2e:web:admin`**.

---

## Thread handoff (read this first)

### What broke CI / local “web-only” runs

The Playwright suite in [`tests/e2e/playwright.config.ts`](../../tests/e2e/playwright.config.ts) runs the **web-admin SPA** (`VITE_ADMIN_WEB`, Vite `:1420` → proxy `/api` → AdminServer `:45819`). It is **not** a mocked-Tauri suite.

| Symptom | Root cause |
|--------|------------|
| Mass `networkidle` timeouts (~30s) | [`BasePage.waitForLoad()`](../../tests/e2e/pages/BasePage.ts) used `waitForLoadState('networkidle')`; admin **SSE** keeps HTTP active forever |
| 89 passed locally *without* backend, 0 passed *with* backend (parallel) | No `:45819` → proxy fails fast → network idles; with backend + many workers → SQLite/SSE contention |
| `ECONNREFUSED` spam in CI step 1 (old) | Job ran web specs with no AdminServer; Vite proxy retried forever |

### Fixes shipped (`f926c31`)

| Area | Change |
|------|--------|
| Load gate | `domcontentloaded` + [`waitForWebAppReady()`](../../tests/e2e/helpers/web-app-ready.helpers.ts) (space-switcher + `[useDataSync] Data sync complete`) |
| Diagnostics | `[e2e:web]` console / `/api/v1/*` response / requestfailed logs |
| Config | `workers: 1`, `maxFailures: 1` (config default), `expect.timeout: 15s`, `delete process.env.NO_COLOR` |
| Backend boot | `webServer` → [`scripts/admin-e2e-fixture.mjs`](../../scripts/admin-e2e-fixture.mjs); Linux CI `xvfb-run` for `tauri dev` |
| Smoke scripts | `pnpm test:e2e:web:wiring`, `pnpm test:e2e:web:smoke` |
| Tauri-only skips | `test.describe.skip('Software Updates')`, `test.skip` open-logs button |
| Locator refresh | `clients-title` (not `getByRole('Connections')` — sidebar duplicate), `Connect a client`, `gateway-status-chip`, `stat-active-space-value`, copy `getByText(/Copied/)`, registry `expect.poll` on server count |

### Full-suite numbers

| Run | When | Result |
|-----|------|--------|
| Pre-fix audit | 2026-06-04, ~4.6m | **83 passed · 15 failed · 26 skipped** (124 tests) — log `769675.txt` |
| Post-fix audit | 2026-06-04, ~1.0m | **94 passed · 0 failed · 30 skipped** (124 tests) — commit `f926c31` |

Skipped count rose by 4: entire **Software Updates** describe + **open logs folder** (intentional Tauri-only).

### Next agent actions

1. Babysit fork PR `e2e-web` on Ubuntu — fixture + xvfb + `MCPMUX_ADMIN_TEST=1` (`gh pr checks 4`).
2. Optional: add `test:e2e:web:group-*` npm scripts from [Group commands](#group-commands-copy-paste).
3. Optional: wire [`desktop-only.helpers.ts`](../../tests/e2e/helpers/desktop-only.helpers.ts) into remaining `test.skip` titles (Phase 5).
4. Do not mix `test:e2e:web:admin` with web-only groups — separate config.

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
| 5 | Stale copy | **Update locators** to current UI (`clients-title`, `Connect a client`, etc.) | Renames in ConnectionCard / dashboard; use testids where sidebar duplicates headings |
| 6 | Fail-fast default | **`maxFailures: 1`** in config; **`--max-failures=0`** for full audit runs | Smokes stay fast; full runs enumerate all reds |
| 7 | CI cost | **Smoke-on-PR, full-on-push** for web E2E; **all heavy jobs paths-gated** via a `changes` job (`dorny/paths-filter`): `rust-test` (rust), `build` (app), `e2e-desktop` (e2e_desktop), `e2e-web` (e2e_web). Rust report steps + admin report gated to match. | Full web+admin × every commit was the dominant cost; the rust matrix, Tauri build, and desktop WDIO are also heavy. Doc-only commits now skip all of them (skipped = success; only `rust-check`/`ts-check` are required). CI edits trip every filter so changes self-validate. |

---

## Failure catalog (15) — resolved in `f926c31`

Pre-fix run indexed failures G1–G8 (83/15/26). All 15 reds fixed or skipped; post-fix audit **0 failed**. Use **`file:line`** for grouped re-runs — list indices shift if spec files are added.

| Group | Spec locator | Title | Was | Resolution |
|-------|--------------|-------|-----|------------|
| **G1** | `settings.spec.ts:54–77` | Software Updates (3 tests) | `update-checker` missing | ✅ `describe.skip` entire block |
| **G1** | `settings.spec.ts:164` | Logs › open logs folder | `open-logs-btn` missing | ✅ `test.skip` — Tauri-only |
| **G1** | `settings.spec.ts:186` | Page Layout › sections in order | Expected `Software Updates` | ✅ Drop Tauri section from list |
| **G2** | `confirm-dialog.spec.ts:78` | ConfirmDialog – Clients | `Connected Clients` h1 | ✅ `getByTestId('clients-title')` (strict mode: sidebar also has "Connections") |
| **G3** | `dashboard.spec.ts:25,34` | Connect IDEs + copy JSON | Stale copy / popover | ✅ `Connect a client`, scroll + `.first()`, `getByText(/Copied/)` |
| **G3** | `user-flows.spec.ts:123` | Dashboard Interactions | same as dashboard:25 | ✅ Same locators |
| **G4** | `navigation.spec.ts:21`, `user-flows.spec.ts:32` | Navigate all sections | `Connected Clients` h1 | ✅ `clients-title` |
| **G5** | `spaces.spec.ts:49` | Current space on dashboard | `Currently viewing` | ✅ `stat-active-space-value` |
| **G6** | `servers.spec.ts:16` | Gateway status banner | `Gateway Running` text | ✅ `gateway-status-chip` |
| **G7** | `registry.spec.ts:34` | Filter when searching | count race (`0` vs `3`) | ✅ `expect.poll` until count &gt; 0, then `≤ initial` |
| **G8** | `settings.spec.ts:282` | Start minimized disabled | timing | ✅ Wait `settings-startup-section` |

**Skipped (not failures):** 30 tests `-` in list reporter — 26 pre-existing toast/desktop-only + 4 new Tauri-only skips above.

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

## Files created / modified (`f926c31`)

| File | Purpose | Status |
|------|---------|--------|
| [`tests/e2e/helpers/web-app-ready.helpers.ts`](../../tests/e2e/helpers/web-app-ready.helpers.ts) | App-ready gate + diagnostics | ✅ |
| [`tests/e2e/helpers/desktop-only.helpers.ts`](../../tests/e2e/helpers/desktop-only.helpers.ts) | `DESKTOP_TAURI_ONLY` skip reason constant | ✅ created; not wired into specs yet |
| [`tests/e2e/specs/web-wiring.spec.ts`](../../tests/e2e/specs/web-wiring.spec.ts) | Minimal wiring smoke | ✅ |
| [`tests/e2e/pages/BasePage.ts`](../../tests/e2e/pages/BasePage.ts) | Remove `networkidle`; call `waitForWebAppReady` | ✅ |
| [`tests/e2e/pages/DashboardPage.ts`](../../tests/e2e/pages/DashboardPage.ts) | ConnectionCard testids; dedupe `navigate()` | ✅ |
| [`tests/e2e/playwright.config.ts`](../../tests/e2e/playwright.config.ts) | Fixture, CF headers, workers, timeouts, NO_COLOR | ✅ |
| [`tests/e2e/playwright.admin.config.ts`](../../tests/e2e/playwright.admin.config.ts) | `delete process.env.NO_COLOR` (same IDE warning fix) | ✅ |
| [`scripts/admin-e2e-fixture.mjs`](../../scripts/admin-e2e-fixture.mjs) | `xvfb-run` on Linux CI | ✅ |
| [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) | Step 1 label + `MCPMUX_ADMIN_TEST=1` | ✅ |
| [`package.json`](../../package.json) | `test:e2e:web:wiring`, `test:e2e:web:smoke` | ✅ |
| `tests/e2e/specs/{settings,dashboard,navigation,user-flows,spaces,servers,registry,confirm-dialog}.spec.ts` | Skips + locator updates | ✅ |
| [`docs/planning/web-e2e-parity-handoff.md`](web-e2e-parity-handoff.md) | This doc | ✅ |

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

### Phase 3 — Locator + settings parity (~2h) ✅ done (`f926c31`)

- Applied G1–G8 fixes (skips, `clients-title`, Connect a client, chips, registry poll, layout list, copy assertion)
- `pnpm test:e2e:web:smoke` green; full chromium audit **94 passed · 0 failed · 30 skipped** (~1m)

**Outcome:** Local web suite green against live AdminServer.

### Phase 4 — CI proof (~1h + Actions time) 🟡 in progress

- ✅ Commit + push to fork PR #4 (`f926c31`)
- ⬜ Confirm `e2e-web` step 1 on Ubuntu: fixture boots app, chromium suite green
- ⬜ Step 2 `test:e2e:web:admin` validated separately (CF secrets **not** required for step 1)

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

After Phase 3, also skipped (+4): entire **Software Updates** describe; **open logs folder** button. Total intentional skips: **30**.

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
