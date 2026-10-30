# m5-fix-cost-cache-write

## SPEC §9

- Row (F-NEW): **the cache-write class is priced in every cost figure** (fix feature from user-testing-validator-m5 round 1, VAL-METRIC-008 failed; fix plan F1, pass-36 ruling item 2; product fix, no amendment; rule 24, no scope, cap or model change). `PriceTable::cache_write_usd(role, model, tokens) -> Option<f64>` is the cache-write rate of the dated price table times the role multiplier, `None` when the model has no row. Each `UsageReported` event adds it to the feature and milestone rows (CSV and JSON `cost_usd`), `cost_so_far`, `cost_by_role`, the lane and milestone figures, the mission level (overhead), the model comparison group, the Details Cost tab, the Panel cumulative cost line and `observatory.cost_usd`. `observatory.tokens.cache_write` is now sampled from `usage.reported` per role and model (the instrument was planned and listed, never sampled). The turn sums (`tokens_input`, `tokens_output`, `tokens_cache_read`) and `tokens_cache_write` are unchanged: no double counting, the usage log has no cache-write count. A model with no price row keeps its cache-write tokens in the report's `unpriced` list (new `cache_write` field on each entry). Credits stay the session report's primary unit in `session_costs`; the rows are USD, so their cache-write term is priced from the table either way. Alternative (rejected, plan F1): keep three classes for rows and price cache write only in `session_cost`, which leaves cost 46 % low on the row that was measured.
- Row (F-NEW): **M0 figures after the change** (`observatory-cli metrics fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --csv` and `--json`, pasted): `m0-ci-rust-job-linux` cost_usd 0.2909 to 0.5408 (cache write 99975 tokens x 2.5 / 1e6 = 0.2499, the validator's delta); milestone `m0` 3.1224 to 4.6695 (tokens_cache_write 618816); milestone `s` 12.3475 (tokens_cache_write 1537232); `cost_so_far` 177.4538 USD; `cost_by_role` orchestrator 98.5626, worker 67.4958, scrutiny_validator 4.8123, user_testing_validator 6.5831; mission overhead 160.4369. The library headline (`Headline.cost` = sum of the milestone rows) on M0 is 4.6695 + 12.3475 = 17.0170 USD (2026-10-22) with 160.44 overhead; the row in `m5-export-and-parity-ui.md` read 9.40 and 71.08 before this change (the plan's "about 4.9" for `m0` was an estimate; the tool says 4.6695). The golden (`fixtures/streams/*.events.json`) is untouched: costs are derived, not stored. No test held a hard-coded M0 cost literal, so no existing expectation moved; the new tests recompute from the golden events and the table.
- Row (F-NEW): **unpriced models on M0**: `gpt-6-sol` (cache_write 0), `gpt-6.1-sol` (cache_write 154434) and `kimi-k3` (cache_write 0) have no row in `config/prices/2026-10-22.toml`; their tokens, now including the cache-write count, are listed under `unpriced`, never priced (rule 18).

## Decisions

None.

## BACKLOG

- B-new: `CostTab.unpriced` is a `TokenCounts` and the Cost tab UI shows input, output and cache read of unpriced models only; the cache-write tokens of an unpriced model are in the report's `unpriced[].cache_write` (CLI JSON, Panel `CostReport`) but not on the tab. Reason: the tab's UI file is in `apps/`, which this core-only fix does not touch; a later UI feature adds a `cache_write` figure to the `cost-unpriced` line and the tab model gets the field with it.
- B-new: a role that has a session report with cache write and no turn now gets a `tokens_by_role` and `cost_by_role` row with zero tokens (its cost is the cache-write term). Reason: the cost has to land on a role; showing it with zero turn tokens is the honest reading of two sources that do not overlap. Not seen on M0 (all four roles have turns).

## Deferred clauses

- VAL-METRIC-024 wording (pass-37: "the classes the source carries", four token classes): kept true. `cost_by_role` and the lane figures equal the sum over the four classes x rates x multiplier; the role and lane token columns stay the three turn classes plus the sidecar's `cache_write` attributed as the CSV column attributes it. Nothing deferred to a later feature.

### Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-METRIC-008 (credits primary when present, else the four dated rates; a hand recompute of one fixture row within 0.01 USD) | `cargo test -p observatory-core --test metrics_cost`: `cost_uses_credits_when_the_stream_exposes_them_else_the_price_table` (both branches, unchanged and green), `a_row_cost_prices_the_cache_write_tokens_of_its_session_report` (hand sum 2.0 + 1.0 + 0.2 + 1.0 = 4.2 within 0.01, feature and milestone row), `the_row_cost_of_m0_ci_rust_job_linux_equals_the_four_class_recompute` (CSV 0.5408 against the recompute 0.5408323, red was 0.2909); CLI `metrics ... --csv` row `m0-ci-rust-job-linux,...,99975,0.5408,2026-10-22` |
| VAL-METRIC-008 / TELEM-009 note "all eleven" (the cache-write instrument is sampled) | `cargo test -p observatory-core --test telemetry_sample`: `the_telemetry_sample_carries_cache_write_tokens_per_role_and_model`, `cost_usd_in_the_telemetry_sample_includes_the_cache_write_term`, `the_m0_sample_cache_write_total_is_the_sum_of_the_session_reports` |
| VAL-METRIC-024 under its pass-37 wording (cost is the sum over four token classes x rates x multiplier for role and lane figures; tokens_cache_write attributed as the CSV attributes it) | `role_lane_milestone_mission_and_comparison_costs_include_the_cache_write_term` (worker multiplier 2.0 applied to the cache-write term, role, lane, milestone, mission level, comparison group); `an_unpriced_model_keeps_its_cache_write_tokens_in_the_unpriced_list` |
| CSV, JSON, tab and Panel agree to the cent | `view_parity`: `the_cost_tab_the_rows_and_the_panel_headline_name_the_same_cost_with_cache_write` (tab totals, features, roles, Panel headline and cumulative line against the milestone row), plus the three M0 parity tests, all green |

## Library

- `PriceTable::cache_write_usd` is the only place a cache-write count becomes USD; every cost figure (rows, report, tab, Panel line, telemetry) calls it for each `UsageReported` event with `cache_write > 0`. A new cost surface must add that loop beside its turn loop, or it will read 46 % low on sessions that bill cache write.
- Hand-off model line: `model = claude-sonnet-5-5`.
