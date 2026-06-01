# Security and Credentials

**Last Updated:** Jun 1, 2026

McpMux handles two distinct credential concerns: **inbound auth** (AI clients connecting to the gateway) and **outbound auth** (gateway connecting to backend MCP servers). Both use OAuth 2.1+PKCE. Credential material never appears in plain-text JSON files — it lives in AES-256-GCM encrypted SQLite rows with the master key in the OS keychain or DPAPI.

---

## Inbound: AI Client Authentication

### OAuth 2.1 + PKCE (gateway as authorization server)

The gateway exposes a full OAuth 2.1 authorization server on `:45818`:

| Endpoint | RFC | Purpose |
| -------- | --- | ------- |
| `GET /.well-known/oauth-authorization-server` | 8414 | Metadata discovery |
| `GET /.well-known/oauth-protected-resource` | 9728 | Protected resource metadata |
| `POST /oauth/register` | 7591 | Dynamic Client Registration |
| `GET /oauth/authorize` | 6749 + 7636 | PKCE authorization endpoint |
| `POST /oauth/token` | 6749 | Token exchange + refresh |
| `GET /oauth/callback` | 8252 §7.3 | Loopback redirect handler |

**Flow for a new AI client:**

```
1. Client → GET /.well-known/oauth-protected-resource (discovers auth server URL)
2. Client → POST /oauth/register  (DCR; receives client_id)
3. Client → GET /oauth/authorize?code_challenge=…&code_challenge_method=S256  (PKCE)
4. User approves in browser (or desktop dialog)
5. Client → POST /oauth/token  (code + code_verifier exchange)
6. Gateway → { access_token, refresh_token, token_type: "Bearer" }
7. Client → all /mcp requests: Authorization: Bearer <access_token>
```

Access tokens are JWTs signed with a per-install secret (see keychain section below). `mcp_oauth_middleware` validates the JWT on every `/mcp` request, extracts `{ client_id, space_id }` from claims, and injects `OAuthContext` into Axum request extensions.

### Dynamic Client Registration (DCR)

AI clients that don't yet have a `client_id` POST to `/oauth/register` with an optional `client_name`. The gateway creates a `Client` row (`crates/mcpmux-core/src/domain/client.rs`) and returns the `client_id`. No secret is issued — the gateway uses the PKCE `S256` code challenge method (`none` for `token_endpoint_auth_methods_supported`), so the client authenticates by proving it knows the `code_verifier`.

MCP spec 2025-11-25: the gateway advertises `client_id_metadata_document_supported: true` in its OAuth metadata, allowing clients to discover their own metadata at `GET /oauth/clients/{client_id}`.

### JWT signing secret

The JWT signing secret is stored in the OS keychain under the `jwt-signing-secret` key name (same `KeychainKeyProvider` used for the master encryption key). It is loaded at gateway startup and held in `GatewayState`. It never touches SQLite.

---

## Outbound: Backend Server Authentication

### OAuth 2.1 + PKCE (gateway as OAuth client)

HTTP-transport MCP servers that require OAuth use `OutboundOAuthManager` (`crates/mcpmux-gateway/src/pool/oauth.rs`). The `rmcp` SDK's `OAuthState` state machine handles:

1. **Metadata discovery** — fetch `/.well-known/oauth-authorization-server` from the backend server URL
2. **DCR** — register the gateway as a client with the backend's authorization server
3. **PKCE flow** — open browser to backend's auth URL; receive redirect at `http://127.0.0.1:{ephemeral_port}/oauth2redirect`
4. **Token storage** — persist `{ access_token, refresh_token, expires_at }` via `DatabaseCredentialStore` (AES-256-GCM encrypted)

**Automatic token refresh:** When `RoutingService::call_tool` receives a `401` from a backend server, `TokenService::refresh_token` exchanges the stored refresh token for a new access token without user interaction. The tool call is retried once. If refresh fails (expired or revoked), the server's status is set to `Error` and the UI surfaces a re-authorize prompt.

