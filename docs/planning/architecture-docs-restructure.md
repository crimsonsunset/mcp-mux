# Docs Restructure — Domain Architecture Layout

**Last Updated:** Jun 1, 2026
**Status:** Planning — decisions locked; not yet started
**Branch:** TBD (docs-only; no Rust/TS changes)
**Base branch:** `main`
**Depends on:** nothing — pure doc reorganization; all source material exists
**Unblocks:** future agents navigating the codebase without having to reconstruct architecture from 7+ fragmented phase docs

---

## Problem

Every subsystem McpMux has shipped lives in a planning doc — but planning docs are mixed with QA runbooks, PR reviews, and cleanup plans. There is no top-level architecture layer: no single file explains how consent, discovery, search, and the embedding cache fit together as one system. An agent (or a new engineer) bootstrapping from scratch has to read seven interrelated docs in the right order just to understand the capability flow.

The design docs themselves are good — they're just filed in the wrong place with no synthesis on top.

---

## Decisions

| #   | Decision                | Choice                                                                                                    | Rationale                                                                                                                                          |
| --- | ----------------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Layout shape            | **Domain folders** — `docs/backend/` (Rust crates) and `docs/frontend/` (Tauri + React)                   | Matches established project conventions; separates engine from UI naturally. Each domain is independently navigable.                               |
| 2   | Per-domain structure    | **`technical/` + `guides/` + `reference/` + `README.md`** in each domain                                  | `technical/` = durable "how it works"; `guides/` = how-to procedures; `reference/` = moved originals. Consistent split is navigable without a map. |
| 3   | Filenames               | **Reuse standard names** — `architecture.md`, `data-model.md`, `services-overview.md`                     | Creates cross-codebase muscle memory. `architecture.md` is always the entry point regardless of repo.                                              |
| 4   | ADRs                    | **No ADR extraction.** Decision tables stay inside their docs.                                            | Existing decision tables are already well-structured. Renaming them as ADRs adds ceremony without value at this scale.                             |
| 5   | Existing design docs    | **`git mv` as-is into `<domain>/reference/`** — no rewrites at move time                                  | Preserves git history. Content cleanup is a separate concern from relocation.                                                                      |
| 6   | New synthesis docs      | **Author `<domain>/technical/*.md` as the primary deliverable**                                           | The moved originals become supporting reference; the synthesis docs become the navigable source of truth.                                          |
| 7   | QA/testing docs         | **`docs/testing/` sibling** — moved from `planning/`; distinct from both architecture and process         | Testing docs have a different audience and lifecycle than design docs or active plans.                                                             |
| 8   | `planning/` residue     | **Only active process artifacts stay in `planning/`** — code reviews, cleanup plans, active planning docs | Anything with a durable purpose moves out; `planning/` becomes a working scratchpad, not a permanent home.                                         |
| 9   | `project/` + `quality/` | **Defer** — create when there is content (roadmap, sonar findings, etc.)                                  | Empty folders add noise. Neither has content today.                                                                                                |

---

## The Model

### Target folder shape

```
docs/
├── backend/                             # Rust: gateway, core, storage, mcp crates
│   ├── README.md                        # domain index
│   ├── technical/
│   │   ├── architecture.md              # THE entry point — end-to-end capability flow + diagram
│   │   ├── services-overview.md         # Axum request path: auth, routing, FeatureSet filter, OAuth refresh
│   │   ├── consent-and-binding.md       # FeatureSet as consent unit, WorkspaceBinding, approval broker
│   │   ├── tool-discovery-and-search.md # search→schema→invoke, hybrid ranking, active-index cache
│   │   ├── embedding-cache.md           # EmbeddingService lifecycle, warmer, SQLite persistence
│   │   ├── server-lifecycle-and-pool.md # connection pool, session readiness, clones, transports
│   │   ├── security-and-credentials.md  # OAuth 2.1+PKCE, DCR, AES-256-GCM, keychain/DPAPI
│   │   └── data-model.md               # entities: Space, FeatureSet, WorkspaceBinding, EventBus
│   ├── guides/
│   │   ├── run-from-source.md           # promoted from docs/run-from-source-macos.md
│   │   └── dev-workflow.md              # dev:stop/rebuild/admin, ports, log paths
│   └── reference/                       # design docs moved as-is (git mv)
│
├── frontend/                            # Tauri desktop + React + web admin UI
│   ├── README.md
│   ├── technical/
│   │   ├── backend-facade.md            # @/lib/backend transport abstraction, event channels
│   │   └── web-admin-and-remote-access.md # :45819, Cloudflare Tunnel + Access, CF-Access-Jwt
│   ├── guides/
│   └── reference/                       # design docs moved as-is (git mv)
│
├── testing/                             # QA runbooks + verification gates (moved from planning/)
│
└── planning/                            # active process docs only
```

