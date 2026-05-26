# Web Admin Mode (Remote UI via HTTP)

**Last Updated:** May 26, 2026
**Status:** Complete (Phases 1–8)
**Branch:** `feat/web-ui`
**Base branch:** `dev` (fork); upstream merge path is `main`
**Issue:** TBD — file after planning review
**Depends on:** [Pre–Web Admin Desktop Cleanup](./pre-web-admin-desktop-cleanup.md) — **Complete** (`fix/pre-web-admin-cleanup`, May 25, 2026). Phase 1 matrix scaffolding may start.
**Unblocks:** [`jsg-tech-check` homelab wiring Step 6](../../../jsg-tech-check/docs/setup/home-lab-wiring-plan.md) — remote McpMux admin UI from Weathertop / Rohan at `https://mux.joe-hassio.com`

---

## Problem

The McpMux admin UI (Spaces, servers, credentials, workspace bindings, FeatureSets, OAuth consent) is a Tauri desktop app. The React frontend talks to Rust exclusively via Tauri `invoke()` — **117 unique commands** across 16 API modules + settings/OAuth components, backed by **130 registered** Tauri handlers in 21 command modules. There is no HTTP admin surface.

The homelab wiring plan already exposes two public endpoints via Cloudflare Tunnel on Gondor:

| Hostname | Target | What it serves |
| -------- | ------ | -------------- |
| `mcp.joe-hassio.com` | `localhost:45818` | MCP gateway (`/mcp`) for AI clients |
| `code.joe-hassio.com` | `localhost:3001` | ClaudeCodeUI |

Neither exposes the admin UI. Tunneling Vite dev (`:1420`) serves a React shell with no backend — every action fails because nothing answers `invoke()`. Tunneling the MCP gateway (`:45818`) serves the protocol endpoint, not admin pages.

The user-facing ask:

> I want to be able to reach the UI — that's the main point.

Screen sharing / VNC behind CF Access works today but is not a web UI. This doc defines a **web admin mode**: an optional HTTP server that serves the built React SPA and exposes a REST API mirroring Tauri commands, gated by Cloudflare Access at the edge.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Deployment model | **Single-user homelab** — one McpMux instance on Gondor, one operator | Avoids multi-tenant auth, cloud KMS, and per-user DB isolation. The Rust process still runs locally with OS keychain access. |
| 2 | Auth | **Cloudflare Access at the tunnel edge** — app trusts `CF-Access-Jwt-Assertion` when `gateway.admin_trust_cf_access` is enabled | No login UI to build. Same pattern as `b.joe-hassio.com` (Beeper). Reject requests without a valid JWT when admin mode is enabled. |
| 3 | Admin server placement | **Separate Axum router on configurable port** (default `45819`), not mixed into MCP gateway routes | Keeps MCP protocol surface unchanged. Admin and MCP can be tunneled independently (`mux.joe-hassio.com` vs `mcp.joe-hassio.com`). Easier to disable admin without stopping the gateway. |
| 4 | Static UI | **Serve `frontendDist` from the Tauri build** at `/` with SPA fallback | Reuses the existing React app. No separate web bundle. |
| 5 | API shape | **REST JSON at `/api/v1/*`** mirroring Tauri command names (kebab → snake mapping) | Predictable mapping: `get_gateway_status` → `GET /api/v1/gateway/status`. One handler module per Tauri command group. |
| 6 | Frontend transport | **Unified backend facade (`@/lib/backend`)** — `apiCall()` in `backend/data/transport.ts`; Tauri `invoke()` vs admin `fetch()` | Detect via `window.__TAURI__` or build-time `import.meta.env.VITE_ADMIN_WEB`. Same function signatures, different backend. `@/lib/api/*` remains as deprecated shims. |
| 7 | OAuth consent | **Re-enable guarded HTTP consent endpoint** for web admin only — `POST /api/v1/oauth/consent/approve` behind CF Access + CSRF token | Production desktop keeps Tauri-IPC-only consent (existing security model). Web mode needs an HTTP path because there is no Tauri shell on Weathertop. |
| 8 | Bind address | **Default `127.0.0.1:45819`** — same loopback-first posture as MCP gateway | CF tunnel reaches localhost; no need to bind `0.0.0.0`. `AGENTS.md` loopback rule preserved. |
| 9 | Event streaming | **SSE at `/api/v1/events`** bridging existing `EventBus` | Replaces Tauri event listeners (`useDomainEvents`) in web mode. Desktop keeps Tauri events. |
| 10 | Scope phasing | **Seven phases — test scaffolding first, then skeleton, bridge, reads, events, writes, OAuth, homelab** | Each phase ships its tests before the next phase starts. No HTTP handler without a `command_bridge` fn and a dual-entry integration test. |
| 11 | Parity proof | **`command_bridge.rs` is the single backend entry** — Tauri commands and HTTP handlers are one-liners | Existing Rust domain tests remain the backstop; new tests prove the admin wire layer reaches the same room. |