The OAuth client registration record (`OutboundOAuthRegistration`) and the tokens themselves are stored in separate, independently encrypted rows in SQLite.

### stdio servers (credential injection at spawn)

Stdio servers receive credentials via environment variables injected at process spawn time. The credential values are decrypted from SQLite immediately before `Command::spawn`, used to build the env map, and never held in memory beyond the spawn call. The child process receives them in its environment — the gateway does not retain a copy post-spawn.

---

## Credential Encryption (AES-256-GCM)

All credential material in SQLite is encrypted at the **field level** before the row is written.

### FieldEncryptor (`crates/mcpmux-storage/src/crypto.rs`)

- **Algorithm:** AES-256-GCM (authenticated encryption) via `ring::aead`
- **Key:** 32-byte (256-bit) master key from `MasterKeyProvider`
- **Nonce:** 12-byte random nonce generated per encrypt call via `ring::rand::SystemRandom`
- **Format on disk:** `hex(nonce || ciphertext || GCM_tag)` — 12 + N + 16 bytes, hex-encoded

The GCM authentication tag ensures ciphertext integrity: any tampering causes decryption to fail rather than return corrupt data.

Each credential row gets an independently random nonce, so two encryptions of the same plaintext produce different ciphertexts.

### Master key storage

| Platform | Storage | Implementation |
| -------- | ------- | -------------- |
| macOS | Keychain (login keychain) | `KeychainKeyProvider` via `keyring` crate |
| Linux | Secret Service (GNOME Keyring / KWallet) | `KeychainKeyProvider` via `keyring` crate |
| Windows | DPAPI-encrypted file | `DpapiKeyProvider` (`keychain_dpapi.rs`) |
| Fallback | Encrypted file (WSL/headless Linux) | `KeychainFileProvider` (`keychain_file.rs`) |

The master key is a 32-byte random value generated once on first run and stored in the OS keychain. All subsequent starts load it from the keychain before initializing `FieldEncryptor`.

`MasterKeyProvider` is a trait (`crates/mcpmux-storage/src/keychain.rs`) — the concrete implementation is selected at runtime based on platform capability.

### zeroize

Secrets are wiped from memory after use via the `zeroize` crate. Key material is wrapped in `Zeroizing<[u8; 32]>` so the stack buffer is zeroed on drop. Credential strings decoded from SQLite for injection into child processes are held in `Zeroizing<String>` for the same reason.

---

## What Is Never Stored in Plain Text

| Item | Where it lives |
| ---- | -------------- |
| Master key | OS keychain / DPAPI file |
| JWT signing secret | OS keychain |
| OAuth access tokens | AES-256-GCM encrypted SQLite `credentials` row |
| OAuth refresh tokens | AES-256-GCM encrypted SQLite `credentials` row |
| API keys / env secrets | AES-256-GCM encrypted SQLite `credentials` row |
| DCR client secrets | Not issued — gateway uses PKCE `none` auth method |

---

## Network Security

- Gateway binds to `127.0.0.1` only — no LAN exposure.
- Admin API (`:45819`) is also loopback-only. Remote access goes through Cloudflare Tunnel + Cloudflare Access on a dedicated hostname. Mutating admin routes require a valid `CF-Access-Jwt-Assertion` header when `gateway.admin_trust_cf_access` is enabled.
- OAuth callbacks use the loopback redirect method (RFC 8252 §7.3) at `http://127.0.0.1:{ephemeral_port}/oauth2redirect` — universally compatible with enterprise IdPs that block custom URL schemes.

---

## Related docs

- [`services-overview.md`](./services-overview.md) — how auth middleware fits into the Axum request path
- [`data-model.md`](./data-model.md) — `Client`, `Credential`, `OutboundOAuthRegistration` entities
- [`server-lifecycle-and-pool.md`](./server-lifecycle-and-pool.md) — outbound OAuth manager and token refresh
