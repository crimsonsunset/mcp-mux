# Backend Facade

**Synthesis of:** [`docs/frontend/reference/unified-backend-facade.md`](../reference/unified-backend-facade.md)

The `@/lib/backend` module is the single controlled boundary between the React UI and the Tauri/HTTP backend. All UI code imports from here — never from `@tauri-apps/*` directly.

---

## The problem it solves

Web admin mode (`localhost:45819`) and desktop (Tauri IPC) share the same React codebase. Before the facade, three failure modes existed:

- `listen()` / `transformCallback` crashes when components ran in a browser context outside Tauri.
- `isTauri()` guards scattered across pages, hooks, and `App.tsx` instead of one enforceable boundary.
- Direct `invoke()` calls outside `lib/api/` making the transport swap to `fetch` impossible.

The facade collapses all three surface areas into one import and enforces the boundary via ESLint.

---

## Three channels

| Channel                | Desktop                             | Web admin                               | Module                         |
| ---------------------- | ----------------------------------- | --------------------------------------- | ------------------------------ |
| **Commands**           | `invoke` → Tauri → `command_bridge` | `fetch` → admin HTTP → `command_bridge` | `backend.data.*` via `apiCall` |
| **Live updates**       | `listen(channel, …)`                | SSE `/api/v1/events`                    | `backend.events`               |
| **OS / control plane** | `invoke` only                       | N/A (hidden or message)                 | `backend.shell`                |

The three-channel split is the key insight: **data** and **events** both have parity implementations; **shell** is desktop-only by definition.

---

## Module map

```text
features/, components/, hooks/
        │
        ▼
  lib/backend/index.ts          ← only public entry (preferred)
        │
        ├── data/
        │     transport.ts      ← isTauri() + invoke vs fetchApi
        │     fetch-api.ts      ← HTTP transport (CSRF, retry)
        │     fetch-api.routes/ ← per-resource routeFor switches
        │     spaces.ts, gateway.ts, …
        │
        ├── events/
        │     subscribe.ts      ← useDomainEvents (Tauri + SSE unified)
        │
        └── shell/
              dialogs.ts
              updater.ts
              icons.ts          ← convertFileSrc wrapper
              client-install.ts
              admin-settings.ts
```

The `lib/api/*` modules still exist as deprecated shims that re-export the same surface from `lib/backend`. New code should import from `@/lib/backend`.

---

## `apiCall` transport switch

`lib/backend/data/transport.ts` contains the single runtime branch:

```typescript
isTauri() ? invoke(command, args) : fetchApi(route, args);
```

`isTauri()` is true inside the Tauri webview; false in a plain browser tab. Both paths call through `command_bridge` on the Rust side — there is no duplicate business logic.

The `fetch-api.routes/` directory holds per-resource `routeFor` maps that translate command names to HTTP method + path. Adding a new command to the bridge requires updating both the Rust router and the matching route file.

---

## Events facade

`backend/events/subscribe.ts` exports `useDomainEvents` with two internal adapters:

- **Tauri adapter** — wraps `listen(channel, handler)` from `@tauri-apps/api/event`.
- **SSE adapter** — subscribes to `/api/v1/events` and parses the `channel` field from each event payload.

Components and hooks import `useDomainEvents`; the adapter selection is invisible. The 16 live channels are documented in [`web-admin-parity-matrix.md`](../reference/web-admin-parity-matrix.md#sse-event-channels-phase-5).

---

## Shell channel

`backend/shell` contains helpers that have no HTTP equivalent:

| Module              | Purpose                                                                                                 |
| ------------------- | ------------------------------------------------------------------------------------------------------- |
| `dialogs.ts`        | `plugin-dialog` file picker                                                                             |
| `updater.ts`        | Tauri updater plugin                                                                                    |
| `icons.ts`          | `convertFileSrc` for `local:` icon refs                                                                 |
| `client-install.ts` | `add_to_cursor`, `add_to_vscode` deep OS integration                                                    |
| `admin-settings.ts` | `get_admin_web_settings` / `update_admin_web_settings` — control plane for the admin HTTP server itself |

Web views hide or disable any UI surface that calls `backend.shell`. Shell calls are never reachable from a browser tab.

---

## ESLint enforcement

An ESLint `no-restricted-imports` rule blocks `@tauri-apps/*` imports anywhere outside `apps/desktop/src/lib/backend/**`. The rule fires at lint time, so a new component that accidentally imports `@tauri-apps/api/event` fails `pnpm lint` before it reaches CI.

The enforcement is the guarantee. The three-channel model is correct _by construction_ for new code, not by convention.

---

## Key files

| File                                                  | Role                                                       |
| ----------------------------------------------------- | ---------------------------------------------------------- |
| `apps/desktop/src/lib/backend/index.ts`               | Public facade entry — re-exports `data`, `events`, `shell` |
| `apps/desktop/src/lib/backend/data/transport.ts`      | `isTauri()` switch — the single runtime branch             |
| `apps/desktop/src/lib/backend/data/fetch-api.routes/` | Per-resource HTTP route maps                               |
| `apps/desktop/src/lib/backend/events/subscribe.ts`    | Unified domain event hook                                  |
| `apps/desktop/src/lib/backend/shell/`                 | Desktop-only OS integrations                               |
| `apps/desktop/eslint.config.js`                       | `no-restricted-imports` rule for `@tauri-apps/*`           |
