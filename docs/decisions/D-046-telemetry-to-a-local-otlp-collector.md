# D-046: Telemetry exports to a local OTLP collector, configurable to any observability provider

**Date:** 2026-10-29
**Status:** accepted; owner inbox 2026-10-29-2118 (21:18), relayed by pre-roci; supersedes the Grafana Cloud default of D-010 and B-017

## Context

The owner, verbatim: "I'm ok with a local collector integration that can be configured to an observability provider". This is
product scope, so it applies when M5 is re-cut at this boundary, not to a running slice (no M5 slice has run). D-004 chose Grafana
Cloud over OTLP and D-010 fixed its contract (the endpoint in config, the Authorization header from the keychain item
`droid-observatory/otlp-headers`). A vendor endpoint in the app binds the product to one provider and puts a provider secret in the
app. The consumer of the series is the observer, whose input is the observatory's own store; the export is for whoever runs the host.

## Decision

1. The export target is a local OTLP collector: `[telemetry].endpoint`, default `http://localhost:4318` (OTLP http/protobuf);
   `[telemetry].headers_keychain_item` is optional and empty by default. The collector, not the app, is configured for the sink:
   Grafana Cloud, a local Grafana stack (collector to Prometheus or Mimir to Grafana on the host) or any other provider. The app
   never holds a provider credential and never names a provider.
2. `m5-telemetry-series-plan-core` is unchanged: the same eleven instruments, the same label bounds, the same 182 under 200 budget
   (SPEC §5.7); the budget stays because the provider behind the collector may still count active series.
3. `m5-telemetry-exporter-and-status`: the attended "Grafana screenshot" clause becomes an unattended local receipt. A collector with
   a file or debug exporter receives the series and the test reads them back. The dev compose may add a local Grafana stack behind the
   same collector as the human view, outside the contract (the `service.name`, the label set and the series count are the contract).
   Keychain parsing stays only for the optional header item. The status line reads `telemetry{state, endpoint}`.
4. CI telemetry (B-002) stays deferred. Grafana stays one configuration of the collector.

Applied at this boundary: SPEC §3 M5 (outcome and acceptance wording), §9 row, `.env.example` and the README observability
paragraph describe the collector; `config/observatory.example.toml` takes the two keys when `m5-telemetry-exporter-and-status` lands
(config is a code path, no code edit at an integration).

## Alternatives

- **Keep Grafana Cloud as the default (D-010):** rejected by the owner; it binds the app to one vendor and holds its secret.
- **Drop the export:** rejected; the series budget and the exporter are M5 outcomes.

## Consequences

- The M5 slice text of `m5-telemetry-exporter-and-status` is amended by the orchestrator in `features.json` (receipt, not
  screenshot); `m5-telemetry-series-plan-core` keeps its text.
- VAL-TELEM clauses that name Grafana Cloud or a screenshot become collector receipts at the M5 contract pass.
