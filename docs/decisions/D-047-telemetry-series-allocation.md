# D-047: Telemetry series allocation, 182 worst case under `max_series` 200

**Date:** 2026-10-30
**Status:** accepted (rule 24: the allocation fills in what D-010 left to the spec; no scope, cap or model change)

## Context

D-010 fixed the label set, the instrument kinds and the budget ("under 200") but left the arithmetic to the spec (B-017) and sized
the shape at up to six milestones. SPEC §5.7 states the table with eight `milestone` values (seven named plus `overflow`). The
exporter must refuse to start when its computed worst case is not under `[telemetry].max_series`, so the allocation has to be
computed in code, from the configuration, not read from prose. D-046 changes the destination (a local collector), not the series.

## Decision

`core::telemetry::SeriesPlan::from(&TelemetryConfig)` computes the worst case from the config and these fixed shapes:

| Metric | Instrument | Labels | Terms | Series |
|---|---|---|---|---|
| `observatory.mission.cycle_seconds` | histogram | `mission` | 6 buckets + `+Inf` + `_sum` + `_count` | 9 |
| `observatory.feature.cycle_seconds` | histogram | `mission` | 5 buckets + 3 | 8 |
| `observatory.waste_seconds` | counter | `mission`, `milestone`, `cause` | `max_milestone_labels` 8 × 8 `WasteCause` | 64 |
| `observatory.rework_windows` | counter | `mission`, `milestone`, `cause` | 8 × 4 `ReworkKind` | 32 |
| `observatory.fix_loops` | counter | `mission`, `milestone` | 8 | 8 |
| `observatory.tokens.{input,output,cache_read,cache_write}` | 4 counters | `mission`, `role`, `model` | 4 × 3 roles × 4 models | 48 |
| `observatory.cost_usd` | counter | `mission`, `role`, `model` | 3 × 4 | 12 |
| `observatory.unknown_events` | counter | `mission` | 1 | 1 |
| **Total** | | | 9 + 8 + 64 + 32 + 8 + 48 + 12 + 1 | **182** |

- `mission` has one value at a time (`mission_switch_min_interval_min`). No instrument has a unit and none is a gauge.
- `role` has three values: `orchestrator`, `worker` and `validator` (the scrutiny and user-testing validators share it). `human`,
  `external` and `unresolved` burn no tokens and are not exported. This is how "3 roles" of D-010 and SPEC §5.7 meets the seven
  values of the `Role` enum.
- `milestone` keeps `max_milestone_labels - 1` = 7 named values, then `overflow`; `model` keeps four, then `other`.
- `start_check` refuses to start (an off `TelemetryStatus` whose reason names the computed count and the limit) when the worst case
  is not under `max_series`, when a configured metric is outside the eleven or one is missing, or when a label is outside
  `mission`, `milestone`, `role`, `model`, `cause`.
- D-010 is not amended; this file is the as-built allocation (it equals SPEC §5.7).

## Alternatives

- Compute `role` from `Role::ALL` (seven values): 7 × 4 × 5 = 140 series for the tokens alone, over the budget. Rejected.
- Put the role and model counts in `config/observatory.example.toml`: they are shape constants of the contract, not thresholds an
  operator tunes; the config keeps the numbers that move (`max_milestone_labels`, the buckets, `max_series`).

## Consequences

- The worst case counts four `model` values, as SPEC §5.7 does; the fifth and later models export as `other`, which is a fifth
  value. Counting it, the worst case is 197 (tokens 4 × 3 × 5 = 60, cost 15), still under 200 but with little room. The test pins
  the contract's 182; a change of `max_series` or the buckets must read both figures.
- A new metric, label or bound is a change to `core::telemetry`, to this table and to SPEC §5.7 together.
