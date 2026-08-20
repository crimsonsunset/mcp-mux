# Pool Invalidation + Session Survival — Manual Test Playbook

**For:** verifying Phases 1–4 on `root-resolution`
**Last Updated:** Aug 20, 2026
**Implements:** [`pool-invalidation-and-session-survival.md`](./pool-invalidation-and-session-survival.md)
**Shipped:** `dcc2977` — config-save `reconnect_fresh`, stdio OAuth refuse, hold `X-Mcpmux-Workspace` until session id
**Parent playbook:** [`backend-connection-resilience-test.md`](./backend-connection-resilience-test.md) (Cases A/B already PASS)

---

## Results (Aug 20, 2026)

Ran live against `127.0.0.1:45818` (`{"status":"ok","version":"0.5.0"}`). Binary `target/debug/mcpmux` contains the new strings (`held until mcp-session-id`, `stdio cannot reconnect via OAuth`, `reconnect_fresh after config update`). Process up since 11:36:44 local / recycle log `17:37:21Z`. Mux only via `user-mcpmux`. `set_workspace_root` not used.

| Case | Result |
| ---- | ------ |
| E header pin | **PASS** — this repo `ready` (wakatime, HA, …). Session `60e5261f…` header `~/Desktop/Repos/Personal/mcp-mux` → pin `/Users/joe/Desktop/Repos/Personal/mcp-mux`. Log: `held until mcp-session-id exists` then `pinned explicit workspace root`. Not 6-way `bindable`. |
| F stdio refuse / Case B regression | **PASS** — `wakatime` / `wakatime_wakatime_summaries`. Killed child pids 74227/74257 at 17:39:38Z. Second invoke succeeded. `trigger=transport_closed`, `reconnect_fresh completed ok=true` (1544ms). No `Unexpected STDIO`, no OAuth reconnect. Source grep of that string: 0. |
| G config-save reconnect | **BLOCKED** — admin `:45819/api/v1/health` is 401, no CF probe headers in this shell. Did not sqlite-edit (no event). Did not mutate `wakatime` env. Needs Configure UI or a working admin token. |
| H rebuild 404 | **PASS** (same-day recycle, no second rebuild) — new handler start at `17:37:21Z` (line 42 = this binary). `/health` 200 after. New session ids + Case E pin within 3s. 404s are expected-noise (not warned), so no 404 warn lines. Did not `pkill` / rebuild again (would 404 this chat). |

---

## Answer first

Same rules as the parent playbook. **Do not** call `mcpmux_set_workspace_root` to "fix" a reconnect. **Do not** `pkill -f mcpmux`. **Do not** rebuild mid-case unless you are *in* Case H, and Case H goes last because it 404s this chat.

| Action | Use for this test? |
| --- | --- |
| Same Cursor chat, same workspace (`/Users/joe/Desktop/Repos/Personal/mcp-mux`) | **Yes — required** |
| Reload MCP before starting (only if `/mcp` is already 404) | Yes, once, then stop |
| `set_workspace_root` as a reconnect | **No** |
| `set_workspace_root` only if Case E shows 6-way `bindable` *and* the workspace header is empty | Last resort, note it as a confounder |
| `pnpm dev:stop` / rebuild | **Case H only**, last |
| `pkill -f mcpmux` | **No** |

---

## Snapshot

```
DB=~/Library/Application\ Support/com.mcpmux.desktop/mcpmux.db
LOG=~/Library/Application\ Support/com.mcpmux.desktop/logs/mcpmux.$(date +%Y-%m-%d).log
```

| Fact | Value |
| ---- | ----- |
| Space | `00000000-0000-0000-0000-000000000001` |
| This root | `/Users/joe/Desktop/Repos/Personal/mcp-mux` |
| Stdio for F | `wakatime` / `wakatime_wakatime_summaries` |
| Config-save target for G | enabled stdio or HTTP that is `ready` — prefer a harmless `env_overrides` / `extra_headers` bump, revert after |
| Pool stats | in-memory only |

Health: `curl -sf http://127.0.0.1:45818/health` → `{"status":"ok",…}`.

---

## Preconditions

1. Debug gateway on `:45818`. This binary must include `dcc2977` (or later on `root-resolution`). If `tauri dev` has not rebuilt since that commit, Cases G/E will exercise the *old* evict-only / pin-skip code — mark INCONCLUSIVE and rebuild *before* starting, not mid-case.
2. Mux only via `user-mcpmux`.
3. `mcpmux_list_servers` is not all-`bindable`. If it is, Case E failed (or header is empty). Do not paper over it with `set_workspace_root` until E is scored.
4. Grep guard for the deleted landmine (any time):

```bash
rg "Unexpected STDIO transport for OAuth reconnection" \
  crates/mcpmux-gateway/src "$LOG"
```

Must be **zero** hits in source. Historical log lines from before `dcc2977` do not fail this run.

---

## Case E — header pin, no `set_workspace_root` (~2 min)

**Goal:** a non-empty `X-Mcpmux-Workspace` makes this repo bound. A 6-folder `roots/list` is not a binding.

1. `mcpmux_list_servers`.
2. Grep the last minute of `$LOG`:

```bash
rg "SessionRoots|X-Mcpmux-Workspace|pinned explicit workspace|held until mcp-session-id|PendingRoots|multiple roots reported" "$LOG" | tail -n 80
```

