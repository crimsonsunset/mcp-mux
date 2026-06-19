# CI Strategy

**Last updated:** 2026-06-05  
**Workflow:** [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)  
**Applies to:** fork `crimsonsunset/mcp-mux` (PRs into `dev`) and upstream `mcpmux/mcp-mux` (PRs into `main`)

---

## Overview

CI runs on every `push` and `pull_request` to `main` or `dev`. A `changes` detector job
(`dorny/paths-filter`) gates all heavy jobs so doc-only and unrelated commits skip them
entirely — those jobs report **skipped = success** and never block merges.

Concurrent runs for the same PR are cancelled when a new commit is pushed
(`concurrency: cancel-in-progress: true`).

---

## Jobs and when they run

| Job | Trigger | Path gate | Cost |
|-----|---------|-----------|------|
| `rust-check` | every push + PR | — (always) | ~2 min, Ubuntu |
| `ts-check` | every push + PR | — (always) | ~4 min, Ubuntu |
| `changes` | every push + PR | — (always) | ~5s, Ubuntu |
| `rust-test` | push + PR | `rust` | ~5–6 min × 3 OS |
| `build` | push + PR | `app` | ~15 min, macOS |
| `coverage-report` | push + PR (ts-check passed) | — | ~30s |
| `test-report` | push + PR (always after rust-test + ts-check) | rust reports gated | ~1 min |
| `e2e-web` | push + PR | `e2e_web` | **PR: smoke (~1 min), push: full (~8 min)** |
| `e2e-desktop` | push + PR | `e2e_desktop` | ~20–30 min × 2 OS (reusable workflow) |

### Always-on checks

`rust-check` and `ts-check` run on every commit regardless of path. These are the two
required status checks for branch protection on fork `dev`; everything else can skip
without blocking merge.

---

## Path filter reference

The `changes` job maps areas to filter outputs. A job only runs when its output is `'true'`.

| Output | Paths covered |
|--------|--------------|
| `rust` | `crates/**`, `apps/desktop/src-tauri/**`, `tests/rust/**`, Cargo files, clippy/rustfmt/nextest config, `.github/**` |
| `app` | All of the above + `apps/**`, `packages/**`, `schemas/**`, `scripts/**`, `package.json`, `pnpm-lock.yaml` |
| `e2e_desktop` | Same as `app` + `tests/e2e/**` |
| `e2e_web` | `apps/desktop/**`, `crates/**`, `packages/ui/**`, `tests/e2e/**`, fixture/env/build scripts, `package.json`, `pnpm-lock.yaml`, `ci.yml` |

**Important:** every filter includes `.github/**` so CI workflow edits always trip their
downstream jobs and self-validate the gating on the next push.

---

## Web E2E split: PR smoke vs post-merge full

The `e2e-web` job behaves differently depending on the GitHub event:

| Event | What runs | Approx. cost |
|-------|-----------|-------------|
| `pull_request` | `pnpm test:e2e:web:smoke` (wiring + ~4 curated tests) | 1 fixture boot, ~1 min |
| `push` (post-merge to `dev`/`main`) | `pnpm test:e2e:web` (full ~124-test chromium catalog) then `pnpm test:e2e:web:admin` | 2 fixture boots, ~8 min total |

**Rationale:** PR iteration should not pay the full Tauri + `xvfb` + AdminServer boot tax
on every locator or docs commit. The smoke gate covers app-ready wiring, dashboard, settings, and spaces — enough to catch the most common regressions. The full catalog and admin parity suite run once per merge, preserving depth as the gate before code reaches `dev`/`main`.

The smoke scripts are defined in `package.json`:
- `pnpm test:e2e:web:wiring` — single SPA load + data-sync check (~1s with dev running)
- `pnpm test:e2e:web:smoke` — wiring + dashboard + settings appearance + spaces info (~5s)

---

## Escape hatches

| Mechanism | Effect |
|-----------|--------|
| `[skip e2e]` in commit message | Skips both `e2e-web` and `e2e-desktop` |
| Path filter — no matching files | All gated jobs skip; `rust-check` + `ts-check` still run |
| `reuseExistingServer: !process.env.CI` | Locally, Playwright reuses a running `pnpm dev` — no cold boot tax |

---

## Desktop E2E

`e2e-desktop` is a reusable workflow (`e2e-desktop.yml`) invoked from `ci.yml`. It runs
WebDriver IO (WDIO) tests against the full Tauri desktop app on **Ubuntu + Windows**
(macOS covered by the `build` job). It only triggers when the `e2e_desktop` path filter
is true and `[skip e2e]` is absent.

Secrets (`TAURI_SIGNING_PRIVATE_KEY` + password) are optional — macOS uses ad-hoc signing
when unset.

---

## Other workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `nightly.yml` | `workflow_dispatch` (manual) | Full 3-OS Tauri build; uploads artifacts for 7 days |
| `release.yml` | `push` tags matching `v*` | release-please semantic versioning + multi-platform Tauri release |
| `docs-deploy.yml` | `push` to `main` (upstream only) | Deploy documentation site; gated to `github.repository == 'mcpmux/mcp-mux'` |
| `download-stats.yml` | scheduled | GitHub release download counters |

---

## Required status checks (fork `dev` branch protection)

Minimum required: `rust-check`, `ts-check`. Both always run regardless of path.

Additional checks (`rust-test`, `build`, `e2e-web`, `e2e-desktop`) can be added when
stricter gating is desired. Admins may bypass. See [`docs/planning/fork-pr-ci.md`](../planning/fork-pr-ci.md).

---

## Related docs

- [`docs/planning/fork-pr-ci.md`](../planning/fork-pr-ci.md) — why `dev` is a CI trigger, branch protection setup
- [`docs/planning/web-e2e-parity-handoff.md`](../planning/web-e2e-parity-handoff.md) — web E2E fixture, locator decisions, run commands
- [`docs/testing/README.md`](./README.md) — manual QA runbooks index
- [`tests/e2e/README.md`](../../tests/e2e/README.md) — E2E test conventions and commands
