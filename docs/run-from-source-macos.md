# Run McpMux from Source (macOS)

Two flows for working against this repo, picked by what you're doing:

| Flow | Use when | Speed | Cursor / Claude / VS Code see it? |
| ---- | -------- | ----- | --------------------------------- |
| **Dev watch mode** (`pnpm dev`) | Iterating on UI or Rust — you want HMR for React and auto-recompile for Tauri commands | Vite HMR is instant; Rust changes ~5–15s incremental | Yes — same `localhost:45818` endpoint while `pnpm dev` is running |
| **Web admin dev** (`pnpm dev:admin` / `pnpm dev:web:admin`) | Full UI in the browser over HTTP (`fetch` + SSE), same DB as desktop — remote/homelab parity | HMR on `:1420` when using Vite; `:45819` needs `build:web:admin` for static SPA | MCP gateway still `:45818`; admin UI is separate |
| **Build + swap** (replace `/Applications/McpMux.app`) | You want a real installed app on this branch — autostart, system tray, runs without a terminal, survives reboot | Full build ~5–10 min, incremental ~1–3 min | Yes — and stays running after you close your editor |

Quick rule of thumb: **`pnpm dev` while you're coding, swap when you're done** so other AI clients keep working when Cursor isn't open.

---

## What survives between flows

Both dev mode and a swapped `.app` use the same `com.mcpmux.desktop` bundle identifier, so they share data:

| Data | Location |
| ---- | -------- |
| SQLite DB (spaces, servers, clients, settings) | `~/Library/Application Support/com.mcpmux.desktop/mcpmux.db` |
| Per-space files | `~/Library/Application Support/com.mcpmux.desktop/spaces/` |
| Logs | `~/Library/Application Support/com.mcpmux.desktop/logs/` |
| Encryption master key | macOS Keychain (`com.mcpmux.desktop` service) |
| OAuth tokens / credentials | Encrypted in SQLite + Keychain |

The new binary reads the same data dir and keychain entries as the release. Spaces, server installs, and access keys persist across `pnpm dev` ↔ `/Applications` swaps.

**What you might need to redo:** OAuth re-auth in Cursor/Claude Desktop if DCR or token validation changed on your branch.

---

## Prerequisites

From repo root (`mcp-mux/`):

- Rust 1.75+
- Node.js 20+
- pnpm 9+
- Xcode Command Line Tools (`xcode-select --install`)

