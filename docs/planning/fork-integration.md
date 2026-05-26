# Personal fork integration

**Last updated:** May 26, 2026  
**Purpose:** Single source of truth for local/homelab work on a personal fork of [mcpmux/mcp-mux](https://github.com/mcpmux/mcp-mux). Upstream PR stacking is separate and optional.

---

## Canonical branch: `dev`

| Branch | Role |
|--------|------|
| **`dev`** | **Default fork branch** — all integrated work; run `pnpm dev` here |
| `main` | Upstream mirror only — **not** where fork features land |
| `feat/meta-gateway-invoke` | Legacy name (same commits as `dev`); safe to delete locally after switching |
| `feat/dynamic-mcp-toggle-meta-tools` | Legacy alias; optional delete |
| `feat/server-account-clones` | Legacy alias; optional delete |

**Rule:** If you are not on `dev`, you are not running your fork.

### What is on `dev`

1. Meta-gateway invoke Phases **A–D** (14 meta tools, resource/prompt hard cut, TF-IDF, Levenshtein) — GAIT QA **SHIP** ([v1](./meta-gateway-invoke-gait-qa.md) tools, [v2](./meta-gateway-invoke-gait-qa-v2.md) Phase D)
2. Server account clones
3. Dynamic MCP toggle meta-tools + workspace/session routing
4. Workspace binding icons, server display rename
5. Dev-env restart workflow (`scripts/dev-env.mjs`, `pnpm dev:restart`)
6. Planning docs and homelab QA sign-offs

See [`meta-gateway-invoke.md`](./meta-gateway-invoke.md), [`agent-mcp-session-readiness.md`](./agent-mcp-session-readiness.md), [`gateway-warm-pool-startup.md`](./gateway-warm-pool-startup.md).

---

## Daily workflow

```bash
git checkout dev
git pull origin dev
pnpm dev                                   # gateway on localhost:45818
```

### Dev commands (gateway / UI work)

| Command | When |
| ------- | ---- |
| `pnpm dev` | Normal iteration — runs `predev` (free `:1420` / `:45818`, quit installed app) then Tauri + Vite |
| `pnpm dev:restart` | After **gateway crate** changes — stop orphans, **rebuild** `mcpmux-gateway` + `mcpmux`, start dev |
| `pnpm dev:restart:fast` | Clean restart without cargo rebuild (UI-only or binary already fresh) |
| `pnpm dev:stop` | Kill repo dev processes only; does not start dev |
| `pnpm dev:rebuild` | Rebuild gateway binary only (no start) |

**Do not** run `./target/debug/mcpmux` alone — skips Vite and leaves stale/orphan processes. If `Finished dev profile in 0.20s` at startup after a gateway edit, use `pnpm dev:restart`.

Implementation: `scripts/dev-env.mjs` (cross-platform port cleanup + optional rebuild).

New feature work:

```bash
git checkout dev
git pull origin dev
git checkout -b feat/my-topic
# ... commits ...
git checkout dev
git merge feat/my-topic                    # or rebase topic onto dev first
git push origin dev
```

Homelab Cursor config: one `mcpmux` entry → `http://localhost:45818/mcp`. Migration tracker: [mcpmux-server-migration.md](../../../jsg-tech-check/docs/setup/mcpmux-server-migration.md).

---

## Upstream PR policy (mcpmux/mcp-mux)

| PR | Action | Why |
|----|--------|-----|
| [#152](https://github.com/mcpmux/mcp-mux/pull/152) `fix/dcr-skip-invalid-redirect-uris` | **Keep open** | Small, standalone OAuth fix (~47 lines) → `main` |
| [#154](https://github.com/mcpmux/mcp-mux/pull/154) `feat/dynamic-mcp-toggle-meta-tools` | **Keep open (draft)** | Proper stack: base `feat/workspace-root-routing`, not a megapr |
| [#155](https://github.com/mcpmux/mcp-mux/pull/155) `feat/meta-gateway-invoke` | **Closed** | Wrong base (`main`); entire fork stack (~28k LOC). Work lives on fork `dev` only |

**Not owned by this fork:** [#151](https://github.com/mcpmux/mcp-mux/pull/151) workspace-root-routing (upstream). #154 targets that branch when contributing meta-tools upstream.

Future upstream contributions: branch off fresh `upstream/main` (or merged upstream feature branches), cherry-pick or restack **one topic per PR** — do not reopen a megapr to `main`.

---

## Next implementation priorities (on `dev`)

1. [`gateway-warm-pool-startup.md`](./gateway-warm-pool-startup.md) — cold start / `gateway_warming` (**next feature**)
2. Homelab `mcp.json` cutover (bindings + `bundle:core`)
3. Replace stock `McpMux.app` with a build from `dev` ([`run-from-source-macos.md`](../run-from-source-macos.md))

**Meta-gateway invoke:** Phases A–D complete on fork; deferred items (batch invoke, `gateway_execute_code`) stay in [`meta-gateway-invoke.md`](./meta-gateway-invoke.md).

---

## Reconciliation

When `dev` gains a major milestone, update this file's **Last updated** and **What is on `dev`**. Do not use `feat/*` branch names for new integration work — merge into `dev` instead.
