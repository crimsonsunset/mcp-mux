# Upstream Sync, Oct 2026 (`upstream/main` @ `2e6a5b92` → fork)

**Last Updated:** Oct 30, 2026
**Status:** Planned. Integration branch created, no checkpoint merged yet
**Integration branch:** `sync/upstream-2026-10` (cut from `origin/main` @ `d03db501`)
**Lands on `main`:** only after every checkpoint below is merged into the integration branch and the soak in Phase 6 passes
**Base:** last sync was `eb5ae35a` (Jul 17), which merged `upstream/main` @ `87df4a26` (#203)
**Depends on:** [`upstream-client-mapping-reconciliation.md`](./upstream-client-mapping-reconciliation.md) (migration renumbering precedent, `Unbound` terminus), [`dev-to-main-port.md`](./dev-to-main-port.md)
**Estimated effort:** 6 to 9 working days, about half of it in Phase 2 (headless) and Phase 5 (security sweep)

---

## Problem

Since `87df4a26` the fork is **210 commits ahead and 59 behind** `upstream/main`. Upstream shipped three things in that window:

- **Headless runtime (#231).** Startup, state, and gateway commands moved out of `apps/desktop/src-tauri` into new crates `mcpmux-runtime` and `mcpmux-control`, plus `apps/daemon` (`mcpmuxd`) and `apps/cli`.
- **Onboarding (#243).** Auto-include Starter, auth off by default on loopback, browser origin guard, `@mux` prompt.
- **A security sweep on Oct 9 (#259 to #290).** Per-connection Space confinement, auth forced on when exposed, origin checks on every route, encryption at rest for server overrides, row-bound ciphertexts, refresh-token rotation, stdio caps and stderr scrubbing, registry signature checks, and deletion of dead OAuth code.

A single `git merge upstream/main` conflicts in **95 files**, 7 of them modify/delete. Upstream is linear (no merge commits), so the merge can be cut at any upstream commit. Each cut is a checkpoint: merge to that commit, resolve, validate, push, then go to the next one. Git records every checkpoint as merged, so a later checkpoint never replays an earlier resolution.

The fork changed the same surfaces heavily: machine-scoped routing and `Unbound`, the meta-tools split, web admin on `:45819`, Cloudflare tunnel support, the workspaces UI, and migrations 023 to 043.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Where it lands | **Every checkpoint merges into `sync/upstream-2026-10`, not `main`.** One final PR `sync/upstream-2026-10` → `main` | `main` is what the installed app is built from. The integration branch can be red between checkpoints without affecting daily use. |
| 2 | Merge style | **`git merge <upstream-sha>` per checkpoint, never rebase. Final PR lands with a merge commit** | Rebasing or rebase-merging drops the upstream parent, and the next sync re-conflicts on everything. Squash is already disabled on the repo. |
| 3 | Headless runtime (#231) | **Adopt.** Move admin server startup and machine ID onto `mcpmux-runtime`'s `AppState` | Skipping it makes `apps/daemon`, `apps/cli` and `mcpmux-runtime` conflict at checkpoints 4 through 8 and in every future sync. A one-time move is cheaper than permanent drift. |
| 4 | Migration numbering | **Fork numbers stay. Upstream migrations are appended after the fork's last one** | `database.rs` runs only migrations with `version > current`. The live DB is at v43. Anything inserted at 023 to 026 is skipped silently. Renumbering the fork's 027 to 043 instead would re-run them on existing DBs. Same rule as `eb5ae35a` and PR #14. |
| 5 | Onboarding (#243) | **Land PRs #14 and #15 on the integration branch first, then resolve the #243 checkpoint to the fork side** | #14 is the fork's version of #243's Starter half with full web-admin parity (migration 044). #15 is the loopback auth half (migration 045). Taking upstream's #243 would mean redoing work that already exists. |
| 6 | Auth when exposed (#267) and origin checks (#262) | **Take upstream's code. At those checkpoints, drop #15's overlapping copies** | The tunnel setup already runs with auth on: Cloudflare Access service token on `/mcp`, plus McpMux OAuth. Forcing auth on when `gateway.public_base_url` is set matches today's live config. Upstream reads the same `gateway.public_base_url` key. The web admin's own `/api/v1/oauth/clients` routes are on `:45819`, not the gateway. |
| 7 | Meta-tools (#266, #270) | **Port the behavior into the fork's split layout. Don't restore `meta_tools/tools.rs`** | The fork split `tools.rs` (972 lines) into about 25 files and added `approval_broker` (`124baa36`). Space confinement is a real security fix that the machine-scoped resolver must also enforce. The approval diff is ported if the broker doesn't already show the change. |
| 8 | Encryption at rest (#276, #285) | **Adopt. Back up the live DB before the checkpoint that adds it** | Today the fork stores env, header and argument overrides as plaintext, which goes against AGENTS.md. The backfill is one-way. |
| 9 | Deleted files | **Fork deletions win for `HomePage.tsx`, `WorkspaceBindingSheet.tsx`, `meta_tools/tools.rs`, `crates/mcpmux-mcp`. Upstream deletions win for `oauth/flow.rs`, `oauth/pkce.rs`, `oauth/discovery.rs`, `oauth/token.rs`** | The fork's only edit to `flow.rs` and `pkce.rs` was a one-line `u8` annotation (`2b4765b7`). Their only caller is `OAuthManager` in `oauth/mod.rs`, which has no callers outside a re-export. Upstream edits to fork-deleted files are ported to where that logic lives now (see the Architecture section). |
| 10 | Lockfiles | **Take upstream's `Cargo.lock` and `pnpm-lock.yaml` on every checkpoint, then let `cargo check` and `pnpm install` reconcile** | Hand-merging 32 + 7 lockfile hunks produces inconsistent graphs. `cargo update --workspace` would also bump unrelated crates. |
| 11 | Release manifest | **Take upstream's `.release-please-manifest.json` and `CHANGELOG.md`** | The fork is stuck at 0.5.0, and these files are generated. Keeping upstream's version keeps future merges clean. |
| 12 | Nav labels | **Keep the fork's Projects / Bundles / Clients** | Same call as the Jul 17 sync (`eb5ae35a`, Decision 5 of the reconciliation doc). Upstream's #238 copy is used where it clarifies, not as a route rename. |
| 13 | `rerere` | **Enable `git config rerere.enabled true` before Phase 1** | If a checkpoint is redone, the same hunks resolve automatically. |

---

## Scope

**In:** all 59 upstream commits from `c92117cd` (#228) to `2e6a5b92` (#294), plus landing PRs #14 and #15.

**Out:**

| Item | Reason |
| ---- | ------ |
| Upstream's nav rename (Clients → Mapping) | Decision 12 |
| Upstream's `SpaceDefault`/Starter fallback anywhere in the resolver | The fork's `Unbound` deny-by-default stays (see [`deny-by-default-bindable-callers.md`](./deny-by-default-bindable-callers.md)). #266 confinement narrows Spaces. It does not add a fallback. |
| Restoring `HomePage.tsx` or `WorkspaceBindingSheet.tsx` | Decision 9. #14 already ported their #243 additions into Dashboard and the binding panel |
| Running `mcpmuxd` as a service on Gondor | The daemon crate is adopted so it builds and stays in sync. Switching the installed app to daemon mode is a separate decision, not part of the sync |
| Upstreaming fork features (web admin, machine scoping) to `mcpmux/mcp-mux` | Not needed to catch up. Revisit after the sync lands |
| Linux desktop E2E stale selectors (`TODO(playwright-migration)`) | Pre-existing, tracked in [`mux-followups-35-36-37.md`](./mux-followups-35-36-37.md) "Not fixed here". The gate below uses Rust, vitest and web E2E |
| Static gateway key ("Option 4" in the jsg-tech-check remote access doc) | Unrelated to the sync |

---

## Architecture

### Branch and checkpoint flow

```text
origin/main (d03db501)
   │
   └─► sync/upstream-2026-10
          ├─ Phase 0: merge PR #14, PR #15 (rebased)         migrations 044, 045
          ├─ CP1  merge ee6f2473   #228–#234
          ├─ CP2  merge 5d1f34ca   #231 headless (alone)
          ├─ CP3  merge b94eca8f   #235–#242
          ├─ CP4  merge 2f153a32   #243 onboarding (alone)
          ├─ CP5  merge 603b3174   0.6.0, #245–#251, 0.7.0
          ├─ CP6  merge 60369acb   #252–#258                migrations → 046, 047
          ├─ CP7a merge 44f36325   #263, #266–#269, #279, #287
          ├─ CP7b merge c57bb1a9   #259, #270, #273–#275, #278, #281, #282, #288
          ├─ CP8a merge 3e9c134b   #261, #264, #271, #276, #277, #289
          └─ CP8b merge 2e6a5b92   #262, #272, #280, #283–#286, #290–#294   migration → 048
                 │
                 └─► PR sync/upstream-2026-10 → main (merge commit)
```

Each checkpoint is one merge commit on the integration branch, with a message in the same style as `eb5ae35a`: which side won per file cluster and why. Push after every checkpoint so CI runs on it.

### Migration map

Live DB (`~/Library/Application Support/com.mcpmux.desktop/mcpmux.db`) is at v43 today.

| Final number | Source | File | Content |
| ------------ | ------ | ---- | ------- |
| 044 | PR #14 (upstream 023, part 1) | `044_onboarding_defaults.sql` | `feature_sets.auto_include`, onboarding seed |
| 045 | PR #15 (upstream 023, part 2) | `045_pin_inbound_auth.sql` | Pins `gateway.auth_disabled=false` on installs with clients or servers |
| 046 | upstream 024 (#252, CP6) | `046_outbound_oauth_client_secret.sql` | `outbound_oauth_clients.client_secret_encrypted` |
| 047 | upstream 025 (#252, CP6) | `047_outbound_oauth_client_auth.sql` | `token_endpoint_auth_method`, `client_secret_expires_at` |
| 048 | upstream 026 (#283, CP8b) | `048_inbound_refresh_families.sql` | `inbound_refresh_families`, `inbound_spent_legacy_refresh` tables |

Upstream's `023_onboarding_defaults.sql` is **not** added as a file. 044 and 045 together cover it. In the CP4 conflict, delete upstream's 023 and keep 044/045.

Encryption (#276) and binding (#285) are not SQL migrations. Their backfill runs in Rust at startup, after all migrations. The backfill has to run before anything requires bound ciphertexts. Check that ordering at CP8a and CP8b against `crates/mcpmux-storage/src/crypto.rs` and the startup code in `mcpmux-runtime`.

For each renumbered upstream migration at a checkpoint:

1. `git mv` upstream's file to the next free fork number and update its header comment.
2. Register it in `MIGRATIONS` in `database.rs` at the end.
3. Bump the expected schema version assert in `database.rs` and `tests/rust/tests/database/migrations.rs`.
4. Extend the rollback batch in the older-database test (the #36 pattern in `mux-followups-35-36-37.md`).

### Headless runtime adoption (CP2)

Upstream #231 touches 4 desktop files (`lib.rs`, `state/mod.rs`, `commands/gateway.rs`, `Cargo.toml`) and adds about 10.7k lines in new crates. The fork's versions of those 4 files carry fork-only startup logic that has to survive:

| Fork-only piece | Lives today in | After CP2 |
| --- | --- | --- |
| Web admin server on `:45819` (`admin_server.rs`, `admin_write_runtime.rs`, `ui_events.rs`) | `apps/desktop/src-tauri/src/services/`, started from `lib.rs` | Stays in the desktop app. Started from `lib.rs` after `mcpmux-runtime` init, reading `AppState` from the runtime instead of building its own |
| Local machine ID (`gateway.local_machine_id`, `feature_set_resolver.set_local_machine_id()`) | `lib.rs`, `commands/settings.rs` | Set once on the runtime-built resolver, and re-set from the settings command as today |
| CF Access trust and team domain for admin | `admin/config.rs`, app settings | Unchanged. Admin config is read from settings, not from the runtime |
| Public URL for tunnel (`gateway.public_base_url`) | `app_settings_service.rs` `PUBLIC_URL` | Unchanged key. Upstream's `load_public_base_url_from_repo` reads the same key |
| macOS dock, permissions, unix signal, Cursor hook installer | `apps/desktop/src-tauri/src/*` | Desktop-only. Untouched by the runtime split |

Resolution rule for CP2: take upstream's structure in `lib.rs`, `state/mod.rs` and `commands/gateway.rs`, then re-add each fork-only piece in the table at the matching point. Don't keep a second copy of runtime init in the desktop app.

### Meta-tools port (CP7a, CP7b)

| Upstream change | Upstream file | Fork home |
| --- | --- | --- |
| `confined_space()`: a connection may only reach Spaces it is allowed | `services/meta_tools/approval.rs`, `feature_set_resolver.rs` | `feature_set_resolver.rs` (merge) plus the fork's `approval_broker.rs` and `meta_tool_common.rs` call sites |
| Always-allow grants by key (`insert_always_allow`, `list_always_allow`, `revoke_always_allow`) | `approval.rs` | `approval_broker.rs`, `approval_types.rs` |
| Tool-level edits from #266 and #270 | `meta_tools/tools.rs` (deleted in fork) | Split files: `invoke_tool.rs`, `search_tools.rs`, `feature_set_tools.rs`, `bind_workspace.rs` |
| Approval diff UI (#270) | `MetaToolApprovalDialog.tsx`, `MetaToolGrantsPanel.tsx`, `lib/api/metaTools.ts` | Same files (both sides edited them) |

Confinement is a narrowing filter, the same way `locked_space_id` works in the Jul 17 reconciliation. No match inside the allowed Spaces ends at `Unbound`.

### Tunnel and exposure (CP7a, CP8b)

From the live config and [`remote-mcp-gateway-access.md`](../../../jsg-tech-check/docs/mcp-mux/remote-mcp-gateway-access.md):

- `mcp.joe-hassio.com` → `:45818` behind CF Access (service token). Only `/oauth/*` and `/.well-known/*` are bypassed.
- `mux.joe-hassio.com` → `:45819` (web admin), CF Access by email.
- `gateway.public_base_url = https://mcp.joe-hassio.com`, `gateway.auth_disabled` unset, 5 inbound clients.

After CP7a (#267), `exposed()` is true because the public URL is set, so auth is forced on. That's the current behavior. After CP8b (#262), any request carrying a non-allowed `Origin` is rejected on every gateway route. Cursor and Hermes send no `Origin`, and a top-level browser navigation to `/oauth/authorize` doesn't either, so the OAuth flow is fine. Run `pnpm remote:smoke` after both checkpoints.

### Gateway client-management routes (CP8b)

Upstream #262 removes `/oauth/clients` list/update/delete from the gateway router. The fork still mounts them (`crates/mcpmux-gateway/src/server/mod.rs`, `is_management_path`). Take upstream's removal, then grep `apps/desktop/src` for any caller still hitting `:45818/oauth/clients` and point it at the Tauri command or the admin `/api/v1/oauth/clients` route. The admin routes on `:45819` stay.

---

## Files to create / modify

Conflict clusters by checkpoint. Paths are relative to the repo root. "Fork" means keep the fork's side and port upstream's intent. "Upstream" means take upstream's side and re-add fork pieces.

| Checkpoint | File cluster | Resolution |
| --- | --- | --- |
| CP1 | `.github/workflows/ci.yml`, `e2e-desktop.yml`, `.gitignore` | Hunk merge. Keep #22's Playwright 1.64 and app-log upload, take upstream's E2E-fails-CI gating |
| CP1 | `crates/mcpmux-gateway/src/pool/transport/stdio.rs`, `pool/service.rs`, `Cargo.toml` | Hunk merge (#229 Windows process trees). Keep `configure_child_process_platform()` as the only spawn path |
| CP2 | `apps/desktop/src-tauri/src/lib.rs`, `state/mod.rs`, `commands/gateway.rs`, `Cargo.toml` | Upstream, then re-add the fork pieces in the headless table |
| CP2 | `crates/mcpmux-runtime/**`, `crates/mcpmux-control/**`, `apps/daemon/**`, `apps/cli/**` | Upstream (new) |
| CP3 | `apps/desktop/src/features/clients/ClientsPage.tsx`, `ClientApiKeysSection.tsx`, `RegisterApiKeyClientModal.tsx` | Fork layout, add upstream's clients-by-name display (#238) |
| CP3 | `apps/desktop/src/features/featuresets/FeatureSetsPage.tsx`, `components/SpaceSwitcher.tsx` | Hunk merge |
| CP3 | `apps/desktop/src-tauri/src/commands/workspace_binding.rs`, `commands/oauth.rs` | Hunk merge (#236 Codex, #235 client metadata doc) |
| CP4 | `FeatureSetPanel.tsx`, `ConnectIDEs.tsx`, `ConnectionCard.tsx`, `App.tsx`, `SettingsPage.tsx`, `WorkspacesPage.tsx`, `WorkspaceSetupWizard.tsx` | Fork (PR #14/#15 already port #243) |
| CP4 | `features/home/HomePage.tsx`, `features/workspaces/WorkspaceBindingSheet.tsx` | Delete (fork) |
| CP4 | `crates/mcpmux-storage/src/migrations/023_onboarding_defaults.sql` | Delete. 044 and 045 cover it |
| CP4 | `apps/daemon/tests/daemon.rs` | Upstream, adjusted to the fork's schema version |
| CP5 | `ServersPage.tsx`, `SpacesPage.tsx`, `ServerLogViewer.tsx`, `lib/api/logs.ts`, `commands/space.rs` | Hunk merge (#245 rename Space, #246 sort, #251 logs per Space) |
| CP5 | `.release-please-manifest.json`, `CHANGELOG.md`, versions in `Cargo.toml`, `tauri.conf.json`, `package.json` | Upstream |
| CP6 | `crates/mcpmux-gateway/src/pool/oauth.rs`, `pool/oauth_utils.rs`, `crates/mcpmux-core/src/domain/outbound_oauth_registration.rs`, `crates/mcpmux-core/src/branding.rs` | Hunk merge (#252 logo, client secret) |
| CP6 | migrations 024, 025 | Renumber to 046, 047 |
| CP7a | `crates/mcpmux-gateway/src/services/feature_set_resolver.rs`, `mcp/oauth_middleware.rs`, `services/session_roots.rs`, `pool/routing.rs` | Hunk merge, `Unbound` stays (#266, #267, #269) |
| CP7a | `apps/desktop/src-tauri/tauri.conf.json`, `capabilities/*` | Upstream (#263 CSP). Then confirm the web admin build and `:1420` dev still load |
| CP7b | `services/meta_tools/approval.rs`, `meta_tools/tools.rs` (deleted), `MetaToolApprovalDialog.tsx`, `MetaToolGrantsPanel.tsx` | Port per the meta-tools table |
| CP7b | `crates/mcpmux-storage/src/repositories/installed_server_repository.rs`, `crates/mcpmux-runtime/src/init.rs` | Hunk merge (#273 master key, #278 erase credentials) |
| CP8a | `installed_server_repository.rs`, `crates/mcpmux-storage/src/crypto.rs` | Upstream encryption (#276), then re-add the fork's version tracking and `default_params` columns |
| CP8a | `apps/desktop/src/features/registry/*`, `components/ServerInstallModal.tsx` | Hunk merge (#277 show what a server runs) |
| CP8b | `oauth/flow.rs`, `oauth/pkce.rs`, `oauth/discovery.rs`, `oauth/token.rs`, `oauth/mod.rs` | Upstream (deleted) |
| CP8b | `crates/mcpmux-gateway/src/server/mod.rs`, `server/handlers.rs` | Upstream (#262), see the client-management routes section |
| CP8b | `crypto.rs`, all repositories using encrypted fields | Upstream (#285 row binding) |
| CP8b | migration 026 | Renumber to 048 |
| Every CP | `Cargo.lock`, `pnpm-lock.yaml` | Decision 10 |
| Every CP | `tests/rust/**`, `tests/ts/**` | Follow the source side chosen. Fork tests that assert `Unbound` must keep passing unchanged |

---

## Gate (run on every checkpoint)

1. `cargo check --workspace` and `pnpm install` (lockfiles reconcile)
2. `pnpm validate` (fmt, clippy `-D warnings`, check, eslint, typecheck)
3. `pnpm test:rust` and `pnpm test:ts`
4. From CP7a onward: `pnpm test:e2e:web`, plus `pnpm remote:smoke` against a dev build
5. Push the integration branch and let CI run. A checkpoint is done when CI's `rust-check`, `rust-test`, `ts-check` and `e2e-web` are green
6. Before CP8a: copy `mcpmux.db` (and `-wal`/`-shm` if present) to a dated backup outside the data dir

Don't start the next checkpoint on a red gate. Desktop WDIO E2E is advisory (see Out).

---

## Phases

### Phase 0: Prep (~half day)

- [ ] `git config rerere.enabled true`
- [ ] Rebase `feat/onboarding-auto-include` (#14) onto `sync/upstream-2026-10`, then `feat/auth-default-loopback` (#15) onto that. Both are 14 commits behind `main`
- [ ] Retarget PRs #14 and #15 at `sync/upstream-2026-10`. CI green, merge (merge commit)
- [ ] Confirm migrations 044 and 045 apply on a copy of the live DB (v43 → v45)
- [ ] Commit this doc on the integration branch

**Outcome:** `sync/upstream-2026-10` has #14 and #15, the schema is at v45, the gate is green, and PRs #14/#15 show as merged into the integration branch.

---

### Phase 1: Low-risk base (CP1, ~half day)

- [ ] `git merge ee6f2473` (#228, #229, #230, #232, #233, #234). About 9 conflicted files
- [ ] CI workflows: keep #22's changes, take upstream's E2E gating
- [ ] Windows process tree cleanup (#229) merged into `stdio.rs` and `service.rs`
- [ ] Gate

**Outcome:** Integration branch contains upstream through #234. `pnpm validate` and `pnpm test:rust` pass, and `async-trait` is at 0.1.92 or later in `Cargo.lock`.

---

### Phase 2: Headless runtime (CP2, ~1.5 to 2 days)

- [ ] `git merge 5d1f34ca` (#231 alone)
- [ ] Take upstream's `lib.rs`, `state/mod.rs`, `commands/gateway.rs`, then re-add every row in the headless adoption table
- [ ] Web admin starts on `:45819` after runtime init
- [ ] `gateway.local_machine_id` reaches the resolver at startup and after a settings change
- [ ] `cargo build -p mcpmuxd` and `-p mcpmux-cli` succeed (crate names as upstream defines them)
- [ ] Gate, plus a manual `pnpm dev:admin` run: desktop window opens, `:1420` admin loads, `/api/v1/health` returns 200

**Outcome:** The desktop app boots through `mcpmux-runtime`, and the web admin, machine-scoped routing and tunnel public URL all work on a dev build. `mcpmuxd` and the CLI compile in the workspace.

---

### Phase 3: Features up to onboarding (CP3 and CP4, ~1 day)

- [ ] CP3: `git merge b94eca8f` (#235, #236, #238, #240, #241, #242). Clients-by-name onto the fork's ClientsPage. Codex added to "Connect apps to this folder". Linux Mesa fallback
- [ ] Gate
- [ ] CP4: `git merge 2f153a32` (#243 alone). Resolve to the fork side for every file in the CP4 rows. Delete upstream's `023_onboarding_defaults.sql`, `HomePage.tsx`, `WorkspaceBindingSheet.tsx`
- [ ] Check that upstream's #243 has nothing #14/#15 missed: diff `2f153a32^..2f153a32` against PRs #14/#15 by file and list any leftovers in the merge commit message
- [ ] Gate

**Outcome:** Integration branch contains upstream through #243 with schema still at v45 (no new migration). Starter auto-include, loopback auth default and the `@mux` prompt behave as they did after Phase 0.

---

### Phase 4: Releases, UI and OAuth client (CP5 and CP6, ~1 day)

- [ ] CP5: `git merge 603b3174` (0.6.0, #245, #246, #247, #248, 0.7.0, #251). Take the upstream release manifest and changelog. Rename Space, sort servers, logs per Space
- [ ] Gate
- [ ] CP6: `git merge 60369acb` (#252, #253, #256, #257, #258). Renumber 024 → 046 and 025 → 047 per the migration procedure
- [ ] Gate, plus a fresh outbound OAuth connect to one server that uses DCR (Notion or Atlassian) on a dev build

**Outcome:** Version reads 0.7.0 across `Cargo.toml`, `tauri.conf.json` and `package.json`. Schema is at v47. A new DCR registration stores the client secret encrypted and sends McpMux's logo.

---

### Phase 5: Security sweep (CP7a, CP7b, CP8a, CP8b, ~2 to 3 days)

- [ ] CP7a: `git merge 44f36325` (#263, #266, #267, #268, #269, #279, #287). Resolver merge keeps `Unbound`. Port `confined_space()` into the fork's resolver and broker. CSP from #263, then confirm `:1420` and the built admin still load
- [ ] Gate plus `pnpm remote:smoke`
- [ ] CP7b: `git merge c57bb1a9` (#259, #270, #273, #274, #275, #278, #281, #282, #288). Meta-tools approval diff into split files. Master key never replaced. Stdio caps
- [ ] Gate
- [ ] Back up the live DB
- [ ] CP8a: `git merge 3e9c134b` (#261, #264, #271, #276, #277, #289). Encrypt server overrides at rest. Keep the fork's `installed_servers` columns
- [ ] Gate, plus run a dev build against a **copy** of the live DB: every installed server starts and its env/headers are intact after the backfill
- [ ] CP8b: `git merge 2e6a5b92` (#262, #272, #280, #283, #284, #285, #286, #290, #291, #293, #294). Delete the old OAuth files. Remove gateway client-management routes. Renumber 026 → 048. Row-bound ciphertexts
- [ ] Gate plus `pnpm remote:smoke` and a refresh-token rotation check (reconnect `remote-mux` in Cursor after a token refresh)

**Outcome:** `git rev-list --count sync/upstream-2026-10..upstream/main` is 0. Schema is at v48. On a copy of the live DB, all servers start, overrides are stored as `v2:` ciphertexts, `remote-mux` over `mcp.joe-hassio.com` lists servers and invokes a tool, and an unmapped folder still gets zero tools.

---

### Phase 6: Soak and land on `main` (~half day plus 1 to 2 days of daily use)

- [ ] Build the app from `sync/upstream-2026-10` and run it as the daily driver with the real DB (backup taken in Phase 5)
- [ ] Watch: Hermes `chief` over the tunnel, Cursor local and `remote-mux`, workspace routing on Gondor and Rohan, web admin on `mux.joe-hassio.com`
- [ ] Update [`docs/manual`](../manual) and the jsg-tech-check mcp-mux docs where behavior changed (auth forced on when exposed, origin checks, daemon available)
- [ ] Mark this doc's Status as shipped with a Resolution section, same as the reconciliation doc
- [ ] Open PR `sync/upstream-2026-10` → `main`, land with a merge commit

**Outcome:** `main` contains `upstream/main` @ `2e6a5b92`. `git rev-list --count main..upstream/main` is 0, and the installed app runs on it with no regressions found during the soak.

---

## Risks

| Risk | Where | Mitigation |
| ---- | ----- | ---------- |
| Encryption backfill corrupts or loses overrides on the real DB | CP8a, CP8b | Backup before CP8a. Verify on a copy first. The soak uses the real DB only after the copy passes |
| Upstream migration silently skipped | CP4, CP6, CP8b | Migration map. Schema version assert bumped every time |
| Fork-only startup logic lost in the runtime move | CP2 | Headless table is the checklist. Manual `pnpm dev:admin` check |
| Confinement added without the `Unbound` terminus | CP7a | Existing `Unbound`-asserting resolver tests must pass unchanged |
| CSP (#263) blocks the web admin or Vite dev | CP7a | Load `:1420` and `:45819` after the merge |
| Tunnel clients rejected by origin checks | CP8b | `pnpm remote:smoke`, plus a live Hermes and `remote-mux` check in the soak |
| Long-lived integration branch drifts from `main` | All | Merge `main` into the integration branch whenever a fork PR lands during the sync |

---

## Key files referenced

| File | Note |
| ---- | ---- |
| [`crates/mcpmux-storage/src/database.rs`](../../crates/mcpmux-storage/src/database.rs) | `MIGRATIONS`, `version > current` runner, schema version assert |
| [`tests/rust/tests/database/migrations.rs`](../../tests/rust/tests/database/migrations.rs) | Older-database rollback test to extend per migration |
| [`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`](../../crates/mcpmux-gateway/src/services/feature_set_resolver.rs) | `Unbound` terminus, where #266 confinement lands |
| [`crates/mcpmux-gateway/src/services/meta_tools/`](../../crates/mcpmux-gateway/src/services/meta_tools) | Fork's split meta-tools and `approval_broker` |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | Gateway client-management routes that #262 removes |
| [`crates/mcpmux-gateway/src/admin/router.rs`](../../crates/mcpmux-gateway/src/admin/router.rs) | Web admin routes on `:45819`, unaffected by #262 |
| [`crates/mcpmux-core/src/service/app_settings_service.rs`](../../crates/mcpmux-core/src/service/app_settings_service.rs) | `gateway.public_base_url`, admin CF Access trust |
| [`apps/desktop/src-tauri/src/lib.rs`](../../apps/desktop/src-tauri/src/lib.rs) | Desktop startup that CP2 reshapes |
| [`apps/desktop/src-tauri/src/services/admin_server.rs`](../../apps/desktop/src-tauri/src/services/admin_server.rs) | Web admin startup to re-wire after CP2 |
| [`crates/mcpmux-storage/src/repositories/installed_server_repository.rs`](../../crates/mcpmux-storage/src/repositories/installed_server_repository.rs) | Plaintext overrides today, encrypted after CP8a |

---

## Related documentation

- [`upstream-client-mapping-reconciliation.md`](./upstream-client-mapping-reconciliation.md): the Jul 17 sync, migration renumbering precedent, `Unbound` vs `SpaceDefault`
- [`dev-to-main-port.md`](./dev-to-main-port.md): the first renumbering wave
- [`deny-by-default-bindable-callers.md`](./deny-by-default-bindable-callers.md): why `Unbound` must stay the deny terminus
- [`mux-followups-35-36-37.md`](./mux-followups-35-36-37.md): migration test rollback pattern, CI follow-ups
- [PR #14](https://github.com/crimsonsunset/mcp-mux/pull/14) and [PR #15](https://github.com/crimsonsunset/mcp-mux/pull/15): the fork's port of upstream #243
- [`remote-mcp-gateway-access.md`](../../../jsg-tech-check/docs/mcp-mux/remote-mcp-gateway-access.md) (jsg-tech-check): tunnel, CF Access apps, `remote:smoke`
- Upstream range: [`87df4a26...2e6a5b92`](https://github.com/mcpmux/mcp-mux/compare/87df4a26...2e6a5b92)
