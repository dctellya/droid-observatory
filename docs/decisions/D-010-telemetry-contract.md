# D-010: Telemetry contract for the Grafana Cloud stack

**Date:** 2026-10-22
**Status:** accepted (owner-provided contract; code in the telemetry milestone)

## Context

D-004 chose Grafana Cloud over OTLP and sketched metric names with a wide attribute list
(`feature`, `session`, `class`, `unit`, `price_table`, a gauge for active agents). The owner
has provisioned the stack and set the operating constraints below. Cardinality on Grafana
Cloud is billed per active series, so the attribute set is the contract that matters.

## Decision

- **Transport:** OTLP over HTTP (`http/protobuf`) to
  `https://otlp-gateway-prod-eu-west-2.grafana.net/otlp` (`/v1/metrics`, `/v1/traces`,
  `/v1/logs`). The endpoint is not a secret and lives in `config/observatory.example.toml`.
- **Credential:** the full `Authorization=Basic …` header value is read at runtime from the
  macOS keychain item with service `droid-observatory/otlp-headers` and account = the
  current user. It is never written to config, the store or logs. If the item is missing,
  telemetry stays off and the UI says so (a visible "telemetry: off, no keychain item"
  state, not a silent failure). The environment variable `OTEL_EXPORTER_OTLP_HEADERS` is
  for CI only (B-002); the app does not read it.
- **Resource attributes:** `service.name=droid-observatory` and `service.version` (the
  crate version). Nothing else. D-004's `deployment.environment` and `service.namespace`
  are dropped.
- **Metric attributes (labels):** only from the set `mission`, `milestone`, `role`,
  `model`, `cause`. Never a session id, event id, feature id or free text. Anything
  that varied by `class`, `unit` or `status` in D-004 becomes part of the metric name
  (for example `observatory.tokens.input`, `observatory.cost_usd`) or is not exported.
- **Instruments:** counters and histograms only. No per-event gauges;
  `observatory.active_agents` is not exported (it stays an in-app view).
- **Series budget:** the spec states the exact metric set and the number of active series
  the app adds, computed from the label cardinalities, and keeps it **under 200**. The
  arithmetic is the spec's (B-017); the shape it must fit is: one mission exporting at a
  time, up to 6 milestones, 3 roles, up to 4 models, a closed list of waste causes, and a
  histogram costs one series per bucket plus `_sum` and `_count`, so explicit bucket
  boundaries are part of the contract.

## Alternatives

- Keep D-004's attribute list and rely on Grafana's adaptive metrics to aggregate: pays for
  the series first, then hides them; rejected.
- Export per-feature series: `feature` would be the largest cardinality by far and is
  already visible in the app; rejected by the owner.
- Read the header from the environment as a fallback in the app: one more place a secret can
  leak into a shell history or a log; rejected by the owner.

## Consequences

- D-004 stays accepted for the choice of backend and the two kinds of telemetry; its
  attribute list, resource attributes and the gauge are superseded here.
- `config/observatory.example.toml` `[telemetry]` carries the endpoint, the keychain
  service name, the allowed label set, the series budget and the revised metric names.
- PRD v0.5 §11.4 states the contract. `.env.example` marks the OTLP variables CI-only.
- B-017: the spec session computes the series count and names the histogram buckets.
