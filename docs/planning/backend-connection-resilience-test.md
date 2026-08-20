# Backend Connection Resilience — Manual Test Playbook

**For:** re-running verification on `root-resolution`
**Last Updated:** Aug 20, 2026
**Implements:** [`backend-connection-resilience.md`](./backend-connection-resilience.md)
**Shipped:** `c09e569` (retry + FK guard), `54d0de2` (literal `transport closed`). Matcher now also normalizes rmcp Display variants (camelCase / punctuation).

---

## Results (Aug 20, 2026)

| Case | Result |
| ---- | ------ |
| A bind FK | **PASS** — `invalid_argument` + `mcpmux_list_feature_sets`; 38/74 counts unchanged |
| B stdio reconnect | **PASS** — `wakatime` / `wakatime_wakatime_summaries`. Kill child → invoke succeeded. Log: `trigger=transport_closed`, `reconnect_fresh completed ok=true`. Live error: `MCP call failed: Transport closed` |
| C HA idle | **SKIPPED** |
| D unmatched | **SKIPPED** — grant-layer "did you mean", never reached classifier |

Confounders: Tauri-watch rebuild wiped inbound sessions (`POST /mcp` → 404 until MCP reload). After reload, 6 unpinned roots required one `mcpmux_set_workspace_root` to *this* repo before invoke was possible. That pin was session disambiguation, not the reconnect path.