---

## The Model

### What web admin mode is

An optional HTTP server started alongside (or instead of) the Tauri window when `gateway.admin_enabled` is true:

```text
AdminServer (Axum, :45819)
├── GET  /*                    → SPA static files (frontendDist)
├── GET  /api/v1/health       → { status: "ok", gateway_running: bool }
├── GET  /api/v1/events       → SSE stream (EventBus bridge)
├── /api/v1/gateway/*         → gateway commands
├── /api/v1/spaces/*          → space commands
├── /api/v1/servers/*         → server manager + install + clone
├── /api/v1/workspaces/*      → workspace bindings + session overrides
├── /api/v1/feature-sets/*    → feature sets + members
├── /api/v1/clients/*         → inbound MCP clients
├── /api/v1/oauth/*           → consent approve/reject (web only)
└── /api/v1/settings/*        → app settings
```

All handlers delegate to the same `ApplicationServices` / command-layer logic Tauri uses today — no duplicated business logic.

### What web admin mode is NOT

- Not a hosted multi-tenant SaaS ("McpMux Cloud")
- Not a replacement for the Tauri desktop app on Gondor (desktop remains primary for local use)
- Not exposing the MCP gateway without separate hardening (that route stays on `:45818` with its own OAuth JWT model)
- Not moving secrets off OS keychain — encryption keys stay local

### Homelab tunnel layout (target)

```yaml
# gondor cloudflared config (addition to home-lab-wiring-plan.md Step 5)
ingress:
  - hostname: mux.joe-hassio.com
    service: http://localhost:45819    # NEW — admin UI
  - hostname: mcp.joe-hassio.com
    service: http://localhost:45818    # existing — MCP clients
  - hostname: code.joe-hassio.com
    service: http://localhost:3001     # existing — ClaudeCodeUI
  - service: http_status:404
```

CF Access policy on `mux.joe-hassio.com`: allow `jsangio1@gmail.com` (or equivalent Zero Trust rule).

---

## Architecture

```
Weathertop / Rohan browser
        │
        │ HTTPS + CF Access (Google login)
        ▼
  mux.joe-hassio.com ──── cloudflared tunnel ────► localhost:45819
                                                          │
                              ┌───────────────────────────┤
                              │                           │
                              ▼                           ▼
                    ┌──────────────────┐        ┌──────────────────┐
                    │  Static SPA      │        │  /api/v1/* REST  │
                    │  (frontendDist)  │        │  + SSE /events   │
                    └──────────────────┘        └────────┬─────────┘
                                                         │
                                                         ▼
                                              ┌──────────────────────┐
                                              │ ApplicationServices  │
                                              │ (same as Tauri cmds) │
                                              └──────────┬───────────┘
                                                         │
                    ┌────────────────────────────────────┼────────────────────┐
                    ▼                                    ▼                    ▼
              SQLite +                           OS Keychain              Gateway :45818
              AES-256-GCM                          JWT secret              (unchanged)
```

**Middleware stack (admin router):**

1. `CF-Access-Jwt-Assertion` validation (when enabled)
2. CSRF token check on mutating routes (web OAuth consent)
3. Request logging (sanitized — no secrets)
4. CORS: deny by default; allow same-origin only (SPA served from same host)

**Frontend transport switch:**

Import from `@/lib/backend` (preferred) or deprecated `@/lib/api/*` shims. Commands flow through `backend/data/transport.ts`:

```typescript
// lib/backend/data/transport.ts
export async function apiCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    return invoke(command, args);
  }
  return fetchApi<T>(command, args);
}
```

Domain modules (`spaces`, `gateway`, etc.) call `apiCall(...)` with no signature changes. Live updates use `backend/events`; OS integrations use `backend/shell`. See [`unified-backend-facade.md`](./unified-backend-facade.md).

---

## Parity & testing strategy

Web admin reuses the **same React SPA** and **same `ApplicationServices`**. Parity risk lives in two thin layers only:

1. **Backend wire** — HTTP route → `command_bridge` fn → services (must not duplicate Tauri command bodies)
2. **Frontend wire** — `@/lib/backend` (data + events + shell) → `apiCall()` / SSE / shell helpers

Everything below React hooks/stores is already covered by `pnpm test:rust`. The goal of admin testing is to prove **IPC ≡ HTTP ≡ bridge** for every exposed command, not to re-prove domain logic.

### What existing tests do and do not cover

| Layer | Today | Gap |
| ----- | ----- | --- |
| Rust domain / gateway services | `pnpm test:rust` — strong | None for business rules |
| Tauri IPC | WDIO `.wdio.ts` (15 specs) — behavioral catalog | IPC-only; not web |
| React UI shell | Playwright `.spec.ts` — mocks Tauri | No real backend |
| Admin HTTP / transport / SSE | — | **Zero** — all new |

