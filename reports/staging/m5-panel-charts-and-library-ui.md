# m5-panel-charts-and-library-ui

Lead worker with three `mechanical-sonnet` seats (owner inbox 2320). The lead wrote the red test, `core::view::panel_metrics`, the library `Headline`, the mounting (`panel.rs`, `board.rs`, `app.rs`, `backend.rs`, `ctx.rs`, `main.rs`) and the shell's `list_missions` headline; the seats wrote the three UI files. Seats (`model = claude-sonnet-5-5` each): `6a9c70b7-18b0-4729-8643-4acbd0539e6c` (`ui/src/panel_charts.rs`, `ui/style.css`), `b3bdf3eb-b8b4-4e58-a410-2927c209c168` (`ui/src/panel_usage.rs`), `06f2244b-935a-4dea-ab0f-d10107b1aec9` (`ui/src/library.rs`). No seat committed. The seat-wait wall time was not timed (the three Task calls returned in one turn).

## SPEC §9

- Row (F-NEW): **The Panel has three SVG charts and a usage block.** `core::view::panel_metrics` returns `PanelMetrics`: waste bars by the five groups (the run first, then one bar per milestone with features; legend = the groups with time; the five groups with their tokens burnt in waste), a timeline of time-state bands and rework outlines per feature, a cumulative-cost sparkline (`Sparkline::path`), the `CostReport` (the figures of `metrics --json`) and `throttled` apart. Geometry is fractions; the UI scales them (`ui/src/panel_charts.rs`, `ui/src/panel_usage.rs`, mounted in `panel.rs`).
- Row (F-NEW): **`throttled` is outside the charts.** It has its own block `panel-throttled` (run minutes, per milestone, intervals with reason); the word does not occur in the chart components or their legends.
- Row (F-NEW): **The library line carries a headline.** `view::Headline {cycle_s, waste_s, tokens_total, cost, currency, price_table}` joins `MissionRow`/`LibraryEntry` (`#[serde(default)]`, so rows saved before M5 still load and read "not measured"). The browser host measures a capture when it is picked; the window's `list_missions` measures each mission and caches by event count. `Eq` is dropped from `MissionRow` and `LibraryEntry` (an `f64` cost).

## Decisions

- None.

## BACKLOG

- B-new: `Headline.waste_s` is the five groups added over every feature, so it can exceed `cycle_s` (parallel features; M0: waste 129978 s, cycle 83027 s). The library column header should say "feature-seconds", or the CSV's definition (`m5-metrics-report-and-oracle`) replaces it. `cycle_s` is first event to last event, throttled time included (754 min on M0); the CSV's elapsed and cycle definitions must be reconciled with it.
- B-new: `metrics --json` carries no `throttled` section yet (`m5-metrics-report-and-oracle`), so the Panel's throttled row (M0: run 754 min, one interval `2026-10-22T18:01:02.011Z` to `2026-10-23T06:34:54.182Z`, reason `unknown`) is compared with core, not with the CLI. `throttled.per_milestone` is empty on M0 (the interval names no milestone).
- B-new: the Panel recomputes `panel_metrics` (card times, cost report) on every batch of a live mission, in the render path of the 1 s measurement. Not measured live here; if the hosted-run 1 s check regresses, memoise per event count or move it to the shell.
- B-new: the window passes `[metrics].budget` to the Panel (`MetricsBudget`) but not the launch form's `budget_usd`; the meter is proven off by default (browser) and by the core budget tests only.
- B-new: the library headline in the window and in the browser uses the bundled price table `2026-10-22` (`details::PRICES`, `telemetry::load_prices` falls back to it); `[metrics].price_table` is honoured only in the shell.
- B-new: the Panel is made over every event held, not the scrubbed position (`at`).
- B-new: pre-existing library rows saved in localStorage before this change show "not measured" until the capture is picked again.

## Deferred clauses

- VAL-METRIC-025 "equal to the CLI `--json`" for `throttled`: the JSON has no throttled section yet (above); claim on `m5-metrics-report-and-oracle`. Every other figure (roles, lanes, milestones, tokens in waste) matches the CLI (46 rows compared, 0 disagreeing).
- Numbers equal `metrics --csv` (parent text): the CSV does not exist yet; claim on `m5-export-and-parity-ui`.
- Tauri window: no smoke run (the Tauri path is `list_missions` only; `cargo check -p observatory-app --all-targets` and clippy are green); claim on `m5-export-and-parity-ui` for the window comparison (VAL-CROSS-008).

