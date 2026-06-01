# Services Overview

**Last Updated:** Jun 1, 2026

The gateway (`mcpmux-gateway`) is an Axum HTTP server bound to `127.0.0.1:45818`. This doc follows an inbound MCP request from TCP accept to backend server call, covering per-client auth, routing, FeatureSet filtering, and outbound OAuth token refresh.

---

## Port Map

| Port | Service | Notes |
| ---- | ------- | ----- |
| `45818` | MCP gateway | All AI clients connect here |
| `45819` | Web admin API | Local-only; Cloudflare Tunnel for remote |
| `1420` | Vite HMR | Dev only |

---

## Axum Route Tree

```
GET  /health                       → health check
GET  /.well-known/oauth-authorization-server  → RFC 8414 metadata
GET  /.well-known/oauth-protected-resource    → RFC 9728 metadata
GET  /oauth/authorize              → PKCE authorization endpoint
POST /oauth/token                  → token exchange + refresh
POST /oauth/register               → Dynamic Client Registration (DCR)
GET  /oauth/callback               → loopback redirect handler
     /mcp                          → StreamableHttpService (rmcp)
       middleware: LoggingMiddleware → mcp_oauth_middleware → McpMuxGatewayHandler
GET  /admin/*                      → web admin API (when enabled)
```

The `/mcp` subtree runs `mcp_oauth_middleware` before every call. All other OAuth endpoints are unauthenticated (they are part of the credential-issuance flow).

---

## Per-Client Auth

### Inbound: Bearer token → OAuthContext

Every `/mcp` request must carry an `Authorization: Bearer <token>` header.

`mcp_oauth_middleware` (`crates/mcpmux-gateway/src/mcp/oauth_middleware.rs`) runs as an Axum middleware layer:

```
Request
  │
  ├─ Strip "Bearer " prefix
  ├─ validate_token(jwt, jwt_secret)        → claims { client_id, space_id, … }
  ├─ Resolve Client from InboundClientRepository
  ├─ Inject OAuthContext into request extensions
  └─ next.run(request)
```

On any failure (missing header, invalid JWT, unknown client) the middleware returns `401 Unauthorized` with a `WWW-Authenticate: Bearer` header containing a `resource_metadata_uri` pointing at the gateway's RFC 9728 metadata endpoint. MCP 2025-11-25 clients interpret this as a signal to run the DCR + PKCE flow automatically.

### Token issuance

AI clients that don't yet have a token discover the gateway's OAuth endpoints via `/.well-known/oauth-authorization-server`. They POST to `/oauth/register` (DCR), receive a `client_id`, then run the PKCE `authorization_code` flow. The gateway issues JWTs signed with a per-install secret stored in the OS keychain (see [`security-and-credentials.md`](./security-and-credentials.md)).

Access tokens (`mcpmux_access`) and refresh tokens (`mcpmux_refresh`) have separate lifetimes. `POST /oauth/token` with `grant_type=refresh_token` issues a new access token without requiring the user to re-authorize.

### Client entity

A `Client` row in SQLite tracks each registered AI client (`client_id`, `client_name`, `space_id`, `created_at`). The space association is set at registration time and determines which `Space`'s servers the client can reach.

---

## McpMuxGatewayHandler

After middleware, the `rmcp` `StreamableHttpService` dispatches to `McpMuxGatewayHandler` (`crates/mcpmux-gateway/src/mcp/handler.rs`), which implements `rmcp::ServerHandler`.

### `tools/list`

```
handler.list_tools(req, ctx)
  │
  ├─ get_oauth_context(extensions)      → OAuthContext { client_id, space_id }
  ├─ roots probe (on-demand if PendingRoots)
  ├─ FeatureSetResolverService::resolve(space_id, roots)
  │     → Tier 1: WorkspaceBinding longest-prefix match
  │       Tier 2: PendingRoots (roots capable but not yet stored)
  │       Tier 3: Grant (pre-approved)
  │       Tier 4: Deny
  ├─ MetaToolRegistry::list()           → ~14 mcpmux_* tools (always present)
  ├─ FeatureService::get_advertised_tools_for_grants(space_id, fs_ids, session_id)
  │     → surfaced tools from binding's FeatureSet (if any)
  └─ merge + return
```

The `~14 mcpmux_*` meta tools always appear in `tools/list`. Surfaced backend tools (those with `surfaced: true` on their `FeatureSetMember`) appear alongside them. Unsurfaced tools are only reachable via `mcpmux_invoke_tool`.

### `tools/call`