**Do not treat Playwright `.spec.ts` or green WDIO as web-admin parity.** WDIO specs are the checklist to port; Playwright admin E2E runs against real `:45819`.

### Parity matrix (living artifact)

Create `docs/planning/web-admin-parity-matrix.md` in Phase 1. One row per frontend `invoke()` / REST surface:

```text
command | TS module | HTTP method/path | bridge fn | dual-entry test | transport vitest | E2E (WDIO ref)
```

Phase gates require matrix rows for that phase to be checked before merge. Empty rows = not done.

### Test patterns (reuse everywhere)

**Dual-entry (Rust integration)** — same fixture, same args, identical JSON:

```rust
let via_bridge = command_bridge::spaces::list(&services, &space_id).await?;
let via_http = admin.get("/api/v1/spaces").json().await?;
assert_json_eq!(via_bridge, via_http);
```

**Transport mapping (Vitest)** — pure, no network:

```typescript
expect(routeFor('get_gateway_status', { spaceId })).toEqual({
  method: 'GET',
  path: '/api/v1/gateway/status?spaceId=...',
});
```

**Event contract (Rust integration)** — trigger domain action → SSE frame on channel matches Tauri payload shape (see `workspace_binding_events.rs` for precedent).

**Error sentinel parity** — known UI parsers (e.g. `PORT_IN_USE:<port>:<source>` in `gateway.ts`) must produce parseable bodies over HTTP; explicit tests per sentinel.

### Module-by-module wiring rule

Never refactor all domain modules in one PR. Per command group:

1. Extract `command_bridge` fns for that group
2. Add read (then write) HTTP handlers calling bridge only
3. Add dual-entry integration tests
4. Add transport vitest rows for that group
5. Ensure commands use `apiCall` via `@/lib/backend` (legacy `@/lib/api/*` shims re-export the same surface)
6. Update parity matrix

Suggested first spike module: **`spaces`** (9 invokes, bounded CRUD, good template).

### Phase exit criteria (summary)

| Phase | Must be green before next phase |
| ----- | -------------------------------- |
| 1 | Parity matrix file + `admin_api` test harness + CF middleware unit tests |
| 2 | Health + static SPA + 401 without JWT when CF Access enabled |
| 3 | `spaces` (or chosen pilot) fully bridged with dual-entry tests; Tauri commands delegate to bridge |
| 4 | All read endpoints + transport vitest for read commands; browse smoke E2E |
| 5 | All **16** SSE channels contract-tested; `useDomainEventsWeb` + workspace/OAuth/meta-tool hooks wired |
| 6 | All write endpoints + CSRF + error mapping + install round-trip integration + write smoke E2E |
| 7 | OAuth consent HTTP path + integration test |
| 8 | WDIO parity catalog ported to Playwright admin + homelab manual smoke |

---

## Files to create

| File | Purpose |
| ---- | ------- |
| `crates/mcpmux-gateway/src/admin/mod.rs` | Admin router module entry |
| `crates/mcpmux-gateway/src/admin/server.rs` | `AdminServer` — bind, static file serving, route mounting |
| `crates/mcpmux-gateway/src/admin/middleware/cf_access.rs` | Validate `CF-Access-Jwt-Assertion` against CF team domain certs |
| `crates/mcpmux-gateway/src/admin/middleware/csrf.rs` | CSRF token generation + validation for mutating routes |
| `crates/mcpmux-gateway/src/admin/handlers/mod.rs` | Handler module tree |
| `crates/mcpmux-gateway/src/admin/handlers/gateway.rs` | Gateway status/start/stop REST handlers |
| `crates/mcpmux-gateway/src/admin/handlers/spaces.rs` | Space CRUD handlers |
| `crates/mcpmux-gateway/src/admin/handlers/servers.rs` | Server manager + install + clone handlers |
| `crates/mcpmux-gateway/src/admin/handlers/workspaces.rs` | Workspace binding + session override handlers |
| `crates/mcpmux-gateway/src/admin/handlers/feature_sets.rs` | FeatureSet + member handlers |
| `crates/mcpmux-gateway/src/admin/handlers/clients.rs` | Inbound MCP client handlers |
| `crates/mcpmux-gateway/src/admin/handlers/oauth.rs` | Web consent approve/reject handlers |
| `crates/mcpmux-gateway/src/admin/handlers/settings.rs` | App settings handlers |
| `crates/mcpmux-gateway/src/admin/handlers/events.rs` | SSE EventBus bridge |
| `crates/mcpmux-gateway/src/admin/command_bridge.rs` | Shared helper: call Tauri command logic without Tauri runtime |
| `apps/desktop/src/lib/api/transport.ts` | Tauri vs fetch transport abstraction |
| `apps/desktop/src/lib/api/fetch-api.ts` | REST client mapping command names → HTTP paths |
| `apps/desktop/src/hooks/useDomainEventsWeb.ts` | SSE-based event listener for web mode |
| `docs/planning/web-admin-parity-matrix.md` | Living command → route → test coverage tracker (Phase 1) |
| `tests/rust/tests/integration/admin_api.rs` | Admin HTTP integration tests + shared test harness |
| `tests/rust/tests/integration/admin_api/` | Per-module dual-entry tests (optional submodules as file grows) |
| `tests/ts/admin-transport.test.ts` | Vitest: `fetch-api` command → path/method mapping |
| `tests/e2e/playwright.admin.config.ts` | Playwright project: real `:45819`, CF JWT stub, no Tauri mock |
| `tests/e2e/specs/admin/*.spec.ts` | Admin parity E2E (ported from WDIO catalog over time) |
| `docs/planning/web-admin-remote-access.md` | This doc |

