# Backend Connection Resilience — Manual Test Playbook

**For:** any agent (or human) verifying `c09e569` / `root-resolution`
**Last Updated:** Aug 20, 2026
**Implements:** [`backend-connection-resilience.md`](./backend-connection-resilience.md)
**Do not implement code.** Execute the cases below, record pass/fail, stop.

---

## Answer first: new sessions in different roots?

**No. Do not open a new Cursor chat, do not switch workspace roots, do not call `mcpmux_set_workspace_root`.**

That was the old workaround. It opens a fresh *inbound* rmcp session and hides the outbound-pool bug. Using it here makes Case B/C inconclusive.

| Action | Use for this test? |
| --- | --- |
| Same Cursor chat, same workspace (`/Users/joe/Desktop/Repos/Personal/mcp-mux`) | **Yes — required** |
| Reload Cursor MCP tools (once, before starting, if the gateway just rebuilt) | Yes, once |
| New chat / different root / `set_workspace_root` | **No** (confounds reconnect) |
| Bind a real FeatureSet onto another repo | **No** (Case A uses a fake UUID; no write) |

Bindings are keyed by exact `workspace_root` + `machine_id`, not by chat. This repo already has two rows for the same path (Gondor vs Rohan), both on FeatureSet `All`. Stay here.

---

## Snapshot (live DB, Aug 20 2026)

```
DB=~/Library/Application\ Support/com.mcpmux.desktop/mcpmux.db
LOG=~/Library/Application\ Support/com.mcpmux.desktop/logs/mcpmux.2026-08-20.log
```

| Fact | Value |
| --- | --- |
| Space | `00000000-0000-0000-0000-000000000001` |
| This root | `/Users/joe/Desktop/Repos/Personal/mcp-mux` |
| Binding (Gondor `ec211deb…`) | `5a588b93-5ed1-4ada-b7cb-8e32a9f11058` → FeatureSet `All` (`fs_default_00000000-0000-0000-0000-000000000001`) |
| Binding (Rohan `5d581ac9…`) | `8e2b36b6-eeff-4818-9259-948c1b9c3b6b` → `All` |
| HA backend | `home-assistant-new` (HTTP, enabled, 95 features) |
| Binding row count | 38 parents / 74 junction rows (re-count before Case A) |
| Pool stats | **in-memory only** — `consecutive_failures` will not appear in SQLite |

Re-count before Case A (numbers drift):

```bash
sqlite3 "$HOME/Library/Application Support/com.mcpmux.desktop/mcpmux.db" \
  "SELECT COUNT(*) FROM workspace_bindings;
   SELECT COUNT(*) FROM workspace_binding_feature_sets;"
```

---

## Preconditions

1. Debug gateway is the listener on `:45818` (ancestor = `launchd`, not Cursor Helper). Health: `curl -sf http://127.0.0.1:45818/health` → `{"status":"ok","version":"0.5.0"}`.
2. Cursor MCP `user-mcpmux` points at `http://localhost:45818/mcp`. Reload tools **once** if the binary was just rebuilt, then do not reload again.
3. Only call mux via `user-mcpmux` (`mcpmux_search_tools` → `mcpmux_get_tool_schema` if needed → `mcpmux_invoke_tool`). No direct backend MCP servers.
4. Do not run `pnpm dev:stop` / rebuild mid-test (evicts the pool and invalidates Case B/C).

---

## Case A — bind FK guard (~2 min)

**Goal:** a nonexistent `feature_set_id` returns `invalid_argument`, not `FOREIGN KEY constraint failed`, and the DB does not grow.

1. Snapshot counts (query above). Call them `B0` / `J0`.
2. Call `mcpmux_bind_current_workspace` with `feature_set_id` = `00000000-0000-0000-0000-00000000dead` (or any other unused UUID). Do **not** approve anything — the guard runs before consent.
3. **Pass** if the tool error JSON has `"error":"invalid_argument"` and the message contains `mcpmux_list_feature_sets`.
4. **Fail** if the message contains `FOREIGN KEY`, `internal_error`, or `constraint`.
5. Re-count. `B0` and `J0` must be unchanged.
6. Grep the log:

```bash
rg "bind_current_workspace rejected" \
  "$HOME/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log"
```

Expect a `warn` with the fake `feature_set_id` and this Space id.

---

## Case B — reconnect after a killed stdio child (~5 min)

**Goal:** a transport-closed error triggers `reconnect_fresh` (not OAuth `reconnect_instance`) and the retry succeeds. Same session.

Pick a **stdio** server that is already invokable under `All`. Cheap options on this machine: `wakatime`, `markitdown`, `chrome-devtools`. Confirm first:

```
mcpmux_search_tools({ "server_id": "wakatime", "mode": "browse", "limit": 5 })
```

If `server_readiness` is not `ready`, pick another stdio id from that browse, or `mcpmux_list_servers` and take one with `ready`.

1. Invoke a cheap read-only tool once so the instance is live. Example (only if search returned it): `mcpmux_invoke_tool` `server_id=wakatime` `tool=wakatime_wakatime_summaries` with explicit `start`/`end` dates for today. Any successful call is enough.
2. Note the time (`date -u +%H:%M:%S`).
3. Kill **only the child**, not McpMux:

```bash
# find the stdio child (example: wakatime). Do not kill target/debug/mcpmux.
pgrep -lf wakatime
# then: kill <child-pid>
```

If you cannot identify a safe child, stop and report INCONCLUSIVE. Do not `pkill -f mcpmux`.

4. In **this same chat**, invoke the same tool again. Do not call `set_workspace_root`.
5. **Pass** if the invoke succeeds (or returns a normal tool error, not `-32000` / `Connection closed`).
6. Grep from the timestamp in step 2:

```bash
LOG="$HOME/Library/Application Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log"
rg "backend call_tool failed|reconnect attempted after call_tool failure|reconnect_fresh completed" "$LOG"
```

**Pass logs:**

- `backend call_tool failed` with `trigger=transport_closed` (or `auth` only if the error was actually 401)
- `reconnect attempted after call_tool failure` with `ok=true`
- `reconnect_fresh completed` with `ok=true` (this is the new path; `reconnect_instance` / `Reconnecting instance ... after OAuth` must **not** be the line for this failure)

**Fail:** invoke returns `MCP error -32000: Connection closed` to the agent, or the log shows OAuth reconnect for a stdio kill.

---

## Case C — original HA idle (optional, 15–20 min)

**Goal:** reproduce the reported HTTP shape against `home-assistant-new`.

1. Search: `mcpmux_search_tools({ "server_id": "home-assistant-new", "mode": "browse", "limit": 5 })`.
2. Invoke one cheap read-only HA tool. Confirm success.
3. Wait 15–20 minutes. Do not invoke that server. Do not reload MCP. Do not re-pin the workspace. Other mux tools are fine.
4. Invoke the **same** HA tool again in this chat.
5. **Pass / fail / log checks** are identical to Case B, except `server_id=home-assistant-new` and the error string historically was `MCP error -32000: Connection closed`.

If you cannot wait, mark Case C SKIPPED and rely on B.

---

## Case D — unmatched errors still surface (~2 min)

**Goal:** a failure that is neither auth nor transport-closed is not swallowed.

1. `mcpmux_invoke_tool` against a ready server with a tool name that does not exist, e.g. `server_id=wakatime` `tool=definitely_not_a_real_tool`.
2. **Pass** if the raw error comes back to the caller (not a silent success, not a reconnect).
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
  second invoke: ok / raw -32000 / other
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
