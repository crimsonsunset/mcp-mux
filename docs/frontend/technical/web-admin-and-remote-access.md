# Web Admin and Remote Access

**Synthesis of:** [`docs/frontend/reference/web-admin-parity-matrix.md`](../reference/web-admin-parity-matrix.md)

McpMux exposes an optional HTTP admin server on `127.0.0.1:45819` that serves the same React SPA and a REST API backed by `command_bridge`. This enables remote management through a Cloudflare Tunnel + Access policy without binding the port on the LAN.

---

## Architecture

```
Browser (remote)
    │
    ▼
Cloudflare Access (JWT validation)
    │
    ▼
Cloudflare Tunnel → localhost:45819
    │
    ├── GET /                → SPA (React, built with pnpm build:web:admin)
    ├── GET /api/v1/*        → command_bridge read handlers
    ├── POST/PUT/DELETE /api/v1/* → command_bridge write handlers (CSRF required)
    └── GET /api/v1/events   → SSE hub (16 channels)
```

The admin server and the MCP gateway (`localhost:45818`) are independent processes. The admin server never handles MCP protocol traffic.

---

## Admin HTTP server

The server is started by `start_admin_web_server` Tauri command when web admin is enabled in Settings. It is desktop-only: `backend.shell.admin-settings` reads and writes the configuration; web clients cannot toggle their own server.

Key behaviors:

- **Loopback only** — binds `127.0.0.1:45819`, never `0.0.0.0`.
- **SPA fallback** — serves built `index.html` for unknown paths; returns a 503 build-hint page if no build exists (run `pnpm build:web:admin` to generate it).
- **CSRF** — all mutating routes (`POST`, `PUT`, `DELETE`) require a `X-CSRF-Token` header matching the server-issued token. Single-token store using `parking_lot::Mutex` + constant-time comparison.
- **CF Access JWT gate** — when `gateway.admin_trust_cf_access` is enabled, every request must carry a valid `CF-Access-Jwt-Assertion` header. The validator fetches the team domain's JWKS endpoint and verifies RS256 signatures. Audience validation is gated on `audience` being configured.

---

## Cloudflare Tunnel + Access setup

Remote access is via Cloudflare Tunnel — not by binding `:45819` on the LAN. A `cloudflared` tunnel process forwards `mux.yourdomain.com` → `localhost:45819`. Cloudflare Access policies sit in front of the tunnel, issuing JWTs to authenticated users.

Configuration in McpMux Settings:

| Field               | Purpose                                                       |
| ------------------- | ------------------------------------------------------------- |
| Enable web admin    | Starts the `:45819` server                                    |
| Trust CF Access     | Enables JWT validation on every request                       |
| CF team domain      | The `<team>.cloudflareaccess.com` prefix — used to fetch JWKS |
| Audience (optional) | If set, JWT `aud` claim is validated                          |

**Important:** populate the CF team domain field _before_ enabling the trust toggle. The toggle persists immediately and the backend rejects "team domain required" if the field is empty at that point.

---

## SSE event channels

The SSE hub at `/api/v1/events` fans in two Rust emit paths:

| Path                           | Channels                                                                                                                                                                                                                                                                                                                                    |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| EventBus → `gateway.rs` bridge | 14 channels (`space-changed`, `server-changed`, `server-status-changed`, `server-auth-progress`, `server-features-refreshed`, `feature-set-changed`, `client-changed`, `client-grant-changed`, `gateway-changed`, `mcp-notification`, `session-roots-changed`, `workspace-binding-changed`, `workspace-needs-binding`, `meta-tool-invoked`) |
| Direct `app.emit`              | `oauth-client-changed` (`oauth.rs`), `session-overrides-changed` (`session_overrides.rs`)                                                                                                                                                                                                                                                   |

All 16 channels are consumed by the same `useDomainEvents` hook via the SSE adapter in `backend/events/subscribe.ts`. Desktop Tauri and web admin receive identical events through different transports.

The complete channel table (with Rust source and hook consumer for each) is in [`web-admin-parity-matrix.md`](../reference/web-admin-parity-matrix.md#sse-event-channels-phase-5).

---

## Command parity

104 of 125 commands have full REST parity. Categories:

| Category           | Count | Notes                                                                                                                          |
| ------------------ | ----- | ------------------------------------------------------------------------------------------------------------------------------ |
| REST (direct)      | 104   | `apiCall` routes to `fetch` in web context                                                                                     |
| REST (web variant) | 6     | Behavior differs slightly (e.g. `get_bundle_version` returns a web string; `open_url` uses `window.open`)                      |
| Desktop-only       | 5     | `add_to_cursor`, `add_to_vscode`, `flush_pending_deep_link`, `open_logs_folder`, `open_space_config_file` — no HTTP equivalent |
| Deferred           | 10    | Backend-only or no FE consumer                                                                                                 |

Desktop-only commands live in `backend.shell` and are never exposed over HTTP. UI surfaces that use them are hidden when `!isTauri()`.

The full command matrix (TS source, Rust module, HTTP method, route, bridge function) is in [`web-admin-parity-matrix.md`](../reference/web-admin-parity-matrix.md#commands).

---

## Security notes

- The test JWT private key is gated behind `#[cfg(any(test, feature = "test-utils"))]` and never compiled into release builds.
- CF Access cert refresh on `UnknownKeyId` is a known open item — a refresh task or refresh-on-unknown-key mechanism is needed for long-running instances.
- CSRF token rotation is not yet implemented (single global token). Acceptable for single-user homelab deployments; revisit for multi-user setups.
- `AdminWebSettings.cf_team_domain` should be validated to `^[a-z0-9-]+$` to prevent SSRF via a crafted team domain string in a profile import.

---

## Development

```bash
# Build the admin SPA (required before using web admin)
pnpm build:web:admin

# Start Tauri dev with admin enabled
pnpm dev:admin

# Start Vite only against a running admin server
pnpm dev:web:admin

# Run Playwright admin catalog tests (requires admin server on :45819)
pnpm test:e2e:web:admin
```

See [`docs/backend/guides/dev-workflow.md`](../../backend/guides/dev-workflow.md) for port map and dev:stop / dev:rebuild flows.
