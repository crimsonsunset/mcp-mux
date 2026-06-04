# E2E Testing

Two E2E test suites for different purposes.

## 1. Web-only E2E (Playwright) - RECOMMENDED

Tests UI components without Tauri backend (mocked IPC). Works everywhere.

```bash
pnpm test:e2e:web
pnpm test:e2e:web:headed  # With browser visible
```

**Supported platforms**: All (Windows, Linux, macOS)  
**Use for**: UI layout, component rendering, navigation, most testing

**Test files**: `specs/*.spec.ts`

## 1b. Admin web E2E (Playwright, real `:45819`)

Exercises the built admin SPA against a live AdminServer (Tauri dev or CI fixture). Not mocked.

```bash
pnpm build:web:admin
pnpm test:e2e:web:admin

# Fast iteration (one test, trace on failure)
pnpm exec playwright test -c tests/e2e/playwright.admin.config.ts -g "read browse" --trace on

# Suite stops after first failure (maxFailures: 1) — read console [e2e:admin] lines, not a 2min hang
# After failure — trace on retry only; use show-trace if needed
pnpm exec playwright show-trace test-results/<run>/trace.zip
```

**Conventions** (aligned with CLI-first scaffold flow):

| Practice | Why |
|----------|-----|
| `specs/admin/_helpers/admin-diagnostics.helpers.ts` | Domain waits (`waitForAdminAppReady`, `waitForServersPage`) + `[e2e:admin]` selector snapshots |
| `attachAdminPageDiagnostics(page)` before `goto` | Logs `useDataSync` / `/api/v1/*` responses when a test flakes |
| `workers: 1` in `playwright.admin.config.ts` | One AdminServer + SQLite — parallel workers race startup sync |
| `waitForAdminAppReady` after shell visible | Space switcher must leave `Loading...` (polls `data-testid="space-switcher"`) |
| CF Access | Set `MCPMUX_CF_ACCESS_CLIENT_ID` + `SECRET` in repo `.env`; Playwright sends service-token headers |

**Prereqs**: `scripts/admin-e2e-fixture.mjs` via config `webServer`, or `pnpm dev:admin` with matching `.env`.

**Test files**: `specs/admin/*.spec.ts`

## 2. Tauri E2E (WebdriverIO) - Full Integration

Tests the actual built Tauri application with real backend. Complex setup.

### Prerequisites

```bash
# 1. Install tauri-driver
cargo install tauri-driver --locked

# 2. Platform-specific WebDriver:

# Windows: Download Edge WebDriver matching your Edge version
# https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/
# Add msedgedriver.exe to PATH

# Linux: 
sudo apt-get install webkit2gtk-driver gnome-keyring
# gnome-keyring provides Secret Service API (org.freedesktop.secrets) for credential storage

# macOS: NOT SUPPORTED (no WKWebView driver)
```

### Running

```bash
# Build the app first
pnpm build

# Run tests
pnpm test:e2e
```

**Supported platforms**: Windows, Linux  
**NOT supported**: macOS

**Test files**: `specs/*.wdio.ts`

## When to Use Which

| Scenario | Suite |
|----------|-------|
| Test full user flows | WebdriverIO (`test:e2e`) |
| Test server connections | WebdriverIO |
| Test OAuth flows | WebdriverIO |
| Test UI components | Playwright (`test:e2e:web`) |
| Test responsive layout | Playwright |
| CI on macOS | Playwright only |

## CI Configuration

```yaml
# Linux/Windows: Full E2E
- run: pnpm build
- run: pnpm test:e2e

# macOS: Web-only
- run: pnpm test:e2e:web
```