## Library

- Browser self-check: the new labels are `panel-chart-waste`, `panel-chart-timeline`, `panel-chart-sparkline`, `waste-legend-item`, `waste-groups`, `panel-timeline-row` (the SVG row; `timeline-row` stays the Details table row), `sparkline-path`, `panel-usage`, `usage-roles`, `usage-lanes`, `usage-milestones`, `usage-waste-tokens`, `usage-throttled-tokens`, `panel-throttled`, `throttled-run`, `throttled-intervals`, `usage-budget`, `usage-model-comparison`, `library-cycle`, `library-waste`, `library-tokens`, `library-cost`. A trunk rebuild returns the page to the library; re-upload.

## Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-METRIC-019 | Core: `waste_bars_use_only_the_five_groups_and_throttled_is_apart`, `the_timeline_has_one_row_per_feature_and_the_sparkline_one_path` (`crates/observatory-core/tests/view_panel_metrics.rs`). Browser (M0 capture): `.panel svg` = `panel-chart-waste`, `panel-chart-timeline`, `panel-chart-sparkline`; legend `Waiting on human`, `Retry`, `Idle` (subset of the five); `panel-timeline-row` count 12 = the golden's distinct features; one sparkline `<path>`; `throttled` occurs nowhere in `panel-charts`; console and errors empty; `target/scratch/panel-charts-2.png`, `panel-timeline-sparkline.png`. |
| VAL-METRIC-020 | Core: `the_headline_cost_equals_the_sum_of_the_milestone_rows_and_the_library_keeps_the_event_count` (within 0.01 USD). Browser: library row `observatory-v1 / 2026-10-23 / 1279 / 83027 s / 129978 s / 94185655 / 80.47 USD (2026-10-22)`; `target/scratch/library-headline.png`. |
| VAL-METRIC-025 | Browser: usage block with role rows (four roles, multiplier `roles.<role> x1`), seven lane rows by template, milestone rows, tokens burnt in waste per cause and per group beside the bars, price table `2026-10-22 (USD)`, `throttled` block apart (754 min, reason `unknown`); `/tmp/cells.json` vs `observatory-cli metrics fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --json`: rows compared 46, rows disagreeing 0, costs to the cent; `target/scratch/panel-usage.png`; console and errors empty. `throttled` vs the JSON is deferred (above). |
| VAL-CROSS-006 | Browser, Details of `m0` and `s`: six tabs `Timeline, Cost and waste, Review notes, Validation criteria, Commits and integration, Gate`; every tab opens; Timeline rows `95` (m0) and `251` (s) equal the golden's non-`turn.usage` events per milestone. The m1 report's figure was not re-read here; the golden counts are the check. |
| VAL-CROSS-009 | Core test above (event count 1279 kept, `headline` present with price table `2026-10-22`); browser library row shows `1279` beside the headline. The m1 library row's `mission_id` is the same `MissionRow.id`. |
| VAL-METRIC-009 (UI part, from `m5-time-states-and-rework-core`) | Panel stacked-bar legend labels are a subset of the five group labels (asserted in `waste_bars_use_only_the_five_groups_and_throttled_is_apart`; browser legend above); the timeline legend names the six states. Tab hover is the Details Cost tab (`m5-details-tabs-ui`). |
| VAL-METRIC-023 (Panel part) | `panel-throttled` block: run minutes 754, intervals with from, to, reason, minutes; not a group, not in the bars or legend. |
| VAL-METRIC-010 (UI), VAL-METRIC-011 (UI), VAL-METRIC-024/025 (Panel block) | Browser: `usage-budget` reads `Budget: off (no budget set)` under the default config; `usage-model-comparison` has ten role-model rows (orchestrator, worker, scrutiny_validator, user_testing_validator models, including `claude-fable-5.1`, `claude-sonnet-5-5`, `claude-opus-5-5`) equal to `model_comparison` of the CLI JSON; tokens per role and lane as in VAL-METRIC-025. The enabled-budget fraction is proven by core tests only. |
| `turn_usage_events_do_not_appear_in_lane_rows_or_cards` | Already present (`crates/observatory-core/tests/view_turn_usage.rs:63`), green in `cargo test -p observatory-core`. |
