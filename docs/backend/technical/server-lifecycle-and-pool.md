# Server Lifecycle and Pool

**Last Updated:** Jun 1, 2026

> **Synthesis:** Consolidates [`server-account-clones.md`](../reference/server-account-clones.md), [`gateway-warm-pool-startup.md`](../reference/gateway-warm-pool-startup.md), [`agent-mcp-session-readiness.md`](../reference/agent-mcp-session-readiness.md), and [`tool-level-session-pin.md`](../reference/tool-level-session-pin.md).

This doc covers how McpMux connects to backend MCP servers, keeps them alive, and handles agent session readiness — from gateway boot through first tool invocation.

---

## Connection Pool Model

McpMux uses an **eager pool**: every `enabled` server connects at gateway startup (not on first use). This is a deliberate architectural choice — routing requires a live `ServerInstance` in the pool; there is no invoke-time lazy connect path.

### Instance key

Each pool entry is keyed by `InstanceKey { space_id: Uuid, server_id: String }`. A single Space can hold multiple server instances (e.g. `posthog` and `posthog-work`) because each `InstalledServer` row has a distinct `server_id`.

### PoolService

`PoolService` (`crates/mcpmux-gateway/src/pool/service.rs`) is the pool orchestrator:

- `connect_server(space_id, server_id)` — instantiates transport, runs MCP handshake, stores instance in `DashMap<InstanceKey, ServerInstance>`
- `disconnect_server(space_id, server_id)` — tears down transport
- `get_instance(key)` — returns `Arc<ServerInstance>` for routing

### ConnectionService

`ConnectionService` handles the per-transport connect/disconnect lifecycle. After a successful connect it calls `FeatureService::discover_and_cache` to load the server's tool/prompt/resource list into the feature cache and emit `ServerConnected { capabilities }` on the `EventBus`.

---

## Transports

| Transport | File | When used |
| --------- | ---- | --------- |
| `StdioTransport` | `pool/transport/stdio.rs` | `command`-type servers (npm/uvx/python scripts) |
| `HttpTransport` | `pool/transport/http.rs` | `url`-type servers (remote HTTP MCP servers) |

All child process spawns go through `configure_child_process_platform()` (re-exported from `pool/transport/mod.rs`):
- **macOS/Linux:** `process_group(0)` — isolates children from parent terminal signals
- **Windows:** `CREATE_NO_WINDOW` — prevents console flash in release GUI builds

Transport selection is resolved at connect time by `TransportResolutionService` (`pool/transport/resolution.rs`) from the server's `cached_definition`.

---

## Startup Connect (Tiered Warm Pool)

**Planned** — see [`reference/gateway-warm-pool-startup.md`](../reference/gateway-warm-pool-startup.md) for full design; current implementation is the sequential baseline below.

### Current (sequential) startup path

```
GatewayServer::run_with_shutdown
  └─ tokio::spawn: StartupOrchestrator::auto_connect_enabled_servers
       1. mark_all_features_unavailable()
       2. resolve_server_prefixes() (all spaces)
       3. for each enabled InstalledServer (sequential):
            set_connecting(server_id)
            connect_server(space_id, server_id)
```

All enabled servers start connecting in sequence. The HTTP listener is up immediately; pool warm-up happens in the background.

### Planned tiered model

The planned upgrade (`gateway-warm-pool-startup.md`) introduces hot/warm tiers:

```
ConnectPlanBuilder
  ├─ hot: enabled ∩ referenced_by_any_binding_FeatureSet
  └─ warm: enabled \ hot

StartupOrchestrator
  ├─ set_connecting(hot only)          ← no fleet-wide CONNECTING burst
  ├─ connect_parallel(hot, semaphore=6)
  └─ connect_parallel(warm, semaphore=6)
```

The **hot set** is the union of server IDs referenced by all `WorkspaceBinding` FeatureSets across all spaces. Those servers connect first so agent-visible tools are ready within seconds.

**Runtime warm triggers** (planned):
- `WorkspaceBindingChanged` → connect newly referenced servers
- Session roots first stored → resolve binding → connect hot set for that binding

---

## Session Readiness

Agent MCP sessions go through four readiness stages, each with a different fix boundary:

| Stage | Symptom | Where controlled |
| ----- | ------- | ---------------- |
| Cold start | All backends `CONNECTING`; github down for minutes | High — gateway (tiered warm pool) |
| Roots timing | `total_invokable: 0` until workspace roots land | Medium-high — gateway + client timing |
| Surface reload | Tool changes require client `Reload tools` | Medium — gateway emits `list_changed`; client may cache |
| FeatureSet authoring | Wrong tools in binding FeatureSet | Medium — UI/agent DX |

### Roots timing

`FeatureSetResolverService` applies a four-tier resolver on every call. When a roots-capable client hasn't yet sent its workspace roots, tier 1b (`PendingRoots`) returns an empty FeatureSet — `total_invokable: 0` — even if the client is correctly bound.

The on-demand roots probe in `handler.rs` retries `set_roots_capable` before `list_tools` and `call_tool` to narrow this window, but the race is not fully eliminable from the gateway side.

**Agent signal (planned):** `mcpmux_list_servers` will include `{ gateway_warming: true, pool: { connected, connecting, hot_pending } }` when any hot server is still warming, so agents can retry `search_tools` instead of misdiagnosing an ACL issue.

---

## Account Clones

A **clone** is a second `InstalledServer` row in the same Space for the same underlying MCP server type. It has a distinct `server_id` of the form `{base_id}-{suffix}` (e.g. `posthog-work`).

```
InstalledServer (clone)
  server_id:          "posthog-work"
  cached_definition:  <copy of source's definition at clone time>
  input_values:       {}   ← empty; user fills in configure step
  source:             ManualEntry
  cloned_from:        Some("posthog")  ← display-only in v1
  enabled:            false
```

The gateway treats the clone as an independent server — the prefix cache assigns `posthog-work` as its tool prefix, so tools appear as `posthog-work_capture_event`. No routing or FeatureSet changes are needed; clones are distinguished by `server_id` like any other install.

**Decision tree for multi-account use:**

```
Multi-account need?
├─ MCP has per-call account param (Google Workspace)
│   └─ ONE install — no clone needed
├─ Accounts map to repo context (Personal / Work / GAIT)
│   └─ Spaces — no clone needed
└─ Two+ accounts in SAME Space, single-account MCP
    └─ Clone via "Add another account" UI action
```

See [`reference/server-account-clones.md`](../reference/server-account-clones.md) for the full implementation detail and phasing.

---

## Tool-Level Session Pin

When a session has 240 tools in scope (e.g. two Google Workspace clones × 120 tools each), `mcpmux_pin_this_session` lets the LLM restrict the visible tool set for the duration of that session without modifying the underlying binding.

**Composition (planned Phase 1–4):**

```
FeatureService::get_tools_for_grants
  …existing steps 1–6 (server composition)…
  7. pinned ← SessionOverrideRegistry.pinned_tools[session_id]
  8. if pinned non-empty:
       tools = base_tools ∩ pinned   ← filter to declared set
     else:
       tools = base_tools
```

Pin is stored in `SessionOverrideRegistry` (in-memory, dies with session reap). It applies to tools only — prompts and resources are unaffected. `mcpmux_clear_session_pin` restores the full set.

See [`reference/tool-level-session-pin.md`](../reference/tool-level-session-pin.md) for decision table and phases.

---

## Server Manager

`ServerManager` (`crates/mcpmux-gateway/src/pool/server_manager.rs`) wraps `PoolService` with status-event emission. It translates connection state changes (`Connecting`, `Connected`, `Error`, `Disconnected`) into `DomainEvent::ServerStatusChanged` emissions on the `EventBus`, which the desktop UI consumes to update the server card status badges.

---

## Related docs

- [`services-overview.md`](./services-overview.md) — Axum request path, auth, routing
- [`consent-and-binding.md`](./consent-and-binding.md) — FeatureSet and WorkspaceBinding model
- [`data-model.md`](./data-model.md) — InstalledServer entity and repository traits
- [`reference/gateway-warm-pool-startup.md`](../reference/gateway-warm-pool-startup.md) — tiered startup design (decisions locked)
- [`reference/agent-mcp-session-readiness.md`](../reference/agent-mcp-session-readiness.md) — full session readiness umbrella plan
- [`reference/server-account-clones.md`](../reference/server-account-clones.md) — clone implementation and phasing
- [`reference/tool-level-session-pin.md`](../reference/tool-level-session-pin.md) — session pin design
