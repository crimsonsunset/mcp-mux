# Per-Server Config Lanes

**Last Updated:** Jun 19, 2026

Each **installed server** in a Space can carry user-specific configuration beyond registry `input_values` (credentials). Four **config lanes** live on the `installed_servers` row and are edited in the desktop or web admin UI under **Servers → Configure** on an installed server.

They answer different questions:

| Lane | When it applies | Transport |
| ---- | --------------- | --------- |
| [`input_values`](#input-values-secrets) | Required credentials from the server definition | All |
| [`env_overrides`](#env_overrides) | Extra env vars for **stdio** child processes | stdio only |
| [`args_append`](#args_append) | Extra CLI args appended to the spawn command | stdio only |
| [`extra_headers`](#extra_headers) | HTTP headers on outbound MCP requests | HTTP/SSE remote only |
| [`default_params`](#default_params) | Default **tool-call arguments** merged at `mcpmux_invoke_tool` | All (invoke meta path only) |

Storage: plaintext JSON columns on `installed_servers` (except `input_values`, which uses the encrypted credential lane). See [`data-model.md`](../technical/data-model.md).

---

## Where to edit

**Desktop:** Space → **Servers** → gear icon on an installed server → **Configure**.

**Web admin:** Same flow when admin is enabled (`pnpm dev:admin` or Settings → Gateway → Web admin).

Changes persist via `save_server_inputs` / `ServerAppService::update_config` and survive gateway restarts. **Cloned servers** start with empty lanes (including `default_params`) — configure each clone independently.

---

## `input_values` (secrets)

Defined by the server catalog `transport.metadata.inputs`. API keys, tokens, paths, and other secrets the child or remote server expects at **connect** time.

- Stored encrypted (AES-256-GCM + OS keychain)
- Not repeated on every tool call — applied when the pool connects
- Use the labeled fields in the Configure modal, not the advanced lanes below

---

## `env_overrides`

Extra environment variables merged into the **stdio child process** environment at spawn time (on top of resolved `input_values` and system env).

**Use when:** a stdio MCP server reads config from env (e.g. self-hosted `sooperset/mcp-atlassian` with `JIRA_URL`, `ATLASSIAN_OAUTH_CLOUD_ID`).

**UI:** key/value rows in Configure.

**Example:**

```json
{
  "JIRA_URL": "https://your-org.atlassian.net",
  "DEBUG": "1"
}
```

**Not for:** the official Atlassian **remote** MCP (`https://mcp.atlassian.com/...`) — it does not read these env vars; use [`default_params`](#default_params) for per-call `cloudId` instead.

---

## `args_append`

Additional command-line arguments appended after the registry-defined command and args when spawning a **stdio** server.

**Use when:** the server accepts flags not modeled as catalog inputs (e.g. `--verbose`, `--port 8080`).

**UI:** one argument per line in Configure.

**Example:**

```
--verbose
--log-level
debug
```

---

## `extra_headers`

Custom HTTP headers sent on every outbound request to an **HTTP/SSE** remote MCP server (auth supplements, tenant routing, etc.).

**Use when:** the remote endpoint expects a static header (e.g. custom auth or routing header documented by the provider).

**UI:** key/value rows in Configure.

**Example:**

```json
{
  "X-Custom-Tenant": "acme-prod"
}
```

**Not for:** Atlassian `X-Atlassian-Cloud-Id` on the official remote server — that header path is a different integration; official remote takes `cloudId` as a **tool argument** → use [`default_params`](#default_params).

---

## `default_params`

A flat JSON object of **default tool-call argument names → values**, shallow-merged into every `mcpmux_invoke_tool` call for that server before the gateway forwards to the backend.

```
effective_args = { ...default_params, ...caller_args }   // caller wins on key collision
```

**Use when:** every tool on a server needs the same sticky argument and the agent should not rediscover it each session — e.g. official Atlassian remote `cloudId`, a default `projectKey`, or `org`.

**UI:** **Default Tool Parameters** JSON textarea in Configure.

**Example (Atlassian):**

```json
{
  "cloudId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

With that set, a meta invoke succeeds on the first call:

```json
{
  "server_id": "com.atlassian-mcp",
  "tool": "getJiraIssue",
  "args": { "issueIdOrKey": "PROJ-123" }
}
```

The gateway forwards `{ "cloudId": "...", "issueIdOrKey": "PROJ-123" }` to the backend.

**Semantics:**

- Values are arbitrary JSON (string, number, boolean, object) — not string-only
- Applied only on the **`mcpmux_invoke_tool`** meta path, not direct gateway `call_tool` or surfaced one-hop tools
- **Non-secret** sticky params only; API keys and tokens stay in `input_values`
- Caller-supplied `args` always override keys present in `default_params` (unless **Collision strategy** is set to Override in Configure)

**Agent visibility:** Agents do not need to call `getAccessibleAtlassianResources` when defaults are configured. After setup:

- `mcpmux_list_servers` includes `prefilled_params: ["cloudId", ...]` per server when defaults are set
- `mcpmux_search_tools` marks matching entries in `required_params` with `"prefilled": true` so agents know those keys are auto-filled at invoke

**Implementation:** `crates/mcpmux-gateway/src/services/meta_tools/invoke_tool.rs` (`merge_default_params`).

---

## Choosing a lane (quick reference)

| Goal | Lane |
| ---- | ---- |
| API key / OAuth secret at connect | `input_values` |
| Stdio server env var | `env_overrides` |
| Stdio CLI flag | `args_append` |
| Remote HTTP header on every request | `extra_headers` |
| Same tool arg on every invoke (e.g. `cloudId`) | `default_params` |

---

## Related docs

- [`tool-discovery-and-search.md`](../technical/tool-discovery-and-search.md) — search → invoke workflow, `bare_name` and `required_params` (`{ name, type }`) on search hits
- [`server-lifecycle-and-pool.md`](../technical/server-lifecycle-and-pool.md) — when env/args/headers are applied (connect time vs invoke time)
- [`meta-tool-invoke-ergonomics.md`](../../planning/meta-tool-invoke-ergonomics.md) — design rationale for `default_params`; Round 3 search UX + agent visibility
