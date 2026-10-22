# D-004: Grafana Cloud over OTLP as the telemetry backend

**Date:** 2026-10-22
**Status:** accepted (contract now, code in the mission)

## Context

The PRD needs two kinds of telemetry: the observatory's own traces and metrics (to debug
the app) and the lane-9 mission series (cycle time, waste, tokens, cost) so they can be
charted and alerted on outside the desktop window. The owner provides a Grafana Cloud stack.

## Decision

- Export via OpenTelemetry OTLP (`http/protobuf`) to the stack's OTLP gateway. Endpoint and
  `Authorization: Basic base64(instanceId:token)` come from `OTEL_EXPORTER_OTLP_ENDPOINT` /
  `OTEL_EXPORTER_OTLP_HEADERS` or the keychain entry `droid-observatory/otlp-headers`.
  Never from the repository.
- Off by default (`[telemetry].enabled = false`); the app must work fully offline.
- App telemetry: `tracing` + `tracing-opentelemetry` spans for ingest, mapping, metrics,
  control; errors as span events (no separate error tracker in v1, see B-003).
- Mission metrics (names are the contract, attributes in parentheses):
  - `observatory.mission.cycle_seconds` (mission, status)
  - `observatory.feature.cycle_seconds` (mission, milestone, feature)
  - `observatory.waste_seconds` (mission, milestone, feature, role, cause)
  - `observatory.rework_windows` (mission, feature, cause, resolved)
  - `observatory.fix_loops` (mission, milestone)
  - `observatory.tokens` (mission, session, role, model, class)
  - `observatory.cost` (mission, role, model, unit, price_table)
  - `observatory.active_agents` (mission, role)
- Resource attributes: `service.name=droid-observatory`, `deployment.environment`,
  `service.namespace`, `service.version`.
- CI telemetry (workflow durations, test timings) is documented only: B-002.

## Alternatives

Prometheus pull (no stable endpoint from a desktop app); Sentry for errors (second vendor;
revisit if span events prove insufficient); no export (loses the external budget meter).

## Consequences

`config/observatory.example.toml` `[telemetry]`, `.env.example`, PRD v0.3 §11.4. GitHub
Actions secret `GRAFANA_OTLP_HEADERS` and variable `GRAFANA_OTLP_ENDPOINT` are reserved
names for B-002.
