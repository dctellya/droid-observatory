# Oracle deviations, classified before any engine change (2026-10-30)

Slice `m6-reference-csv-oracle-parity`. Owner behaviour rule: inbox `2026-10-30-1748` (classify each deviation as engine defect, reference error or tolerance, with the evidence it rests on; engine defects are fixed in the engine; reference errors and tolerance questions go to the owner as one list).

Inputs, read-only: `reports/reference/2026-10-22-observatory-v1-m0.csv` (owner's file, e57aaed); the engine's rows from `cargo run -p observatory-cli -- metrics fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --csv` on tree `e57aaed`; the reference method in `resume/roci/oracle-draft/` (`METHOD.md`, `oracle.py`, `oracle-report.md`). Tolerance as in VAL-METRIC-003: 1 s, 1 token, 0.01 USD, `rework_windows` equal, `throttled_minutes` 1, `price_table` equal. Nothing in this slice changes the CSV, SPEC or the tolerance.

Count: the oracle test reports 102 deviations = 97 cell deviations on the 14 rows both sides have + 5 reference rows missing from the engine (m1 to m5). Classes below: E 36, E+Q 1, Q 41, R 24.

## Classes

- **E** engine defect: the engine contradicts a written definition (PRD, architecture, contract) or its own partition identity. Fixed in the engine, test first.
- **R** reference error: the reference contradicts a written definition or its own stated identity. Goes to the owner with the proposed corrected value (the engine's value on the same tree); the engine is not changed to match.
- **Q** owner question: two written sources disagree, or the number needs data the frozen mapping does not carry. Goes to the owner with a proposal; neither side is changed here. (The rule names the third class "tolerance"; none of these differences is a rounding difference, so no tolerance is proposed.)

## Evidence index

**E-1 (cycle_s, milestone `s`).** Contract, VAL-METRIC-003 and architecture section 9 "Two clocks": milestone cycle = "first `feature.defined` of the milestone". The golden has 6 `feature_defined` for `s`, all at 2026-10-22T17:35:44.452Z (session 6eb88099). `cycle.rs` takes `min` over those and the fallback starts of the milestone's features, so `scrutiny-validator-s` (no `feature_defined`, first `task_started` 17:33:20.272Z) starts the milestone 144 s early: engine 56919, reference 56775 (METHOD 6.3). Engine defect: the fallback may start a milestone only when the milestone has no `feature_defined`.

**E-2 (waste_waiting_human_question_s).** PRD 10.1 "Waiting on human: a question is open and nobody is acting". Golden: 13 `question_asked` and 12 `intent_answered`, all `feature_id: null`, role orchestrator; one question (10:44:19.810Z) is closed by a 107 ms `tool_error` (its AskUser `is_error` result), not by an answer. `cards.rs` leaves every featureless event at the mission level, so the 4 questions that fall inside feature windows on 2026-10-23 (07:14:41 to 07:29:32, 07:59:03 to 08:16:07, 08:16:35 to 08:19:22, 09:09:38 to 09:13:01 = 2285 s) are counted nowhere on a feature, and the time falls into `idle_*` (engine `s` rows: idle_unscheduled +2236 and idle_queued +49 = 2285 exactly; `m0-ci-rust-job-linux` and `m0-fixture-redaction`: 203 s, the 09:09 to 09:13 question; `m0-integrate-s`: 2285 s). `waiting_claims` pairs FIFO, so the unanswered 10:44:19.810 question would also shift every later pair. Engine defect: a featureless open question is a wait of every feature whose window it overlaps and that is not active.

**E-3 (waste_waiting_validation_s, `m0-*` feature rows).** PRD 10.1 "Waiting on validation: work is done, validator not started". D-023 judgement calls: the runner cancels a worker after its `EndFeatureRun`, so a runner worker ends in `task_skipped`. Golden: 0 worker `task_done` carry a feature id (the 29 worker `task_done` are featureless), 13 worker `task_skipped` do. `state.rs` opens a validation wait only at `TaskDone`/`ReworkResolved`, so the state is unreachable for every worker feature of the capture (engine: 0 on all 12 feature rows). `m0-ci-rust-job-linux`: worker cb3408d4 ends 08:44:48.205Z, no validator starts before the capture ends 09:21:59.213Z: 2231 s, of which 203 s are the question (E-2), leaving 2028 s, the reference value. Same for `m0-fixture-redaction` (1552) and `m0-integrate-s` (3276). Engine defect: a worker-role turn that ends cancelled or completed is "work done".

**E-4 (47.5 s of phantom Active, `user-testing-validator-s`).** The golden `subagent_finished` of d0304e2e carries `duration_ms` 46471105 (the whole file span, 17:45:31 on 22-oct to 06:40:02 on 23-oct, METHOD section 8). The engine reads it as an Active span and only removes the throttled interval 18:01:03.356Z to 06:34:54.182Z, so 06:34:54.182Z to 06:35:41.657Z (47.5 s, before the validator's second turn starts) counts as Active: engine first_pass 1703 vs reference active 1655 (930.7 + 260.6 + 463.7 s, raw turn spans) of which 338 s are Q-2. The same 47.5 s makes the other features "queued" instead of "unscheduled" (`s` rows idle_queued +49). Engine defect: PRD 10.1 Active is a tool call in flight or the model generating; a subagent's whole-run span is not.

**R-1 (s-feature rows, first_pass_active_s and waste_waiting_validation_s).** The reference partitions from the feature's first event (METHOD 6.1, "cycle_s is smaller than the sum of the waste columns plus active for these six rows"), but `cycle_s` starts at the scrutiny validator's `feature_defined` 17:35:44.452Z, after the worker's turn (for example `s-app-empty-lanes` worker 17:19:26 to 17:28:45, 558.7 s, and the 420 s wait to the scrutiny start). PRD 10.4: waste = total elapsed minus first-pass active, so a row must close: engine `s-app-empty-lanes` 132 + 11411 + 754 min x 60 = 56783 vs cycle_s 56775 (rounding of the minutes); reference 679 + 11843 + 45240 = 57762 vs 56775. VAL-METRIC-023's sum check (eight `waste_*_s` + 60 x `throttled_minutes` <= `cycle_s`) fails on the reference rows: 11843 + 45240 = 57083 > 56775. Reference error; proposed values are the engine's.

**R-2 (milestone rows).** The reference computes a milestone as one partition over the union of its features (METHOD 6.4: "the owner may prefer the sum"). VAL-METRIC-023 states the contract: "a milestone row is the sum of its feature rows in feature-seconds ... each `waste_*_s` equals the sum of that column over the milestone's feature rows"; `rows.rs` sums. The reference rows fail that check by construction (`s`: idle_queued 5039 vs sum of its feature rows 52927). Reference error; proposed values: the sums of the corrected feature rows.

**Q-1 (tokens_input, tokens_output, cost_usd).** Two written sources. PRD 10.5 and METHOD 12: "token counts are taken from `usage.reported` events per session" (sidecar); the reference sums the sidecar. F-034/D-029, architecture section 9 and VAL-METRIC-024: tokens per role, lane and milestone come from `turn.usage`; `rows.rs` sums `turn_usage` for input, output and cache_read and the sidecar only for cache_write (so one row mixes sources, and `cost_usd` prices that mix). The sources differ by design: the log's `input` is the non-cached part (D-029), and its output differs by 8 to 98 tokens per row from the sidecar. `tokens_cache_read` and `tokens_cache_write` agree on all 14 rows. Needs the owner: either the CSV rows read the sidecar (PRD 10.5; engine change plus its tests) or the reference is recomputed from `turn.usage` (proposed values are the engine's). `cost_usd` differs only through these tokens (`s` 0.0267, `m0` 0.0159).

**Q-2 (waste_retry_failure_s and the retry part of first_pass_active_s).** The reference counts the raw-file spans of every `is_error` tool call (`EndFeatureRun` results the runner marks failed, the 337.6 s UT `Task` call that returned `is_error` at the usage limit, 11.4 s in `s-app-empty-lanes`, 12 s in `scrutiny-validator-s`, 1.4 s in `m0-fixture-redaction`; METHOD 6.7). Mapping v1 puts a `duration_ms` on 2 of the 59 `tool_error` events and the mapping is frozen (VAL-METRIC-022), so no engine reading the event stream can see them (METHOD 6.7: "an engine reading the golden alone sees 0 s of tool errors on features"). Needs the owner: unfreeze the mapping to carry tool-error spans (new mapping version, golden refrozen) or correct the reference to the event stream (proposed values are the engine's).

**Q-3 (rows m1 to m5).** The reference writes five all-zero milestone rows for the `milestone_defined` events with no feature (METHOD 7: "Owner to confirm or drop the five rows"). No clause requires a row for a milestone with no feature; `rows.rs` makes a row from a milestone's features and gate windows. Needs the owner: drop the five rows (proposed) or confirm them and the engine emits a zero row per `milestone_defined`.

## Deviations (one line each)

| row | metric | reference | engine | delta | class | evidence |
|---|---|---|---|---|---|---|
| feature m0-ci-rust-job-linux | waste_waiting_human_question_s | 203 | 0 | -203 | E | E-2 |
| feature m0-ci-rust-job-linux | waste_waiting_validation_s | 2028 | 0 | -2028 | E | E-3 |
| feature m0-ci-rust-job-linux | waste_idle_queued_s | 0 | 1582 | +1582 | E | E-2+E-3 |
| feature m0-ci-rust-job-linux | waste_idle_unscheduled_s | 0 | 649 | +649 | E | E-2+E-3 |
| feature m0-ci-rust-job-linux | tokens_input | 1603 | 52 | -1551 | Q | Q-1 |
| feature m0-ci-rust-job-linux | tokens_output | 17411 | 17399 | -12 | Q | Q-1 |
| feature m0-fixture-freeze | tokens_input | 1574 | 50 | -1524 | Q | Q-1 |
| feature m0-fixture-freeze | tokens_output | 22700 | 22691 | -9 | Q | Q-1 |
| feature m0-fixture-redaction | first_pass_active_s | 415 | 417 | +2 | Q | Q-2 |
| feature m0-fixture-redaction | waste_waiting_human_question_s | 203 | 0 | -203 | E | E-2 |
| feature m0-fixture-redaction | waste_waiting_validation_s | 1552 | 0 | -1552 | E | E-3 |
| feature m0-fixture-redaction | waste_retry_failure_s | 2 | 0 | -2 | Q | Q-2 |
| feature m0-fixture-redaction | waste_idle_queued_s | 0 | 1165 | +1165 | E | E-2+E-3 |
| feature m0-fixture-redaction | waste_idle_unscheduled_s | 0 | 589 | +589 | E | E-2+E-3 |
| feature m0-fixture-redaction | tokens_input | 1607 | 46 | -1561 | Q | Q-1 |
| feature m0-fixture-redaction | tokens_output | 25980 | 25970 | -10 | Q | Q-1 |
| feature m0-integrate-s | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature m0-integrate-s | waste_waiting_validation_s | 3276 | 0 | -3276 | E | E-3 |
| feature m0-integrate-s | waste_idle_queued_s | 0 | 2152 | +2152 | E | E-2+E-3 |
| feature m0-integrate-s | waste_idle_unscheduled_s | 764 | 4173 | +3409 | E | E-2+E-3 |
| feature m0-integrate-s | tokens_input | 78838 | 74246 | -4592 | Q | Q-1 |
| feature m0-integrate-s | tokens_output | 100996 | 100957 | -39 | Q | Q-1 |
| feature s-app-empty-lanes | first_pass_active_s | 679 | 132 | -547 | R | R-1 |
| feature s-app-empty-lanes | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature s-app-empty-lanes | waste_waiting_validation_s | 420 | 0 | -420 | R | R-1 |
| feature s-app-empty-lanes | waste_retry_failure_s | 12 | 0 | -12 | Q | Q-2 |
| feature s-app-empty-lanes | waste_idle_queued_s | 6799 | 6848 | +49 | E | E-4+E-2 |
| feature s-app-empty-lanes | waste_idle_unscheduled_s | 2327 | 4563 | +2236 | E | E-2+E-4 |
| feature s-app-empty-lanes | tokens_input | 1296 | 74 | -1222 | Q | Q-1 |
| feature s-app-empty-lanes | tokens_output | 26928 | 26917 | -11 | Q | Q-1 |
| feature s-ci-wiring | first_pass_active_s | 320 | 135 | -185 | R | R-1 |
| feature s-ci-wiring | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature s-ci-wiring | waste_waiting_validation_s | 178 | 0 | -178 | R | R-1 |
| feature s-ci-wiring | waste_idle_queued_s | 6796 | 6845 | +49 | E | E-4+E-2 |
| feature s-ci-wiring | waste_idle_unscheduled_s | 2327 | 4563 | +2236 | E | E-2+E-4 |
| feature s-ci-wiring | tokens_input | 1197 | 62 | -1135 | Q | Q-1 |
| feature s-ci-wiring | tokens_output | 25310 | 25301 | -9 | Q | Q-1 |
| feature s-cli-replay | first_pass_active_s | 501 | 87 | -414 | R | R-1 |
| feature s-cli-replay | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature s-cli-replay | waste_waiting_validation_s | 1040 | 0 | -1040 | R | R-1 |
| feature s-cli-replay | waste_idle_queued_s | 6843 | 6892 | +49 | E | E-4+E-2 |
| feature s-cli-replay | waste_idle_unscheduled_s | 2327 | 4563 | +2236 | E | E-2+E-4 |
| feature s-cli-replay | tokens_input | 1309 | 74 | -1235 | Q | Q-1 |
| feature s-cli-replay | tokens_output | 49092 | 49079 | -13 | Q | Q-1 |
| feature s-core-event-model | first_pass_active_s | 383 | 101 | -282 | R | R-1 |
| feature s-core-event-model | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature s-core-event-model | waste_waiting_validation_s | 1589 | 0 | -1589 | R | R-1 |
| feature s-core-event-model | waste_idle_queued_s | 6829 | 6878 | +49 | E | E-4+E-2 |
| feature s-core-event-model | waste_idle_unscheduled_s | 2327 | 4563 | +2236 | E | E-2+E-4 |
| feature s-core-event-model | tokens_input | 1179 | 84 | -1095 | Q | Q-1 |
| feature s-core-event-model | tokens_output | 35890 | 35882 | -8 | Q | Q-1 |
| feature s-fixture-snapshot | first_pass_active_s | 343 | 150 | -193 | R | R-1 |
| feature s-fixture-snapshot | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature s-fixture-snapshot | waste_waiting_validation_s | 1903 | 0 | -1903 | R | R-1 |
| feature s-fixture-snapshot | waste_idle_queued_s | 6781 | 6830 | +49 | E | E-4+E-2 |
| feature s-fixture-snapshot | waste_idle_unscheduled_s | 2327 | 4563 | +2236 | E | E-2+E-4 |
| feature s-fixture-snapshot | tokens_input | 1244 | 74 | -1170 | Q | Q-1 |
| feature s-fixture-snapshot | tokens_output | 31878 | 31866 | -12 | Q | Q-1 |
| feature s-workspace-scaffold | first_pass_active_s | 755 | 127 | -628 | R | R-1 |
| feature s-workspace-scaffold | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature s-workspace-scaffold | waste_waiting_validation_s | 2150 | 0 | -2150 | R | R-1 |
| feature s-workspace-scaffold | waste_idle_queued_s | 6803 | 6853 | +50 | E | E-4+E-2 |
| feature s-workspace-scaffold | waste_idle_unscheduled_s | 2327 | 4563 | +2236 | E | E-2+E-4 |
| feature s-workspace-scaffold | tokens_input | 1607 | 108 | -1499 | Q | Q-1 |
| feature s-workspace-scaffold | tokens_output | 54652 | 54636 | -16 | Q | Q-1 |
| feature scrutiny-validator-s | first_pass_active_s | 364 | 382 | +18 | Q | Q-2 |
| feature scrutiny-validator-s | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature scrutiny-validator-s | waste_retry_failure_s | 18 | 0 | -18 | Q | Q-2 |
| feature scrutiny-validator-s | waste_idle_queued_s | 6694 | 6742 | +48 | E | E-4+E-2 |
| feature scrutiny-validator-s | waste_idle_unscheduled_s | 2327 | 4563 | +2236 | E | E-2+E-4 |
| feature scrutiny-validator-s | tokens_input | 827 | 28 | -799 | Q | Q-1 |
| feature scrutiny-validator-s | tokens_output | 19667 | 19657 | -10 | Q | Q-1 |
| feature user-testing-validator-s | first_pass_active_s | 1317 | 1703 | +386 | E+Q | E-4+Q-2 |
| feature user-testing-validator-s | waste_waiting_human_question_s | 2285 | 0 | -2285 | E | E-2 |
| feature user-testing-validator-s | waste_retry_failure_s | 338 | 0 | -338 | Q | Q-2 |
| feature user-testing-validator-s | waste_idle_unscheduled_s | 1978 | 4214 | +2236 | E | E-2+E-4 |
| feature user-testing-validator-s | tokens_input | 1941 | 58 | -1883 | Q | Q-1 |
| feature user-testing-validator-s | tokens_output | 34288 | 34269 | -19 | Q | Q-1 |
| milestone s | cycle_s | 56775 | 56919 | +144 | E | E-1 |
| milestone s | first_pass_active_s | 3930 | 2817 | -1113 | R | R-2 |
| milestone s | waste_waiting_human_question_s | 2285 | 0 | -2285 | R | R-2 |
| milestone s | waste_waiting_validation_s | 370 | 0 | -370 | R | R-2 |
| milestone s | waste_retry_failure_s | 371 | 0 | -371 | R | R-2 |
| milestone s | waste_idle_queued_s | 5039 | 52927 | +47888 | R | R-2 |
| milestone s | waste_idle_unscheduled_s | 2327 | 36155 | +33828 | R | R-2 |
| milestone s | tokens_input | 10600 | 562 | -10038 | Q | Q-1 |
| milestone s | tokens_output | 277705 | 277607 | -98 | Q | Q-1 |
| milestone s | cost_usd | 12.3742 | 12.3475 | -0.0267 | Q | Q-1 |
| milestone m0 | first_pass_active_s | 3070 | 3072 | +2 | R | R-2 |
| milestone m0 | waste_waiting_human_question_s | 2285 | 0 | -2285 | R | R-2 |
| milestone m0 | waste_waiting_validation_s | 1588 | 0 | -1588 | R | R-2 |
| milestone m0 | waste_retry_failure_s | 1970 | 1968 | -2 | R | R-2 |
| milestone m0 | waste_idle_queued_s | 0 | 4899 | +4899 | R | R-2 |
| milestone m0 | waste_idle_unscheduled_s | 764 | 5411 | +4647 | R | R-2 |
| milestone m0 | tokens_input | 83622 | 74394 | -9228 | Q | Q-1 |
| milestone m0 | tokens_output | 167087 | 167017 | -70 | Q | Q-1 |
| milestone m0 | cost_usd | 4.6854 | 4.6695 | -0.0159 | Q | Q-1 |
| milestone m1 | (row) | present | absent | - | Q | Q-3 |
| milestone m2 | (row) | present | absent | - | Q | Q-3 |
| milestone m3 | (row) | present | absent | - | Q | Q-3 |
| milestone m4 | (row) | present | absent | - | Q | Q-3 |
| milestone m5 | (row) | present | absent | - | Q | Q-3 |

## Largest deviation per column (orchestrator note, same tree)

cost_usd 0.0267, cycle_s 144, first_pass_active_s 1113, tokens_input 10038, tokens_output 98, waste_idle_queued_s 47888, waste_idle_unscheduled_s 33828, waste_retry_failure_s 371, waste_waiting_human_question_s 2285, waste_waiting_validation_s 3276; the other columns 0. The table above reproduces each of them (`python3` comparison of the two files, 97 cell deviations + 5 missing rows).

## Plan from here

1. Engine defects E-1 to E-4 are fixed test first in separate red and green commits; the oracle test is not touched except to make it fail, not skip, when the CSV is absent.
2. R and Q items cannot be closed by this slice. They go to the owner as one list in the hand-off (`partial`), with the proposed values; the CSV, SPEC and the tolerance are not edited.
3. VAL-METRIC-001/003/026 are graded after the two files agree (owner rule 5), not before.

## Addendum: after the engine fixes (commits `ee62f7f`, red golden `4599a85`)

Engine defects E-1 to E-4 are fixed (tests `crates/observatory-core/tests/oracle_defects.rs`, golden `metrics_m0.csv`). The oracle test now reports 67 deviations (62 cell deviations + 5 missing rows; before: 102), all of them R, Q or T: Q 36, R 28, T 3. No engine defect is left in the table.

**T-1 (`waste_idle_queued_s`, three `s` feature rows, +2 s).** `s-cli-replay` 6843 vs 6845, `s-core-event-model` 6829 vs 6831, `s-workspace-scaffold` 6803 vs 6805. The 6 other `s` rows agree within 1 s, and the sum of `idle_queued` + `idle_unscheduled` agrees per row; the cause is not isolated (the reference rounds float sums of its segments, METHOD 4; the engine rounds each interval's milliseconds). Proposal to the owner: a 2 s tolerance on duration columns, or the engine's value as the corrected reference value.

**Side effect to note (not a deviation):** the `waiting_human_question_s` of an `m0-*` feature row is now non-zero (203 s, 203 s, 2285 s), as in the reference; milestone rows are sums (R-2), so `s` reads 18280 against the reference 2285.

| row | metric | reference | engine | delta | class | evidence |
|---|---|---|---|---|---|---|
| feature m0-ci-rust-job-linux | tokens_input | 1603 | 52 | -1551 | Q | Q-1 |
| feature m0-ci-rust-job-linux | tokens_output | 17411 | 17399 | -12 | Q | Q-1 |
| feature m0-fixture-freeze | tokens_input | 1574 | 50 | -1524 | Q | Q-1 |
| feature m0-fixture-freeze | tokens_output | 22700 | 22691 | -9 | Q | Q-1 |
| feature m0-fixture-redaction | first_pass_active_s | 415 | 417 | +2 | Q | Q-2 |
| feature m0-fixture-redaction | waste_retry_failure_s | 2 | 0 | -2 | Q | Q-2 |
| feature m0-fixture-redaction | tokens_input | 1607 | 46 | -1561 | Q | Q-1 |
| feature m0-fixture-redaction | tokens_output | 25980 | 25970 | -10 | Q | Q-1 |
| feature m0-integrate-s | tokens_input | 78838 | 74246 | -4592 | Q | Q-1 |
| feature m0-integrate-s | tokens_output | 100996 | 100957 | -39 | Q | Q-1 |
| feature s-app-empty-lanes | first_pass_active_s | 679 | 132 | -547 | R | R-1 |
| feature s-app-empty-lanes | waste_waiting_validation_s | 420 | 0 | -420 | R | R-1 |
| feature s-app-empty-lanes | waste_retry_failure_s | 12 | 0 | -12 | Q | Q-2 |
| feature s-app-empty-lanes | tokens_input | 1296 | 74 | -1222 | Q | Q-1 |
| feature s-app-empty-lanes | tokens_output | 26928 | 26917 | -11 | Q | Q-1 |
| feature s-ci-wiring | first_pass_active_s | 320 | 135 | -185 | R | R-1 |
| feature s-ci-wiring | waste_waiting_validation_s | 178 | 0 | -178 | R | R-1 |
| feature s-ci-wiring | tokens_input | 1197 | 62 | -1135 | Q | Q-1 |
| feature s-ci-wiring | tokens_output | 25310 | 25301 | -9 | Q | Q-1 |
| feature s-cli-replay | first_pass_active_s | 501 | 87 | -414 | R | R-1 |
| feature s-cli-replay | waste_waiting_validation_s | 1040 | 0 | -1040 | R | R-1 |
| feature s-cli-replay | waste_idle_queued_s | 6843 | 6845 | +2 | T | T-1 |
| feature s-cli-replay | tokens_input | 1309 | 74 | -1235 | Q | Q-1 |
| feature s-cli-replay | tokens_output | 49092 | 49079 | -13 | Q | Q-1 |
| feature s-core-event-model | first_pass_active_s | 383 | 101 | -282 | R | R-1 |
| feature s-core-event-model | waste_waiting_validation_s | 1589 | 0 | -1589 | R | R-1 |
| feature s-core-event-model | waste_idle_queued_s | 6829 | 6831 | +2 | T | T-1 |
| feature s-core-event-model | tokens_input | 1179 | 84 | -1095 | Q | Q-1 |
| feature s-core-event-model | tokens_output | 35890 | 35882 | -8 | Q | Q-1 |
| feature s-fixture-snapshot | first_pass_active_s | 343 | 150 | -193 | R | R-1 |
| feature s-fixture-snapshot | waste_waiting_validation_s | 1903 | 0 | -1903 | R | R-1 |
| feature s-fixture-snapshot | tokens_input | 1244 | 74 | -1170 | Q | Q-1 |
| feature s-fixture-snapshot | tokens_output | 31878 | 31866 | -12 | Q | Q-1 |
| feature s-workspace-scaffold | first_pass_active_s | 755 | 127 | -628 | R | R-1 |
| feature s-workspace-scaffold | waste_waiting_validation_s | 2150 | 0 | -2150 | R | R-1 |
| feature s-workspace-scaffold | waste_idle_queued_s | 6803 | 6805 | +2 | T | T-1 |
| feature s-workspace-scaffold | tokens_input | 1607 | 108 | -1499 | Q | Q-1 |
| feature s-workspace-scaffold | tokens_output | 54652 | 54636 | -16 | Q | Q-1 |
| feature scrutiny-validator-s | first_pass_active_s | 364 | 382 | +18 | Q | Q-2 |
| feature scrutiny-validator-s | waste_retry_failure_s | 18 | 0 | -18 | Q | Q-2 |
| feature scrutiny-validator-s | tokens_input | 827 | 28 | -799 | Q | Q-1 |
| feature scrutiny-validator-s | tokens_output | 19667 | 19657 | -10 | Q | Q-1 |
| feature user-testing-validator-s | first_pass_active_s | 1317 | 1655 | +338 | Q | Q-2 |
| feature user-testing-validator-s | waste_retry_failure_s | 338 | 0 | -338 | Q | Q-2 |
| feature user-testing-validator-s | tokens_input | 1941 | 58 | -1883 | Q | Q-1 |
| feature user-testing-validator-s | tokens_output | 34288 | 34269 | -19 | Q | Q-1 |
| milestone s | first_pass_active_s | 3930 | 2769 | -1161 | R | R-2 |
| milestone s | waste_waiting_human_question_s | 2285 | 18280 | +15995 | R | R-2 |
| milestone s | waste_waiting_validation_s | 370 | 0 | -370 | R | R-2 |
| milestone s | waste_retry_failure_s | 371 | 0 | -371 | R | R-2 |
| milestone s | waste_idle_queued_s | 5039 | 52593 | +47554 | R | R-2 |
| milestone s | waste_idle_unscheduled_s | 2327 | 18259 | +15932 | R | R-2 |
| milestone s | tokens_input | 10600 | 562 | -10038 | R | R-2 |
| milestone s | tokens_output | 277705 | 277607 | -98 | R | R-2 |
| milestone s | cost_usd | 12.3742 | 12.3475 | -0.0267 | R | R-2 |
| milestone m0 | first_pass_active_s | 3070 | 3072 | +2 | R | R-2 |
| milestone m0 | waste_waiting_human_question_s | 2285 | 2691 | +406 | R | R-2 |
| milestone m0 | waste_waiting_validation_s | 1588 | 6856 | +5268 | R | R-2 |
| milestone m0 | waste_retry_failure_s | 1970 | 1968 | -2 | R | R-2 |
| milestone m0 | tokens_input | 83622 | 74394 | -9228 | R | R-2 |
| milestone m0 | tokens_output | 167087 | 167017 | -70 | R | R-2 |
| milestone m0 | cost_usd | 4.6854 | 4.6695 | -0.0159 | R | R-2 |
| milestone m1 | (row) | present | absent | - | Q | Q-3 |
| milestone m2 | (row) | present | absent | - | Q | Q-3 |
| milestone m3 | (row) | present | absent | - | Q | Q-3 |
| milestone m4 | (row) | present | absent | - | Q | Q-3 |
| milestone m5 | (row) | present | absent | - | Q | Q-3 |
