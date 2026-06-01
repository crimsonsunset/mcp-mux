# Testing Docs

Manual QA runbooks and automated verification gates for McpMux subsystems.

These docs were moved from `docs/planning/`. They are operational artifacts — step-by-step procedures for verifying a specific feature or PR — rather than architecture docs.

---

## Runbooks by subsystem

### Consent model and FeatureSet

| Doc                                                            | Scope                                                                                                                                             |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`consent-model-qa-runbook.md`](./consent-model-qa-runbook.md) | Manual QA for the consent-model PR (`docs/feature-set-consent-model` branch) — FeatureSet binding, approval flow, search ranking, embedding cache |
| [`pr3-fixes-qa.md`](./pr3-fixes-qa.md)                         | Manual QA runbook for PR #3 post-review fixes — embedding cache persistence, search latency                                                       |
| [`pr3-fix-verification.md`](./pr3-fix-verification.md)         | Automated regression gate for PR #3 fixes — tight command sequence, all steps must pass before re-requesting review                               |

Architecture reference: [`docs/backend/technical/consent-and-binding.md`](../backend/technical/consent-and-binding.md), [`docs/backend/technical/embedding-cache.md`](../backend/technical/embedding-cache.md), [`docs/backend/technical/tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md)

---

### Meta-gateway invoke

| Doc                                                                | Scope                                                                                              |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| [`meta-gateway-invoke-qa.md`](./meta-gateway-invoke-qa.md)         | Manual QA for `mcpmux_invoke` meta-tool — end-to-end invoke path, ACL checks, schema batch         |
| [`meta-gateway-invoke-retest.md`](./meta-gateway-invoke-retest.md) | Targeted retest after DX fix commit `85113e7` — ACL error messages, schema batch limits, max_bytes |

Architecture reference: [`docs/backend/technical/tool-discovery-and-search.md`](../backend/technical/tool-discovery-and-search.md), [`docs/backend/reference/meta-gateway-invoke.md`](../backend/reference/meta-gateway-invoke.md)

---

## When to use these docs

- **Before merging a PR** that touches a covered subsystem — run the relevant runbook to catch regressions the automated suite doesn't reach.
- **After post-review fixes** — use the verification gate (e.g. `pr3-fix-verification.md`) as a tight regression sequence before re-requesting review.
- **When investigating a reported bug** — runbooks document the expected behavior step-by-step, making deviation from that behavior easy to locate.
