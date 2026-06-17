# Server Update Policy — Audit & Remediation

**Last Updated:** Jun 17, 2026
**Status:** Audit complete — remediation not started
**Branch:** `feat/meta-surface-lean-core` (the feature shipped here in PR #4)
**Base branch:** `dev`
**Depends on:** The shipped per-server update policy ([`server-update-policy.md`](./server-update-policy.md)) — this doc audits that implementation and plans the fixes
**Unblocks:** Notify mode actually firing for the most common server pattern (`npx -y pkg`); a verified, trusted update lifecycle instead of one that only demos on hand-tested paths

---

## Problem

The per-server package update policy (auto / notify / pinned) merged in PR #4 is **code-complete on its happy paths but unverified and structurally blind on the most common one**. An end-to-end audit of the working tree found the feature works for pinned npx semver, `@latest` auto reconnect, and explicit update when a probe already cached a semver — but the headline use case the original design was written for (`npx -y pkg` with no version) never badges, the uvx probe queries the wrong registry, and there is almost no behavioral test coverage backing any of it.

The original design doc ([`server-update-policy.md`](./server-update-policy.md)) is also stale — it still reads `Status: Planning — not started` despite the feature being merged.

### Findings (audited against the working tree)

| # | Finding | Severity | Where |
| - | ------- | -------- | ----- |
| 1 | **Notify badge never fires for unversioned `npx -y pkg`.** The probe derives `current_version = None` for a bare package arg; `is_newer_version` returns `false` when current is unknown, so no badge, no pending-list row, and "Check for Update" always says "up to date." This is the exact case the original doc was motivated by. | **Critical** | `server_version_probe.rs`, `server-update-policy.helpers.ts` |
| 2 | **uvx latest-version probe hits the npm registry.** When a package isn't in `uv tool list --outdated`, the code falls back to `fetch_npm_latest_version(&package_name)` — wrong registry for PyPI packages; returns nonsense or nothing. | **High** | `server_version_probe.rs` |
| 3 | **Almost no behavioral test coverage.** 3 parser/comparison unit tests total; zero integration/E2E for the probe subprocess, explicit update, scheduler, false-positive guards, or storage columns. No web-admin route parity tests (`update_server_package`, `check_all_server_updates`, `check_server_version`). | **High** | `tests/rust`, `tests/ts` |
| 4 | **"Cache-bust" is arg-rewriting only.** Explicit npx update injects the probed `@X.Y.Z`, but a frozen bare-`pkg` npx cache entry (keyed by the full arg) is never evicted, so unversioned spawns keep serving the stale tarball. No `npm cache npx` eviction. | **Medium** | `resolution.rs` |
| 5 | **False-positive guard logic is triplicated and already drifting.** The floating-tag / unknown-current / pinned-exclusion rules are copy-pasted across `resolution.rs`, `server_version_probe.rs`, and `server-update-policy.helpers.ts`; Rust uses a loose `is_semver_like` while TS uses a strict `isValidSemver` regex. | **Medium** | Rust + TS |
| 6 | **Blocking subprocess calls on async workers.** `npm view`, `uv tool list`, and `uv tool upgrade` run via blocking `std::process::Command` directly on Tokio workers — latency + thread-starvation risk with many servers. | **Medium** | `server_version_probe.rs`, `resolution.rs` |
| 7 | **Headless/web-admin can't run updates.** `GatewayWriteRuntime::update_server_package` is a `gateway_not_running()` stub; only the Tauri `DesktopGatewayWriteRuntime` executes. A headless gateway behind the admin API can't update. | **Medium** | `write_runtime.rs` |
| 8 | **Auto servers can still badge.** Auto-policy servers are probed and can show a pending update even though they self-update on reconnect — informational noise in the pending list. | **Low** | probe + pending list |
| 9 | **Three `ServerVersionProbeService` instances.** Gateway-scheduled, admin-bridge, and per-command ephemeral construction patterns hit the same DB with redundant subprocess traffic on manual checks. | **Low** | gateway wiring |

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Sequencing | **Audit first (this doc), then fix-and-verify** | Lock the findings in writing before touching code so the remediation is measured against a stated baseline, not a moving target. |
| 2 | npx current-version detection (Finding 1) | **Introspect the npx cache via `npm cache npx ls --json` / `npm cache npx info <key>`** | The npx cache (keyed by SHA512 of the full package arg) holds the actually-resolved version. The `npm cache npx` subcommands (npm CLI ≥ 11, Feb 2025) are the only supported way to read it. Gives notify a real `current` for unversioned `npx -y pkg`. |
| 3 | npm < 11 fallback | **Graceful "unknown" — keep today's no-badge behavior, never guess** | `npm cache npx` doesn't exist on older npm. When introspection is unavailable, report unknown rather than emitting a false positive. Detect via `npm --version`. |
| 4 | uvx latest-version source (Finding 2) | **Query the PyPI JSON API (`GET https://pypi.org/pypi/<pkg>/json` → `info.version`); installed version from `uv tool list`** | `uv pip index versions` does not exist (astral-sh/uv#17809 open). PyPI JSON is the canonical source and needs no subprocess. Removes the wrong npm-registry fallback entirely. |
| 5 | Cache-busting (Finding 4) | **On explicit update, evict the stale npx cache entry (`npm cache npx rm` / targeted clear) in addition to injecting `@X.Y.Z`** | Arg rewriting alone leaves the frozen bare-`pkg` entry serving a stale tarball on the next unversioned spawn. Eviction makes "Update Now" actually fresh. |
| 6 | Guard de-duplication (Finding 5) | **One Rust source of truth (shared helper) reused by probe + resolution; TS mirrors it with a single helper + a shared test vector** | Triplicated logic already drifted. Collapse to one Rust path; keep the TS guard thin and pin both to the same fixture cases so they can't silently diverge. |
| 7 | Async hygiene (Finding 6) | **Move all package subprocess/HTTP calls onto `spawn_blocking` (subprocess) / async HTTP (PyPI)** | Keeps probe and resolution off the Tokio worker hot path; bounded concurrency for bulk probe. |
| 8 | Headless updates (Finding 7) | **Implement `GatewayWriteRuntime::update_server_package` for the headless path** | User confirmed headless/web-admin updates are in scope. The reconnect-with-resolution logic is gateway-side already; the stub just needs wiring to the same pool path the Tauri runtime uses. |
| 9 | Auto-policy badging (Finding 8) | **Exclude auto-policy servers from the pending list and badge** | Auto self-updates on reconnect; a pending row is noise. Probing them is still fine for the version display. |
| 10 | Verification depth (Finding 3) | **Both: mocked-subprocess integration tests + a documented manual smoke pass** | Deterministic tests pin the parse/guard/resolution behavior with no network; the manual smoke validates against real npm/uv since the subprocess contracts are the fragile part. |

---

## Scope

**In:**

- Written audit (this doc) + correcting the stale `Status` header in [`server-update-policy.md`](./server-update-policy.md)
- npx current-version detection via `npm cache npx` introspection, with npm-version capability detection and graceful "unknown"
- npx explicit-update cache eviction (real cache-bust, not just arg rewrite)
- uvx latest-version via PyPI JSON API; installed via `uv tool list`; remove the npm-registry fallback
- Single Rust source of truth for floating-tag / unknown-current / pinned / semver-validity guards, mirrored once in TS
- `spawn_blocking` / async HTTP for all probe + resolution package calls; bounded bulk-probe concurrency
- Headless `GatewayWriteRuntime::update_server_package` implementation
- Auto-policy exclusion from pending list + badge
- Mocked-subprocess integration tests (probe, explicit update, guards, storage columns) + web-admin route parity tests + a manual smoke checklist

**Out:**

| Item | Reason / Deferral |
| ---- | ----------------- |
| Collapsing the three `ServerVersionProbeService` instances (Finding 9) | Low severity, no correctness impact. Note it; revisit if redundant subprocess traffic shows up in profiling. |
| Update history / changelog table | Still Phase 4-deferred in the original doc; unaffected by this remediation. |
| Local-path / remote-URL server updates | Same exclusions as the original design — no registry to query / provider-managed. |
| Global `npm cache clean --force` | Blunt; clears every package. Per-entry `npm cache npx rm` (Decision 5) is the precise tool. |
| Rewriting the probe scheduler architecture | Out of scope; the 6h loop works. Only the work it performs is being corrected. |

---

## The Model

### npx current-version detection (Finding 1 → Decision 2/3)

The npx cache lives at `~/.npm/_npx/<sha512-prefix>/` keyed by the **full** package arg, so `pkg`, `pkg@latest`, and `pkg@^1` are distinct entries. A bare `npx -y pkg` entry is frozen — npm only re-resolves when the spec is a range/tag.

```
current_version(npx, unversioned pkg):
  if npm_supports_cache_npx():          # npm --version >= 11.0
      entry = `npm cache npx ls --json` → match package name
      return entry.version              # the resolved, on-disk version
  else:
      return None                       # graceful unknown — no false badge
```

This feeds the existing `is_newer_version(current, latest)` so a real `current` exists and the badge/pending row can fire for the common pattern.

### npx explicit-update cache-bust (Finding 4 → Decision 5)

```
update_server_package(npx):
  inject @<probed semver> into the arg          # already done
  npm cache npx rm <stale bare-pkg entry key>   # NEW — evict frozen tarball
  reconnect
```

### uvx latest-version (Finding 2 → Decision 4)

```
latest_version(uvx pkg):
  GET https://pypi.org/pypi/<pkg>/json  → info.version      # async HTTP, no subprocess
current_version(uvx pkg):
  `uv tool list`  → installed version for <pkg>             # spawn_blocking
```

The `fetch_npm_latest_version` fallback for uvx packages is **removed**.

### One guard, three call sites (Finding 5 → Decision 6)

```
crates/mcpmux-gateway/src/services/package_version.rs   (NEW shared module)
  ├─ is_floating_npm_tag(spec)        // latest | next | * | …
  ├─ is_valid_semver(version)         // strict — replaces loose is_semver_like
  ├─ is_pinned(policy)
  └─ update_available(current, latest, policy, spec)
        ↑ reused by server_version_probe.rs AND resolution.rs

apps/.../server-update-policy.helpers.ts
  └─ thin TS mirror, pinned to the same fixture vector as the Rust tests
```

### Readiness of the pending list (Finding 8 → Decision 9)

```
buildPendingServerUpdates(installed, definitions):
  skip if policy == pinned        # already excluded
  skip if policy == auto          # NEW — auto self-updates on reconnect
  skip if floating tag            # already excluded
  skip if current unknown         # still excluded, but now KNOWN for npx via Decision 2
```

---

## Phases

### Phase 1 — Audit & baseline (~half day) — **done in this doc**

- Capture the nine findings above against the working tree (complete).
- Correct the stale `Status` header in [`server-update-policy.md`](./server-update-policy.md) to reflect that the feature shipped in PR #4.
- Add a one-line repro for Finding 1 (a bare `npx -y pkg` notify server) and Finding 2 (a uvx server) to the manual smoke checklist so the fixes have a before/after.

**Outcome:** A written, agreed baseline: anyone can read this doc and reproduce the blind notify badge and the wrong-registry uvx probe before any code changes land. The original design doc no longer claims "not started."

---

### Phase 2 — npx notify correctness + cache-bust (~1 day) — **P0**

- Add `npm_supports_cache_npx()` capability check (parse `npm --version`).
- Implement npx cache introspection (`npm cache npx ls --json`) to derive `current_version` for unversioned packages; wire it into the probe's current-version derivation.
- On explicit `update_server_package` for npx, evict the stale cache entry (`npm cache npx rm`) alongside the existing `@semver` injection.
- Graceful "unknown" path for npm < 11 — no badge, no false positive.

**Outcome:** A notify-mode `npx -y pkg` server (no version in args) shows an amber badge and a pending-list row when the registry has a newer version, and "Check for Update" reports the real delta instead of "up to date." Clicking "Update Now" evicts the frozen npx entry so the next spawn actually downloads the new tarball. On npm < 11 the server reports unknown rather than a wrong answer.

---

### Phase 3 — uvx PyPI probe correctness (~half day) — **P1**

- Replace the npm-registry fallback with a PyPI JSON API call (`info.version`) for uvx latest-version.
- Derive uvx `current_version` from `uv tool list` (fall back to the `==` pin in args when present).
- Remove `fetch_npm_latest_version` usage from the uvx path entirely.

**Outcome:** A uvx/`uv run` server reports its real latest PyPI version and a correct current/latest delta; it no longer silently compares against an npm package of the same name. Verify against a known-outdated PyPI tool and an up-to-date one.

---

### Phase 4 — Consolidation: guards, async hygiene, headless, auto-exclusion (~1 day) — **P1**

- Extract `package_version.rs` as the single Rust source of truth for floating-tag / semver-validity / pinned / update-available logic; reuse it in `server_version_probe.rs` and `resolution.rs`. Replace loose `is_semver_like` with strict validation.
- Collapse the TS guards in `server-update-policy.helpers.ts` to a single helper mirroring the Rust contract.
- Move `npm view` / `npm cache npx` / `uv tool list` / `uv tool upgrade` onto `spawn_blocking`; make the PyPI call async HTTP; bound bulk-probe concurrency.
- Implement `GatewayWriteRuntime::update_server_package` for the headless path (reuse the gateway pool reconnect-with-resolution that the Tauri runtime already drives).
- Exclude auto-policy servers from `buildPendingServerUpdates` and the badge.

**Outcome:** Version logic lives in one Rust module (plus one thin TS mirror), so a guard change can't drift across three files. Bulk "Check All" no longer blocks Tokio workers. A headless gateway behind the admin API can run `update_server_package`. Auto servers stop cluttering the pending list. Verify: headless update reconnects with the bumped package; Rust and TS guards agree on the shared fixture vector.

---

### Phase 5 — Verification & doc reconciliation (~1 day) — **P1**

- Mocked-subprocess integration tests: probe parsing (npm view, `npm cache npx` JSON, `uv tool list`, PyPI JSON), explicit update injection + cache eviction, false-positive guards, and the readiness of the pending list. No network.
- Storage tests for migrations 024/025 column read/write + defaults.
- Web-admin route parity tests in `tests/ts/admin-transport.test.ts` for `update_server_package`, `check_all_server_updates`, `check_server_version`.
- Run the manual smoke checklist (real npx + uvx servers) captured in Phase 1; record results.
- Reconcile both planning docs (`update-planning-md`): set this doc's status, fill the file inventory with what actually changed, and update [`server-update-policy.md`](./server-update-policy.md) so its phases/status match the shipped + remediated state.

**Outcome:** The behavior the audit flagged as unverified is now pinned by deterministic tests, the web-admin surface has route parity coverage, and a manual smoke run confirms the npx/uvx subprocess contracts against the real tools. Both planning docs reflect what was actually built. This phase is non-optional — the plan is not complete until the docs are reconciled.

---

## Files to create / modify

| File | Change |
| ---- | ------ |
| `crates/mcpmux-gateway/src/services/package_version.rs` | **Create** — single source of truth for floating-tag / semver-validity / pinned / update-available guards (Phase 4) |
| [`crates/mcpmux-gateway/src/services/server_version_probe.rs`](../../crates/mcpmux-gateway/src/services/server_version_probe.rs) | npx cache introspection for current version; PyPI JSON for uvx latest; remove npm fallback; `spawn_blocking`/async HTTP; reuse shared guards (Phases 2–4) |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | npx cache eviction on explicit update; reuse shared guards; `spawn_blocking` for `uv tool upgrade` (Phases 2, 4) |
| [`crates/mcpmux-gateway/src/admin/write_runtime.rs`](../../crates/mcpmux-gateway/src/admin/write_runtime.rs) | Implement `update_server_package` for the headless path (Phase 4) |
| [`apps/desktop/src/features/servers/server-update-policy.helpers.ts`](../../apps/desktop/src/features/servers/server-update-policy.helpers.ts) | Collapse to one guard helper mirroring the Rust contract (Phase 4) |
| [`apps/desktop/src/features/servers/server-pending-updates.helpers.ts`](../../apps/desktop/src/features/servers/server-pending-updates.helpers.ts) | Exclude auto-policy servers from the pending list (Phase 4) |
| [`apps/desktop/src/features/servers/ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) | Badge respects auto-policy exclusion (Phase 4) |
| `tests/rust/tests/integration/server_update_policy.rs` | **Create** — probe parsing, explicit update + eviction, guards, pending-list readiness (Phase 5) |
| `tests/rust/tests/database/` | Migration 024/025 column read/write + default tests (Phase 5) |
| [`tests/ts/admin-transport.test.ts`](../../tests/ts/admin-transport.test.ts) | Add `update_server_package`, `check_all_server_updates`, `check_server_version` route parity (Phase 5) |
| [`docs/planning/server-update-policy.md`](./server-update-policy.md) | Correct stale `Status`; reconcile phases with shipped + remediated state (Phases 1, 5) |

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-gateway/src/services/server_version_probe.rs`](../../crates/mcpmux-gateway/src/services/server_version_probe.rs) | npm/uv subprocess probes; `current_version` derivation; false-positive logic; `ServerUpdateAvailable` emission |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | `apply_update_policy`; sole `apply_package_update` gate; `@latest`/`@semver`/`==` injection; loose `is_semver_like` |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | Only `start_scheduler()` call site (startup + 6h loop) |
| [`crates/mcpmux-gateway/src/server/startup.rs`](../../crates/mcpmux-gateway/src/server/startup.rs) | Gateway autoconnect uses default resolution (auto yes; notify/explicit no) |
| [`apps/desktop/src-tauri/src/commands/server_manager.rs`](../../apps/desktop/src-tauri/src/commands/server_manager.rs) | `update_server_package` vs `retry_connection` divergence; post-update ephemeral probe |
| [`apps/desktop/src-tauri/src/services/admin_write_runtime.rs`](../../apps/desktop/src-tauri/src/services/admin_write_runtime.rs) | Admin API → Tauri command bridge for writes |
| [`crates/mcpmux-gateway/src/admin/write_runtime.rs`](../../crates/mcpmux-gateway/src/admin/write_runtime.rs) | `gateway_not_running()` stub for headless `update_server_package` |
| [`crates/mcpmux-storage/src/repositories/installed_server_repository.rs`](../../crates/mcpmux-storage/src/repositories/installed_server_repository.rs) | Column R/W + `update_version_cache` |
| [`crates/mcpmux-gateway/src/admin/ui_events.rs`](../../crates/mcpmux-gateway/src/admin/ui_events.rs) | SSE mapping for `server-update-available` |
| [`docs/planning/server-update-policy.md`](./server-update-policy.md) | Original design intent (stale status); the spec this remediation measures against |
| [npm CLI `npm cache npx`](https://github.com/npm/cli/pull/8100) | The Feb 2025 subcommand enabling npx cache introspection (Decision 2) |
| [astral-sh/uv#17809](https://github.com/astral-sh/uv/issues/17809) | Confirms `uv pip index versions` does not exist → PyPI JSON API (Decision 4) |

---

## Open questions (deferred, not blocking)

- **`npm cache npx` output stability** — the JSON shape isn't a documented stable contract; parse defensively and treat any parse miss as "unknown," never a false positive. Pin the expected shape in a fixture test so an npm upgrade that changes it fails loudly.
- **PyPI rate limits / offline** — the JSON API call needs a timeout and a swallow-to-unknown on failure, same posture as the npm probe. Bulk "Check All" should bound concurrency to avoid hammering PyPI.
- **Three probe-service instances (Finding 9)** — left as a noted-but-deferred cleanup; correctness is unaffected, but it's worth collapsing if manual-check subprocess traffic shows up in profiling.
- **Cache eviction key derivation** — `npm cache npx rm` needs the cache key (SHA512 prefix of the full arg). Prefer `npm cache npx ls --json` to find the entry rather than recomputing the hash ourselves, so we don't couple to npm's hashing internals.
