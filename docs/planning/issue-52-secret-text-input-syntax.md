# Issue #52 — `${secret:}` / `${text:}` / `${env:}` Input Syntax

**Last Updated:** May 16, 2026
**Status:** Planning — decisions locked, ready for implementation
**Branch:** `feature/issue-52-secret-text-input-syntax`
**Issue:** [mcpmux/mcp-mux#52](https://github.com/mcpmux/mcp-mux/issues/52)
**Unblocks:** [`jsg-tech-check` secret-management Phase 4](../../../jsg-tech-check/docs/planning/secret-management.md)

---

## Problem

Input fields on installed servers today hold literal credential values. The only resolution syntax is `${input:KEY}`, which substitutes the user's pasted value. There is no way to say "this input pulls from a process env var", "this input pulls from the OS keychain", or "this input pulls from Infisical."

Concrete consequences:

- Every credential for every MCP server lives encrypted in `installed_servers.input_values` (AES-256-GCM, master key in OS Keychain). Better than plaintext, but McpMux becomes a credential silo per machine.
- Adding a new machine (Rivendell, Rohan) means re-pasting ~35 credentials manually.
- No path to centralize secrets in a real secret manager — McpMux always has its own copy.
- The homelab plan's Phase 4 ("McpMux holds no secret values") is blocked on this issue.

The issue title proposes new placeholder syntaxes: `${text:KEY}`, `${secret:KEY}`, `${env:KEY}`. This doc fleshes out what each means and how they get resolved.

---

## Decisions

| # | Decision | Choice | Rationale |
| - | -------- | ------ | --------- |
| 1 | Syntax semantics | `${text:KEY}` = literal user input. `${env:KEY}` = process env only. `${secret:KEY}` = provider chain (first hit wins). | Matches the homelab plan's migration gradient: same `${secret:KEY}` works whether the value lives in env (Phase 1), keychain (Phase 2), or Infisical (Phase 4). No rewrites required as secrets migrate. |
| 2 | Schema migration | `type: "secret"` is sugar for `{ type: "text", secret: true }`. `secret: bool` stays as the storage truth. | Zero migration cost. Existing definitions in `mcp-servers/` keep working unchanged. New definitions can use the nicer syntax. No deprecation purgatory. |
| 3 | Provider architecture | `SecretProvider` trait + `ChainProvider` wrapper + three concrete impls (env, OS keychain, Infisical). | Chain semantics need a wrapper anyway. New providers (Vault, 1Password) slot in without touching the resolver. Matches existing repository pattern in `mcpmux-core`. |
| 4 | Placeholder location | Install-time value layer. User pastes `${secret:GITHUB_PAT}` into the input field in McpMux UI. | Zero changes required to `mcp-servers` definitions. Per-user migration pace. Matches the mental model already documented in `secret-management.md`. |
| 5 | UI scope | Full settings panel: Infisical credentials, OS keychain CRUD, per-space provider chain order, "resolved from: X" indicators. | Locked when "full settings" was chosen — Infisical is in this PR, not deferred. |

---

## The Model

### Resolution syntaxes

| Syntax | Source | Notes |
| ------ | ------ | ----- |
| `${input:KEY}` | User-entered value in `input_values` | Existing behavior, unchanged for backward compat |
| `${text:KEY}` | User-entered value in `input_values` | Alias for `${input:KEY}` — included so the trio reads symmetrically |
| `${env:KEY}` | Process environment variable | Hard fail if unset; logged at resolution time |
| `${secret:KEY}` | `ChainProvider` lookup (first hit wins) | Hard fail if no provider returns a value; resolution source traced in logs |

### Provider chain

Per-space ordered list of providers. Default chain on first run: `[env]` only. User can extend in settings.

```
${secret:GITHUB_PAT}
  → EnvProvider.get("GITHUB_PAT")     → not set, miss
  → KeychainProvider.get("GITHUB_PAT") → not stored, miss
  → InfisicalProvider.get("GITHUB_PAT") → "ghp_xxx", HIT
  ↳ trace: "resolved GITHUB_PAT from infisical"
```

### Where resolution happens

`resolve_placeholders` in `crates/mcpmux-gateway/src/pool/transport/resolution.rs` is the single chokepoint. It already handles `${input:KEY}`; new branches handle the new prefixes. Resolution runs at transport-build time, every time a server is launched or reconnected — never cached at install.

### What McpMux still stores

| Item | Storage | Encryption |
| ---- | ------- | ---------- |
| `${input:KEY}` / `${text:KEY}` values | `installed_servers.input_values` (SQLite) | AES-256-GCM via `FieldEncryptor` (existing) |
| `${env:KEY}` references | The literal string `${env:KEY}` in `input_values` | None needed — it's not a secret |
| `${secret:KEY}` references | The literal string `${secret:KEY}` in `input_values` | None needed — it's not a secret |
| Infisical `clientId` | `secret_provider_configs.config` (plain JSON column) | None — public identifier |
| Infisical `clientSecret` | `secret_provider_configs.config` (encrypted) | AES-256-GCM via existing `FieldEncryptor` |
| OS Keychain entries | OS Keychain (`mcpmux.secrets` service) | Platform-native (DPAPI on Windows, Keychain on macOS, Secret Service on Linux) |
| Per-space chain order | `secret_provider_configs.chain_order` (plain JSON) | None — ordering is not sensitive |

---

## Architecture

```
                ┌──────────────────────────────────────┐
                │  resolve_placeholders (gateway)      │
                │  ${input:K} → input_values[K]        │
                │  ${text:K}  → input_values[K]        │
                │  ${env:K}   → std::env::var(K)       │
                │  ${secret:K} → chain.get(K) ◀────────┼─┐
                └──────────────────────────────────────┘ │
                                                          │
                ┌─────────────────────────────────────────┘
                │
                ▼
        ┌────────────────────┐
        │  ChainProvider     │  ordered list, first-hit-wins, traces source
        │  (impls Provider)  │
        └─┬──────┬──────┬────┘
          │      │      │
          ▼      ▼      ▼
       ┌────┐┌────┐ ┌──────────┐
       │Env ││Key ││Infisical │
       │    ││chain│ │ (REST)   │
       └────┘└─────┘ └──────────┘
```

- `SecretProvider` trait lives in `mcpmux-core::service`. Async, returns `Result<Option<String>>` (None = miss, Err = transient failure).
- `ChainProvider` wraps `Vec<Arc<dyn SecretProvider>>` and is itself a `SecretProvider`. Resolution missing from all members returns `Ok(None)`; the resolver treats that as a hard failure for `${secret:}` (logs which providers were tried).
- Each space owns a `SecretProviderConfig` row: chain order + per-provider configuration (Infisical credentials, project ID, env slug).
- Tauri commands expose CRUD on provider config + keychain entries. The chain is rebuilt on config change; existing connections are not disturbed unless explicitly reconnected.

---

## Files to create

| File | Purpose |
| ---- | ------- |
| `crates/mcpmux-core/src/service/secret_provider/mod.rs` | `SecretProvider` trait, `ChainProvider`, resolution-source trace type |
| `crates/mcpmux-core/src/service/secret_provider/env.rs` | `EnvProvider` — `std::env::var` lookup |
| `crates/mcpmux-core/src/service/secret_provider/keychain.rs` | `KeychainProvider` — reads from OS Keychain under service `mcpmux.secrets` |
| `crates/mcpmux-core/src/service/secret_provider/infisical.rs` | `InfisicalProvider` — REST client using `reqwest` (Universal Auth → bearer → secret fetch with TTL cache) |
| `crates/mcpmux-core/src/domain/secret_provider_config.rs` | Domain entity for per-space chain config |
| `crates/mcpmux-core/src/repository/secret_provider_repository.rs` | Repository trait |
| `crates/mcpmux-storage/src/repositories/secret_provider_repository.rs` | SQLite impl (encrypts Infisical `clientSecret` via `FieldEncryptor`) |
| `crates/mcpmux-storage/src/migrations/00X_secret_providers.sql` | `secret_provider_configs` table |
| `apps/desktop/src-tauri/src/commands/secret_providers.rs` | Tauri commands: list/save chain config, CRUD keychain entries, test Infisical connection |
| `apps/desktop/src/features/settings/SecretProvidersPage.tsx` | Settings UI: per-space chain order (drag-reorderable) + provider config tabs |
| `apps/desktop/src/features/settings/InfisicalConfigForm.tsx` | Infisical creds form + test-connection button |
| `apps/desktop/src/features/settings/KeychainSecretsForm.tsx` | OS Keychain CRUD |
| `docs/planning/issue-52-secret-text-input-syntax.md` | This doc |

## Files to modify

| File | Change |
| ---- | ------ |
| `crates/mcpmux-gateway/src/pool/transport/resolution.rs` | Extend `resolve_placeholders` to handle `${text:}`, `${env:}`, `${secret:}`. Pass `ChainProvider` in. Add tracing per resolution source. |
| `crates/mcpmux-gateway/src/pool/transport/mod.rs` | Wire `ChainProvider` from `state.rs` into `build_transport_config` callsite |
| `crates/mcpmux-gateway/src/server/state.rs` | Hold per-space `Arc<ChainProvider>` cache, rebuild on config-change events |
| `crates/mcpmux-core/src/registry/types.rs` | `InputType` deserializer accepts `"secret"` and expands to `{ type: Text, secret: true }`. Add a `From` impl or custom `Deserialize`. |
| `crates/mcpmux-core/src/lib.rs` | Re-export `SecretProvider`, `ChainProvider`, config entity, repository trait |
| `crates/mcpmux-core/src/event_bus.rs` | New event variants: `SecretProviderConfigChanged`, `SecretResolved { provider, key }` (debug only) |
| `apps/desktop/src/types/registry.ts` | Mirror `type: "secret"` accept in `InputDefinition.type` union |
| `apps/desktop/src/features/servers/ServersPage.tsx` | Helper text in config modal: "Use `${env:NAME}` for process env, `${secret:NAME}` to pull from configured providers." Show "resolved from: X" badge next to fields after a successful connect. |
| `apps/desktop/src/App.tsx` | Add `/settings/secret-providers` route |
| `Cargo.toml` (workspace) | No new deps — `reqwest`, `keyring`, `serde_json` already present |
| (sibling repo) `mcp-servers/schemas/server-definition.schema.json` | Accept `"secret"` in `type` enum. Follow-up PR in that repo. |

---

## Phasing

### Phase 1 — Schema sugar + placeholder parser

**Effort:** 1 evening

- Extend `InputType` deserializer in `crates/mcpmux-core/src/registry/types.rs` to accept `"secret"` and expand to `{ type: Text, secret: true }`. Round-trip serializer keeps emitting the canonical form.
- Add `${env:KEY}` resolution path to `resolve_placeholders` (process env lookup via `std::env::var`). Log the resolution and miss cases.
- Add `${text:KEY}` as a literal alias for `${input:KEY}` (same lookup map). Documented as a forward-compat name.
- Add `${secret:KEY}` stub that always misses (no chain wired yet) and logs a clear "no provider configured" warning.
- Mirror the `"secret"` type acceptance in `apps/desktop/src/types/registry.ts`.
- Unit tests in `resolution.rs` for each new prefix.

**Outcome:** A server definition with `type: "secret"` loads and renders as a masked text field. A user can paste `${env:HOME}` into any input value and at connect time `HOME` is resolved from the McpMux process environment. `${secret:KEY}` logs "no provider configured" but does not crash. No UI changes yet.

### Phase 2 — `SecretProvider` trait + `EnvProvider` + `ChainProvider`

**Effort:** 1 day

- Define `SecretProvider` trait in `mcpmux-core::service::secret_provider`: `async fn get(&self, key: &str) -> Result<Option<String>>`.
- Implement `EnvProvider` (delegates to `std::env::var`, returns `Ok(None)` on `NotFound`).
- Implement `ChainProvider`: holds `Vec<Arc<dyn SecretProvider>>`, iterates in order, returns first `Ok(Some(_))`. Returns `Ok(None)` if all miss. Returns provider-side errors only if every provider errored.
- Add tracing: every resolution emits `tracing::debug!` with `key`, `provider_name`, `outcome`.
- Add `SecretProviderConfig` domain entity (just chain order for now, no provider-specific config).
- Add a repository trait + SQLite impl. Migration adds `secret_provider_configs` table.
- Wire a `ChainProvider` per space into `server::state::AppState`. Default chain on first run: `[EnvProvider]`.
- Replace the Phase 1 stub in `resolve_placeholders` with a real call to the per-space chain.

**Outcome:** Launching McpMux with `GITHUB_PAT=ghp_xxx` in its environment and a server input value of `${secret:GITHUB_PAT}` resolves correctly at connect time. Logs show `resolved GITHUB_PAT from env`. Behavioral parity with `${env:GITHUB_PAT}`, but routed through the trait.

### Phase 3 — OS Keychain provider

**Effort:** 1 day

- Implement `KeychainProvider`: reads from OS Keychain under service `mcpmux.secrets`, account `<key_name>`. Uses the existing `keyring` crate that backs `MasterKeyProvider`.
- Tauri commands: `keychain_secrets_list`, `keychain_secret_set`, `keychain_secret_delete`. List returns names only, never values.
- Settings UI: `KeychainSecretsForm` — table of (name, last-set timestamp), "add new" form with name + value paste. Values are write-only after creation.
- `SecretProviderConfig` extended with the chain-order array. Default chain now offerable as `[env, keychain]` when user opts in.
- "Test resolve" button next to the chain: enter a key, see which provider answered.

**Outcome:** A user can paste `GITHUB_PAT` into the Keychain Secrets settings form. The string is stored in the OS Keychain under `mcpmux.secrets/GITHUB_PAT` (not in SQLite, not in McpMux process memory after the call returns). With the chain `[env, keychain]`, `${secret:GITHUB_PAT}` resolves from keychain when env is unset. The settings UI shows "Last set 2 minutes ago" but never displays the value.

### Phase 4 — Infisical provider

**Effort:** 2-3 days

- Implement `InfisicalProvider`: takes `host_url`, `client_id`, `client_secret`, `project_id`, `env_slug`. Uses `reqwest` (already a workspace dep).
  - On first `get()` call, POST `/api/v1/auth/universal-auth/login` to fetch bearer token. Cache token + expiry in memory.
  - On subsequent calls, reuse cached token until 60s before expiry, then re-login.
  - On `get(key)`, GET `/api/v3/secrets/raw/{key}?environment={env_slug}&workspaceId={project_id}`. Map 404 → `Ok(None)`. Map auth failures → `Err`.
- Extend `SecretProviderConfig.infisical: Option<InfisicalConfig>`. Persisted with `clientSecret` encrypted via the existing `FieldEncryptor`.
- Tauri commands: `infisical_config_get`, `infisical_config_set`, `infisical_test_connection` (does login + lists one secret to verify).
- Settings UI: `InfisicalConfigForm` — host URL (default `https://app.infisical.com`), client ID, client secret (paste, then write-only), project ID, per-space env slug mapping.
- Chain can now include `infisical`. Default suggestion: `[env, keychain, infisical]`.

**Outcome:** A user enters Infisical Universal Auth credentials in settings, maps Personal space → `personal` env, clicks "Test connection" and sees a green check. With the chain `[env, infisical]`, an input value of `${secret:GITHUB_PAT}` resolves from Infisical when env is unset. Logs show `resolved GITHUB_PAT from infisical`. McpMux SQLite holds no copy of the secret value.

### Phase 5 — Chain reorder UI + resolution indicators

**Effort:** 1 day

- Drag-reorderable provider list in `SecretProvidersPage` (per space). Saves on drop. New chain takes effect on next connect.
- "Resolved from: <provider>" badge next to filled input fields in the server config modal, populated from the last connect's resolution trace.
- Helper text in the config modal explaining the three syntaxes with copy-paste examples.
- README + CHANGELOG updates.

**Outcome:** From any server's config modal, a user can see at a glance which provider supplied each secret. Reordering providers in settings changes which one is hit first on next reconnect, without restarting McpMux. The three syntaxes are discoverable from the UI without reading docs.

---

## Out of scope

| Item | Reason |
| ---- | ------ |
| Vault, 1Password, Bitwarden providers | Trait makes them trivial to add later. Ship the most-asked-for one (Infisical) first. |
| Migration tooling for existing literal credentials | Users move their own values over time. Per-user migration is intentional — no bulk rewrite. |
| Cross-space credential sharing | Each space owns its chain. Sharing Infisical creds across spaces is achieved by entering the same credentials twice — a UI affordance for this can come later. |
| Automated secret rotation | Most providers (GitHub PAT, Apify, OpenRouter) don't expose rotation APIs. Reminders live in Infisical. |
| `${secret:}` syntax inside server-definition transport templates (option 4.3 "both") | Defer until a registry-side use case appears. v1 is install-time only. |
| Custom secret-name aliases (Infisical `MY_GH_TOKEN` → input `GITHUB_PAT`) | YAGNI for v1. Use the same name in both places. |
| Schema validator changes in `mcp-servers/` repo | Cross-repo PR. Accepting `type: "secret"` in the JSON Schema is a follow-up. McpMux already accepts it in its loader; mcp-servers validation is independent. |

---

## Key files referenced

| File | Why |
| ---- | --- |
| [`crates/mcpmux-gateway/src/pool/transport/resolution.rs`](../../crates/mcpmux-gateway/src/pool/transport/resolution.rs) | The single chokepoint for placeholder resolution. All new syntaxes plug in here. |
| [`crates/mcpmux-core/src/registry/types.rs`](../../crates/mcpmux-core/src/registry/types.rs) | `InputType` enum + `InputDefinition` schema. `type: "secret"` sugar lands here. |
| [`crates/mcpmux-core/src/domain/installed_server.rs`](../../crates/mcpmux-core/src/domain/installed_server.rs) | `input_values` map — unchanged by this work, but it's where literal text values continue to live. |
| [`crates/mcpmux-storage/src/repositories/installed_server_repository.rs`](../../crates/mcpmux-storage/src/repositories/installed_server_repository.rs) | Existing `FieldEncryptor` AES-256-GCM pattern, reused for Infisical `clientSecret` storage. |
| [`crates/mcpmux-storage/src/keychain.rs`](../../crates/mcpmux-storage/src/keychain.rs) | `MasterKeyProvider` / `KeychainKeyProvider` — pattern reference for `KeychainProvider`. |
| [`apps/desktop/src/features/servers/ServersPage.tsx`](../../apps/desktop/src/features/servers/ServersPage.tsx) | Config modal lines 1296-1419 — where the `type` switch and `secret` masking render. |
| [`apps/desktop/src/types/registry.ts`](../../apps/desktop/src/types/registry.ts) | TS mirror of `InputDefinition`. Must stay in sync with the Rust schema. |

---

## Related work

- [mcpmux/mcp-mux Issue #52](https://github.com/mcpmux/mcp-mux/issues/52) — the feature request driving this doc.
- [mcpmux/mcp-mux PR #152](https://github.com/mcpmux/mcp-mux/pull/152) — DCR redirect URI fix (merged into local v0.3.0). Unblocks Cursor connectivity; complementary to this work.
- [`jsg-tech-check` secret-management.md](../../../jsg-tech-check/docs/planning/secret-management.md) — the homelab plan that this PR unblocks. Phase 4 of that plan depends on this issue shipping.
- [Infisical SDK docs (Universal Auth)](https://github.com/infisical/infisical/blob/main/docs/sdks/languages/node.mdx) — reference for the Phase 4 REST client behavior. No Rust SDK exists; REST is the supported integration path.

---

## Reconciliation

This doc is the source of truth for what gets built. When implementation completes, update the **Status** field at the top and reconcile any deviations (extra files, dropped phases, scope changes) per [`update-planning-md`](~/.cursor/commands/update-planning-md.md).
