# Build from Source and Replace the Installed App (macOS)

Replace the release McpMux in `/Applications` with a locally built copy from this repo. Useful when running a fork, a feature branch, or patches that haven't shipped yet.

---

## What survives a swap

Replacing the `.app` bundle does **not** touch your data. McpMux stores everything outside the app:

| Data | Location |
| ---- | -------- |
| SQLite DB (spaces, servers, clients, settings) | `~/Library/Application Support/com.mcpmux.desktop/mcpmux.db` |
| Per-space files | `~/Library/Application Support/com.mcpmux.desktop/spaces/` |
| Logs | `~/Library/Application Support/com.mcpmux.desktop/logs/` |
| Encryption master key | macOS Keychain (`com.mcpmux.desktop` service) |
| OAuth tokens / credentials | Encrypted in SQLite + Keychain |

The app identifier (`com.mcpmux.desktop`) is unchanged between release and source builds, so the new binary reads the same data directory and keychain entries.

**What you might need to redo:** OAuth re-auth in Cursor/Claude Desktop if DCR or token validation changed on your branch. Your McpMux config, spaces, and server installs stay put.

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

## Option A — Full build (recommended)

Rebuilds the React frontend and produces a fresh `.app` bundle. Use this when frontend or Tauri config changed, or when you want a clean bundle.

### 1. Quit the running app

```bash
osascript -e 'tell application "McpMux" to quit' 2>/dev/null || true
# Give it a moment to release the gateway port
sleep 2
```

### 2. Build

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

### 3. Backup and swap

```bash
# Backup current install (skip if you already have a recent .bak)
sudo mv /Applications/McpMux.app /Applications/McpMux.app.bak

# Install the new build
sudo cp -R target/release/bundle/macos/McpMux.app /Applications/

# Fix ownership (sudo cp leaves root-owned files)
sudo chown -R "$(whoami):admin" /Applications/McpMux.app
```

### 4. Re-sign (required after manual swap)

macOS Gatekeeper rejects a bundle whose binary was replaced without re-signing:

```bash
xattr -dr com.apple.quarantine /Applications/McpMux.app 2>/dev/null || true
codesign --force --deep --sign - /Applications/McpMux.app
```

### 5. Launch

```bash
open /Applications/McpMux.app
```

Verify: spaces, installed servers, and gateway on `localhost:45818` should look exactly as before.

---

## Option B — Binary-only swap (fast path)

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

## Rollback

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

| Symptom | Fix |
| ------- | --- |
| "App is damaged" / won't open | Re-run `codesign --force --deep --sign - /Applications/McpMux.app` |
| Gateway port already in use | Old process still running — `pkill -f mcpmux` then relaunch |
| Cursor OAuth fails after swap | Re-trigger MCP OAuth in Cursor (DCR redirect URI validation may have changed) |
| Empty app / missing UI | You used binary-only swap but frontend changed — run Option A (full build) |
| Permission denied on `/Applications` | Use `sudo` for mv/cp/chown, or install to `~/Applications/` and skip sudo |

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
