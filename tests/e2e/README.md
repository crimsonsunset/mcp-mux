# E2E Testing

Two E2E test suites for different purposes.

## 1. Web-only E2E (Playwright) - RECOMMENDED

Tests the real React app in Chromium with no backend. The web build talks to
the gateway's admin REST API (`/api/v1/*`), which is not running here, so
`helpers/admin-api-mock.ts` answers those calls from fixtures. Every spec
installs it in a top-level `beforeEach`.

```bash
pnpm test:e2e:web
pnpm test:e2e:web:headed  # With browser visible
```

**Supported platforms**: All (Windows, Linux, macOS)
**Use for**: UI layout, component rendering, navigation, most testing

**Test files**: `specs/*.spec.ts`

### Writing and fixing tests

- The app code is the source of truth. When a test and the UI disagree, change the test.
- Prefer `getByTestId` over copy. Test ids are stable; labels get renamed (Feature Sets became Bundles, Workspaces became Projects).
- Fixtures mirror the TypeScript shapes in `apps/desktop/src/lib/api/*.ts`. If a page renders blank or throws, a fixture is probably `null` where the page expects a list or object. Check the browser console.
- Without a Tauri shell, `isTauri()` is false, so desktop-only controls (the update checker, "Open Logs Folder") are absent on purpose. Assert their absence, or cover them in the desktop suite.
- Do not wrap assertions in `if (count > 0)`. The mock guarantees one client, one space and a complete machine; assert on them.

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
