# m5-fix-gate-window-card

## SPEC §9

- Row (F-NEW): **A gate rework window reaches its card** (fix feature from user-testing-validator-m5 round 1, VAL-CROSS-005; fix plan F3; rule 24, no scope, cap or model change; owner decision D-1 of the plan is not taken). (a) `rows()` makes a milestone row for every milestone that owns a mission-level rework span, even when no feature event names it (the M4 r2-walk store: 0 events with `feature_id`, 9 with `milestone_id`), so `rework_windows` 1, `fix_loops` 1 and the cause `gate` show on row `m5`. (b) `card_times` gives each feature card the mission-level spans whose milestone is one of the feature's milestones in a new `CardTime.milestone_rework`; `first_pass`, `rework_time` and `waste` of the card, `feature_row` (`rows.rs`) and the model comparison's first-pass time read `CardTime::rework_spans()` (own plus milestone spans). The window counts (`rework_windows`, `fix_loops`) keep their source (`card.rework`, `card.windows`, plus the mission-level spans added to the milestone row as before), so a window is counted once: one on the milestone row, none on the feature rows, `mission.fix_loops` one per milestone window. PRD §10.2 is kept: only Active time inside the window is rework; the red-to-green wall time is not. On the M4 store `rework_s` of `m5` stays 0 (the store holds only the orchestrator's session, no Active time in the window), so the wording of VAL-CROSS-005 "`waste_rework_s` above 0" passes only where Active time exists (a hosted run whose workers' events carry `milestone_id`); the validator's re-run on a scratch mission with active turns decides. A window with no milestone stays on the mission-level card (`overhead.waste_s[rework]`), where the Active time of the mission's own events inside it counts. Alternative (rejected, the plan's D-1): count the wall time of the window as `rework` (+8 turns, changes PRD §10.2).
- Row (F-NEW): **M0 and S metrics CSV after the change**: `observatory-cli metrics fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --csv` and `...-s.jsonl --csv` are byte-identical to the output of the tree before the change (`cmp` printed `m0 identical`, `s identical`); the fixtures have no gate window without feature events. The Details Cost tab needed no change: it already reads the milestone row (`cost.rs` `metrics_report` row), so its rework and waste equal the row.

## Decisions

None.

## BACKLOG

- B-new: `live::appended_lines_reach_the_ui_in_batches_within_one_second` (`tests/event_batches.rs:296`) failed in the full `cargo test -p observatory-core` run (`slowest line took 1.107622625s, bound 1s`, machine load average about 6) and again in the first isolated rerun of the file; it passed on the next isolated rerun (`appended_lines`, 6.21 s). The same flake is B-new in `m5-fix-metrics-intervals`; wall-clock bounds in that file are load-sensitive and the change reads no watcher code. A later feature should widen the margin or serialise those tests.
- B-new: a milestone with a gate window but no feature has `Cycle::NEVER` (`cycle_s` 0) in its new row: the milestone clock starts from feature events, and a gate-only milestone has none. Reason: the plan names no clock change; a later feature could start it at the first event naming the milestone.
- B-new: Active time of the mission's own events (orchestrator) inside a milestone's gate window sits on the mission-level card (`overhead`), not on the milestone row, so the milestone's `rework_s` counts only its features' Active time. Reason: `overhead_apportionment = "equal"` already shares the mission-level rework over the features, and adding it to the milestone row too would count it twice; the owner decides whether the milestone row should carry it.

## Deferred clauses

None. VAL-CROSS-005 live half (a hosted run with `milestone_id` on its events) is not reproduced here: the hosted scratch mission has no plan table (fix plan section 1, F5 is the board-level fix and is not in this round).

### Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-CROSS-005, window and fix loop on the card (one window, `cause` gate, `fix_loops` at least 1, opened by `CheckFailed` and closed by `CheckPassed`) | `cargo test -p observatory-core --test metrics_gate_windows`: `a_milestone_with_only_a_gate_window_gets_a_row_with_one_window_and_one_fix_loop` (red `d7ce490`, green `649aa72`: row `m5`, `rework_windows` 1, `fix_loops` 1, `mission.fix_loops` 1, `rework_s` 0); `tests/metrics_rework.rs` `red_check_opens_gate_rework_window` (cause `gate`, opened by `pr:red`, closed by `pr:green`) is untouched and green |
| VAL-CROSS-005, `waste_rework_s` above 0 where Active time exists | `active_time_of_a_milestones_features_inside_its_gate_window_counts_as_rework` (features 120 s and 20 s Active inside the window: feature rows `rework_s` and `waste_s[rework]` 120 and 20, milestone row 140, `mission.rework_s` 140); `the_cost_tab_rework_equals_the_milestone_row_with_a_gate_window` in `tests/view_parity.rs` (tab rework 140 = row) |
| Window counted once; another milestone's feature left alone; no milestone goes to the mission-level card | `a_gate_window_is_counted_once_in_the_milestone_row_and_the_mission_summary`; `a_gate_window_of_another_milestone_leaves_a_feature_alone`; `a_gate_window_without_a_milestone_lands_on_the_mission_level_card` (characterization, committed green in `d7ce490`: `overhead.waste_s[rework]` 120, no milestone row) |
| M0 and S CSV unchanged | `the_m0_and_s_metrics_csv_are_unchanged_by_gate_window_attribution` (characterization, green in `d7ce490`, goldens in `tests/golden/`); CLI `cmp` of `metrics ... --csv` before and after on M0 and S: identical |
| VAL-CROSS-005 on the M4 r2-walk store and on a hosted scratch mission with active turns | not run by this feature: the validator's re-run decides (the store has no Active time inside the window, so `waste_rework_s` stays 0 there by PRD §10.2) |

## Library

- `CardTime.rework` and `CardTime.windows` are the card's own windows (counts, unresolved flag, timeline bands); `CardTime.milestone_rework` is only for time; use `CardTime::rework_spans()` wherever Active time inside rework is measured.
- Hand-off model line: `model = claude-sonnet-5-5`.