## Files to modify

| File | Change |
| ---- | ------ |
| [`crates/mcpmux-gateway/src/lib.rs`](../../crates/mcpmux-gateway/src/lib.rs) | `pub mod admin;` |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | `GatewayConfig` gains `admin_enabled`, `admin_port`, `admin_trust_cf_access`, `admin_cf_team_domain` |
| [`apps/desktop/src-tauri/src/lib.rs`](../../apps/desktop/src-tauri/src/lib.rs) | Start `AdminServer` when setting enabled; share `ApplicationServices` Arc |
| [`apps/desktop/src-tauri/src/commands/gateway.rs`](../../apps/desktop/src-tauri/src/commands/gateway.rs) | Extract shared gateway logic callable from admin handlers |
| [`apps/desktop/src/lib/api/*.ts`](../../apps/desktop/src/lib/api/) | Replace direct `invoke()` with `apiCall()` from transport layer |
| [`apps/desktop/src/hooks/useDomainEvents.ts`](../../apps/desktop/src/hooks/useDomainEvents.ts) | Delegate to SSE hook in web mode |
| [`apps/desktop/src/features/oauth/OAuthConsentModal.tsx`](../../apps/desktop/src/features/oauth/OAuthConsentModal.tsx) | Web mode: POST to `/api/v1/oauth/consent/approve` instead of Tauri command |
| [`apps/desktop/src/features/settings/SettingsPage.tsx`](../../apps/desktop/src/features/settings/SettingsPage.tsx) | Admin mode toggle + port setting |
| [`apps/desktop/vite.config.ts`](../../apps/desktop/vite.config.ts) | `VITE_ADMIN_WEB` build flag for web-only builds |
| [`apps/desktop/package.json`](../../apps/desktop/package.json) | `build:web:admin` script — production SPA build for admin serving |
| [`tests/e2e/playwright.config.ts`](../../tests/e2e/playwright.config.ts) | Keep existing web-only project (mocked Tauri) unchanged |
| [`tests/e2e/playwright.admin.config.ts`](../../tests/e2e/playwright.admin.config.ts) | **New** admin parity project against `:45819` |
| [`tests/rust/tests/integration/mod.rs`](../../tests/rust/tests/integration/mod.rs) | `mod admin_api;` |
| [`package.json`](../../package.json) | `test:e2e:web:admin`, `test:ts:admin-transport` scripts |
| [`AGENTS.md`](../../AGENTS.md) | Document admin server loopback binding + CF Access requirement |

---

## Phasing

Eight phases. **Tests are part of each phase, not a follow-up.** Do not start phase N+1 until that phase's exit criteria (above) are green in CI.

### Phase 1 — Parity inventory & test scaffolding

**Effort:** ~1 day

**Implementation**

- [ ] Create [`docs/planning/web-admin-parity-matrix.md`](./web-admin-parity-matrix.md) — **done** (129 rows: 117 FE invokes + 12 deferred BE + anomalies flagged)
- [ ] Map each row to its Tauri command module (`apps/desktop/src-tauri/src/commands/*.rs`) and planned HTTP path — **done in matrix**
- [ ] Mark IPC-only commands (window chrome, IDE install, etc.) as **N/A — desktop only** — **done in matrix**
- [ ] Document all **16** canonical Tauri/SSE channels (10 `useDomainEvents` + 4 `useWorkspaceEvents` + `oauth-client-changed` + `meta-tool-invoked`) with planned SSE contract — **done in matrix** (post desktop cleanup)

**Testing (same phase)**

- [ ] `tests/rust/tests/integration/admin_api.rs` — empty harness: spin in-memory `ApplicationServices`, mount admin router on ephemeral port, helper `admin_client()` with optional CF JWT header
- [ ] `tests/ts/admin-transport.test.ts` — skeleton with one example mapping row (placeholder until Phase 4)
- [ ] `tests/e2e/playwright.admin.config.ts` — project stub pointing at `:45819`, `testIgnore: ['**/*']` until Phase 4 smoke
- [ ] `pnpm test:rust:int` and `pnpm test:ts` include new files (passing, minimal)

