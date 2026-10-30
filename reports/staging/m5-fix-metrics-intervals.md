# m5-fix-metrics-intervals

## SPEC §9

- Row (F-NEW): **`MetricsReport` lists the partition of every card** (fix feature from user-testing-validator-m5 round 1, VAL-CROSS-004 interval half; fix plan F2, pass-36 ruling item 8, product half; rule 24, no scope, cap or model change). New JSON key `intervals: Vec<CardIntervals { feature: Option<FeatureId>, milestone: Option<MilestoneId>, intervals: Vec<IntervalRow { from, to, state, cause }>, rework: Vec<ReworkWindowRow { kind, cause, opened_by, closed_by, from, to, key }> }>`, built from the `cards` map `metrics_report` already makes (no new computation) and the milestone membership `rows` already returns. Order is the map's: the mission-level card (`feature: null`) first, then the features by id; a card with no events has an empty list. `state` is the `TimeState` as the report already serialises it (`"active"`, `{"waiting_human": "question"}`); `cause` is that state's waste cause (`waiting_human_question`, ...), `null` for Active. The interval ends at the answer's `at`: a hosted question that names no feature reads `{"state": {"waiting_human": "question"}, "from": <ask at>, "to": <answer at>}` on the mission-level card. The CSV is unchanged (22 names, same rows). Attribution is unchanged (a question that names no feature stays on the mission-level card; the owner decision of row 73 is not pre-empted). Alternative (rejected): reuse `ReworkSpan` (merged windows) for `rework`; the plan names windows, and the merged span is derivable from them.
- Row (F-NEW): **M0 after the change** (`observatory-cli metrics fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --json`, parsed): 13 cards, the first the mission-level one (33 intervals, 0 rework windows); 16 question intervals over the cards, the first `2026-10-22T10:29:02.253Z` to `2026-10-22T10:44:06.336Z`, cause `waiting_human_question`.

## Decisions

None.

## BACKLOG

- B-new: `cargo test --workspace` failed once with `live::appended_lines_reach_the_ui_in_batches_within_one_second` (`event_batches.rs:296`), and `cargo test -p observatory-core` once with `live::lines_written_through_one_open_handle_flush_within_half_a_poll_and_a_batch`; both passed on an isolated rerun (`cargo test -p observatory-core --test event_batches`: 15 passed, 18.72 s) and on a second full run (558 passed, 0 failed). Wall-clock live-timing assertions in `tests/event_batches.rs` are load-sensitive when other test binaries run in parallel; not touched by this change (it reads no watcher code). A later feature should widen the margin or serialise those two tests.
- B-new: the Panel timeline (`view/panel_metrics.rs`, `apps/observatory-app/ui`) has no mission-level row, so the interval list is read from `metrics --json` or the Tauri `metrics` payload only. Reason: the plan puts the optional UI row outside this core-only slice (ui-worker, about 20 turns).

## Deferred clauses

- VAL-CROSS-004 attribution half ("the affected feature" against the mission-level card): not changed here; owner decision pending in row 73. Nothing is deferred to a later feature by this file; if the owner picks the feature card, the change is in `events_of_feature` and `mission_level` (`metrics/cards.rs`, `metrics/notes.rs`), and the interval list follows with no further code.

### Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-CROSS-004, interval half (the report shows `WaitingHuman(Question)` from the ask to the answer, `to` equal to the answer's `at`) | `cargo test -p observatory-core --test metrics_intervals`: `the_report_lists_the_waiting_human_question_interval_of_the_mission_level_card` (red `3d65766`, green `a7796dc`; `to == stamp(400)`, the `IntentAnswered` event's `at`); CLI on M0 `metrics ... --json` prints the first question interval from `10:29:02.253Z` to `10:44:06.336Z` |
| Intervals per card are the partition (lengths still sum to the card's elapsed time) | `the_report_lists_the_intervals_of_every_feature_card_in_partition_order` (contiguous, last `to` is the end); `the_json_intervals_carry_the_waste_cause_of_each_state` (per-cause seconds of the intervals equal the row's `waste_s` for the seven non-rework causes); the existing `tests/metrics_time_states.rs` partition test is untouched and green |
| CSV unchanged | `the_csv_header_and_rows_do_not_change_with_the_interval_list` (characterization, committed green in `3d65766`: `CSV_HEADER` has 22 names, one row per feature and milestone, no interval column); `metrics_report.rs` CSV tests untouched and green |
| Attribution unchanged | no change in `cards.rs`, `notes.rs`; the hosted-question test puts the unanswered-by-feature question on the card with `feature: null` |

## Library

- `MetricsReport.intervals` is the one place a UI or a script reads the partition. The Tauri `metrics` command returns `MetricsReport` by `Serialize` only; `apps/observatory-app/ui` does not deserialise it (it calls `panel_metrics` and `report_at_last_event` in-process, `ui/src/export.rs`, `ui/src/backend.rs`), so the new key needs no missing-field default there and no file under `apps/` was touched.
- Hand-off model line: `model = claude-sonnet-5-5`.