```
handler.call_tool(name, args, ctx)
  │
  ├─ if name starts with "mcpmux_":
  │     MetaToolRegistry::call(name, args, ctx)    → meta-tool handler
  └─ else:
        FeatureSetResolverService::resolve(…)
        RoutingService::call_tool(space_id, name, args, grants)
          │
          ├─ verify tool is in effective grant set (FeatureService)
          ├─ strip prefix → (server_id, bare_tool_name)
          ├─ PoolService::get_instance(space_id, server_id)
          ├─ instance.call_tool(bare_name, args)
          │     on 401 → TokenService::refresh → retry once
          └─ return CallToolResult
```

---

## FeatureSet Filtering

`FeatureService` (`crates/mcpmux-gateway/src/pool/features/`) is the permission chokepoint. Every tool list and every call passes through it.

**Composition steps for a tool call:**

```
1. resolve(space_id, workspace_roots) → feature_set_ids   [FeatureSetResolverService]
2. servers_for(space_id, feature_set_ids)                  → effective_servers
3. effective = (binding_servers ∪ session_on) − session_off
4. base_tools = available tools whose server_id ∈ effective_servers
5. if pinned_tools[session_id] non-empty:
       tools = base_tools ∩ pinned_tools                  [tool-level session pin]
   else:
       tools = base_tools
```

Step 5 is the `mcpmux_pin_this_session` filter described in [`reference/tool-level-session-pin.md`](../reference/tool-level-session-pin.md).

`FeatureSetResolverService` uses a four-tier resolver:

| Tier | Condition | Result |
| ---- | --------- | ------ |
| 1a | Binding found, roots match | Use binding's FeatureSet IDs |
| 1b | Roots capable, roots not yet stored | `PendingRoots` — empty FS, retry after roots land |
| 2 | Pre-approved client grant | Use grant's FeatureSet IDs |
| 3 | No binding, no grant | `Deny` |

---

## OAuth Token Refresh (Outbound)

Backend MCP servers that use OAuth (HTTP transport) get tokens managed by `OutboundOAuthManager` (`crates/mcpmux-gateway/src/pool/oauth.rs`). The `rmcp` SDK's `AuthorizationManager` handles the full OAuth 2.1+PKCE state machine: metadata discovery → DCR → PKCE flow → token storage.

Token refresh is **automatic**: when `RoutingService::call_tool` receives a `401` from a backend server, it calls `TokenService::refresh_token(space_id, server_id)`, which asks `OutboundOAuthManager` for a fresh access token using the stored refresh token. The tool call is then retried once with the new token.

`DatabaseCredentialStore` (`crates/mcpmux-gateway/src/pool/credential_store.rs`) persists tokens encrypted with AES-256-GCM before writing to SQLite. See [`security-and-credentials.md`](./security-and-credentials.md) for the encryption details.

---

## ServiceContainer

`ServiceContainer` (`crates/mcpmux-gateway/src/server/service_container.rs`) is the dependency-injection root. It wires all gateway services at startup and exposes `Arc<T>` handles to each:

| Field | Type | Role |
| ----- | ---- | ---- |
| `pool_services.pool_service` | `Arc<PoolService>` | Manages server instances |
| `pool_services.feature_service` | `Arc<FeatureService>` | Permission resolution + tool listing |
| `pool_services.connection_service` | `Arc<ConnectionService>` | Connect/disconnect transport |
| `pool_services.token_service` | `Arc<TokenService>` | Outbound token refresh |
| `pool_services.oauth_manager` | `Arc<OutboundOAuthManager>` | Backend OAuth state machines |
| `feature_set_resolver` | `Arc<FeatureSetResolverService>` | Workspace binding → FeatureSet IDs |
| `grant_service` | `Arc<GrantService>` | Centralized grant writes + notifications |
| `approval_broker` | `Arc<ApprovalBroker>` | Desktop/web approval dialog bridge |
| `meta_tools` | `Arc<MetaToolRegistry>` | `mcpmux_*` tool registry |
| `startup_orchestrator` | `Arc<StartupOrchestrator>` | Boot pool connect |
| `session_roots` | `Arc<SessionRootsRegistry>` | Per-session workspace roots |
| `embedding_warmer` | `Arc<EmbeddingWarmer>` | Embedding cache warm-up |
| `server_manager` | `Arc<ServerManager>` | UI status events |

Services are initialized once and shared via `Arc`. No global state — all dependencies flow through `ServiceContainer`.

---

## Related docs

- [`architecture.md`](./architecture.md) — end-to-end capability flow
- [`consent-and-binding.md`](./consent-and-binding.md) — WorkspaceBinding and FeatureSet model
- [`tool-discovery-and-search.md`](./tool-discovery-and-search.md) — `mcpmux_search_tools` and `mcpmux_invoke_tool`
- [`server-lifecycle-and-pool.md`](./server-lifecycle-and-pool.md) — connection pool, session readiness, transports
- [`security-and-credentials.md`](./security-and-credentials.md) — OAuth 2.1+PKCE, token encryption, keychain