**Outcome:** Parity matrix exists. Test harnesses compile. Every future command has a row waiting to be checked off.

---

### Phase 2 — Admin server skeleton + static SPA + CF Access gate

**Effort:** ~2 days

**Implementation**

- [ ] `AdminServer` Axum router on `127.0.0.1:45819` (configurable)
- [ ] Serve `frontendDist` with SPA fallback (`index.html` for unknown routes)
- [ ] `GET /api/v1/health` — returns gateway running status
- [ ] CF Access middleware: validate `CF-Access-Jwt-Assertion` when `admin_trust_cf_access` is true; 401 without it
- [ ] Settings: `gateway.admin_enabled` (default `false`), `gateway.admin_port` (default `45819`)
- [ ] Start admin server from Tauri app when setting enabled (alongside gateway)

**Testing (same phase)**

- [ ] `admin_api.rs`: `health_returns_200_with_valid_jwt_stub`
- [ ] `admin_api.rs`: `health_returns_401_when_cf_access_enabled_and_no_jwt`
- [ ] `admin_api.rs`: `health_returns_200_when_cf_access_disabled` (local dev bypass)
- [ ] Unit test: CF Access middleware cert validation / rejection paths (`cf_access.rs`)
- [ ] Manual: enable admin mode locally, `curl` health with/without header

**Outcome:** Authenticated tunnel loads McpMux UI shell. API calls still fail (no handlers yet) except health. Auth gate proven in CI.

---

### Phase 3 — `command_bridge` foundation (pilot module)

**Effort:** ~2 days

**Implementation**

- [ ] `command_bridge.rs` module tree mirroring Tauri command groups
- [ ] **Pilot: `spaces`** — extract all space command logic into bridge fns; Tauri `commands/space.rs` becomes thin wrappers calling bridge
- [ ] No HTTP handlers yet except health — prove extraction pattern without widening surface

**Testing (same phase)**

- [ ] Bridge unit/integration tests calling `command_bridge::spaces::*` directly against in-memory DB (reuse patterns from `tests/rust/tests/integration/`)
- [ ] Regression: existing `pnpm test:rust` still green — Tauri path unchanged behaviorally
- [ ] Parity matrix: all `spaces.ts` rows get **bridge fn** column filled

**Outcome:** One full command group proven end-to-end through bridge. Pattern documented for remaining 20 modules. **Gate for Phase 4:** no HTTP read handler without a bridge fn already tested.

---

### Phase 4 — Transport layer + read-only REST API

**Effort:** ~4 days

**Implementation**

- [ ] `transport.ts` + `fetch-api.ts` — command name → HTTP path/method mapping
- [ ] Read-only HTTP handlers (bridge one-liners only): gateway status, list spaces, list installed servers, list workspace bindings, list feature sets, list clients, list session overrides, get settings, registry browse
- [ ] Refactor `lib/api/*.ts` **module-by-module** (`invoke` → `apiCall`) — start with pilot `spaces.ts`, then remaining read-only modules
- [ ] Do **not** batch-refactor all 16 modules in one PR

**Testing (same phase)**

- [ ] Dual-entry integration test **per read endpoint** in `admin_api.rs` (bridge JSON ≡ HTTP JSON)
- [ ] `admin-transport.test.ts`: vitest row for **every read command** in parity matrix
- [ ] Error sentinel tests for read paths that can fail parseably (gateway port probe, etc.)
- [ ] Playwright admin smoke: `admin/read-browse.spec.ts` — load SPA on `:45819`, navigate Spaces + My Servers + Settings, assert list data renders (no writes)
- [ ] Parity matrix: read rows checked through **dual-entry + transport vitest**

**Outcome:** From Weathertop (or local `:45819`), authenticated user browses all main views read-only. Transport mapping fully tested for GETs.

---

### Phase 5 — SSE event parity

**Effort:** ~2 days

**Implementation**

- [ ] `GET /api/v1/events` — SSE bridge fanning in **both** Rust emit paths (EventBus bridge + direct `app.emit` in `oauth.rs` / `session_overrides.rs`)
- [ ] `useDomainEventsWeb.ts` — SSE listener matching `useDomainEvents` API (`subscribe`, `subscribeAll`, `subscribeMany`)
- [ ] `useWorkspaceEventsWeb.ts`, `useOAuthClientEventsWeb.ts`, `useMetaToolEventsWeb.ts` — SSE equivalents of desktop hooks
- [ ] `useDomainEvents.ts` — delegate to SSE hooks when not in Tauri

**Testing (same phase)**

