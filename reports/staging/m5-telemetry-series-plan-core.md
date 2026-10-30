# m5-telemetry-series-plan-core

## SPEC §9

- Row (F-NEW): **`core::telemetry` series plan, label allow-list, bounds and start check** (feature `m5-telemetry-series-plan-core`, batch 1; rule 24, no scope, cap or model change). `crates/observatory-core/src/telemetry.rs` (pure, no feature flag, compiles with `--no-default-features` on wasm32): `SeriesPlan::from(&TelemetryConfig)` plans the eleven `[telemetry].mission_metrics` instruments (two histograms with the configured buckets, nine counters, no unit, no gauge) and computes the worst case from the config: 9 + 8 + 64 + 32 + 8 + 48 + 12 + 1 = **182** under `max_series` 200; `Label::parse` accepts exactly `mission|milestone|role|model|cause` (`LabelError` otherwise); `BoundedLabel::milestone(cfg)` keeps `max_milestone_labels - 1` = 7 named values then `overflow`, `BoundedLabel::model()` keeps 4 then `other`; `role_label(Role)` maps to `orchestrator|worker|validator` (the two validators share one value; human, external, unresolved are not exported); `start_check(&TelemetryConfig) -> Result<SeriesPlan, TelemetryStatus>` refuses with `TelemetryStatus { enabled: false, reason: "series plan refused: worst case 182 series is not under max_series 182" }` when the worst case reaches `max_series`, a configured metric is outside the eleven or missing, or a label is outside the five or missing from `metric_labels_allowed`.
- Row (F-NEW): **role count of the budget.** "3 roles" of D-010 and SPEC §5.7 meets the seven-value `Role` enum as three exported `role` values (orchestrator, worker, validator); the shape constants (3 roles, 4 models, the eight `WasteCause`s, four `ReworkKind`s) are code constants of the contract, not config keys (`max_milestone_labels`, the buckets and `max_series` stay in config).

## Decisions

- `docs/decisions/D-047-telemetry-series-allocation.md` (new): the as-built allocation, the 182 table, the role mapping, the bounds; D-010 is not amended. `docs/decisions/README.md` regenerated with `make decisions-index` (one row added).

## BACKLOG

- B-new: the 182 counts four `model` values as SPEC §5.7 does, but the fifth model exports as `other`, a fifth value. Counting it the worst case is 197 (tokens 4 × 3 × 5 = 60, cost 15), under 200 with 3 to spare. The contract pins 182 (VAL-TELEM-001); if the budget tightens, `SeriesPlan` should count `other` (MODEL_LABEL_VALUES + 1) and the SPEC table follow.
- B-new: the `[telemetry]` config keys still read `endpoint = https://otlp-gateway-prod-eu-west-2.grafana.net/otlp` and `keychain_service` (D-010 shape); D-046's `endpoint = http://localhost:4318` and `headers_keychain_item = ""` land with the exporter slice (VAL-TELEM-005, `example_config_equals_defaults`). Not touched here.

## Deferred clauses

Claims on later features (owner item 18), each named:

- `m5-telemetry-exporter-and-status`: VAL-TELEM-005 to -008 (config keys `endpoint` default `http://localhost:4318` and `headers_keychain_item`, `telemetry_status` command, the exporter in `src-tauri/telemetry.rs`, the status bar and the `telemetry{state, endpoint}` line, the header parse `parse_header` with its two unit tests and the redaction test, the `secrets` stub tests `telemetry_stays_off_without_keychain_item_and_ui_says_so` and `header_is_split_on_first_equals_and_never_logged`, the live Mimir receipt and the rule 26 stack readiness row for SPEC `## Tooling`). It can call `telemetry::start_check` for the refusal and `BoundedLabel`, `role_label` for the label values; `TelemetryStatus` (`enabled`, `reason`) exists with `TelemetryStatus::off`, the other states are the slice's.
- The runtime side of VAL-TELEM-004 (the exporter applying `BoundedLabel` to the mission's milestone and model ids) and of VAL-TELEM-009 (the startup line listing the eleven names, the Mimir reads) belongs to the exporter slice; the core half is claimed below.

### Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-TELEM-001 | `cargo test -p observatory-core --test telemetry exported_series_count_is_under_budget_on_worst_case_labels` green; test body asserts 9, 8, 64, 32, 8, 48 (four counters), 12, 1 per metric, `plan.worst_case == 182` and `182 < cfg.max_series` (200): `crates/observatory-core/tests/telemetry.rs`, function of that name. |
| VAL-TELEM-002 | `exporter_refuses_to_start_when_worst_case_reaches_max_series`: `max_series` 182 and 100 refused, 183 and 200 accepted; refused status `enabled = false`, reason `series plan refused: worst case 182 series is not under max_series 182` (printed by the test with `--nocapture`). The "nothing is sent" half is the exporter slice's (this core check runs before any export). |
| VAL-TELEM-003 | `labels_outside_allowed_set_are_rejected`: five accepted names, eleven rejected (the eight of the clause, plus `""`, `Mission`, free text), every label of every planned instrument is among the five; `a_configured_label_outside_the_allow_list_stops_the_start` for the config side. |
| VAL-TELEM-004 | `label_values_are_bounded_by_overflow_and_other`: nine milestone ids give `m1..m7, overflow, overflow`; five model ids give `a, b, c, d, other`; named values stay named, overflowed ids stay `overflow`. Output printed with `--nocapture`. |
| VAL-TELEM-009 | `exactly_eleven_instruments_are_exported`: plan names equal the eleven in order and `[telemetry].mission_metrics`; two histograms with the buckets `[3600,7200,14400,28800,86400,172800]` and `[600,1800,3600,7200,14400]`, nine counters, `unit()` is `None` for all, no `active_agents` or overhead name; `an_instrument_outside_the_contract_stops_the_start` for an extra, duplicated or missing name. The startup line and the Mimir reads of the clause are the exporter slice's (see Deferred). |

Red/green: `0e45469` (red, does not compile: no `observatory_core::telemetry`) → `36bd0ba` (green). Characterization: none.

## Library

- `observatory_core::telemetry::{Label, LabelError, BoundedLabel, role_label, SeriesPlan, InstrumentPlan, InstrumentKind, TelemetryStatus, start_check}`; the exporter registers `plan.instruments` as they are (name, kind with buckets, `unit()` None).
- Inner loop on the final tree: `cargo test -p observatory-core --test telemetry` 9 passed; `cargo test -p observatory-core` 47 result lines all ok; clippy (`--all-targets --all-features -D warnings -W clippy::cognitive_complexity`) 23.23 s, exit 0; `cargo check -p observatory-core --no-default-features --target wasm32-unknown-unknown` 3.19 s, exit 0; `cargo machete` clean. The keyring entries are not in a machete ignore list (already removed).
