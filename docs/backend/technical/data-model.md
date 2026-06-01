# Data Model

**Last Updated:** Jun 1, 2026

All domain entities live in `crates/mcpmux-core/src/domain/`. This doc covers the entities, repository trait pattern, and the `EventBus` that wires producers to consumers.

---

## Entities

### Space

```rust
pub struct Space {
    pub id:          Uuid,
    pub name:        String,           // e.g. "Work", "Personal"
    pub icon:        Option<String>,   // emoji or icon URL
    pub description: Option<String>,
    pub is_default:  bool,
    pub sort_order:  i32,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}
```

A `Space` is an isolated configuration environment with its own credential set. Each AI client (`Client`) is bound to one Space at registration time. All `InstalledServer` rows, `FeatureSet` rows, and `WorkspaceBinding` rows are scoped to a Space.

Spaces are the answer to **context-level** account separation (work vs personal vs client project). They are not the answer to "two accounts for the same MCP in one context" — that is handled by account clones (see [`server-lifecycle-and-pool.md`](./server-lifecycle-and-pool.md#account-clones)).

---

### InstalledServer

```rust
pub struct InstalledServer {
    pub id:                 Uuid,
    pub space_id:           String,            // FK → Space.id
    pub server_id:          String,            // e.g. "github", "posthog-work"
    pub server_name:        Option<String>,    // display name
    pub cached_definition:  Option<String>,    // JSON snapshot at install time
    pub input_values:       HashMap<String, String>, // user credentials/config
    pub enabled:            bool,              // auto-connect on gateway start
    pub env_overrides:      HashMap<String, String>,
    pub args_append:        Vec<String>,
    pub extra_headers:      HashMap<String, String>,
    pub oauth_connected:    bool,
    pub source:             InstallationSource,
    pub cloned_from:        Option<String>,    // display-only clone lineage
    pub created_at:         DateTime<Utc>,
    pub updated_at:         DateTime<Utc>,
}
```

`UNIQUE(space_id, server_id)` is enforced in SQLite. Connection status (`Connected`, `Connecting`, `Error`, `Disconnected`) is **not** stored here — it's runtime state managed by `ServerManager` and communicated via `DomainEvent::ServerStatusChanged`.

`InstallationSource` variants: `Registry` (installed from catalog), `UserConfig { file_path }` (synced from `~/.cursor/mcp.json`), `ManualEntry` (UI add / clone).

---

### FeatureSet and FeatureSetMember

```rust
pub struct FeatureSet {
    pub id:          Uuid,
    pub space_id:    String,
    pub name:        String,
    pub description: Option<String>,
    pub set_type:    FeatureSetType,   // Starter | Custom
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

pub struct FeatureSetMember {
    pub feature_set_id: Uuid,
    pub server_id:      String,
    pub feature_type:   FeatureType,   // Tool | Prompt | Resource | ServerAll
    pub feature_name:   Option<String>, // None when FeatureType = ServerAll
    pub mode:           MemberMode,    // Include | Exclude
    pub surfaced:       bool,          // expose directly in tools/list
}
```

A `FeatureSet` is the unit of consent — it defines which tools, prompts, or resources from which servers are accessible. `FeatureSetType::Starter` is auto-created with each Space (historically called `Default`); `Custom` covers operator-defined sets. Both behave identically at routing time.

`MemberMode::Exclude` lets you add a `ServerAll` include and then carve out specific tools. `surfaced: true` means the tool appears directly in `tools/list` alongside the meta tools (not only via search and invoke).

---

### WorkspaceBinding

```rust
pub struct WorkspaceBinding {
    pub id:              Uuid,
    pub space_id:        String,
    pub workspace_root:  String,       // absolute path, normalized
    pub feature_set_ids: Vec<Uuid>,    // ordered; resolver unions members
    pub created_at:      DateTime<Utc>,
    pub updated_at:      DateTime<Utc>,
}
```

A `WorkspaceBinding` maps a folder root (e.g. `/Users/joe/code/my-app`) to one or more `FeatureSet` IDs. `FeatureSetResolverService` does **longest-prefix matching** against the client's reported workspace roots to find the binding for each session. Multiple bindings can exist in one Space; each maps a different subtree to a different capability set.

See [`consent-and-binding.md`](./consent-and-binding.md) for the full consent model.

---

### Client

```rust
pub struct Client {
    pub id:           Uuid,
    pub client_id:    String,   // OAuth client_id issued via DCR
    pub client_name:  Option<String>,
    pub space_id:     String,   // Space this client is scoped to
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}
```

One row per AI client registered via DCR. The `space_id` set at registration time determines which Space's servers the client can reach for the lifetime of that registration.

---

### ServerFeature

```rust
pub struct ServerFeature {
    pub id:          Uuid,
    pub space_id:    String,
    pub server_id:   String,
    pub feature_type: FeatureType,
    pub name:        String,          // qualified: "{server_alias}_{tool_name}"
    pub description: Option<String>,
    pub input_schema: Option<String>, // JSON schema for the tool's args
    pub is_available: bool,           // false while server is disconnected
}
```

`ServerFeature` rows are written by `FeatureService::discover_and_cache` when a server connects. They are the materialization of a connected server's capability list and are the records that `FeatureSetMember` references when `feature_type = Tool | Prompt | Resource`. `is_available` is toggled false when the server disconnects.

---

### Credential and OutboundOAuthRegistration

`Credential` (`domain/credential.rs`) stores one encrypted credential value per `(space_id, server_id, credential_type)` row. `CredentialType` variants include `ApiKey`, `OAuthAccessToken`, `OAuthRefreshToken`, etc.

`OutboundOAuthRegistration` (`domain/outbound_oauth_registration.rs`) stores the gateway's DCR client registration with each backend OAuth server — `client_id`, encrypted `client_secret`, scopes.

Both are encrypted at rest by `FieldEncryptor`. See [`security-and-credentials.md`](./security-and-credentials.md).

---

## Repository Trait Pattern

All persistence is behind `async_trait` repository interfaces defined in `crates/mcpmux-core/src/repository/mod.rs`. The SQLite implementations live in `crates/mcpmux-storage/src/repositories/`.

Example:

```rust
#[async_trait]
pub trait SpaceRepository: Send + Sync {
    async fn list(&self) -> RepoResult<Vec<Space>>;
    async fn get(&self, id: &Uuid) -> RepoResult<Option<Space>>;
    async fn create(&self, space: &Space) -> RepoResult<()>;
    async fn update(&self, space: &Space) -> RepoResult<()>;
    async fn delete(&self, id: &Uuid) -> RepoResult<()>;
    async fn get_default(&self) -> RepoResult<Option<Space>>;
    // …
}
```

Repository traits are defined in `mcpmux-core` so the domain and service layers have no SQLite dependency. The concrete `SqliteSpaceRepository` in `mcpmux-storage` implements the trait. Gateway and Tauri command code call only the trait — never SQLx directly.

This allows in-memory repositories in tests and clean DI without an ORM.

---

## EventBus

`EventBus` (`crates/mcpmux-core/src/event_bus.rs`) is a `tokio::sync::broadcast` channel (capacity 256) wrapping `DomainEvent`.

```rust
pub struct EventBus {
    sender: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    pub fn sender(&self) -> EventBusSender;
    pub fn subscribe(&self) -> EventBusReceiver;
}
```

**Producers** — application services emit via `EventBusSender::emit(event)`:
- `SpaceAppService` → `SpaceCreated`, `SpaceUpdated`, `SpaceDeleted`
- `ServerAppService` → `ServerInstalled`, `ServerUninstalled`, `ServerStatusChanged`, `ServerConnected`
- `PermissionAppService` → `FeatureSetCreated`, `FeatureSetMembersChanged`, `WorkspaceBindingChanged`, `ClientGrantChanged`
- `ClientAppService` → `ClientRegistered`

**Consumers** — each subscribes via `EventBus::subscribe()` and runs in its own tokio task:
- `UIEventBridge` → Tauri events → React frontend
- `MCPNotifier` → `notifications/tools/list_changed` to connected MCP peers
- `AuditLogger` → structured log or cloud sync

Consumers decide which events to handle; they receive every event but filter by variant. Cross-layer communication goes through the `EventBus` — services never call each other directly across module boundaries.

### DomainEvent (selected variants)

```rust
pub enum DomainEvent {
    SpaceCreated { space: Space },
    ServerInstalled { server: InstalledServer },
    ServerStatusChanged { space_id, server_id, status: ConnectionStatus },
    ServerConnected { space_id, server_id, capabilities: DiscoveredCapabilities },
    FeatureSetMembersChanged { feature_set_id, space_id },
    WorkspaceBindingChanged { binding: WorkspaceBinding },
    ClientGrantChanged { client_id, space_id },
    MetaToolInvoked { session_id, tool_name, decision, space_id },
    // …
}
```

Each variant is `Serialize + Deserialize` for transport over Tauri events and potential cloud audit logging.

---

## Application Services

Application services in `crates/mcpmux-core/src/application/` consume repository traits via DI and emit domain events:

| Service | File | Responsibilities |
| ------- | ---- | ---------------- |
| `SpaceAppService` | `application/space.rs` | Space CRUD, default space management |
| `ServerAppService` | `application/server.rs` | Install, uninstall, clone, enable/disable |
| `PermissionAppService` | `application/permission.rs` | FeatureSet CRUD, workspace binding, grants |
| `ClientAppService` | `application/client.rs` | Client registration and deletion |

These services are the only layer that writes to repositories and emits events. Gateway services (in `mcpmux-gateway`) consume `Arc<dyn SpaceRepository>` etc. injected via `GatewayDependencies` — they never instantiate storage directly.

---

## Related docs

- [`architecture.md`](./architecture.md) — end-to-end capability flow
- [`consent-and-binding.md`](./consent-and-binding.md) — FeatureSet and WorkspaceBinding model in depth
- [`services-overview.md`](./services-overview.md) — how repositories are consumed at the gateway layer
- [`security-and-credentials.md`](./security-and-credentials.md) — Credential and OutboundOAuthRegistration encryption
