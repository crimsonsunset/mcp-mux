# Dev Workflow

**Last Updated:** Jun 1, 2026

This guide covers the daily dev loop for `mcp-mux/`: starting the app, recovering from stale processes, rebuilding after gateway changes, and using the admin mode. All commands run from the repo root (`mcp-mux/`).

---

## Port Map

| Port | Process | Notes |
| ---- | ------- | ----- |
| `45818` | MCP gateway (Axum) | AI clients connect here |
| `45819` | Web admin API | Dev admin mode or Cloudflare Tunnel target |
| `1420` | Vite HMR | Frontend hot-reload in dev mode |

`pnpm dev` frees all three ports via `predev` before starting.

---

## Standard Dev Cycle

```bash
pnpm dev          # start Tauri desktop app (Rust + React hot-reload)
```

`predev` runs `dev-env.mjs prep` automatically:
1. Quit any running McpMux app process
2. Kill orphaned Vite/Tauri processes from this repo
3. Wait for ports 1420, 45818, 45819 to be free

The Vite HMR server starts on `:1420` and the gateway starts on `:45818` as part of the Tauri dev session.

---

## Stopping and Restarting

```bash
pnpm dev:stop     # kill dev processes, free ports (no restart)
```

Use `dev:stop` when:
- Switching branches that change Rust code
- Freeing ports before running integration tests
- Recovering from a hung gateway

### After gateway crate changes

When you edit any `crates/mcpmux-gateway/` or `crates/mcpmux-core/` Rust code:

```bash
pnpm dev:restart       # stop + cargo build -p mcpmux-gateway -p mcpmux + pnpm dev
pnpm dev:restart:fast  # same but skips the cargo rebuild (if already built)
pnpm dev:rebuild       # cargo build only — no restart
```

`dev:restart` is the safe choice after Rust changes. It calls `cargo build -p mcpmux-gateway -p mcpmux` before relaunching so the running binary is fresh. Tauri hot-reload does not trigger Rust rebuilds.

---

## Admin Mode

The admin API (`:45819`) exposes the same data surface as Tauri commands over HTTP. Useful for:
- Developing the web admin UI without the Tauri shell
- Remote access via Cloudflare Tunnel

```bash
pnpm dev:admin          # Tauri dev + enable admin HTTP API on :45819
pnpm dev:web:admin      # Vite-only UI against a running admin API on :45819
pnpm dev:web            # Vite-only UI (requires VITE_ADMIN_WEB env; no Tauri)
```

`dev:admin` opens the HMR URL after startup. The admin API uses the same loopback-only posture as the gateway — bind to `127.0.0.1:45819`. Remote access is only through Cloudflare Tunnel + Cloudflare Access on a dedicated hostname.

---

## Logs

### Gateway logs (Rust)

Rust logs go to stderr in the Tauri dev terminal. The log level is controlled by `RUST_LOG`:

```bash
RUST_LOG=mcpmux_gateway=debug pnpm dev
RUST_LOG=mcpmux_gateway=trace,mcpmux_storage=debug pnpm dev
```

Default level in dev is `info`.

In production builds, logs go to the platform app-data directory:

| Platform | Path |
| -------- | ---- |
| macOS | `~/Library/Logs/McpMux/` |
| Windows | `%APPDATA%\McpMux\logs\` |
| Linux | `~/.local/share/McpMux/logs/` |

### Frontend logs

React logs appear in the Tauri webview console. Open it via **View → Toggle Developer Tools** in the running app (dev builds only).

### Admin API logs

When running `pnpm dev:admin`, admin API request logs appear in the same terminal as the gateway logs.

---

## Cursor MCP Reload

After changing gateway behavior that affects the tool list (new meta tool, FeatureSet member change, Surface toggle), Cursor must reload its tool descriptor cache:

1. **Cursor → Settings → MCP** (or `Ctrl+Shift+P` → "MCP: Reload tools")
2. Or restart the Cursor agent session

The gateway emits `notifications/tools/list_changed` when the tool set changes (via `MCPNotifier`). Whether Cursor auto-applies this depends on the client version. Until Cursor consistently handles `list_changed`, a manual reload is the reliable path.

---

## Validation Before PR

```bash
pnpm validate      # cargo fmt + clippy -D warnings + cargo check + eslint + typecheck
pnpm test:rust     # cargo nextest run --workspace
pnpm test:ts       # vitest run
```

The pre-commit hook runs `cargo clippy --workspace -- -D warnings`. CI runs on Linux — `#[cfg(unix)]` code is linted there, `#[cfg(windows)]` code is not. Always verify cross-platform conditional code compiles on both platforms before pushing.

---

## Useful One-Liners

```bash
# Run a single Rust integration test
pnpm test:rust:int -- <test_name>

# Run a single TypeScript test file
pnpm test:ts -- tests/ts/<file>.test.ts

# Check a specific Rust crate
cargo check -p mcpmux-gateway

# Watch TypeScript tests
pnpm test:ts:watch

# Coverage (Rust + TS)
pnpm test:coverage
```

---

## Related docs

- [`run-from-source.md`](./run-from-source.md) — first-time setup, prerequisites, initial build
- [`../technical/services-overview.md`](../technical/services-overview.md) — port roles and gateway internals