### Docs being moved

**→ `docs/backend/reference/`** (12 files)

`feature-set-consent-model.md`, `dynamic-mcp-toggle-meta-tools.md`, `meta-gateway-invoke.md`,
`search-tools-hybrid-semantic-ranking.md`, `search-tools-embedding-search-read-path.md`,
`search-tools-latency-and-root-race.md`, `mcpmux-diagnose-server.md`,
`search-tools-persistent-embedding-cache.md`, `server-account-clones.md`,
`gateway-warm-pool-startup.md`, `agent-mcp-session-readiness.md`, `tool-level-session-pin.md`

**→ `docs/frontend/reference/`** (4 files)

`unified-backend-facade.md`, `web-admin-parity-matrix.md`, `workspace-binding-icons.md`,
`server-display-rename.md`

**→ `docs/testing/`** (5 files)

`consent-model-qa-runbook.md`, `meta-gateway-invoke-qa.md`, `meta-gateway-invoke-retest.md`,
`pr3-fix-verification.md`, `pr3-fixes-qa.md`

**Stays in `docs/planning/`**

`pr-2-web-admin-code-review.md`, `pre-web-admin-desktop-cleanup.md`,
`architecture-docs-restructure.md` (this doc)

---

## Phases

### Phase 1 — Scaffolding + moves (~30 min)

- Create `docs/backend/technical/`, `docs/backend/guides/`, `docs/backend/reference/`
- Create `docs/frontend/technical/`, `docs/frontend/guides/`, `docs/frontend/reference/`
- Create `docs/testing/`
- `git mv` all 12 backend design docs into `docs/backend/reference/`
- `git mv` all 4 frontend design docs into `docs/frontend/reference/`
- `git mv` all 5 QA/verification docs into `docs/testing/`
- Promote `docs/run-from-source-macos.md` → `docs/backend/guides/run-from-source.md`
- Commit: `chore(docs): scaffold domain layout and move design docs`

**Outcome:** All existing docs are in their final permanent homes. No content has changed. Git history is fully intact. `docs/planning/` no longer mixes architecture with QA with process. Any link that broke is surfaced at this commit boundary, easy to fix before writing begins.

---

### Phase 2 — Backend domain index + core synthesis docs (~2–3 hr)

- Write `docs/backend/README.md` — index of `technical/`, `guides/`, `reference/`; when to read each
- Write `docs/backend/technical/architecture.md` — the headline doc:
  - One Mermaid diagram of the end-to-end capability flow (agent → search → bind → invoke)
  - Subsystem map linking out to each `technical/` doc
  - "What McpMux is / is not" framing
- Write `docs/backend/technical/consent-and-binding.md` — synthesizes `feature-set-consent-model.md` + `dynamic-mcp-toggle-meta-tools.md`
- Write `docs/backend/technical/tool-discovery-and-search.md` — synthesizes the 4 search/invoke docs + `mcpmux-diagnose-server.md`
- Write `docs/backend/technical/embedding-cache.md` — synthesizes `search-tools-persistent-embedding-cache.md`
- Add a one-line banner to each moved `reference/` doc pointing at its synthesis parent

**Outcome:** `docs/backend/technical/architecture.md` exists and is the single correct entry point for understanding McpMux's capability system. Consent, search, and cache each have a synthesis doc that replaces the need to read 2–3 phase docs in sequence. An agent can answer "how does bind-and-invoke work" from one read.

