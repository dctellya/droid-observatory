# m5-cost-and-tokens-core

## SPEC §9

- Row (F-NEW): **core::metrics costs and tokens: `PriceTable`, `cost_report`, `budget_meter`, `model_comparison`, `tokens_in_waste`, `reconcile`** (feature `m5-cost-and-tokens-core`, batch 1; rule 24 decisions inside the slice, none changes scope, caps or models). Modules `crates/observatory-core/src/metrics/{price,tokens,cards,budget,compare,report}.rs`, pure over mapped events and the dated price table; compile with `--no-default-features` on wasm32. Every cost carries `price_table` (the table's date, `2026-10-22`); a session's cost is the sidecar's credits when the stream exposes them, else tokens × the dated table.
- Row (F-NEW): **role multiplier lives in the price table.** `config/prices/2026-10-22.toml` gained a `[roles]` table (every role 1.0, comment gives the reason); cost = tokens × price × `roles.<role>`, and the report names the key (`multiplier_key`) and the value it used. No other threshold file changed.
- Row (F-NEW): **cost, token and model-comparison definitions fixed where the contract left a choice.** First-pass active per role-model = that role-model's event spans outside rework windows and stops; a "completed feature" in the model comparison = `task.done` + `scrutiny.passed` + no unresolved rework window (`integration.merged` is not required: the mapper emits none per feature and the M0 capture has none); `tokens_by_lane` has one row per template lane (zero rows kept, the gate lane is zero on M0) plus `tokens_other_lanes`; a turn with no session joins the `UNJOINED_SESSION` row of the reconciliation; a model with no price row is counted and listed in `unpriced`, never priced, and its tokens stay in the token totals but not in the USD totals.
- Row (F-NEW): **budget meter.** The launch's `budget_usd` > 0 wins over `[metrics].budget`; a value ≤ 0 (or none) disables the meter (fraction `null`); with a budget the fraction is cost so far ÷ budget and the report names the budget and the table date. CLI: `observatory-cli metrics <capture> --json [--price-table <path>] [--budget-usd <n>]`.
- Row (F-NEW): **tokens burnt in waste** = tokens of turns whose timestamp falls inside a waste band, per cause (the eight measured causes plus `throttled` apart; the bands come from `WasteTotals::bands()`/`cause_at()`, the stops from `Throttled::spans()`). On the M0 capture: waiting_human_question 516 in / 317339 out / 26665888 cache-read, retry_failure 74048/7186/262623, idle_queued 6/5103/237394, idle_unscheduled 24/6240/205825, `throttled` 0 (the stop has no turns).
- Row (F-NEW): **both hosts load the capture header's usage log.** `store::replay_file` and the browser's `replay_files` read the file named by the header's `usage_log` (`UsageLogSource` + `TreeMapper::map_usage`, as the CLI does) so the Panel usage block and the metrics have the turns; a selection without the named extract replays without turns and says so with a warning naming the extract (`capture::missing_usage_log_warning`). The extract is not a session candidate. Board and digest still ignore `turn.usage` (rule 24 decision of 2026-10-24).
- Row (F-NEW): **`ingest::mission_events`** assembles root, children and usage log into the mapped event list of a mission (one place for the CLI `metrics` and later `MetricsReport`).

## Decisions

None.

## BACKLOG

- B-new: `reasoning` and `total_input` token counts are not stored (the store round trip keeps input/output/cache_read only); reporting them is a new migration (`0002_*.sql` style, never an edit of `0001`). Not needed by the contract's five assertions.
- B-new: `gpt-6-sol`, `gpt-6.1-sol` and `kimi-k3` have no row in `config/prices/2026-10-22.toml` (the 2026-10-29 table has plan prices), so on the M0 capture their tokens are counted and listed in `unpriced` and their cost is not in the 80.4743 USD total. A price table is dated; the M0 oracle uses the 2026-10-22 one.
- B-new: `turn.usage` carries no cache-write count, so the turn-derived cost has no cache-write term; the sidecar session cost has it. The reconciliation shows the signed difference per session (turns − sidecar); first rows are 0.
- B-new: the browser's file selection still filters `.jsonl`, `.settings.json` and `.capture.json`; the usage extract is picked as a `.jsonl`. A selection that omits it is told so (warning), but nothing stops a user choosing a different file with the same name.
- B-new: the mission-level overhead (apportionment `separate`, the M0 orchestrator): tokens 243629/1218124/58867366, cost 71.0758 USD, active 16449.5 s, waiting_human_question 20682 s, idle_unscheduled 664 s. `equal` splits it over features in `cost_report`; the CSV overhead row and the oracle's treatment are `m5-metrics-report-and-oracle`'s.

## Deferred clauses

Claims on later features (owner item 18), each named:

- `m5-metrics-report-and-oracle`: `MetricsReport` assembly (merge `cost_report` with time states, rework, throttled), `--csv` with the 22-name header and the `tokens_*`/`cost_usd`/`price_table` columns, `--json` merge of the time half, the oracle test, `Notes::from_store`, the mission-level partition of human waits as a CSV row, and the Panel/Details rendering of cost and the table date (the UI part of VAL-METRIC-007, -010, -011 and the Panel block of VAL-METRIC-024/025 are host work, not done here).
- Not done here and not claimed: `turn_usage_events_do_not_appear_in_lane_rows_or_cards` (the view already excludes `turn.usage`; the full core run is green with the turns loaded by `replay_file`).

### Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-METRIC-007 | `cost_names_the_price_table_date` (tests/metrics_cost.rs); CLI `metrics --json` prints `price_table: 2026-10-22` on every cost (tests/metrics.rs in observatory-cli). UI rendering deferred. |
| VAL-METRIC-008 | `cost_uses_credits_when_the_stream_exposes_them_else_the_price_table`: credits primary; price-table path hand-recomputed 1M in @2.0 + 0.1M out @10 + 2M cache-read @0.10 + 0.4M cache-write @2.5 = 4.2 USD. |
| VAL-METRIC-010 | `budget_meter_compares_against_optional_budget`; CLI `--budget-usd 200` gives fraction cost/200, default is disabled (`null`). |
| VAL-METRIC-011 | `model_comparison_groups_by_role_and_model`; CLI rows for fable-5.1, sonnet-5-5, opus-5-5 per role with completed_features 5 on M0. |
| VAL-METRIC-024 | `tokens_per_role_and_lane_sum_turn_usage_events`, `tokens_burnt_in_waste_bands_are_attributed_per_cause`, `throttled_*` apart; M0 sums 318585 in / 1662748 out / 92204322 cache-read equal the golden `turn_usage` sum, the sum over the 4 roles and the sum over the 7 template lanes; 56 sidecar sessions reconciled. |

Red/green pairs: metrics_cost (2ade671 → eec5940), replay_usage_log (f2b62c8, 7a8f4ea → 4c6c3fa), CLI metrics (f0890f9 → 8f2077c); characterization (green, covers existing behavior) b7205b2.

## Library

- `metrics::{PriceTable, cost_report, budget_meter, model_comparison, tokens_in_waste, reconcile}`; `ingest::mission_events`; `UsageLogSource::{in_memory, read_bytes}`; `capture::missing_usage_log_warning`.
- M0 numbers (`observatory-cli metrics fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --json`): cost so far 80.4743 USD (orchestrator 35.3134, worker 39.7877, scrutiny_validator 2.4132, user_testing_validator 2.9601), price table 2026-10-22.