First-time setup (if deps aren't installed):

```bash
pnpm install
```

---

## Flow 1 — Dev watch mode (`pnpm dev`)

Live-reload while you code. Best for tight iteration on UI or Rust.

### What it does

| Layer | Behavior |
| ----- | -------- |
| React / Tailwind / TS | Vite dev server on `localhost:1420` with **HMR** — change a `.tsx`/`.css`, see it instantly without losing app state |
| Rust (Tauri commands, gateway, storage) | Recompiles + relaunches the Tauri window on any `.rs` save under `src-tauri/` or `crates/` |
| Bundle ID | Same `com.mcpmux.desktop` — reads your real DB and Keychain entries |

### Run it

```bash
pnpm dev
```

`predev` runs automatically: quits `/Applications/McpMux.app` if running, stops orphaned Vite/Tauri processes from this repo, and waits until `:1420` and `:45818` are free.

After **gateway Rust changes**, prefer a clean rebuild:

```bash
pnpm dev:restart
```

A Tauri window opens. Edit `.tsx` files for instant HMR; edit Rust and the window will relaunch on its own after recompile.

### Stopping dev

Use **`pnpm dev:stop`** — same script `predev` runs before every start. It:

1. Quits `/Applications/McpMux.app` (macOS)
2. Stops orphaned dev processes: `pnpm --filter @mcpmux/desktop dev`, Tauri, Vite (`:1420`), debug `mcpmux` (`:45818` / `:45819`), and `dev:admin` wrappers
3. Waits until `:1420`, `:45818`, and `:45819` are free

Implementation: [`scripts/dev-kill.helpers.mjs`](../scripts/dev-kill.helpers.mjs) (called from [`scripts/dev-env.mjs`](../scripts/dev-env.mjs)).

If a crashed session left ports busy and `pnpm dev:stop` still reports a conflict, run stop again (the second pass SIGKILLs stragglers). On macOS you can also verify nothing is listening:

```bash
lsof -nP -iTCP:1420,45818,45819 -sTCP:LISTEN
```

Then restart:

```bash
pnpm dev:stop && pnpm dev:admin   # web admin + HMR in browser
# or
pnpm dev:stop && pnpm dev          # Tauri window only
```

**Do not** use bare `pkill -f mcpmux` — that can hit unrelated processes. Prefer `pnpm dev:stop`.

### Frontend-only iteration (desktop transport)

If you're only changing UI inside the Tauri shell and don't need the HTTP admin transport:

```bash
pnpm dev
```

Use the Tauri window (Vite on `:1420`, `invoke()` IPC). Fastest for layout work that doesn't touch web-only code paths.

For **web admin transport** (`fetch` + SSE), use the web admin section below — plain `pnpm dev:web` still has no backend unless something is listening on `:45819`.

---

## Web admin UI (HTTP on `:45819`)

Optional loopback admin server: static SPA + REST `/api/v1/*` + SSE. Same SQLite DB and Keychain as desktop. **Not** the MCP gateway (`:45818`).

**Frontend boundary:** Browser code imports `@/lib/backend` (data commands via `apiCall`, live updates via `backend/events`, OS integrations via `backend/shell`). Do not import `@tauri-apps/*` outside `lib/backend/**` — ESLint enforces this. Deprecated `@/lib/api/*` shims still work during transition. See [`unified-backend-facade.md`](planning/unified-backend-facade.md).

| Port | Role |
| ---- | ---- |
| `1420` | Vite dev server (HMR). Proxies `/api` → admin port. |
| `45818` | MCP gateway for AI clients |
| `45819` | Web admin API + (when built) production SPA at `/` |

### Workflows (fastest → production parity)

1. **Tauri + `invoke()` (desktop transport)** — `pnpm dev`, use the Tauri window. Best for Rust + UI when you don't need to validate browser/HTTP behavior. Settings → Gateway → Web admin can stay off.

2. **HMR + HTTP transport (recommended for web UI work)** — backend + browser:

   ```bash
   pnpm dev:admin
   ```

   Starts `pnpm dev` with `MCPMUX_DEV_ADMIN=1` (admin API on for this session), frees ports `1420` / `45818` / `45819`, opens `http://127.0.0.1:1420` when `/api/v1/health` is up. Vite uses `VITE_ADMIN_WEB=true` and proxies `/api` to `:45819`.

   Alternative (two-process, same result):

   ```bash
   pnpm dev:web:admin
   ```

   Runs `dev-env` prep, starts `pnpm dev` in the background if admin isn't up yet, then Vite on `:1420`.

3. **Production-parity static SPA** — build + hit admin origin directly:

   ```bash
   pnpm build:web:admin
   pnpm dev:admin   # or pnpm dev with web admin enabled in Settings
   ```

   Open `http://127.0.0.1:45819/` (hard refresh after rebuilds; no HMR on this origin). Use before homelab deploy or `pnpm test:e2e:web:admin`.

**First-time dev default:** debug builds persist `gateway.admin_enabled=true` when that setting was never saved. If you previously disabled web admin in Settings, use `pnpm dev:admin` or enable it again under **Settings → Gateway**.

**Disable admin in dev:** `MCPMUX_DEV_DISABLE_ADMIN=1 pnpm dev`

### Cloudflare Access profiles

| Profile | Settings | Local URL | Playwright |
| ------- | -------- | --------- | ---------- |
| **Local fast** | Trust CF Access **off** | `http://127.0.0.1:45819` or HMR `http://127.0.0.1:1420` | No JWT env vars |
| **Tunnel parity** | Trust CF Access **on** + team domain | Same loopback; tunnel adds JWT at edge | Copy `.env.example` → `.env` with `MCPMUX_CF_ACCESS_*`; Playwright: `MCPMUX_ADMIN_CF_JWT` or service-token env vars; `pnpm remote:smoke` for gateway/admin health |

**Service token on both hostnames:** The gateway (`mcp.*`) and admin (`mux.*`) Access applications each need the same service token under **Zero Trust → Access → Service auth**. A 302 to Cloudflare SSO on `mux.*` means the token is missing from that app's policy. When trust is on, the admin server also accepts matching `CF-Access-Client-Id` / `CF-Access-Client-Secret` headers at the origin when `MCPMUX_CF_ACCESS_CLIENT_ID` and `MCPMUX_CF_ACCESS_CLIENT_SECRET` are set in the McpMux process environment (automation / tunnel smoke without a JWT).

Homelab hostname and tunnel layout: [`docs/guide/gateway.mdx`](guide/gateway.mdx) (generic placeholders). Operator-specific tunnel wiring lives in private homelab docs outside this repo.

### Commands

| Command | What it does |
| ------- | ------------ |
| `pnpm dev:admin` | Tauri dev + session admin on + browser opens `:1420` |
| `pnpm dev:web:admin` | Prep ports, ensure backend, Vite with admin web flags |
| `pnpm build:web:admin` | Production admin SPA → `apps/desktop/dist` |
| `pnpm test:e2e:web:admin` | Playwright against `:45819` (needs running app + built dist) |
| `pnpm remote:smoke` | Health + OAuth metadata over `MCPMUX_REMOTE_*` URLs (reads `.env`) |

`predev` / `pnpm dev:stop` also free `:45819` (see `scripts/dev-env.mjs`).

### Web admin gotchas

| Symptom | Why | Fix |
| ------- | --- | --- |
| Dashboard empty / "Waiting for admin API" on `:1420` | Admin server off or not ready | `pnpm dev:admin` or enable Web admin in Settings; wait for health |
| 503 HTML "Web admin UI not built" on `:45819` | No `index.html` in `apps/desktop/dist` | `pnpm build:web:admin` |
| `invoke` / `transformCallback` errors in browser | Opened `:1420` without admin web transport, or a component imported `@tauri-apps` directly | Use `dev:web:admin` / `dev:admin` (sets `VITE_ADMIN_WEB` + proxies `/api` → `:45819`). Post-facade, feature code should only touch Tauri via `@/lib/backend/shell` or `@/lib/backend/events` — grep for stray `@tauri-apps` imports if errors persist after a clean reload |
| Mutations 403 | CSRF or CF Access | Local fast: trust off; tunnel: pass `CF-Access-Jwt-Assertion` |
| Playwright can't reach admin | App not running or CF trust without JWT | Start app + build; set `MCPMUX_ADMIN_CF_JWT` when trust on |

### Gotchas (desktop dev)

| Symptom | Why | Fix |
| ------- | --- | --- |
| Keychain prompts on first launch of the dev binary | Different signer than `/Applications/McpMux.app` | Click **Always Allow** once — sticks for that built artifact. See `Keychain prompts` below for detail |
| `Address already in use: 45818` | Installed `.app` or stale dev process | `pnpm dev:stop` then `pnpm dev` (or `pnpm dev:restart`) |
| `Port 1420 is already in use` | Orphan Vite / `pnpm dev` from a crashed session | `pnpm dev:stop` (run twice if needed) — see **Stopping dev** above |
| Gateway code changed but filter/behavior unchanged | Stale binary (`Finished in 0.20s`, no `Compiling mcpmux-gateway`) | `pnpm dev:restart` |
| Cursor's MCP server "disconnected" mid-session | You stopped `pnpm dev` | Cursor reconnects when the gateway is back on `localhost:45818` (either flow) |
| Rust recompile feels slow | Big edits in `mcpmux-gateway` / `mcpmux-storage` | Expected — keep edits scoped or use `pnpm dev:web` for UI |
| `pnpm dev` keeps crashing with "Master key not found" | DB/keychain mismatch from manual deletion | Don't manually delete keychain entries — see `Keychain prompts` below |

### Keychain prompts

McpMux reads two secrets from Keychain on startup:

1. **Master encryption key** — every app launch (decrypts SQLite credentials)
2. **JWT signing secret** — first time you start the gateway in a session

macOS scopes Keychain access to the **specific signed binary**, not just the bundle ID. So:

- First launch of a `pnpm dev` build → 1–2 prompts
- First launch after a fresh `pnpm build` swap → 1–2 prompts
- Subsequent launches of the **same** built binary → silent if you clicked **Always Allow**
- Alternating between `pnpm dev` and `/Applications/McpMux.app` → may re-prompt because each is a different signer

This is expected. Click **Always Allow** the first time you see each prompt for a new build.

---

## Flow 2 — Build and swap into `/Applications`

Use when you want the source build to behave like an installed app: launch from Spotlight/Dock, autostart, run in the background without a dev terminal, survive reboots.

### Option A — Full build (recommended)

Rebuilds the React frontend and produces a fresh `.app` bundle. Use this when frontend or Tauri config changed, or when you want a clean bundle.

#### 1. Quit the running app

```bash
osascript -e 'tell application "McpMux" to quit' 2>/dev/null || true
# Give it a moment to release the gateway port
sleep 2
```

#### 2. Build

```bash
cd /path/to/mcp-mux
pnpm build
```

First build: ~5–10 min. Incremental: ~1–3 min.

Output:

```
target/release/bundle/macos/McpMux.app
target/release/bundle/dmg/McpMux_*.dmg   # optional installer artifact
```

> **Note:** the build may exit non-zero at the very end with `TAURI_SIGNING_PRIVATE_KEY` missing. That only blocks the auto-update artifact; the `.app` and `.dmg` are still produced and usable.

#### 3. Backup and swap

```bash
# Backup current install (skip if you already have a recent .bak)
sudo mv /Applications/McpMux.app /Applications/McpMux.app.bak

# Install the new build
sudo cp -R target/release/bundle/macos/McpMux.app /Applications/

# Fix ownership (sudo cp leaves root-owned files)
sudo chown -R "$(whoami):admin" /Applications/McpMux.app
```

#### 4. Re-sign (required after manual swap)

macOS Gatekeeper rejects a bundle whose binary was replaced without re-signing:

```bash
xattr -dr com.apple.quarantine /Applications/McpMux.app 2>/dev/null || true
codesign --force --deep --sign - /Applications/McpMux.app
```

#### 5. Launch

```bash
open /Applications/McpMux.app
```

Verify: spaces, installed servers, and gateway on `localhost:45818` should look exactly as before. First launch will trigger 1–2 Keychain prompts because the new ad-hoc signature is a different signer than the previous build — click **Always Allow** once and you're set until the next swap.

### Option B — Binary-only swap (fast path)

When you changed **Rust only** (no frontend, no `tauri.conf.json` changes). Skips the Vite build and DMG step.

```bash
osascript -e 'tell application "McpMux" to quit' 2>/dev/null || true
sleep 2

cd /path/to/mcp-mux
cargo build --release -p mcpmux

cp /Applications/McpMux.app/Contents/MacOS/mcpmux \
   /Applications/McpMux.app/Contents/MacOS/mcpmux.bak
cp target/release/mcpmux /Applications/McpMux.app/Contents/MacOS/mcpmux

xattr -dr com.apple.quarantine /Applications/McpMux.app 2>/dev/null || true
codesign --force --deep --sign - /Applications/McpMux.app

open /Applications/McpMux.app
```

Keeps the existing bundle shell (icons, Info.plist, embedded frontend from last full build). Only the Rust binary updates.

---

## Rollback (Flow 2 only)

If a swapped build is broken, restore the previous `/Applications/McpMux.app`. Dev-mode (`pnpm dev`) doesn't need a rollback — just stop the dev process.

### Full build rollback

```bash
osascript -e 'tell application "McpMux" to quit' 2>/dev/null || true
sudo rm -rf /Applications/McpMux.app
sudo mv /Applications/McpMux.app.bak /Applications/McpMux.app
open /Applications/McpMux.app
```

### Binary-only rollback

```bash
osascript -e 'tell application "McpMux" to quit' 2>/dev/null || true
cp /Applications/McpMux.app/Contents/MacOS/mcpmux.bak \
   /Applications/McpMux.app/Contents/MacOS/mcpmux
codesign --force --deep --sign - /Applications/McpMux.app
open /Applications/McpMux.app
```

---

## Troubleshooting

| Symptom | Applies to | Fix |
| ------- | ---------- | --- |
| "App is damaged" / won't open | Flow 2 | Re-run `codesign --force --deep --sign - /Applications/McpMux.app` |
| Gateway port already in use | Both | `pnpm dev:stop` then relaunch — avoid bare `pkill -f mcpmux` |
| Cursor OAuth fails after swap | Both | Re-trigger MCP OAuth in Cursor (DCR redirect URI validation may have changed) |
| Empty app / missing UI | Flow 2 (Option B) | You used binary-only swap but frontend changed — run Option A (full build) |
| Permission denied on `/Applications` | Flow 2 | Use `sudo` for mv/cp/chown, or install to `~/Applications/` and skip sudo |
| Keychain prompts on every launch of the same binary | Both | Click **Always Allow** (not just **Allow**); check Keychain Access for duplicate `master-encryption-key` / `jwt-signing-secret` entries from old signers |
| `pnpm dev` won't start — `EADDRINUSE 45818` | Flow 1 | The installed `.app` is still running — quit it before `pnpm dev` |

---

## One-liner (full build + swap)

Assumes you're in repo root and have a recent backup:

```bash
osascript -e 'tell application "McpMux" to quit' 2>/dev/null; sleep 2 && \
pnpm build && \
sudo rm -rf /Applications/McpMux.app && \
sudo cp -R target/release/bundle/macos/McpMux.app /Applications/ && \
sudo chown -R "$(whoami):admin" /Applications/McpMux.app && \
xattr -dr com.apple.quarantine /Applications/McpMux.app 2>/dev/null; \
codesign --force --deep --sign - /Applications/McpMux.app && \
open /Applications/McpMux.app
```

---

## Related

- [`AGENTS.md`](../AGENTS.md) — build commands and project layout
- [`CLAUDE.md`](../CLAUDE.md) — full dev environment reference
- Private homelab operator docs (outside this repo) — web admin architecture and parity matrix
- [`docs/planning/unified-backend-facade.md`](planning/unified-backend-facade.md) — three-channel frontend boundary (data / events / shell)