**Inbound 404 after gateway rebuild (Decision 3, follow-on):** process death drops `LocalSessionManager`. `POST /mcp` with a stale `Mcp-Session-Id` is a spec-correct 404. [`mcp-remote`](https://www.npmjs.com/package/mcp-remote) and the [TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk/issues/1708) do **not** re-`initialize` on that 404 (they stay stuck on the dead session id). Do not persist sessions. Recovery is Reload MCP once. `/health` staying 200 means the gateway is up; 404 is not "gateway is down."

**Header pin after reload (Decision 4):** a non-empty `X-Mcpmux-Workspace` is held across initialize (no session id yet) and applied when `mcp-session-id` appears. After Reload MCP, `mcpmux_list_servers` should be bound for this repo without `set_workspace_root`. Empty `${workspaceFolder}` is still the Agents-window hole.

---

## Answer first: new sessions in different roots?

**No. Do not open a new Cursor chat, do not switch workspace roots, do not call `mcpmux_set_workspace_root` to "fix" a closed connection.**

That was the old workaround. It opens a fresh *inbound* rmcp session and hides the outbound-pool bug. Using it as the Case B/C recovery makes those cases inconclusive.

| Action | Use for this test? |
| --- | --- |
| Same Cursor chat, same workspace (`/Users/joe/Desktop/Repos/Personal/mcp-mux`) | **Yes — required** |
| Reload Cursor MCP tools (once, if the gateway just rebuilt) | Yes, once, *before* Case B/C |
| `set_workspace_root` to this repo after reload, **only if** `mcpmux_list_servers` says multiple roots are unpinned | Yes — otherwise nothing is `ready` |
| `set_workspace_root` after a transport-closed invoke | **No** |
| Bind a real FeatureSet onto another repo | **No** (Case A uses a fake UUID; no write) |

Bindings are keyed by exact `workspace_root` + `machine_id`, not by chat. Stay in this repo.

---

## Snapshot (live DB, Aug 20 2026)

```
DB=~/Library/Application\ Support/com.mcpmux.desktop/mcpmux.db
LOG=~/Library/Application\ Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log
```

| Fact | Value |
| --- | --- |
| Space | `00000000-0000-0000-0000-000000000001` |
| This root | `/Users/joe/Desktop/Repos/Personal/mcp-mux` |
| Binding (Gondor `ec211deb…`) | `5a588b93-5ed1-4ada-b7cb-8e32a9f11058` → FeatureSet `All` (`fs_default_00000000-0000-0000-0000-000000000001`) |
| Binding (Rohan `5d581ac9…`) | `8e2b36b6-eeff-4818-9259-948c1b9c3b6b` → `All` |
| HA backend | `home-assistant-new` (HTTP) |
| Stdio used for B | `wakatime` |
| Pool stats | **in-memory only** — `consecutive_failures` will not appear in SQLite |

Re-count before Case A (numbers drift):

```bash
sqlite3 "$HOME/Library/Application Support/com.mcpmux.desktop/mcpmux.db" \
  "SELECT COUNT(*) FROM workspace_bindings;
   SELECT COUNT(*) FROM workspace_binding_feature_sets;"
```

---

## Preconditions

1. Debug gateway is the listener on `:45818`. Health: `curl -sf http://127.0.0.1:45818/health` → `{"status":"ok",…}`.
2. Cursor MCP `user-mcpmux` points at `http://localhost:45818/mcp`. Reload tools **once** if the binary was just rebuilt, then do not reload again mid-case.
3. Only call mux via `user-mcpmux`. No direct backend MCP servers.
4. Do not run `pnpm dev:stop` / rebuild mid-test (evicts the pool *and* all inbound sessions).
5. If `mcpmux_list_servers` reports several unpinned roots **and** this chat's `X-Mcpmux-Workspace` is empty/absent, pin `/Users/joe/Desktop/Repos/Personal/mcp-mux` once, then start Case A/B. If the header is a real path, pin should happen without `set_workspace_root`.

---

## Case A — bind FK guard (~2 min)

**Goal:** a nonexistent `feature_set_id` returns `invalid_argument`, not `FOREIGN KEY constraint failed`, and the DB does not grow.

1. Snapshot counts (query above). Call them `B0` / `J0`.
2. Call `mcpmux_bind_current_workspace` with `feature_set_id` = `00000000-0000-0000-0000-00000000dead`. Do **not** approve anything — the guard runs before consent.
3. **Pass** if the tool error JSON has `"error":"invalid_argument"` and the message contains `mcpmux_list_feature_sets`.
4. **Fail** if the message contains `FOREIGN KEY`, `internal_error`, or `constraint`.
5. Re-count. `B0` and `J0` must be unchanged.
6. Grep: `rg "bind_current_workspace rejected" "$LOG"`

---

## Case B — reconnect after a killed stdio child (~5 min)

**Goal:** a transport-closed error triggers `reconnect_fresh` (not OAuth `reconnect_instance`) and the retry succeeds. Same session.

Pick a **stdio** server that is `ready` under `All`. Cheap options: `wakatime`, `markitdown`, `chrome-devtools`.

```
mcpmux_search_tools({ "server_id": "wakatime", "mode": "browse", "limit": 5 })
```

If `server_readiness` is not `ready`, pick another stdio id, or pin this root first (see Preconditions).

1. Invoke a cheap read-only tool once so the instance is live. `wakatime`: `mcpmux_invoke_tool` `server_id=wakatime` `tool=wakatime_wakatime_summaries` with explicit `start`/`end` for today (qualified name; bare `wakatime_summaries` can fail invoke).
2. Note the time (`date -u +%H:%M:%S`).
3. Kill **only the child**, not McpMux:

```bash
pgrep -lf wakatime
# then: kill <child-pid>
# Do not kill target/debug/mcpmux. Do not `pkill -f mcpmux`.
```

4. In **this same chat**, invoke the same tool again. Do not call `set_workspace_root`.
5. **Pass** if the invoke succeeds (or a normal tool error, not raw `Transport closed` / `-32000` to the agent).
6. Grep from the timestamp in step 2:

```bash
LOG="$HOME/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log"
rg "backend call_tool failed|reconnect attempted after call_tool failure|reconnect_fresh completed" "$LOG"
```

**Pass logs:**

- `backend call_tool failed` with `trigger=transport_closed` (stdio kill has been `error=mcp call failed: transport closed`)
- `reconnect attempted after call_tool failure` with `ok=true`
- `reconnect_fresh completed` with `ok=true` (`reconnect_instance` / OAuth must **not** be the line for this failure)

**Fail:** invoke returns `Transport closed` / `-32000` / `Connection closed` to the agent, or `trigger=unmatched`, or OAuth reconnect for a stdio kill.

---

## Case C — original HA idle (optional, 15–20 min)

**Goal:** the reported HTTP shape against `home-assistant-new`.

Same as B except `server_id=home-assistant-new`, wait 15–20 min with no traffic to that server, and the historical error was `MCP error -32000: Connection closed`.

If you cannot wait, mark Case C SKIPPED and rely on B.

---

## Case D — unmatched errors still surface (~2 min)

**Goal:** a failure that is neither auth nor transport-closed is not swallowed.

1. `mcpmux_invoke_tool` against a ready server with a tool name that does not exist, e.g. `server_id=wakatime` `tool=definitely_not_a_real_tool`.
2. **Pass** if the raw error comes back (not a silent success, not a reconnect).
3. If the failure is classified unmatched, the log line is `trigger=unmatched` and there is **no** `reconnect attempted after call_tool failure` for that call.

A permission / not-found error that never hits the backend is also acceptable — note it as "never reached classifier" rather than fail.

---

## Do not

- Call `mcpmux_set_workspace_root` "to fix" a closed connection.
- Open a second agent chat in another repo to "compare sessions."
- `pnpm dev:stop` / rebuild / quit McpMux mid-case.
- `pkill -f mcpmux`.
- Write a real FeatureSet bind as part of Case A.
- Dump `credentials` / OAuth tables from the DB.

---

## Follow-on cases (pool-invalidation-and-session-survival)

Playbook for [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md). Same mux-only / no-`pkill` rules.

### Config save reconnect

Change an enabled server's header or stdio arg in Configure, **do not** click Retry Connection, invoke a tool on that server. Expect `reconnect_fresh completed ok=true` from `[ServerConfigHandler]` and the new value in use.

### Rebuild 404

Trigger a Rust rebuild. `curl -sf http://127.0.0.1:45818/health` stays 200. Existing Cursor chats 404 on `/mcp`. Reload MCP once. Do not treat 404 as a dead gateway.

### Header pin

Multi-folder Cursor window with a real `X-Mcpmux-Workspace` pointing at this repo. Reload MCP. `mcpmux_list_servers` is bound, not 6-way `bindable`. `set_workspace_root` is not the success path.

---

## Report back (copy this)

```
Case A bind FK: PASS | FAIL | BLOCKED
  error code/message:
  binding counts before/after:
  log line present: yes/no

Case B stdio reconnect: PASS | FAIL | INCONCLUSIVE | SKIPPED
  server_id / tool:
  first invoke: ok/err
  child kill: pid / skipped why
  second invoke: ok / raw Transport closed / other
  trigger= :
  reconnect_fresh ok= :
  oauth reconnect used: yes/no

Case C HA idle: PASS | FAIL | SKIPPED
  (same fields)

Case D unmatched: PASS | FAIL | SKIPPED
  error returned:
  trigger= :

Confounders (reload MCP / set_workspace_root / new chat / rebuild): none | list
```