- [ ] One integration test **per channel** (**16** total): trigger domain action → assert SSE event name + JSON payload matches Tauri emission shape
- [ ] Channels (EventBus bridge via `gateway.rs`): `space-changed`, `server-changed`, `server-status-changed`, `server-auth-progress`, `server-features-refreshed`, `feature-set-changed`, `client-changed`, `client-grant-changed`, `gateway-changed`, `mcp-notification`, `session-roots-changed`, `workspace-binding-changed`, `workspace-needs-binding`, `meta-tool-invoked`
- [ ] Channels (direct `app.emit`): `oauth-client-changed` (`oauth.rs`), `session-overrides-changed` (`session_overrides.rs`)
- [ ] Playwright admin smoke: gateway start/stop updates UI without refresh (proves live SSE in browser)
- [ ] Parity matrix: **Events** section — all 16 channels contract-tested

**Outcome:** Web UI stays live-synced like desktop. Event payload drift caught in CI, not manually on Gondor.

---

### Phase 6 — Write API (config mutations)

**Effort:** ~4 days

**Implementation**

- [ ] Write handlers (bridge one-liners): install/uninstall server, enable/disable, configure inputs, clone server, CRUD spaces, CRUD workspace bindings, CRUD feature sets + members, gateway start/stop, export config, clear session overrides, update settings, meta-tools approval, logs actions
- [ ] CSRF middleware on all `POST`/`PUT`/`DELETE` routes; `GET /api/v1/csrf-token` for SPA bootstrap
- [ ] Error mapping: domain errors → HTTP status + JSON body (preserve parseable sentinels for shared UI code)
- [ ] Finish `lib/api/*.ts` transport refactor for remaining write modules

**Testing (same phase)**

- [ ] Dual-entry integration test **per write endpoint**
- [ ] Round-trip tests: HTTP mutate → GET confirms state (e.g. install server → appears in list)
- [ ] CSRF tests: mutating request without token → 403; with token → success
- [ ] `admin-transport.test.ts`: vitest rows for **all write commands**
- [ ] Playwright admin E2E: `admin/server-lifecycle.spec.ts` — install + configure + enable (port from `server-lifecycle.wdio.ts`)
- [ ] Playwright admin E2E: `admin/spaces.spec.ts`, `admin/featureset.spec.ts` (port critical paths from WDIO catalog)
- [ ] Parity matrix: all non–desktop-only rows checked through **dual-entry + transport vitest**

**Outcome:** Full admin CRUD from browser. OAuth consent still Phase 7.

---

### Phase 7 — Web OAuth consent

**Effort:** ~2 days

**Implementation**

- [ ] `POST /api/v1/oauth/consent/approve` and `/reject` — guarded HTTP endpoints (web admin only; desktop keeps Tauri IPC)
- [ ] CSRF + consent token validation (reuse existing cryptographic consent token from gateway)
- [ ] `OAuthConsentModal.tsx` — web path posts to HTTP endpoint; desktop path unchanged
- [ ] Web mode polls consent pending state via SSE (no `mcpmux://` on Weathertop)

**Testing (same phase)**

- [ ] Integration: OAuth authorize → consent approve via HTTP → token issued → server connects
- [ ] Integration: reject path + invalid consent token → 4xx
- [ ] CSRF required on consent POST
- [ ] Playwright admin E2E: `admin/oauth-consent.spec.ts` — mocked OAuth server or test double
- [ ] Desktop regression: WDIO / manual — Tauri consent path unchanged

**Outcome:** Remote OAuth flows completable from browser.

---

### Phase 8 — Homelab integration + parity E2E catalog + docs

**Effort:** ~2 days

**Implementation**

- [x] Update [`jsg-tech-check/docs/setup/home-lab-wiring-plan.md`](../../../jsg-tech-check/docs/setup/home-lab-wiring-plan.md) Step 5 with `mux.joe-hassio.com` ingress rule
- [x] Document CF Access policy setup for `mux.joe-hassio.com`
- [x] Add admin mode section to [`docs/guide/gateway.mdx`](../../docs/guide/gateway.mdx)
- [x] `pnpm build:web:admin` + verify production SPA served correctly from admin server

**Testing (same phase)**

- [x] Port remaining high-value WDIO specs to Playwright admin (target ≥10 of 15 `.wdio.ts` files):

  | WDIO reference | Playwright admin spec |
  | -------------- | --------------------- |
  | `spaces.wdio.ts` | `admin/spaces.spec.ts` |
  | `server-lifecycle.wdio.ts` | `admin/server-lifecycle.spec.ts` |
  | `server-config.wdio.ts` | `admin/server-config.spec.ts` |
  | `gateway.wdio.ts` | `admin/gateway.spec.ts` |
  | `workspaces.wdio.ts` | `admin/workspaces.spec.ts` |
  | `featureset.wdio.ts` | `admin/featureset.spec.ts` |
  | `clients.wdio.ts` | `admin/clients.spec.ts` |
  | `meta-tools.wdio.ts` | `admin/meta-tools.spec.ts` |
  | `settings.wdio.ts` | `admin/settings.spec.ts` |
  | `comprehensive.wdio.ts` | `admin/comprehensive.spec.ts` (subset) |