**Pass:** at least one server is `ready` (or `bound` with a real block, not "unpinned roots"). Log shows a pin (`pinned explicit workspace root` or `held until mcp-session-id` then pin), not `multiple roots reported, no pinned header — PendingRoots`.
**Fail:** every server `bindable` *and* the log shows a non-empty workspace header that was skipped (`pin skipped` / `present without mcp-session-id` without a later pin).
**Inconclusive:** header is empty (`present but empty`) — Agents-window hole, out of scope. Note it.

Do **not** call `set_workspace_root` before scoring.

---

## Case F — stdio OAuth refuse + transport-closed still reconnects (~5 min)

**Goal:** killing a stdio child still uses `reconnect_fresh`. It must not go through `reconnect_after_oauth` / HTTP.

1. `mcpmux_search_tools({ "server_id": "wakatime", "mode": "browse", "limit": 5 })`. Need `server_readiness: ready`.
2. Invoke `wakatime` / `wakatime_wakatime_summaries` with explicit `start`/`end` for today. Warm the instance.
3. Note `date -u +%H:%M:%S`.
4. Kill **only the child**:

```bash
pgrep -lf wakatime
# kill <child-pid>
# Do not kill target/debug/mcpmux.
```

5. Same chat, same invoke. No `set_workspace_root`.
6. Grep from step 3:

```bash
rg "backend call_tool failed|reconnect attempted after call_tool failure|reconnect_fresh completed|reconnect_after_oauth|stdio cannot reconnect via OAuth|Unexpected STDIO" "$LOG"
```

**Pass:** invoke succeeds (or a normal tool error). Logs: `trigger=transport_closed`, `reconnect_fresh completed ok=true`. No `Unexpected STDIO`, no OAuth reconnect for this kill.
**Fail:** raw `Transport closed` / `-32000` to the agent, or HTTP OAuth reconnect for stdio.

---

## Case G — config save reconnects (~5 min)

**Goal:** `ServerConfigUpdated` runs `reconnect_fresh` for an enabled server. Next invoke does not say "bound but not connected." Do **not** click Retry Connection.

Harmless write (revert after):

1. Snapshot the row (`extra_headers` / `env_overrides`) for the target server.
2. `PUT` admin `save_server_inputs` (or Configure save in the UI) adding one dummy key, e.g. `MCPMUX_TEST_PIN=1` in `env_overrides`. Do not change command/url.
3. Do **not** call retry/reconnect from the UI.
4. Invoke a cheap tool on that server.
5. Grep:

```bash
rg "ServerConfigHandler|reconnect_fresh after config update|re-resolve failed, evicting only" "$LOG"
```

**Pass:** `[ServerConfigHandler] reconnect_fresh after config update` with `ok=true` (or `ok=false` if the server cannot spawn, but the stale instance is gone and the error is a connect failure, not "bound but not connected"). Invoke works if the server is healthy.
**Fail:** invoke returns "bound but not connected" / `diagnose_server`, or the handler only logs evict with no `reconnect_fresh`.
6. Revert the dummy key the same way.

If admin write is unavailable (no token / desktop-only), use the Configure UI. If neither is possible, **BLOCKED** — do not sqlite-edit the row (no event).

---

## Case H — rebuild 404, last (~5 min + MCP reload)

**Goal:** process death → inbound 404 is expected. `/health` is the liveness check. Recovery is Reload MCP. Do not persist sessions.

This **will** 404 the current Cursor MCP session. Run it last.

1. `curl -sf http://127.0.0.1:45818/health` → ok.
2. Trigger a Rust rebuild (`touch` a gateway `.rs` or wait for `tauri dev` to recycle). Do **not** `pkill -f mcpmux`.
3. While the new process is up: `/health` is 200. A `POST /mcp` with the old `Mcp-Session-Id` is 404. Logs do not `warn` the 404 (`expected session noise`).
4. Reload MCP **once**.
5. Re-run Case E (list_servers). Expect bound, not 6-way `bindable`, without `set_workspace_root` if the header is a real path.

**Pass:** health 200 throughout the new process; chats 404 until Reload MCP; after reload, Case E still passes.
**Fail:** `/health` down and we treat 404 as "gateway dead"; or after reload + real header, everything is `bindable` (Case E fail).
**Skip if:** you cannot afford to 404 this chat. Score from an earlier same-day rebuild in `$LOG` if the lines are unambiguous; otherwise SKIPPED.

---

## Do not

- `set_workspace_root` to recover a closed backend.
- `pkill -f mcpmux`.
- Rebuild except Case H.
- Leave the Case G dummy env/header in place.
- Widen the transport-closed matcher if F fails with a new string — log `trigger=` and stop.

---

## Report back

```
Case E header pin: PASS | FAIL | INCONCLUSIVE
  list_servers: ready/bound/all-bindable
  pin log:
  set_workspace_root used: no

Case F stdio refuse: PASS | FAIL | INCONCLUSIVE | SKIPPED
  server_id / tool:
  first invoke:
  child pid:
  second invoke:
  trigger=:
  reconnect_fresh ok=:
  Unexpected STDIO / OAuth: yes/no

Case G config save: PASS | FAIL | BLOCKED
  server_id:
  write path (admin/UI):
  handler log:
  invoke after save:
  reverted: yes/no

Case H rebuild 404: PASS | FAIL | SKIPPED
  health during/after:
  404 observed:
  reload MCP:
  Case E after reload:
```
