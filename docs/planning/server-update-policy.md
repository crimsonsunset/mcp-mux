# Server Update Policy — Per-Server Package Update Control

**Last Updated:** Jun 17, 2026
**Status:** Shipped in PR #4 — audit findings and remediation tracked in [`server-update-policy-audit-and-fixes.md`](./server-update-policy-audit-and-fixes.md)
**Branch:** `feat/meta-surface-lean-core` (merged in PR #4)
**Base branch:** `dev`
**Depends on:** Nothing — additive; sits alongside `env_overrides` / `args_append` / `default_params` config lanes
**Unblocks:** Servers drifting silently on stale npm/PyPI packages; no current way to force or block a package update

---

## Problem

McpMux has no package update lifecycle for stdio servers. The situation is worse than "no update button":

- **`cache-max` is `Infinity`** — npm never expires its cache by default. `npx -y inngest-cloud-mcp` (no `@latest`) will serve the same tarball it downloaded months ago, forever, unless the user manually clears their npm cache.
- **`npx -y pkg@latest`** (firebase, sonarqube, canva) resolves the `latest` tag from the registry on each spawn, but if the resolved version tarball is already cached, npm serves the cache. Only a version bump triggers a real download.
- **`--prefer-online` does not exist in npm 11.** There is no "force fresh" flag — the correct mechanism is tag annotation (`@latest`) or cache eviction.
- **`uvx`** caches tools in `~/.cache/uv/`; `uv tool upgrade <pkg>` is the idiomatic upgrade path.
- **Local-path servers** (typesense, beeper, hass-mcp, chrome-proxy) have no package source; they're always "manual update."
- **Remote URL servers** (jambase, cloudflare, posthog) are provider-controlled; McpMux has no update role.

The net effect: a `notify`-default policy lets users see when a package has a newer version and decide whether to update. `auto` forces `@latest` injection at spawn time (correct for most dev tools). `pinned` lets production servers lock to a known-good version explicitly.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Policy model | **`update_policy` enum per server: `auto` / `notify` / `pinned`** | Per-server control matches the existing per-install config pattern (`env_overrides`, `args_append`, `default_params`). A global setting alone would force the same behavior on `firebase-prod` and `inngest-local` — wrong. |
| 2 | Default policy | **`notify`** | Safe default: never silently change a running server's package, but surface available updates. `auto` on production servers (firebase-prod) without opt-in is dangerous. |
| 3 | Auto mode mechanism (npx) | **Inject `@latest` into the package arg at transport resolution time** | `--prefer-online` doesn't exist in npm 11. Injecting `@latest` forces tag re-resolution from the registry on every spawn. Implemented in `resolution.rs` alongside existing placeholder resolution — no subprocess shelling required. |
| 4 | Auto mode mechanism (uvx) | **Run `uv tool upgrade <pkg>` as a pre-spawn step** | `uv tool upgrade` is the idiomatic, stable uvx upgrade path. A pre-spawn shell call adds ~100–300ms but is correct; uvx doesn't have an `--upgrade` spawn flag. |
| 5 | Notify mode mechanism | **`npm view <pkg> version` + `uv tool list --outdated` probed on gateway startup and a configurable interval (default: 6h)** | Avoids hitting npm/PyPI on every spawn. Results cached in two new columns on `installed_servers`. Background task runs per-space on the existing gateway task pool. |
| 6 | Pinned mode mechanism | **Store `pinned_version TEXT` on `installed_servers`; inject `@<version>` (npx) or `==<version>` (uvx) at resolution time** | Same injection site as Auto. User sets the version once in server Settings; resolution enforces it on every spawn. |
| 7 | Transport scope | **`npx` and `uvx`/`uv run` commands only** — local-path, binary, and URL transports excluded | Local binaries don't have a package registry to query. URL servers are provider-managed. Attempting to update them is out of scope and harmful. |
| 8 | UI surfaces | **`ServerActionMenu` ellipsis + Settings "Server Updates" section** | Ellipsis for per-server actions (Update Now, Check for Update, update badge). Settings for default policy + bulk "Check All." Matches the existing Startup & System Tray pattern. |
| 9 | Update history / changelog | **Phase 4 — deferred** | History table adds meaningful DB complexity. Changelog URL field exists on `ServerDefinition.changelog_url` but most community servers don't populate it. Revisit when Phase 2 notification flow is validated. |

---

## Scope

**In:**

- `update_policy` column on `installed_servers` with `auto` / `notify` / `pinned` variants
- `pinned_version`, `latest_available_version`, `version_checked_at` columns on `installed_servers`
- Transport resolution (`resolution.rs`) injection for Auto (npx `@latest`) and Pinned (npx `@version`, uvx `==version`)
- Pre-spawn `uv tool upgrade` call for Auto-mode uvx servers
- Background version probe service (npm view + uv tool list --outdated) with interval scheduling
- `ServerActionMenu` additions: update available badge, "Update Now," "Check for Update"
- Settings "Server Updates" section: default policy dropdown, "Check All for Updates" button, per-server policy override visible in the Configure sheet

**Out:**

| Item | Reason / Deferral |
| ---- | ----------------- |
| Global `npm cache clean` / `uv cache clean` | Blunt — clears all servers. Per-server `@latest` injection is precise and doesn't touch other tools. |
| Update history log table | Phase 4 — deferred until notify flow is validated in production. |
| Changelog surfacing | Most registry definitions omit `changelog_url`; limited value until server catalog covers it. |
| Local-path server update (typesense, beeper, hass-mcp) | No package registry to query. Out of scope — user pulls their repos manually. Show "Manual update" label in UI for these. |
| Remote URL server update (jambase, posthog, cloudflare) | Provider-managed. McpMux has no role. Show "Provider managed" label. |
| Auto-update on a schedule (without user action) | Too aggressive for production servers. `auto` mode only applies at spawn time (when the user reconnects), not on a background schedule. |
| npm global install (`npm install -g`) path | Introduces a separate global package copy that conflicts with `npx -y` cache behavior. `@latest` injection is simpler and consistent. |

---

## The Model

### npm cache behavior (confirmed)

```
npm config get cache-max → Infinity (never expires)

npx -y pkg          → serves cached tarball indefinitely; effectively frozen
npx -y pkg@latest   → resolves 'latest' tag from registry; serves cached tarball
                       for that version — re-downloads only on version bump
npx --prefer-online  → does not exist in npm 11

Fix for auto mode:  inject @latest into the package arg at resolution time
Fix for pinned mode: inject @<semver> into the package arg at resolution time
```

### uvx behavior (confirmed)

```
uvx pkg               → uses ~/.cache/uv/; no auto-upgrade
uv tool upgrade <pkg> → upgrades in-place; correct pre-spawn hook for auto mode
uv tool list --outdated → lists outdated tools; correct probe for notify mode
```

### Storage additions

Two new migrations, mirroring the existing per-install config pattern:

**Migration 024** — `update_policy` + `pinned_version`:

```sql
ALTER TABLE installed_servers
  ADD COLUMN update_policy    TEXT NOT NULL DEFAULT 'notify';
ALTER TABLE installed_servers
  ADD COLUMN pinned_version   TEXT;
```

**Migration 025** — notify mode version cache:

```sql
ALTER TABLE installed_servers
  ADD COLUMN latest_available_version  TEXT;
ALTER TABLE installed_servers
  ADD COLUMN version_checked_at        TEXT;
```

### Policy resolution at spawn time (`resolution.rs`)

```
build_transport_config(installed, definition)
  ...
  existing placeholder resolution
  ...

  if installed.update_policy == Auto && transport == Stdio(npx/uvx):
    if command == "npx": inject @latest into package arg
    if command in ["uvx", "uv"]: run uv tool upgrade <pkg> first

  if installed.update_policy == Pinned && installed.pinned_version.is_some():
    if command == "npx": inject @<pinned_version> into package arg
    if command in ["uvx", "uv"]: inject ==<pinned_version> into package arg
```

Package arg detection: the package name is the first arg after `-y` (npx) or the first positional arg (uvx). Both are already available from `cached_definition.transport.args` at resolution time.

### Background probe (Notify mode)

Runs at gateway start and then every 6 hours (configurable). For each `notify`-policy server with a resolvable package:

- `npx` → `npm view <package_name> version` (shells out, ~50ms)  
- `uvx` → `uv tool list --outdated` (parse output)  
- Stores `latest_available_version` + `version_checked_at` in DB  
- Emits `ServerUpdateAvailable { server_id, current, latest }` domain event  
- Gateway notifies UI via existing `AdminUiEventBus`

### UI surfaces

**`ServerActionMenu` additions:**

- Update available badge (amber dot on the `MoreVertical` button) when `latest_available_version > current`
- "Update Available (vX.Y.Z)" item at top of menu when update detected (clicking triggers Update Now)
- "Check for Update" item (always visible for npx/uvx servers; hidden for local/URL)
- "Update Now" item (auto-eligible servers only)

**Settings → Server Updates section:**

- Default policy dropdown (`Auto` / `Notify` / `Pinned`) — applied to newly installed servers
- "Check All for Updates" button — triggers probe across all notify/auto servers
- Last checked timestamp

**Configure sheet (per-server Settings):**

- "Update Policy" dropdown overriding the global default
- "Pinned Version" text input (shown when Pinned selected)
- "Current / Latest" version display (from `version_checked_at` + `latest_available_version`)

---

## Phases

### Phase 1 — Schema + Auto mode (~1 day)

- Migration `024_server_update_policy.sql` — `update_policy TEXT NOT NULL DEFAULT 'notify'`, `pinned_version TEXT`
- Add `update_policy: UpdatePolicy` (enum: Auto/Notify/Pinned) and `pinned_version: Option<String>` to `InstalledServer` domain entity
- Read/write in `SqliteInstalledServerRepository`
- Plumb through `ApplicationServices`, admin write route, and Tauri `configure_server` command
- `resolution.rs` — detect `npx` / `uvx` commands, inject `@latest` (npx) or call `uv tool upgrade <pkg>` (uvx) when `update_policy == Auto`
- `ServerActionMenu` — add "Update Now" (visible for auto-eligible servers when enabled); clicking calls `retry_connection` (transport resolution already does the rest)
- Settings page — "Server Updates" section with default policy dropdown (persisted to app settings)

**Outcome:** A firebase or inngest server set to Auto will always spawn the latest published package version on reconnect. Clicking "Update Now" from the ellipsis menu on an Auto server forces a fresh reconnect with `@latest` injected. Non-npx/uvx servers show the menu item as disabled/hidden. Pinned version field is stored in DB but not yet enforced in resolution (Phase 3).

---

### Phase 2 — Notify mode: version probe + badge (~1 day)

- Migration `025_server_version_cache.sql` — `latest_available_version TEXT`, `version_checked_at TEXT`
- Add fields to entity + repository read/write
- `ServerVersionProbeService` — background Tokio task on gateway start; iterates `notify`-policy servers with resolvable package names; shells `npm view <pkg> version` / parses `uv tool list --outdated`; writes results to DB; emits `ServerUpdateAvailable` domain event
- Interval scheduling: run at startup + configurable interval (default 6h; stored in gateway config)
- `AdminUiEventBus` plumbing — `ServerUpdateAvailable` surfaces to frontend via SSE
- Amber badge on `MoreVertical` button when `latest > current` (UI subscribes to event)
- "Update Available (vX.Y.Z)" menu item in `ServerActionMenu`
- "Check for Update" item (explicit single-server probe, not waiting for interval)
- Settings — "Check All for Updates" button + last checked timestamp

**Outcome:** On gateway start, McpMux checks npm/PyPI for each notify-mode server. Within a few seconds the UI shows amber badges next to servers with available updates. Clicking "Update Available" in the ellipsis triggers an Update Now reconnect (same as Phase 1 Auto path). Clicking "Check for Update" on a single server forces an immediate probe and reflects the result in the menu.

---

### Phase 3 — Pinned mode: version lock (~half day)

- `resolution.rs` — enforce `pinned_version` for Pinned-policy servers: inject `@<semver>` into npx package arg, `==<semver>` for uvx
- Configure sheet — "Pinned Version" text input visible when policy is Pinned; validation against basic semver pattern
- "Lock to current version" quick action in `ServerActionMenu` — reads `latest_available_version` (or probes if empty), writes it as `pinned_version`, sets policy to Pinned
- `resolution.rs` — warn in logs (not error) if pinned version differs from `latest_available_version` (so operator can see drift without being forced to update)

**Outcome:** A server set to Pinned with `pinned_version = "2.1.0"` always spawns `npx -y pkg@2.1.0` regardless of what `@latest` resolves to. "Lock to current version" in the ellipsis captures today's version as the pin without the user needing to find and type it. Operator can pin `firebase-prod` while leaving `firebase-dev` on Auto without touching each other.

---

### Phase 4 — Update history + changelog (deferred) *(punted)*

**What it is:** A `server_update_history` table logging every update event (server_id, old_version, new_version, policy, outcome, timestamp). Surface in a new "Update History" tab inside View Logs, or as a separate sheet. Link to `changelog_url` from `ServerDefinition` when present.

**Why deferred:** History table adds a migration, a new repository, and UI surface for marginal immediate value. `changelog_url` is sparsely populated in the current server catalog. Revisit once Phase 2 notify flow has been validated in production and there's signal that users want the audit trail.

**Unblocked by:** Phase 2 shipped and stable; `changelog_url` coverage in the `mcp-servers` catalog improved.

---

## Files to create / modify

| File | Change |
| ---- | ------ |
| `crates/mcpmux-storage/src/migrations/024_server_update_policy.sql` | **Create** — `update_policy`, `pinned_version` columns |
| `crates/mcpmux-storage/src/migrations/025_server_version_cache.sql` | **Create** — `latest_available_version`, `version_checked_at` columns |
| [`crates/mcpmux-core/src/domain/installed_server.rs`](../../crates/mcpmux-core/src/domain/installed_server.rs) | Add `update_policy: UpdatePolicy`, `pinned_version`, `latest_available_version`, `version_checked_at` |
| [`crates/mcpmux-storage/src/repositories/installed_server_repository.rs`](../../crates/mcpmux-storage/src/repositories/installed_server_repository.rs) | Read/write new columns |
| [`crates/mcpmux-core/src/application/server.rs`](../../crates/mcpmux-core/src/application/server.rs) | Plumb new fields through service layer |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | Inject `@latest` / `@version` / run `uv tool upgrade` based on `update_policy` |
| `crates/mcpmux-gateway/src/services/server_version_probe.rs` | **Create** — background probe service (Phase 2) |
| [`crates/mcpmux-core/src/domain/event.rs`](../../crates/mcpmux-core/src/domain/event.rs) | Add `ServerUpdateAvailable { server_id, current_version, latest_version }` variant |
| [`crates/mcpmux-gateway/src/admin/command_bridge/write.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/write.rs) | Accept `update_policy` + `pinned_version` on server update route |
| [`apps/desktop/src-tauri/src/commands/server_manager.rs`](../../apps/desktop/src-tauri/src/commands/server_manager.rs) | `configure_server_update_policy` Tauri command |
| [`apps/desktop/src/features/servers/ServerActionMenu.tsx`](../../apps/desktop/src/features/servers/ServerActionMenu.tsx) | Add update badge, "Update Now," "Check for Update," "Lock to current version" |
| [`apps/desktop/src/features/servers/ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) | Wire new menu props and update event handling |
| `apps/desktop/src/features/settings/ServerUpdatesSection.tsx` | **Create** — default policy dropdown, Check All, last checked timestamp |
| `apps/desktop/src/features/settings/SettingsPage.tsx` | Add `ServerUpdatesSection` |

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | Placeholder resolution + args assembly — injection point for Phase 1/3 |
| [`crates/mcpmux-gateway/src/pool/transport/stdio.rs`](../../crates/mcpmux-gateway/src/pool/transport/stdio.rs) | Stdio spawn; `get_shell_path()` already resolves PATH-based commands |
| [`crates/mcpmux-core/src/domain/installed_server.rs`](../../crates/mcpmux-core/src/domain/installed_server.rs) | Entity to extend with policy fields |
| [`crates/mcpmux-storage/src/migrations/001_initial.sql`](../../crates/mcpmux-storage/src/migrations/001_initial.sql) | Original `installed_servers` schema; existing `env_overrides`/`args_append`/`default_params` lanes |
| [`crates/mcpmux-storage/src/migrations/022_installed_server_default_params.sql`](../../crates/mcpmux-storage/src/migrations/022_installed_server_default_params.sql) | Migration pattern to follow (ALTER TABLE + DEFAULT) |
| [`apps/desktop/src/features/servers/ServerActionMenu.tsx`](../../apps/desktop/src/features/servers/ServerActionMenu.tsx) | UI component to extend with update actions |
| [`apps/desktop/src/features/settings/SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx) | Settings page to add "Server Updates" section alongside Startup & System Tray |
| [`crates/mcpmux-core/src/domain/event.rs`](../../crates/mcpmux-core/src/domain/event.rs) | EventBus domain events; pattern for new `ServerUpdateAvailable` |
| [`docs/backend/guides/server-config-lanes.md`](../backend/guides/server-config-lanes.md) | Documents the existing per-server config pattern this feature extends |

---

## Open questions (deferred, not blocking)

- **Probe interval configurability** — 6h default is reasonable but operators with many servers may want daily. Add to gateway config (`gateway.toml`) in Phase 2; don't gate Phase 1 on it.
- **Version detection for non-`@latest` pinned packages** — `npm view pkg@2.x.x version` can fuzzy-resolve ranges; is that useful for Pinned mode? Likely not — Pinned should mean exact semver. Defer.
- **`npx -y` vs `npm exec` semantics** — npm 7+ changed how `npx` resolves packages. The `@latest` injection works identically for both. No behavioral difference found in testing.
- **Pre-spawn `uv tool upgrade` failure handling** — if the uvx upgrade fails (network, bad version), fall back to whatever is cached and log a warning. Don't fail the server connection over a version check.
- **Phase 4 trigger condition** — ship when (a) Phase 2 notify is in prod ≥2 weeks, and (b) `changelog_url` is populated for ≥10 servers in the `mcp-servers` catalog.