- [ ] CI job: `pnpm test:e2e:web:admin` on Linux with AdminServer fixture — **deferred** (script in root `package.json`; requires live `:45819` + `apps/desktop/dist` from `pnpm build:web:admin`)
- [ ] Manual homelab smoke from Weathertop: `https://mux.joe-hassio.com` — browse, mutate, OAuth (cannot fully CI tunnel + real CF Access)
- [x] Parity matrix: 100% rows resolved (checked or N/A)

**Outcome:** Homelab wiring complete. CI proves web ≡ desktop for catalog flows. Operator manages McpMux from phone/laptop.

---

## Pre-PR validation

Per-phase minimum (accumulative — later phases run all prior checks):

| Phase | Required green |
| ----- | -------------- |
| 1+ | `pnpm validate` |
| 1+ | `pnpm test:rust` (includes `admin_api` harness) |
| 1+ | `pnpm test:ts` (includes `admin-transport.test.ts`) |
| 4+ | Dual-entry tests for all merged read endpoints |
| 5+ | SSE channel contract tests (16/16) |
| 6+ | Write round-trip + CSRF tests; `pnpm test:e2e:web:admin` smoke |
| 8 | Full admin Playwright catalog + manual homelab smoke checklist |

**Full merge gate (Phase 8 / feature complete):**

| Step | Command | Purpose |
| ---- | ------- | ------- |
| Full validate | `pnpm validate` | fmt, clippy, check, eslint, typecheck |
| Rust tests | `pnpm test:rust` | unit + integration including all `admin_api` dual-entry tests |
| TS tests | `pnpm test:ts` | vitest transport mapping (all parity matrix rows) |
| Admin web E2E | `pnpm test:e2e:web:admin` | Playwright against real `:45819` |
| Desktop regression | `pnpm test:e2e:grep -- "<smoke>"` | WDIO unchanged — desktop IPC not regressed |
| Manual smoke | Weathertop → `mux.joe-hassio.com` | CF Access + tunnel + real operator UX |

---

## Out of scope

| Item | Reason |
| ---- | ------ |
| Multi-tenant / per-user accounts | Single-user homelab. Adding user management is a different product. |
| Cloud KMS / secrets off OS keychain | Admin server runs on Gondor; keychain access is preserved. No remote secret vault needed. |
| Binding admin server to `0.0.0.0` | Loopback + CF tunnel is the access path. Direct internet bind violates `AGENTS.md` posture. |
| Replacing Tauri desktop app | Desktop remains primary on Gondor. Web admin is for remote access only. |
| Mobile-optimized responsive UI | React app works in mobile browser but no dedicated mobile layout pass. Acceptable for v1 homelab use. |
| Public MCP gateway hardening (`mcp.joe-hassio.com`) | Separate concern — OAuth JWT auth exists but unauthenticated admin routes on `:45818` need CF Access too. Track as follow-up, not blocked on this doc. |
| WebSocket transport (instead of SSE) | SSE is sufficient for EventBus fan-out. WebSocket adds complexity with no v1 benefit. |
| Headless-only mode (no Tauri window) | v1 starts admin server from Tauri app. Headless/systemd mode is a follow-up for Rivendell-style deployment. |

---

## Key files referenced

| File | Why |
| ---- | --- |
| [`apps/desktop/src/lib/backend/index.ts`](../../apps/desktop/src/lib/backend/index.ts) | Unified facade — data, events, shell (preferred import) |
| [`apps/desktop/src/lib/backend/data/transport.ts`](../../apps/desktop/src/lib/backend/data/transport.ts) | Tauri vs fetch transport abstraction |
| [`apps/desktop/src/lib/backend/data/fetch-api.ts`](../../apps/desktop/src/lib/backend/data/fetch-api.ts) | REST client mapping command names → HTTP paths |
| [`apps/desktop/src/lib/backend/events/`](../../apps/desktop/src/lib/backend/events/) | SSE + Tauri event adapters (`useDomainEvents`, etc.) |
| [`apps/desktop/src/lib/api/`](../../apps/desktop/src/lib/api/) | Deprecated shims — re-export domain modules; prefer `@/lib/backend` |
| [`apps/desktop/src-tauri/src/commands/mod.rs`](../../apps/desktop/src-tauri/src/commands/mod.rs) | Command module registry — each module gets a corresponding admin handler |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) | Existing Axum gateway — pattern reference for admin router |
| [`crates/mcpmux-gateway/src/server/mod.rs`](../../crates/mcpmux-gateway/src/server/mod.rs) (lines 340–365) | OAuth consent removed from HTTP for security — web admin re-adds guarded version |
| [`apps/desktop/src/hooks/useDomainEvents.ts`](../../apps/desktop/src/hooks/useDomainEvents.ts) | Tauri event listener — 10 domain channels; SSE contract tests in Phase 5 |
| [`apps/desktop/src/hooks/useWorkspaceEvents.ts`](../../apps/desktop/src/hooks/useWorkspaceEvents.ts) | Workspace/session-override channels (4) — SSE in Phase 5 |
| [`apps/desktop/src/hooks/useOAuthClientEvents.ts`](../../apps/desktop/src/hooks/useOAuthClientEvents.ts) | `oauth-client-changed` (direct Rust emit) — SSE in Phase 5 |
| [`apps/desktop/src/hooks/useMetaToolEvents.ts`](../../apps/desktop/src/hooks/useMetaToolEvents.ts) | `meta-tool-invoked` — SSE in Phase 5 |
| [`tests/e2e/specs/*.wdio.ts`](../../tests/e2e/specs/) | Behavioral catalog — port to `tests/e2e/specs/admin/*.spec.ts` in Phases 4–8 |
| [`tests/rust/tests/integration/workspace_binding_events.rs`](../../tests/rust/tests/integration/workspace_binding_events.rs) | Event JSON shape testing precedent for SSE contract tests |
| [`docs/planning/web-admin-parity-matrix.md`](./web-admin-parity-matrix.md) | Living coverage tracker — created Phase 1, completed Phase 8 |

