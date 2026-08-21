# Clone Auth Header Config Editing

**Last Updated:** Aug 20, 2026
**Status:** Shipped — desktop and web admin. Phase 5 (`f024d9e`) un-stubbed `clone_server`, `set_server_display_name`, `is_clone_id_available`, `suggest_clone_suffix`, and `list_clone_dependents` in the admin bridge, closing the last desktop-only gap. Clone auth now works identically from web admin and desktop. Decision 4 pool half **shipped** (`dcc2977`, [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md)); `retry_connection` uses `reconnect_fresh` on both runtimes. Bonus fix landed in the same commit: `save_server_inputs` was silently dropping `display_name_override` on both runtimes (desktop's Tauri command never declared the param; `update_config()` has no such param either) and dropping `update_policy`/`pinned_version` on web admin specifically (hardcoded to `None, None`) — all three now forward correctly on both runtimes.
**Depends on:** Clone lineage (`cloned_from`, migration 021) and `manual_entry` install source — both already shipped
**Unblocks:** Editing/fixing auth headers on any `manual_entry` clone through the UI instead of raw `sqlite3`

---

## Problem

`posthog-personal-mesh`, a clone of `posthog-personal`, silently queried the **parent's** PostHog project (`345911`, "set-times-app") instead of its own (`501917`, "Mesh") — no error, just wrong data every call. Trying to fix it through the McpMux UI's Definition editor returned:

```
Server 'posthog-personal-mesh' not found in config
```

Traced live (DB inspection + code reading, not guessed):

- `clone_server()` (`crates/mcpmux-core/src/application/server.rs:374-443`) copies the source's `cached_definition` wholesale into the new row, including the definition's embedded `source: ServerSource` field (`crates/mcpmux-core/src/domain/server.rs:38,80`). If the parent was `UserSpace`-sourced, the clone's `cached_definition` inherits that same `source` tag even though the clone's own row lives only in SQLite (`installation_source: ManualEntry`).
- The Definition editor gates edit vs. read-only purely on that inherited tag — `isEditable = server.source.type === 'UserSpace'` (`ServerDefinitionModal.tsx:80`), `canEditDefinition={server.source.type === 'UserSpace'}` (`ServersPage.tsx:1967`) — never checking `installation_source`. So the UI shows an "editable" definition, but Save calls `updateServerInConfig()` → `update_server_in_config` (`apps/desktop/src-tauri/src/commands/space.rs:322`, mirrored in `crates/mcpmux-gateway/src/admin/command_bridge/space.rs:182`), which only reads/writes `spaces/*.json`. The clone has no key there → `"not found in config"`. Every `manual_entry` clone hits this, not just this one server.
- Separately, `clone_server()` copies definition/lineage but **not** `extra_headers`, `input_values`, `env_overrides`, `args_append`, or credentials (`server.rs:413-420`) — the new row starts with `extra_headers: {}`. There is **no runtime fallback to the parent's `extra_headers`** — `build_transport_config()` (`crates/mcpmux-gateway/src/pool/transport/resolution.rs:47-127`) only reads the clone's own row and merges `installed.extra_headers` last (line 127). The wrong-project behavior came from the clone's copied `cached_definition` carrying the parent's baked-in header/template values forward while the clone's own override column stayed empty — not from any live parent lookup. Working sibling `posthog-personal-gait` has both `Authorization` and `x-posthog-project-id` set in its **own** `extra_headers`, proving the override mechanism works fine once populated.
- The Configure modal (`ServersPage.tsx`, `save_server_inputs` → `update_config()` at `server.rs:217-269`) already has key/value editors for `extra_headers` (HTTP transports) and does correctly persist per-clone overrides — the gap is that nothing seeds or prompts for them at clone time, and nothing warns when they're left empty.
- After patching `cached_definition` directly via SQL, a plain client-side reconnect/retry returned "Connection closed"; only killing and relaunching `McpMux.app` picked up the change. Root cause at investigation: `PoolService::connect_server` returns early whenever `is_healthy()`, and `ServerConfigUpdated` was UI-toast-only. **Shipped since:** `ServerConfigUpdatedHandler` evicts + `reconnect_fresh`; `UserSpaceSync` emits the event; `LiveGatewayWriteRuntime::retry_connection` is implemented; Definition editor edit/save and the Configure-side seeding/warning UI are also implemented; web-admin `clone_server`/`set_server_display_name` and the other clone bridge stubs are also implemented (see Status above).

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Edit path for `manual_entry` clones | **Both**: add a DB-backed definition save path for clones (writes `cached_definition` on the `installed_servers` row instead of `spaces/*.json`), and tighten Configure/clone-wizard so headers are visible up front | Fixes the false-affordance bug (editor claims editable, save 404s) *and* the discoverability gap (headers only reachable by digging into Configure after the fact) |
| 2 | Clone creation: auth seeding | **Copy parent's `extra_headers` / `input_values`** into the new clone as editable starting values | Directly prevents the exact bug — a header-auth'd clone no longer starts silently blank; user still must swap the project-specific value but isn't starting from nothing |
| 3 | Empty/inherited-auth footgun | **Warn**, don't block | A banner/toast when a clone with empty `extra_headers` is enabled while its parent/definition required auth headers. Fail-closed was rejected — some clones legitimately don't need headers (e.g. stdio env-only auth), and a hard block would break those |
| 4 | Pool invalidation on save | **Auto-evict + reconnect on `ServerConfigUpdated`** | **Shipped** (`dcc2977`). Handler `reconnect_fresh` + `UserSpaceSync` emit. Clone definition-save still needs to emit the same event when that path lands |
| 5 | Scope | **Desktop + web admin parity, both required** | Clone auth is meant to work identically from the web admin UI, not just desktop. `retry_connection` already had parity (`reconnect_fresh` on both). Clone create/list/display-name in `command_bridge/{read,write}.rs` did not — **shipped** (`f024d9e`), closing that gap. Not tracked in `dev-to-main-port.md`; its Phase 6 is desktop-only cloning UI and never touches these bridge functions |
| 6 | Clone-time `source` rewrite | **Yes** — `clone_server()` rewrites the copied definition's embedded `source` to reflect the clone's own storage (not `UserSpace`) | Removes the root cause of the false editability signal at its origin, so the UI gate (`isEditable`/`canEditDefinition`) stays correct without needing to special-case `installation_source` everywhere it's checked |

---

## Scope

**In (all shipped):**
- `clone_server()`: rewrites `definition.source` on the copy; copies `extra_headers`/`input_values` from the source row into the new row
- DB-backed `update_definition()` for `manual_entry` rows (Tauri command + admin bridge command), targeting `installed_servers.cached_definition`
- `ServerDefinitionModal` / `ServersPage`: save dispatch on `installation_source === manual_entry` (hybrid gate, see Decision 6 note below); editability no longer purely `source.type`
- Clone wizard: read-only preview of parent header key names; Configure auto-opens post-clone with seeded values (in place of inline editable fields at creation time)
- Warning banner + toast when a clone with auth-requiring parent/definition has empty `extra_headers`
- `ServerConfigUpdated` gateway subscriber + `reconnect_fresh` — **shipped**
- `UserSpaceSync` emit + `LiveGatewayWriteRuntime::retry_connection` — **shipped**
- `command_bridge/write.rs`: `clone_server` and `set_server_display_name` un-stubbed, calling `ctx.services.server().clone_server(...)` / `.set_display_name_override(...)` — **shipped** (`f024d9e`)
- `command_bridge/read.rs`: `is_clone_id_available`, `suggest_clone_suffix`, `list_clone_dependents` un-stubbed the same way — **shipped** (`f024d9e`)
- Web admin UI: clone wizard (`CloneAccountModal.tsx`) and display-name field (Configure modal) already existed in the shared SPA with no `isTauri()` gating and were already wired to the bridge routes — no new frontend work needed, just backend un-stubbing
- Bonus fix in the same commit: `save_server_inputs` now forwards `display_name_override` (via a follow-up `set_display_name_override` call) and `update_policy`/`pinned_version` (previously hardcoded to `None, None` on web admin) on both desktop and web admin

**Out:**

| Item | Reason |
| ---- | ------ |
| Fail-closed enforcement (blocking enable/connect on missing headers) | Rejected in Decision 3 — would break legitimate no-auth clones; warn-only is the chosen behavior |
| Transport fingerprint check on `connect_server`'s healthy-reuse path | Not needed once `ServerConfigUpdated` reliably evicts on every config/definition write; revisit only if the event-driven path proves to miss cases in practice |
| Copying OAuth credentials / `credentials` table rows on clone | Separate concern — credentials are per-install by design (OAuth tokens shouldn't be shared across clones); only header/input overrides are in scope here |
| Fixing `docs/guide/gateway.mdx`'s outdated `server_id + sha256(config)` pooling description | Docs correction, unrelated to the actual code fix; flag separately |

---

## Architecture

### Clone-time fixes (`clone_server`)

```rust
// crates/mcpmux-core/src/application/server.rs, inside clone_server()
definition.id = new_server_id.clone();
definition.name = format!("{} ({})", source.display_name(), normalized_suffix);
definition.alias = Some(alias);
definition.source = ServerSource::ManualEntry; // new variant — add to enum at crates/mcpmux-core/src/domain/server.rs:80 (today only UserSpace/Bundled/Registry exist)

let server = InstalledServer::new(&space_id_str, &new_server_id)
    .with_definition(&definition)
    .with_source(InstallationSource::ManualEntry)
    .with_cloned_from(source_server_id)
    .with_display_name_override(display_name_override)
    .with_update_policy(source.update_policy)
    .with_pinned_version(source.pinned_version.clone())
    .with_extra_headers(source.extra_headers.clone())   // new — seed, still user-editable
    .with_input_values(source.input_values.clone())     // new — seed, still user-editable
    .with_enabled(false);
```

**Pre-flight confirmed:** `ServerSource` (`crates/mcpmux-core/src/domain/server.rs:80-91`) currently has only `UserSpace { space_id, file_path }`, `Bundled` (default), and `Registry { url, name }` — no `ManualEntry` variant. Add one (unit struct, no fields needed) mirroring `InstallationSource::ManualEntry`.

### Definition editability gate (frontend)

```typescript
// Before (ServerDefinitionModal.tsx:80, ServersPage.tsx:1967)
const isEditable = server.source.type === 'UserSpace';

// After — check installation storage, not definition provenance
// Field is snake_case on the view model (confirmed live usage: ServersPage.tsx:148,185,227,960,1331,1351,1766)
const isEditable =
  server.installation_source?.type === 'UserConfig' ||
  server.installation_source?.type === 'ManualEntry'; // manual_entry now routes to the new DB save path
```

Save dispatch branches on the same field: `UserConfig` → existing `updateServerInConfig()`; `ManualEntry` → new `updateClonedServerDefinition()`.

### New DB-backed definition save path

Mirrors the existing JSON path (`update_server_in_config`) but targets the row directly:

```rust
// New: crates/mcpmux-core/src/application/server.rs
pub async fn update_definition(
    &self,
    space_id: Uuid,
    server_id: &str,
    definition: ServerDefinition,
) -> Result<InstalledServer> {
    // require installation_source == ManualEntry — reject otherwise with a clear error
    // update cached_definition + server_name via existing update_cached_definition() or repo.update()
    // emit DomainEvent::ServerConfigUpdated
}
```

Exposed as a new Tauri command (desktop) and admin bridge command (web admin), parallel to `update_server_in_config` / `command_bridge/space.rs`.

### Pool invalidation on `ServerConfigUpdated`

**Shipped** in [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md): `ServerConfigUpdatedHandler` resolves transport and `reconnect_fresh`s enabled servers. `UserSpaceSync` emits `ServerConfigUpdated`. `LiveGatewayWriteRuntime::retry_connection` uses the same helper. A new `update_definition()` path still needs to emit that event when it lands.

### Auth-seeding + warning surfaces (frontend)

- Clone wizard step reads the parent's `extra_headers` keys (and any registry input schema requiring headers) and pre-fills them into the new clone's Configure state, editable before the clone is enabled.
- Enable/connect path checks: parent (or definition) declares required headers, clone's own `extra_headers` is empty for those keys → non-blocking warning banner ("This clone may be using the wrong credentials — review its headers in Configure").

---

## Files to Modify

| File | Change |
| ---- | ------ |
| [`crates/mcpmux-core/src/application/server.rs`](../../crates/mcpmux-core/src/application/server.rs) | `clone_server()`: rewrite `definition.source`, seed `extra_headers`/`input_values` from source. New `update_definition()` method for `manual_entry` rows, emits `ServerConfigUpdated` |
| [`crates/mcpmux-core/src/domain/server.rs`](../../crates/mcpmux-core/src/domain/server.rs) | Add `ServerSource::ManualEntry` unit variant (currently only `UserSpace`/`Bundled`/`Registry`, L80-91) |
| [`crates/mcpmux-core/src/application/user_space_sync.rs`](../../crates/mcpmux-core/src/application/user_space_sync.rs) | **Shipped:** emits `ServerConfigUpdated` on definition updates |
| [`crates/mcpmux-gateway/src/consumers/server_config_handler.rs`](../../crates/mcpmux-gateway/src/consumers/server_config_handler.rs) | **Shipped:** `reconnect_fresh` on enabled servers |
| [`crates/mcpmux-gateway/src/admin/write_runtime.rs`](../../crates/mcpmux-gateway/src/admin/write_runtime.rs) | **Shipped:** `retry_connection` → `reconnect_fresh` |
| [`crates/mcpmux-gateway/src/admin/command_bridge/space.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/space.rs) | New bridge command for `update_definition()`, parallel to the existing `update_server_in_config` (L182 error site) |
| [`apps/desktop/src-tauri/src/commands/space.rs`](../../apps/desktop/src-tauri/src/commands/space.rs) | New Tauri command wrapping `update_definition()`, parallel to `update_server_in_config` (L322 error site) |
| [`apps/desktop/src-tauri/src/commands/server_clone.rs`](../../apps/desktop/src-tauri/src/commands/server_clone.rs) | Thread through any new clone-time auth-seeding params if the wizard needs them at create time rather than post-clone Configure |
| [`apps/desktop/src/components/ServerDefinitionModal.tsx`](../../apps/desktop/src/components/ServerDefinitionModal.tsx) | Editability gate switches from `source.type === 'UserSpace'` (L80) to `installation_source`-based check; save dispatch branches to new DB path for `manual_entry` |
| [`apps/desktop/src/features/servers/ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) | `canEditDefinition` prop (L1967) updated to match; `server-changed` handler (L522) extended to also reconnect on `config_updated`; warning banner for empty-header clones |
| [`apps/desktop/src/features/servers/CloneAccountModal.tsx`](../../apps/desktop/src/features/servers/CloneAccountModal.tsx) | Add header/input seeding step or pre-fill Configure with parent's values post-clone |
| [`apps/desktop/src/lib/api/spaces.ts`](../../apps/desktop/src/lib/api/spaces.ts) | New API shim for the DB-backed definition update command |
| [`apps/desktop/src/lib/backend/events/useDomainEvents.ts`](../../apps/desktop/src/lib/backend/events/useDomainEvents.ts) | Confirm `config_updated` payload shape is sufficient for the new reconnect-on-save handler |
| [`crates/mcpmux-gateway/src/admin/command_bridge/write.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/write.rs) | Phase 5 **shipped** (`f024d9e`): un-stubbed `clone_server`, `set_server_display_name`; fixed `save_server_inputs` to forward `display_name_override`/`update_policy`/`pinned_version` |
| [`crates/mcpmux-gateway/src/admin/command_bridge/read.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/read.rs) | Phase 5 **shipped** (`f024d9e`): un-stubbed `is_clone_id_available`, `suggest_clone_suffix`, `list_clone_dependents` |
| [`apps/desktop/src-tauri/src/commands/server_clone.rs`](../../apps/desktop/src-tauri/src/commands/server_clone.rs) | Phase 5 reference implementation — the admin bridge calls the same `ServerAppService` methods this file already calls |

---

## Phases

### Phase 1 — Clone-time fixes: source rewrite + auth seeding — **shipped**

`clone_server()` rewrites `definition.source` and seeds `extra_headers`/`input_values` from the source row. Unit test `clone_server_rewrites_source_and_seeds_auth_headers` covers it.

**Outcome (landed):** A freshly cloned server starts with the parent's auth headers already in `extra_headers` (user still swaps project-specific values), and its Definition editor no longer falsely claims to be `UserSpace`-editable.

### Phase 2 — DB-backed definition edit path — **shipped**

`update_definition()` exists on `ServerAppService`, rejects non-`ManualEntry` rows, and emits `ServerConfigUpdated`. Wired through the Tauri command, the admin bridge command, and `ServerDefinitionModal`'s editability gate + save dispatch (hybrid check: `UserSpace` source type or `manual_entry` installation source).

**Outcome (landed):** Opening the Definition editor on any `manual_entry` clone allows edit + save, persisting to `installed_servers.cached_definition` instead of 404ing on `spaces/*.json`.

### Phase 3 — Pool invalidation on config/definition change — **shipped** (`dcc2977`)

See [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md). Remaining for *this* doc: the new definition-save path must emit `ServerConfigUpdated` so the shipped handler runs.

**Outcome (landed):** Configure / file-sync / `update_config` reconnects without an app relaunch. Manual Case G (admin PUT) is still BLOCKED on CF Access in the Aug 20 playbook.

### Phase 4 — Footgun warning UI — **shipped**

`hasCloneMissingAuthHeaders()` / `getExpectedCloneHeaderKeys()` in `ServersPage.tsx` drive a persistent banner plus a toast on enable/retry/refresh/reconnect. Clone wizard shows parent header key names (read-only) and auto-opens Configure with seeded values post-clone, instead of inline editable fields at creation time.

**Outcome (landed):** A clone left with genuinely empty required headers surfaces a visible warning instead of connecting silently against the wrong (or missing) credentials. No frontend test coverage for this warning logic yet.

### Phase 5 — Web admin clone parity — **shipped** (`f024d9e`)

Mechanical port of the desktop Tauri clone commands onto the admin bridge, exactly as scoped — no new logic:

- `command_bridge/write.rs`: `clone_server(ctx, body)` calls `ctx.services.server().clone_server(space_uuid, &body.source_server_id, &body.suffix, body.alias.as_deref(), body.display_name.as_deref())`; `set_server_display_name(ctx, id, body)` calls `.set_display_name_override(...)`
- `command_bridge/read.rs`: `is_clone_id_available`, `suggest_clone_suffix`, `list_clone_dependents` delegate to the existing service methods
- Ponytail "Phase 6" stub comments removed on all five
- Web admin frontend already had the clone wizard (`CloneAccountModal.tsx`) and display-name field (Configure modal) with no `isTauri()` gating, already wired to these bridge routes — confirmed no frontend work was needed
- Found in the process: `save_server_inputs` was silently dropping `display_name_override` on both runtimes (desktop's Tauri command never declared the param) and `update_policy`/`pinned_version` on web admin specifically (hardcoded to `None, None`) — fixed alongside the un-stubbing since it's the same code path
- Added 4 unit tests at the `ServerAppService` layer (`is_clone_id_available`, `suggest_clone_suffix`, `list_clone_dependents`, `set_display_name_override`) — no `command_bridge`-level test harness exists yet, so tests target the actual logic layer the bridge delegates to
- `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` all clean on `mcpmux-core`/`mcpmux-gateway`/desktop Tauri crate; `tests/ts/admin-transport.test.ts` (118 tests) unaffected

**Outcome (landed):** Cloning a server, checking clone-id availability, listing clone dependents, and overriding a display name all work identically from the web admin UI and from desktop. No `command_bridge` function returns "not yet available" for clone operations.

---

## Key Files Referenced

| File | Notes |
| ---- | ----- |
| [`crates/mcpmux-core/src/application/server.rs`](../../crates/mcpmux-core/src/application/server.rs) | `clone_server()` L374-443 (what is/isn't copied), `update_config()` L217-269 (existing override save path, unaffected by this fix) |
| [`crates/mcpmux-core/src/domain/server.rs`](../../crates/mcpmux-core/src/domain/server.rs) | `ServerSource` enum L80, `source` field L38 — inherited on clone today |
| [`crates/mcpmux-core/src/domain/installed_server.rs`](../../crates/mcpmux-core/src/domain/installed_server.rs) | `InstallationSource` enum L77-83 (`Registry` \| `UserConfig` \| `ManualEntry`), `source` field L159 |
| [`crates/mcpmux-core/src/domain/event.rs`](../../crates/mcpmux-core/src/domain/event.rs) | `ServerConfigUpdated` — gateway handler + UI toast |
| [`crates/mcpmux-core/src/application/user_space_sync.rs`](../../crates/mcpmux-core/src/application/user_space_sync.rs) | Emits `ServerConfigUpdated` on cached-definition updates |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | `build_transport_config()` L47-127 — confirms no parent/`cloned_from` lookup exists; `extra_headers` merge is the last step (L127) |
| [`crates/mcpmux-gateway/src/pool/service.rs`](../../crates/mcpmux-gateway/src/pool/service.rs) | `connect_server()` L267-297 — healthy-instance reuse (`reused: true` L291) skips config reload unless evicted first; `remove_instance()` L344 |
| [`apps/desktop/src-tauri/src/commands/server_manager.rs`](../../apps/desktop/src-tauri/src/commands/server_manager.rs) | `retry_connection()` L512-545 — existing evict-then-reconnect pattern this fix generalizes via the event subscriber |
| [`crates/mcpmux-gateway/src/admin/write_runtime.rs`](../../crates/mcpmux-gateway/src/admin/write_runtime.rs) | `retry_connection` implemented (`reconnect_fresh`) |
| [`crates/mcpmux-gateway/src/admin/command_bridge/write.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/write.rs) | `clone_server` / `set_server_display_name` un-stubbed — Phase 5 shipped (`f024d9e`) |
| [`crates/mcpmux-gateway/src/admin/command_bridge/read.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/read.rs) | `is_clone_id_available` / `suggest_clone_suffix` / `list_clone_dependents` un-stubbed — Phase 5 shipped (`f024d9e`) |
| [`apps/desktop/src-tauri/src/commands/space.rs`](../../apps/desktop/src-tauri/src/commands/space.rs) | `update_server_in_config` error site L322 — the exact "not found in config" message users hit today |
| [`crates/mcpmux-gateway/src/admin/command_bridge/space.rs`](../../crates/mcpmux-gateway/src/admin/command_bridge/space.rs) | Same error, web-admin bridge copy, L182 |
| [`apps/desktop/src/components/ServerDefinitionModal.tsx`](../../apps/desktop/src/components/ServerDefinitionModal.tsx) | `isEditable` gate L80 — root of the false-affordance bug |
| [`apps/desktop/src/features/servers/ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) | `canEditDefinition` L1967; `retryConnectionV2` call sites L1087,1184,1221,1446,1472; `server-changed` handler L522 (doesn't yet react to `config_updated`) |
| `~/Library/Application Support/com.mcpmux.desktop/mcpmux.db`, `installed_servers` table | `posthog-personal-mesh` (broken, empty `extra_headers`) vs `posthog-personal-gait` (working, both headers set) — reference rows for any migration/validation logic |
| [Diagnostic session transcript](08ac92fe-f240-4cd1-a0d3-755f654cb613) | Jul 21/22 debugging session that produced the raw-SQL workaround this fix replaces |

---

## Related Documentation

- [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md) — Decision 4 pool reconnect, shipped
- [`dev-to-main-port.md`](./dev-to-main-port.md) — original clone lineage work (migration 021, `cloned_from`), Phase 6 desktop cloning UI. Its Phase 6 does not cover the web-admin `command_bridge/{read,write}.rs` stubs — those were Phase 5 in this doc, now shipped
- [`resilience-routing-leftovers.md`](./resilience-routing-leftovers.md) — item 4 there tracked this same Phase 5 work, now shipped in both docs
- [`user-config-sync-collision-fix.md`](./user-config-sync-collision-fix.md) — separate `UserSpaceSyncService` bug fixed same layer; `ServerConfigUpdated` emission already shipped
- [`dev-rebased-post-port-completion.md`](./dev-rebased-post-port-completion.md) — QA checklist item "Clone account — independent config" this fix is meant to finally satisfy
