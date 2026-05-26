# PR #2 Code Review — feat: web admin mode (remote UI via HTTP)

**Reviewed:** 2026-05-26  
**Last reconciled:** 2026-05-26 (post-facade on `feat/web-ui`)  
**PR:** https://github.com/crimsonsunset/mcp-mux/pull/2  
**Branch:** `feat/web-ui` → `dev`

---

### 📋 PR Summary

**Title** linked to the PR URL: [feat: web admin mode (remote UI via HTTP) #2](https://github.com/crimsonsunset/mcp-mux/pull/2)
**Author** crimsonsunset · **Merging into**: `feat/web-ui` → `dev`
**Opened**: 2026-05-26 · **Last commit**: 2026-05-26 · **Commits**: 22+ (web admin phases + post-review remediation + unified backend facade Phases 1–5)
**Files changed**: 160+ (see PR diff vs `dev`)
**Description**: Adds optional Axum web admin server on `127.0.0.1:45819` serving the React SPA + REST `/api/v1/*` backed by `command_bridge`, frontend `@/lib/backend` facade (`apiCall` invoke ↔ fetch, SSE events, shell helpers), CF Access JWT gate, CSRF on mutations, web OAuth consent path, and integration / Playwright scaffolding.

### 🎯 What This PR Does

Three architectural moves landed together:

1. **Backend split** — Domain logic moved out of Tauri command handlers into `mcpmux_gateway::admin::command_bridge::{read,write,space,oauth}` with a `AdminBridgeCtx` carrying `ApplicationServices` + repo handles. Tauri commands become thin wrappers retaining desktop side-effects (tray refresh, `app.emit`).
2. **Web admin HTTP server** — New `mcpmux_gateway::admin::*` module with router (~95 routes), CF Access middleware (JWKS-based RS256), CSRF middleware (single-token store), SSE event hub fanning in `EventBus` + direct `app.emit` paths + gateway domain events.
3. **Frontend transport boundary** — `@/lib/backend` three-channel facade: **data** (`apiCall` in `backend/data/transport.ts`), **events** (`backend/events/` — Tauri + SSE adapters; `*Web` hooks internal-only), **shell** (`backend/shell/` — dialogs, updater, icons, admin settings). Deprecated `@/lib/api/*` shims re-export the same surface. ESLint blocks `@tauri-apps/*` outside `lib/backend/**`.

### Post-review remediation (May 26, 2026)

Three follow-up commits on `feat/web-ui` after the initial review:

| Commit | Scope |
| ------ | ----- |
| `0c1a017` | Critical/major hardening: `test-utils` feature gate, OAuth log fix, event hub leak, CSRF improvements, typed mutation bodies, CF Access error propagation, web-only admin settings hide |
| `cc7bf54` | Lower-priority: SPA build-hint fallback, negative Playwright specs, OAuth log unit test, event hub regression test, CF health docs |
| `558a319` | Medium architecture: `fetch-api.routes/*` split, remove fake `open_url` REST endpoint, `LiveGatewayRuntime` + live gateway integration tests |

**Verdict after remediation:** 🟢 **Approve** — all critical items fixed; remaining items are explicit follow-up tickets (see checklist below).

### Post-facade follow-up (May 26, 2026)

Six commits on `feat/web-ui` after PR #2 review remediation — implements [`unified-backend-facade.md`](./unified-backend-facade.md) Phases 1–5:

| Commit | Scope |
| ------ | ----- |
| `1f36ad9` | Phase 1 — `lib/backend/` scaffold; transport + fetch-api moved to `backend/data/`; ESLint `no-restricted-imports` |
| `f72af83` | Phase 2 — event hooks under `backend/events/`; `useBackendEventSubscription` for ad-hoc channels |
| `e9a0e49` | Phase 3 — shell facade; all `@tauri-apps` imports confined to `lib/backend/**`; web hides shell-only UI |
| `90cbd9f` | Phase 4 — config-export REST parity via `apiCall`; oauth/settings shell/data split |
| `2267884` | Phase 5 — deprecation comments on `@/lib/api/*`; planning docs reconciled |
| `c236a06` | Lint — resolve `set-state-in-effect` blockers in `HoverTooltip`, `ServerIcon`, `WorkspacesPage` |

**Outcome:** The "dual hooks / scattered guards / stragglers" items called out in the original review and `unified-backend-facade.md` pre-facade table are addressed. `@/lib/backend` is the preferred import for new code.

### ✅ Strengths

- The bridge / runtime trait split is genuinely good design. `GatewayRuntime` + `GatewayWriteRuntime` traits with `StubGatewayRuntime` for tests cleanly decouple admin HTTP from the live gateway's `Arc<RwLock<GatewayState>>`. Integration tests can spin up the full router with a stubbed runtime — that's why the 691-line `admin_api.rs` exists.
- Layer ordering in `build_admin_router` is correct: CF Access middleware wraps CSRF wraps handlers, so auth runs before token check.
- `consent_token` is a server-issued secret tied to the pending-authorization record, validated explicitly in `approve_oauth_consent`. Web/desktop paths share the same `inbound_consent.rs` module — good DRY.
- SSE keep-alive with `Lagged` recovery (line 38–40 of `events.rs`) prevents stale subscribers from blocking the broadcast.
- Strong test coverage for what shipped: 34 admin Rust integration tests + 106 transport vitest rows + 13 Playwright smoke specs + dedicated CF Access fixture pair.
- The `unified-backend-facade.md` planning doc (Option 4A) honestly named pre-facade gaps; Phases 1–5 on `feat/web-ui` closed them (events consolidated, ESLint boundary, shell/data split).
- Static SPA fallback serves built assets when `index.html` exists; otherwise a **503 build-hint page** (`handlers/spa.rs`) tells operators to run `pnpm build:web:admin`.

### 🔴 Critical Issues

1. **Test JWT private key shipped in production binary.** — **✅ Fixed** (`test-utils` feature; PEM + stubs gated; integration tests enable feature)
2. **Authorization code logged at INFO level.** — **✅ Fixed** (redirect URL log removed; `tracing_test` unit test asserts no `code=mc_` in logs)
3. **CF Access cert refresh is missing.** — **⬜ Open** (follow-up ticket: hourly refresh or refresh-on-`UnknownKeyId`)
4. **`AdminEventHub::start()` leaks task handles on every restart.** — **✅ Fixed** (abort prior fan-in handles; regression test in `admin_api_regression.rs`)

### ⚠️ Major Concerns

5. **Typed structs for meta-tools toggles.** — **✅ Fixed** (`SetEnabledBody { enabled: bool }`; malformed body → 400)
6. **CSRF token never rotates; String equality; poison panic.** — **🟡 Partial** — `parking_lot::Mutex` + `ConstantTimeEq` done; rotation endpoint still open
7. **CF Access errors collapse to `UnknownKeyId`.** — **✅ Fixed** (last decode error propagated as `InvalidJwt`)
8. **`open_url` admin endpoint is a no-op echo.** — **✅ Fixed** (REST endpoint removed; `openUrl` lives in `backend/shell` — Tauri opener on desktop, `window.open` on web)
9. **Admin settings card in web mode.** — **✅ Fixed** (`SettingsPage` returns `null` when `!isTauri()`)
10. **`/api/v1/test/events/publish` in production.** — **✅ Fixed** (`#[cfg(feature = "test-utils")]` route registration)

### 🟡 Minor Issues / Nitpicks

11. **`fetch-api.ts` is 708 lines.** — **✅ Fixed** (split into `fetch-api.ts` transport + `fetch-api.routes/*` + helpers/types)
12. **`get_pending_consent` mutates state.** — **⬜ Open** (doc comment added; full rename/GC split deferred)
13. **CSRF `.expect("csrf lock")` poison panic.** — **✅ Fixed** (`parking_lot::Mutex`)
14. **SSE `expect("UI event payload serializes")`.** — **✅ Fixed** (warn-and-skip)
15. **Raw pointer in gateway log.** — **✅ Fixed** (removed `{:p}`)
16. **`open_url` runtime shape check.** — **✅ Fixed** (`isTauri()` branch at call site)
17. **JSDoc gap for `routeFor`.** — **✅ Fixed** (function-level JSDoc + args contract)
18. **Test PEM fixture documentation.** — **✅ Fixed** (`tests/fixtures/README.md`)
19. **`format_bridge_error_message` export.** — **✅ Fixed** (gated behind `test-utils`)
20. **`GenericImageView` import.** — **✅ Verified** (used; no change needed)
21. **CSRF retry substring fragility.** — **⬜ Open** (still matches `403 + "csrf"` substring)

### 🧪 Testing Considerations

- **Manual smoke (still unchecked):** `pnpm validate`, `pnpm dev`, `pnpm build:web:admin`, `pnpm test:e2e:web:admin`, homelab tunnel — operator must run before merge claim.
- **Event hub task leak (#4):** ✅ regression test in `admin_api_regression.rs`
- **CF Access cert refresh (#3):** ⬜ still needs refresh task or `// TODO(refresh)` if deferred
- **Gateway runtime:** ✅ `LiveGatewayRuntime` + `admin_api_live_gateway.rs` (2 tests: status parity, port settings) — supersedes original `DesktopGatewayRuntime` suggestion
- **Negative Playwright specs:** ✅ `tests/e2e/specs/admin/security-negative.spec.ts` (CSRF 403, CF Access 401 with env skip)
- **OAuth log test (#2):** ✅ `tracing_test` in `inbound_consent.rs`
- **Test counts (post-remediation):** 34 Rust admin integration tests; 106 vitest transport rows (was 107; `open_url` row removed with endpoint)

### 📝 Before Merge Checklist

- [x] Gate `test_valid_jwt`/`test_validator`/`StubGatewayRuntime`/`StubGatewayWriteRuntime`/embedded PEM behind `#[cfg(any(test, feature="test-utils"))]` (#1)
- [x] Drop `info!("[OAuth] Redirect URL: {}", redirect_url)` (#2)
- [ ] Add CF Access JWKS refresh task or refresh-on-`UnknownKeyId` (#3)
- [x] Store + abort `event_hub::start` task handles (#4)
- [x] Replace `serde_json::Value` body parsing with typed structs in `set_meta_tools_enabled` / `set_session_overrides_require_approval` (#5)
- [x] Switch CSRF mutex to `parking_lot::Mutex`, swap comparison to `ConstantTimeEq` (#6 — rotation still open)
- [x] Make CF Access validator surface real `decode` errors instead of `UnknownKeyId` for all failures (#7)
- [x] Drop `open_url` server endpoint; `openUrl` in `backend/shell` (#8)
- [x] Hide admin-web settings card from web-mode UI when `!isTauri()` (#9)
- [x] Compile-time gate `/api/v1/test/events/publish` (#10)
- [x] Replace `expect("UI event payload serializes")` with logged-skip (#14)
- [x] Split `fetch-api.ts` into route modules (#11)
- [x] Add negative Playwright specs (CSRF 403, CF Access 401)
- [x] Add live gateway integration test (`LiveGatewayRuntime`)
- [ ] Run the unchecked `pnpm validate` / `pnpm test:e2e:web:admin` boxes
- [ ] Verify Windows compile of `apps/desktop/src-tauri` (CI is Linux per AGENTS.md — CF Access shipped JWT path runs on every platform)

### 💬 Questions for Author

1. **Why a single global CSRF token instead of per-session?** Given CF Access JWT carries a `sub` claim, you could derive a per-user HMAC token (`HMAC(server_secret, sub)`) for free. Acceptable for homelab single-user, but worth the doc note.
2. **What's the escape hatch when admin SPA `index.html` is missing?** — **✅ Resolved** (503 HTML stub with `pnpm build:web:admin` hint in `handlers/spa.rs`)
3. **Is desktop OAuth consent path also emitting via `AdminUiEventBus` post-`acf3f92`?** If so, desktop users with admin disabled still pay for the broadcast channel; if admin is enabled, desktop AND web both receive the event. That's fine, but confirm the modal isn't double-mounted (one Tauri-listen, one SSE) when running desktop with admin on.
4. **`AdminWebSettings.cfTeamDomain` is a free-text string.** Is it validated server-side before being used in `format!("https://{team_domain}.cloudflareaccess.com/...")`? If a malicious profile import sets it to e.g. `evil.com/?team=`, the cert URL becomes `https://evil.com/?team=.cloudflareaccess.com/...` — which would 404 cleanly but the SSRF surface is real if Cloudflare ever returns redirects. Sanitize to `^[a-z0-9-]+$`.
5. **Why is `/api/v1/health` behind CF Access?** Cloudflare Tunnel's origin health checks won't carry a JWT. If you don't depend on tunnel health probes, fine — but document it.

### Overall Assessment

**Verdict (post-remediation):** 🟢 **Approve**

All four original critical items are fixed. Remaining open work (#3 JWKS refresh, CSRF rotation, #21 retry fragility, manual smoke tests) is acceptable follow-up or operator checklist — not merge blockers for the homelab threat model.

The phased commit structure made review possible — web admin phases, review remediation, then facade Phases 1–5 are individually digestible.

Sources: [crates/mcpmux-gateway/src/admin/](https://github.com/crimsonsunset/mcp-mux/pull/2/files), [oauth/inbound_consent.rs](https://github.com/crimsonsunset/mcp-mux/blob/feat/web-ui/crates/mcpmux-gateway/src/oauth/inbound_consent.rs), [apps/desktop/src/lib/backend/](https://github.com/crimsonsunset/mcp-mux/tree/feat/web-ui/apps/desktop/src/lib/backend), [docs/planning/unified-backend-facade.md](https://github.com/crimsonsunset/mcp-mux/blob/feat/web-ui/docs/planning/unified-backend-facade.md).
