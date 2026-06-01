# Backend Docs

Rust crates: `mcpmux-gateway`, `mcpmux-core`, `mcpmux-storage`, `mcpmux-mcp`.

---

## technical/

Durable "how it works" docs. Read these to understand the system.

| Doc | What it covers |
| --- | -------------- |
| [`architecture.md`](./technical/architecture.md) | Entry point — end-to-end capability flow, subsystem map, what McpMux is/is not |
| [`consent-and-binding.md`](./technical/consent-and-binding.md) | FeatureSet as the consent unit, WorkspaceBinding, approval broker, session-override removal |
| [`tool-discovery-and-search.md`](./technical/tool-discovery-and-search.md) | search → schema → invoke, hybrid ranking, active-index cache, diagnostics |
| [`embedding-cache.md`](./technical/embedding-cache.md) | EmbeddingService lifecycle, on-connect warmer, SQLite persistence |
| [`services-overview.md`](./technical/services-overview.md) | Axum request path: per-client auth, routing, FeatureSet filtering, OAuth refresh |
| [`server-lifecycle-and-pool.md`](./technical/server-lifecycle-and-pool.md) | Connection pool, session readiness, account clones, transports |
| [`security-and-credentials.md`](./technical/security-and-credentials.md) | OAuth 2.1+PKCE, DCR, AES-256-GCM, keychain/DPAPI, `zeroize` |
| [`data-model.md`](./technical/data-model.md) | Entities: Space, FeatureSet, WorkspaceBinding, InstalledServer, EventBus |

Start with `architecture.md`. Each technical doc is self-contained — read only what the task requires.

---

## guides/

How-to procedures. Read these when you need to do something, not just understand it.

| Doc | What it covers |
| --- | -------------- |
| [`run-from-source.md`](./guides/run-from-source.md) | Local build, first-run setup, prerequisites |
| [`dev-workflow.md`](./guides/dev-workflow.md) | `dev:stop` / `dev:rebuild` / `dev:admin`, port map, log paths, Cursor MCP reload |
| [`server-config-lanes.md`](./guides/server-config-lanes.md) | Per-server `env_overrides`, `args_append`, `extra_headers`, `default_params` — when to use each |

---

## reference/

Original design docs, moved verbatim from `docs/planning/`. Git history is fully intact.

Read these when you need implementation detail, decision rationale, or phasing history beyond what the synthesis docs cover.

| Doc | Synthesized into |
| --- | ---------------- |
| [`feature-set-consent-model.md`](./reference/feature-set-consent-model.md) | `consent-and-binding.md` |
| [`dynamic-mcp-toggle-meta-tools.md`](./reference/dynamic-mcp-toggle-meta-tools.md) | `consent-and-binding.md` |
| [`meta-gateway-invoke.md`](./reference/meta-gateway-invoke.md) | `tool-discovery-and-search.md` |
| [`search-tools-hybrid-semantic-ranking.md`](./reference/search-tools-hybrid-semantic-ranking.md) | `tool-discovery-and-search.md` |
| [`search-tools-embedding-search-read-path.md`](./reference/search-tools-embedding-search-read-path.md) | `tool-discovery-and-search.md` |
| [`search-tools-latency-and-root-race.md`](./reference/search-tools-latency-and-root-race.md) | `tool-discovery-and-search.md` |
| [`mcpmux-diagnose-server.md`](./reference/mcpmux-diagnose-server.md) | `tool-discovery-and-search.md` |
| [`search-tools-persistent-embedding-cache.md`](./reference/search-tools-persistent-embedding-cache.md) | `embedding-cache.md` |
| [`server-account-clones.md`](./reference/server-account-clones.md) | `server-lifecycle-and-pool.md` |
| [`gateway-warm-pool-startup.md`](./reference/gateway-warm-pool-startup.md) | `server-lifecycle-and-pool.md` |
| [`agent-mcp-session-readiness.md`](./reference/agent-mcp-session-readiness.md) | `server-lifecycle-and-pool.md` |
| [`tool-level-session-pin.md`](./reference/tool-level-session-pin.md) | `server-lifecycle-and-pool.md` |