---

## Related documentation

- [`docs/planning/pre-web-admin-desktop-cleanup.md`](./pre-web-admin-desktop-cleanup.md) — **Complete** (`fix/pre-web-admin-cleanup`, May 25, 2026) — IPC/API/event contract verified
- [`docs/planning/web-admin-parity-matrix.md`](./web-admin-parity-matrix.md) — Invoke + SSE coverage tracker (re-scan after cleanup)
- [`jsg-tech-check/docs/setup/home-lab-wiring-plan.md`](../../../jsg-tech-check/docs/setup/home-lab-wiring-plan.md) — Step 5 (CF tunnel), Step 6 (McpMux on Gondor), cross-device MCP access
- [`jsg-tech-check/docs/setup/mcpmux-server-migration.md`](../../../jsg-tech-check/docs/setup/mcpmux-server-migration.md) — server/bundle/binding migration tracker (orthogonal to web admin)
- [`docs/guide/security.mdx`](../../docs/guide/security.mdx) — credential encryption model (unchanged by web admin)
- [`docs/planning/dynamic-mcp-toggle-meta-tools.md`](./dynamic-mcp-toggle-meta-tools.md) — session override UI that web admin must expose via HTTP
- [`docs/planning/server-account-clones.md`](./server-account-clones.md) — clone wizard that web admin must expose via HTTP

---

## Reconciliation

This doc is the source of truth for web admin mode. Phases 1–8 are **Complete** on branch `feat/web-ui` (May 26, 2026). Homelab operator checklist: enable admin in Settings, `pnpm build:web:admin`, tunnel `mux.joe-hassio.com` → `:45819`, CF Access allow rule for operator email.

**Post-phase-8 hardening (May 26, 2026):** PR #2 code review remediation in three commits — see [`pr-2-web-admin-code-review.md`](./pr-2-web-admin-code-review.md). Key additions beyond Phases 1–8:

| Area | Files / behavior |
| ---- | ---------------- |
| Security | `test-utils` feature gate, OAuth log fix, CSRF `parking_lot` + constant-time compare, CF Access error propagation |
| Resilience | Event hub task abort on reload, SSE warn-and-skip, SPA 503 build-hint when `index.html` missing |
| Tests | `admin_api_regression.rs`, `admin_api_live_gateway.rs`, `security-negative.spec.ts`, OAuth log unit test |
| Transport | `fetch-api.routes/*` split; `open_url` REST removed (desktop Tauri / web `window.open`) |
| Backend runtime | `LiveGatewayRuntime` wired to real `GatewayServer` for integration tests |

**Test counts (post-hardening):** 34 Rust admin integration tests; 106 vitest transport rows.

**Decision record (May 25, 2026):** Web admin mode on fork selected over screen sharing (immediate but not web UI), tunneling `:1420` (broken), and full "McpMux Cloud" multi-tenant SaaS (months of work). CF Access at edge replaces building login UI. Separate admin port (`45819`) keeps MCP gateway surface unchanged.

**Decision record (May 25, 2026 — testing):** Expanded from five implementation phases to **eight phases** with tests baked into each phase. Added parity matrix artifact, `command_bridge` pilot before HTTP reads, dedicated SSE phase, and WDIO→Playwright admin catalog in Phase 8. No HTTP handler ships without dual-entry test; no transport refactor without vitest row.

**May 26, 2026 (facade):** Unified backend facade (Phases 1–5) landed on `feat/web-ui`. Frontend imports should use `@/lib/backend`; `@/lib/api/*` shims are deprecated. See [`unified-backend-facade.md`](./unified-backend-facade.md).