---

### Phase 3 — Remaining backend technical docs + guides (~2 hr)

- Write `docs/backend/technical/services-overview.md` — Axum request path, per-client auth, routing, FeatureSet filtering, OAuth token refresh
- Write `docs/backend/technical/server-lifecycle-and-pool.md` — synthesizes `server-account-clones.md`, `gateway-warm-pool-startup.md`, `agent-mcp-session-readiness.md`, `tool-level-session-pin.md`
- Write `docs/backend/technical/security-and-credentials.md` — OAuth 2.1 + PKCE, DCR, AES-256-GCM field encryption, keychain/DPAPI, `zeroize`
- Write `docs/backend/technical/data-model.md` — entities (Space, FeatureSet, WorkspaceBinding, InstalledServer, Client), repository-trait pattern, EventBus
- Write `docs/backend/guides/dev-workflow.md` — dev:stop / dev:rebuild / dev:admin workflow, port map, log paths, Cursor MCP reload steps

**Outcome:** `docs/backend/technical/` is complete. Every Rust subsystem has a durable architecture doc. A reader can understand the full backend in one folder without touching `planning/` or the codebase.

---

### Phase 4 — Frontend domain + cross-cutting wiring (~1 hr)

- Write `docs/frontend/README.md` — domain index
- Write `docs/frontend/technical/backend-facade.md` — synthesizes `unified-backend-facade.md`; covers `@/lib/backend` transport switch, event channels, ESLint guard
- Write `docs/frontend/technical/web-admin-and-remote-access.md` — synthesizes `web-admin-parity-matrix.md`; covers :45819, Cloudflare Tunnel + Access, `CF-Access-Jwt-Assertion`, mutating-route protection
- Write `docs/testing/README.md` — index mapping each runbook to its subsystem
- Update `CLAUDE.md` and `AGENTS.md`: replace any planning-doc links with new `backend/technical/` or `frontend/technical/` paths where relevant

**Outcome:** Entire `docs/` tree is coherent. `frontend/technical/` covers the UI transport and admin layer. `docs/testing/` is indexed. `CLAUDE.md`/`AGENTS.md` point agents at architecture docs, not phase planning docs. The restructure is complete.

---

## Key files referenced

| File                                                                                                       | Note                                           |
| ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| [`docs/planning/feature-set-consent-model.md`](./feature-set-consent-model.md)                             | Primary source for consent + binding synthesis |
| [`docs/planning/meta-gateway-invoke.md`](./meta-gateway-invoke.md)                                         | Primary source for tool-discovery synthesis    |
| [`docs/planning/search-tools-hybrid-semantic-ranking.md`](./search-tools-hybrid-semantic-ranking.md)       | Hybrid ranking design                          |
| [`docs/planning/search-tools-persistent-embedding-cache.md`](./search-tools-persistent-embedding-cache.md) | Embedding cache design                         |
| [`docs/planning/server-account-clones.md`](./server-account-clones.md)                                     | Clones; feeds server-lifecycle doc             |
| [`docs/planning/unified-backend-facade.md`](./unified-backend-facade.md)                                   | Frontend transport; feeds backend-facade doc   |
| [`docs/planning/web-admin-parity-matrix.md`](./web-admin-parity-matrix.md)                                 | Web admin; feeds web-admin doc                 |
| `CLAUDE.md`                                                                                                | Updated in Phase 4 to link new paths           |
| `AGENTS.md`                                                                                                | Updated in Phase 4 to link new paths           |

## Open questions (deferred, not blocking)

- **`backend/` vs `gateway/`** for the Rust domain name — `backend/` chosen for consistency; revisit if it feels wrong after Phase 1.
- **`services-overview.md` vs `server-lifecycle-and-pool.md` overlap** — kept separate (request-path runtime vs connection lifecycle); merge in a later pass if they feel redundant.
- **`project/` + `quality/`** — deferred until there's content (roadmap, sonar/lint findings).
