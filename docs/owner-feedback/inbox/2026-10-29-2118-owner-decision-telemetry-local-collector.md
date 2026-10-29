# Owner decision for the M4 boundary (not in flight): telemetry exports to a local OTLP collector, configurable to any observability provider

Source: owner ("I'm ok with a local collector integration that can be configured to an observability provider", 21:18, 2026-10-29), relayed by pre-roci. This is product scope, so it applies at the boundary when M5 is re-cut, not to any running slice.

## The change to §5.7 and the two M5 telemetry slices
1. The export target is a local OTLP collector: `[telemetry].endpoint`, default `http://localhost:4318` (OTLP http/protobuf), `[telemetry].headers_keychain_item` optional and empty by default. The collector, not the app, is configured for the sink: Grafana Cloud, a local Grafana stack (collector to Prometheus or Mimir to Grafana on the host, the owner's example), or any other provider; the app never holds a provider credential and never names a provider.
2. The series plan slice (m5-telemetry-series-plan-core) is unchanged: same eleven instruments, same label bounds, same 182 under 200 budget; the budget stays because the provider behind the collector may still count active series.
3. The exporter slice (m5-telemetry-exporter-and-status): the attended "Grafana screenshot" clause becomes an unattended local receipt: a collector with a file or debug exporter receives the series and the test reads them back; the dev compose may add a local Grafana stack behind the same collector as the human view, outside the contract (the `service.name`, the label set, the series count). Keychain parsing stays only for the optional header item. The status line reads `telemetry{state, endpoint}`.
4. Decision file for the change (supersedes the Grafana Cloud contract line of D-010/B-017 as the default; Grafana remains one configuration of the collector). CI telemetry (B-002) stays deferred.

## Why
The series' consumer is the observer (Roci, Holden), whose input is the observatory's own store; the export is for whoever runs the host. A vendor endpoint in the app binds the product to one provider and puts a provider secret in the app; a local collector keeps the app neutral and the credential outside it.
