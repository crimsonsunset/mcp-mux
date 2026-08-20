# MCP 2026-07-28 Spec — What Changed and What It Means for McpMux

**Last Updated:** Aug 13, 2026
**Status:** Informational — no action required yet, tracked for future rmcp upgrade planning
**Related:** [`deny-by-default-bindable-callers.md`](./deny-by-default-bindable-callers.md) (first to flag SEP-2577), [`rootless-declare-root-gate.md`](./rootless-declare-root-gate.md), [`cursor-workspace-routing-bridge.md`](./cursor-workspace-routing-bridge.md)

## TL;DR

MCP shipped a new spec version, `2026-07-28`, that makes the protocol stateless: it **removes** sessions (`Mcp-Session-Id`, SEP-2567) and the `initialize` handshake (SEP-2575) outright, and **deprecates** (not removes) `roots`, sampling, and logging (SEP-2577). McpMux's gateway is a session-keyed, roots-first router — on paper, two of its central design choices are exactly what this spec revision walked away from.

In practice, none of it is urgent:

- **Roots deprecation is soft.** Annotation-only, no wire change, guaranteed to keep working until at least July 2027. McpMux already treats `roots` as one ranked signal rather than a hard dependency (see [Decisions #2](./deny-by-default-bindable-callers.md#decisions) in the deny-by-default doc), so there's nothing to unwind.
- **Session removal is version-gated, not a cutover.** A client that speaks both versions negotiates the old (session-based) protocol with an unmigrated server and the new one with everyone else. Nothing breaks by standing still.
- **McpMux isn't exposed to any of it yet anyway.** The gateway is pinned to `rmcp 1.5.0`, which tops out at protocol version `2025-11-25` — it doesn't recognize `2026-07-28` at all. Every client connecting today negotiates down to the old, session-based, roots-based protocol regardless of what the client itself supports.

The reason this is still worth writing down: this week's live debugging of the `PendingRoots` / phantom-root bug turned up first-hand evidence of exactly the failure mode the roots deprecation SEP cites as its motivation — Cursor's `roots/list` response mixing in folders from unrelated windows and stale, orphaned worker registrations. The spec authors weren't guessing.

## What actually changed

The `2026-07-28` specification (published final, replacing `2025-11-25`) bundles several Specification Enhancement Proposals (SEPs) under one theme: make the protocol stateless so it can sit behind a plain round-robin load balancer with no sticky routing or shared session store.

| Change | SEP | Status | What to do |
|---|---|---|---|
| Sessions / `Mcp-Session-Id` header | [2567](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2567-sessionless-mcp.md) | **Removed** in the new version | Drop shared session stores; mint explicit handles, pass them as ordinary tool arguments |
| `initialize`/`initialized` handshake | 2575 | **Removed** in the new version | Read protocol version + capabilities from `_meta` on every request; implement `server/discover` |
| Blocking `tasks/result` | 2663 | **Removed**, moved to extension | Poll `tasks/get` instead |
| `tools/list` / `resources/list` / `prompts/list` | 2549 | Compatible, new fields | Lists are no longer per-connection; add `ttlMs` + `cacheScope` to enable caching |
| Roots, Sampling, Logging | [2577](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2577-deprecate-roots-sampling-and-logging.md) | **Deprecated**, not removed | Stays in spec ≥12 months (earliest removal July 28, 2027); avoid new hard dependencies |
| Legacy HTTP+SSE transport | 2596 | Deprecated, not removed | Migrate to Streamable HTTP within the window |
| Feature lifecycle policy (Active → Deprecated → Removed, 12-month floor) | 2596 | New governance | Governs the two rows above |

Sources: [MCP blog — the 2026-07-28 specification](https://blog.modelcontextprotocol.io/posts/2026-07-28/), [release-candidate announcement](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/), [SEP-2567 full text](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2567-sessionless-mcp.md), [Stacktree's change-by-change writeup](https://stacktr.ee/blog/mcp-2026-spec-changes).

The distinction that matters most for McpMux: **sessions are a hard removal** (in the new protocol version only — old versions keep working via negotiation), while **roots/sampling/logging are a soft, timed deprecation** (still fully functional, no wire changes, minimum 12-month runway under the new SEP-2596 lifecycle policy).

## Is McpMux actually exposed to this?

Not yet, and not by accident of timing — by version pinning.

```
$ grep -A2 'name = "rmcp"' Cargo.lock
name = "rmcp"
version = "1.5.0"
```

`rmcp 1.5.0`'s protocol version enum stops at `2025-11-25`:

```192:195:/Users/joe/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rmcp-1.5.0/src/model.rs
"2024-11-05" => return Ok(ProtocolVersion::V_2024_11_05),
"2025-03-26" => return Ok(ProtocolVersion::V_2025_03_26),
"2025-06-18" => return Ok(ProtocolVersion::V_2025_06_18),
"2025-11-25" => return Ok(ProtocolVersion::V_2025_11_25),
```

It has no concept of `2026-07-28`. Because MCP negotiates protocol version per-connection, every client that talks to McpMux today — Cursor, Claude Desktop, VS Code, whatever — negotiates down to `2025-11-25` or earlier, the same session-based, roots-based protocol McpMux was built against. The stateless core doesn't apply to a single one of McpMux's live connections right now.

The upstream Rust SDK does support the new spec — `rmcp 3.0.0` added it (sessionless Streamable HTTP by default, `server/discover`, the `_meta`-carried handshake, `resultType` discriminators, `ttlMs`/`cacheScope` caching hints) — but McpMux hasn't picked that up. That's a deliberate, low-risk gap to leave open for now; closing it is a real migration (see [Recommendations](#recommendations)), not a patch.

## Why it's still worth understanding: two SEPs read like they were written about this app

### Sessions (SEP-2567) — "gateways using session ID for sticky routing"

SEP-2567's own backward-compatibility section runs a survey of 1,000 open-source MCP servers and breaks out exactly the categories that get hit:

| Category | Share | Migration |
|---|---:|---|
| No application-level reference to session ID | 90.0% | None |
| `Map<sessionId, Transport>` routing (SDK boilerplate) | 3.5% | Removed by a sessionless SDK transport |
| Transport setup only, session id never read | 2.8% | Delete one constructor option |
| **Session-keyed application state** | 2.5% | Migrate to explicit handles or auth principal |
| **Proxy / gateway sticky routing** | 0.7% | Needs designed replacement |
| Auth artifacts bound to session ID | 0.5% | Replace with a server-generated nonce |

McpMux's `SessionRootsRegistry` (`crates/mcpmux-gateway/src/services/session_roots.rs`) is exactly the second bolded row: every session's reported roots, pinned header, roots-capability flag, and resolved-feature-set snapshot are held in a `DashMap<session_id, _>`. That's session-keyed application state, textbook.

The good news buried in that same section: **McpMux is the cheap case, not the hard one.** SEP-2567 calls out "gateways that spawn one upstream per session" and "sticky routing across stateful replicas" as the categories needing a *designed* replacement. McpMux doesn't do either — it's a single local process per machine (`127.0.0.1` only, no horizontal scaling), so the actual motivating problem this SEP solves (round-robin load balancing across stateless replicas) doesn't exist here. What McpMux uses the session id for is closer to "which open folder is this specific connection about" — and the SEP's own recommended replacement, **explicit server-minted handles passed as ordinary tool arguments**, is a pattern McpMux already ships: `mcpmux_set_workspace_root` is precisely that. A caller that can't or won't rely on session-scoped root probing declares its workspace explicitly, once, and the resolver treats that declaration as authoritative — no different in spirit from a `create_basket()` → `basket_id` handle.

### Roots (SEP-2577) — deprecated for the reason this week's bugs demonstrated

The roots deprecation's stated rationale (per the [release-candidate post](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/) and [aaif.io's migration writeup](https://aaif.io/blog/mcp-2026-07-28-whats-changing-and-how-to-migrate)) is that roots "tightly coupled clients and servers around filesystem assumptions that don't generalise to remote or cloud environments," with low real-world adoption relative to the implementation burden every compliant client and server has to carry.

This week's live investigation of the `PendingRoots` bug produced first-hand evidence of exactly that unreliability, independent of any protocol committee's reasoning:

- **Cross-window contamination.** A single session's `roots/list` response returned four folders from four unrelated, separately-opened Cursor windows (`jsg-pr-quality`, `sync2hire-platform`, `generAIt`, `mcp-mux`) — not a multi-root workspace, just several ordinary windows open at once, all mixed into one connection's "roots."
- **Phantom/stale entries.** A separate session reported the same real project twice — once at its live path, once at a path that hadn't existed on disk since a `Repos/` reorg weeks earlier — traced to an orphaned Cursor background-agent-worker registration that kept getting reconciled into `roots/list` output long after the folder was gone.

Neither of those is a McpMux bug in the sense of "the resolver got the logic wrong." They're the client-side signal itself being unreliable in exactly the way the SEP describes — "clients vary wildly in roots support" (as the deny-by-default doc already noted back in June) turned out to understate it; even a client that *does* report roots can report garbage alongside the real answer.

## McpMux's existing posture: already built for this, mostly by luck of prior design

The June [`deny-by-default-bindable-callers.md`](./deny-by-default-bindable-callers.md) doc made the load-bearing decision before this spec even shipped as final:

> **The `WorkspaceBinding` is canonical; reported root, client identity, and machine are ranked match signals, none mandatory.** Absence of all → `Unbound`. Decouples the feature from `roots` without ripping out roots support. The deprecation just removes one signal from the priority list someday — not a rewrite.

That framing held up. `roots` in `FeatureSetResolverService` (`crates/mcpmux-gateway/src/services/feature_set_resolver.rs`) is Tier 1 of a ranked list, not the only path to a resolution — client identity (`ClientGrant`), the `X-Mcpmux-Workspace` pin, and the `mcpmux_set_workspace_root` declare-root gate all exist specifically so a caller that can't or won't report reliable roots still resolves correctly. Today's phantom-root fix (filesystem-existence check before giving up on an ambiguous multi-root session) is the same philosophy applied one layer deeper: even *within* the roots signal, don't trust it more than the evidence supports.

## What's not urgent, and why

- **Roots removal isn't eligible before July 28, 2027**, and removal requires a separate SEP even after that floor — "deprecated" isn't a countdown, someone has to actually propose removing it.
- **Session removal only applies to peers that negotiate `2026-07-28`.** McpMux's rmcp pin means no live connection negotiates it today. Cursor, Claude Desktop, and friends all speak multiple versions and fall back gracefully to whatever McpMux offers.
- **Nothing here is a security or correctness bug in the current app.** It's a forward-looking compatibility question, not a fire.

## Recommendations

None of these are asks for this session — they're what to pick up whenever rmcp 3.x adoption becomes a live topic:

1. **Track rmcp 3.x, don't rush it.** The jump isn't wire-breaking for legacy peers (`legacy_session_mode`, formerly `stateful_mode`, keeps serving pre-2026-07-28 clients exactly as today), but it is an API-breaking Rust upgrade (`ServerResult` widens for `resultType`, tool handlers change shape). Budget it as a real migration, not a patch bump.
2. **When it happens, implement `server/discover`.** The new spec makes it a MUST for servers speaking `2026-07-28`; it's the closest thing to an `initialize` replacement.
3. **Adopt `ttlMs`/`cacheScope` on `tools/list` once available.** McpMux's gateway is itself a `tools/list` aggregator across many backend servers — cache hints are a direct fit for reducing repeated list calls from clients that reconnect often (exactly the churn this week's cold-start investigation was chasing).
4. **Update the stale spec-version references.** `AGENTS.md`/`CLAUDE.md` still say "Default to the latest stable version (`2025-11-25`)" — that's no longer current as of this spec's publication. Also: the vendored spec this repo's docs point to (`../modelcontextprotocol/docs/specification/`) isn't checked out on this machine at all — either clone it or repoint the reference before anyone tries to "read the relevant section before implementing."
5. **No action needed on `roots`.** The existing ranked-signal design already absorbs the deprecation. Keep leaning on `WorkspaceBinding` + explicit declare-root as the source of truth, exactly as already decided.
