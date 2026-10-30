# m5-fix-completed-integrated

## SPEC §9

- Row (F-NEW): **The completed-feature count applies `integrated` where the capture records it** (fix feature from user-testing-validator-m5 round 1, VAL-METRIC-011; fix plan F4; pass-36 ruling item 3, product half; rule 24, no scope, cap or model change). `completed_features` (`metrics/compare.rs`) requires, besides `task.done`, `scrutiny.passed` and no unresolved rework window, an `integration.merged` event at or after the feature's last `task.done` that names the feature or, when it names no feature, the feature's milestone. The condition applies only when the capture holds at least one `integration.merged` event (`integration_observable`, exported); otherwise it is waived and every `ModelComparisonRow` carries `integrated_observable: false` (serialised in `--json`, `true` when applied). Where the plan's mechanics and the amended contract text (pass-36, validation-contract line 1141) differ, the amended text decides: (1) an event that names the feature counts for that feature (the plan spoke only of the milestone); (2) an event that names a milestone counts only when it names no feature, so another feature's merge does not integrate this one. The plan's "at or after the last `task.done`" is kept: the amended text is silent on order and does not contradict it, and a merge before the feature's last `task.done` did not integrate that work. The M0 capture holds no `integration.merged` event: the count stays 5 and every per-completed-feature figure is unchanged (CLI `--json` on the M0 capture: 10 role-model rows, each `completed_features` 5, `integrated_observable` false; `claude-sonnet-5-5` worker `cost_per_completed_feature` 1.8290521399999995, `first_pass_active_s_per_completed_feature` 1067.0350000000003). `telemetry.rs` reads `completed_features` for `observatory.feature.cycle_seconds`, so a capture with merge events now reports that series for integrated features only; the M0 and S fixtures are unchanged.

## Decisions

None.

## BACKLOG

- B-new: the real `GitSource` emits `integration.merged` with the milestone only and no feature (`ingest/git/mod.rs` `integration_events_of`), so on a store with merges a feature counts as completed once its milestone merged after its last `task.done`, even if the merge predates a later fix of that feature that has no new `task.done`. Reason: the milestone is the finest unit the git source can name; a later feature could map a merge to the features it carries if the plan table ever names them.
- B-new: the Panel model-comparison table (`apps/observatory-app/ui/src/panel_usage.rs`) does not show `integrated_observable`; the plan names only core files and the app was not touched. Reason: a note under the table ("integrated condition waived: the capture records no merge") is a UI feature; a later feature could read the serialised field.

## Deferred clauses

None. The Panel half of VAL-METRIC-011 (the three rows in the Panel) is not changed by this feature; the validator's re-run of `mech-metrics` decides VAL-METRIC-011 as amended.

## Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-METRIC-011 as amended: with a synthetic `integration.merged` the count applies the condition (feature named or milestone-only, at or after the last `task.done`) | `cargo test -p observatory-core --test metrics_report`: `a_feature_is_completed_only_when_its_milestone_was_integrated_after_task_done` (red `d41c46e`, green `529a850`: f1 integrated by the m1 merge, f4 by its own named merge counted; f2 done after the m1 merge and f3 in the never-merged m2 not counted; `completed_features` 2, `integrated_observable` true) |
| VAL-METRIC-011 as amended: on the M0 capture (no `integration.merged`) the count is the features with `task.done` and validation green, and the report says the condition is not observable | `a_session_capture_with_no_integration_event_waives_the_integrated_condition_and_says_so` (synthetic stream: both features count, `integrated_observable` false; M0 golden: every row `completed_features` 5, `integrated_observable` false); CLI `metrics_json_says_the_integrated_condition_is_not_observable_on_the_m0_capture` in `crates/observatory-cli/tests/metrics.rs`; `observatory-cli metrics fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --json` prints `completed_features` 5 and `integrated_observable` false on all 10 rows |
| VAL-METRIC-011 as amended: each figure equals the role-model's total divided by its count of completed features, recomputed from the listed features | `the_per_completed_feature_figures_recompute_from_the_listed_features` (listed `completed_features(...)` = f1, f4; worker `cost_per_completed_feature` and `first_pass_active_s_per_completed_feature` equal total / 2 within 1e-9); existing `model_comparison_groups_by_role_and_model` (no merge event, count 2 unchanged) and `metrics_json_compares_the_three_role_models_per_completed_feature` stay green |

## Library

- `completed_features` is the single definition of "completed" (model comparison and the telemetry series read it); `integration_observable(events)` says whether its `integrated` condition applies.
- Hand-off model line: `model = claude-sonnet-5-5`.
