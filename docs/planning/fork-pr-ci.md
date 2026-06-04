# Fork PR CI (`dev` + `main` triggers)

**Status:** Shipped — CI triggers include `dev` and `main`; fork PR [#4](https://github.com/crimsonsunset/mcp-mux/pull/4) validated after push.

**Last updated:** 2026-06-04

## Problem

Fork [crimsonsunset/mcp-mux](https://github.com/crimsonsunset/mcp-mux) uses **`dev`** as the default branch. [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) only listened for `main`, so PRs into `dev` never started Actions (e.g. PR #4 showed no checks).

## Solution

| Item | Choice |
|------|--------|
| Trigger branches | `push` and `pull_request` on **`main`** and **`dev`** |
| Job set | Unchanged full pipeline (rust-check, ts-check, rust-test matrix, build, coverage, e2e-web, e2e-desktop) |
| E2E on PRs | Every PR unless commit message contains `[skip e2e]` |
| Upstream | No workflow change; upstream PRs use mcpmux/mcp-mux CI on `main` |
| Merge gate | Branch protection on fork `dev` with required status checks; administrators may bypass |

## Files

| File | Change |
|------|--------|
| [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) | Add `dev` to `on.push.branches` and `on.pull_request.branches` |
| [`CONTRIBUTING.md`](../../CONTRIBUTING.md) | CI on pull requests subsection |
| [`docs/planning/fork-pr-ci.md`](fork-pr-ci.md) | This doc |

## Branch protection (fork `dev`)

Configured in GitHub **Settings → Branches** (not in-repo). After the first successful CI run, require at minimum:

- `rust-check`
- `ts-check`

Add matrix legs (`rust-test` per OS), `build`, `e2e-web`, and `e2e-desktop` when stricter gating is desired. Enable **Allow administrators to bypass required pull requests**.

## Fork secrets

| Secret | Required for merge? |
|--------|---------------------|
| `CODECOV_TOKEN` | No (`fail_ci_if_error: false`) |
| `TAURI_SIGNING_PRIVATE_KEY` (+ password) | Only if `build` / `e2e-desktop` fail without them; mac `build` uses ad-hoc signing when unset |

## Verification

- [x] `ci.yml` triggers include `dev` (on `dev` default branch, commit `7f44923`)
- [x] `gh pr checks 4 --repo crimsonsunset/mcp-mux` lists CI jobs after push to `dev` + PR synchronize
- [x] Branch protection on `dev`: `rust-check`, `ts-check` required; `enforce_admins: false` (admins may bypass)

## Notes

- `docs-deploy.yml` remains gated to `github.repository == 'mcpmux/mcp-mux'` only.
- Do not use `pull_request_target` for same-repo fork workflows.
