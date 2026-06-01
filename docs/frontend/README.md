# Frontend Docs

Tauri desktop app and React UI: `apps/desktop/src/` (React frontend) and `apps/desktop/src-tauri/` (Tauri shell + commands).

---

## technical/

Durable "how it works" docs. Read these to understand the UI transport and admin layer.

| Doc                                                                            | What it covers                                                                                                  |
| ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| [`backend-facade.md`](./technical/backend-facade.md)                           | `@/lib/backend` three-channel transport abstraction, `apiCall` switch, ESLint boundary                          |
| [`web-admin-and-remote-access.md`](./technical/web-admin-and-remote-access.md) | Admin HTTP server on `:45819`, Cloudflare Tunnel + Access, `CF-Access-Jwt-Assertion`, mutating-route protection |

---

## guides/

How-to procedures for frontend development. None yet — add here as recurring workflows emerge.

---

## reference/

Original design docs, moved verbatim from `docs/planning/`. Git history is fully intact.

Read these when you need implementation detail, decision rationale, or phasing history beyond what the synthesis docs cover.

| Doc                                                                    | Synthesized into                                                               |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| [`unified-backend-facade.md`](./reference/unified-backend-facade.md)   | [`backend-facade.md`](./technical/backend-facade.md)                           |
| [`web-admin-parity-matrix.md`](./reference/web-admin-parity-matrix.md) | [`web-admin-and-remote-access.md`](./technical/web-admin-and-remote-access.md) |
| [`workspace-binding-icons.md`](./reference/workspace-binding-icons.md) | — (standalone feature doc; no synthesis parent)                                |
| [`server-display-rename.md`](./reference/server-display-rename.md)     | — (standalone feature doc; no synthesis parent)                                |
