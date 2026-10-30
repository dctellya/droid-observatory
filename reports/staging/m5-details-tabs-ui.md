# m5-details-tabs-ui

Lead worker with three `mechanical-sonnet` seats (owner inbox 2320). The lead wrote the red test, the core view functions and `details.rs`; the seats wrote the five tab components. Seats (`model = claude-sonnet-5-5` each): `29e83fa8-5e65-491d-a548-bc914c7343cd` (`tabs/cost.rs`), `85af438c-1b87-482f-b732-53bd75f385e4` (`tabs/review.rs`, `tabs/criteria.rs`), `00418d0e-ae13-4281-a053-717f7ea7b7db` (`tabs/commits.rs`, `tabs/gate.rs`). No seat committed. The seat-wait wall time was not timed (no timer ran; the three Task calls returned in one turn).

## SPEC §9

- Row (F-NEW): **Details has six tabs.** `core::view::{cost_tab, review_tab, criteria_tab, commits_tab, gate_tab}` (`view/tabs/*.rs`) feed the tabs Cost and waste (FR-D6), Review notes (FR-D2), Validation criteria (FR-D3), Commits and integration (FR-D4) and Gate (FR-D5) beside the unchanged Timeline (`ui/src/details.rs`, `ui/src/tabs/*.rs`). Each tab is computed only while it is open, over the events held and `Notes::default()`.
- Row (F-NEW): **Criteria are derived checkpoints.** Events carry no criterion text. The Validation criteria tab lists, per feature of the milestone, six checkpoints the cycle records (Tests run, Committed, Task done, Scrutiny passed, User-testing scenario passed, Gate command passed), each with the event ids that satisfied it or the `unsatisfied` marker, and a `Plan line` button that opens the scrubbed raw line of the feature's `feature.defined` event, where the plan's own text lives.
- Row (F-NEW): **Integration wording.** Integration-lane entries read `integrated in mission <branch>` (core string in `commits_tab`); the word `merged` appears nowhere in the Commits tab (0 matches in the browser, `g1-integrated-wording` now reads the two `tabs/commits.rs` files and is green, no longer pending).

## Decisions

- None.

## BACKLOG

- B-new: the criterion text of the plan is not in any event. FR-D3 as written ("the acceptance criteria from the plan") needs the plan's `expectedBehavior` or `fulfills` read from the plan sidecar or the raw `feature.defined` line into a `Criterion {text}`; until then the tab shows the six cycle checkpoints and links the plan line.
- B-new: a non-blocking finding opens no rework window and has no resolving event, so Review notes shows every non-blocking finding as `open` (24 of 24 on milestone `s`). A `resolved` mark for it needs a source (a later `finding` closing event or an app action).
- B-new: the Cost tab's `Cache write` column comes from the sidecar's `usage.reported` events (the usage log has no cache-write count) and is 0 on the M0 fixture, where those events carry no milestone. The CSV's cache-write source must be the same.
- B-new: both hosts name the bundled `config/prices/2026-10-22.toml` in the Details (`include_str!` in `ui/src/details.rs`); `[metrics].price_table` is not read by the window. The Tauri window should get the configured table through the `metrics` command at `m5-export-and-parity-ui`.
- B-new: the Details tabs use `Notes::default()`: the browser host has no store, and the window does not pass the store's notes either. Scenario keys (Review notes), check keys (rework windows keyed by check) and app answers closing a question (`Notes::with_answer`) are not named in the Details until the notes reach the UI.
- B-new: the M0 fixture has no rework window, no `finding.blocking`, no `qa.scenario.*` result under a milestone and no PR, so those sections render empty in the browser self-check; the unresolved-window row and the blocking/resolved finding rows are proven by core tests over built events, not by a screenshot.
- B-new: pre-existing red contract checks seen in `sh scripts/contract-checks/run-all.sh` on this tree (not touched here): `g2-metric-tests` and `g2-telemetry-tests` want `landed=yes` rows in `g2-tests.tsv`, `g3-metrics-contract` waits for `metrics --csv` (row 62, `m5-metrics-report-and-oracle`), `g6-shared-files` names `4b40a09` (D-047 staging commit).

## Deferred clauses

- VAL-METRIC-014 "totals equal the milestone row of the CLI `--csv`": `observatory-cli metrics --csv` does not exist yet (`m5-metrics-report-and-oracle`). The tab was compared with the CLI's `metrics --json` (same `cost_report`); the CSV diff is owed by `m5-export-and-parity-ui` (claim on that feature, owner item 18).
- Details-tab views of VAL-CROSS-004/-005 (cross checks with `control::record` `HumanAction::answer` and external-role `check.failed`/`check.passed` keyed by `Notes`): not proven. The tabs list the windows a `Notes` produces, but the UI passes `Notes::default()` (B-row above); the app-side proof is a claim on `m5-export-and-parity-ui`.
- No Tauri smoke: `src-tauri/` is untouched; the tabs are the same wasm in both hosts.

## Library

- Browser host self-check: after `snapshot -i`, `upload @<open-files ref>` with the root `.jsonl`, the `.capture.json`, the `.settings.json`, the `usage-log.jsonl` and `children/*.jsonl` (the capture header names the usage log; without it the turns, and so every token cell, read 0). Details buttons are `details-<milestone>` (`s` and `m0` sit in lane 3). A tab body is rendered one tick after the click: sleep before reading it. Each `agent-browser eval` after a page `open` needs a fresh `snapshot` before refs resolve.

## Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-METRIC-014 | Core: `cost_tab_totals_equal_the_milestone_row_of_the_cost_report` and `cost_tab_lists_the_eight_causes_and_every_feature_cost_names_the_price_table` (`crates/observatory-core/tests/view_tabs.rs`, 11 passed). Browser: Cost and waste on milestone `s` shows price table `2026-10-22 (USD)`, totals `562 / 277607 / 17913619 / 0` and `6.28`, eight feature rows, per-role rows, eight waste causes, the rework section (`target/scratch/details-cost-and-waste.png`); `observatory-cli metrics fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --json` prints milestone `s` `{input 562, output 277607, cache_read 17913619}`, `6.28`, `2026-10-22`. CSV diff deferred (above). |
| VAL-METRIC-015 | `review_tab_counts_equal_the_golden_events_of_the_milestone` (counts equal the golden's `finding_*` and `qa_scenario_*` events per `milestone_id`; `s` has 24 findings, 0 scenarios). Browser: Review notes on `s` shows `Findings: 24`; `target/scratch/details-review-notes.png`. |
| VAL-METRIC-016 | `criteria_tab_maps_each_criterion_to_events_or_the_unsatisfied_marker`, `criteria_tab_lists_features_of_the_golden_milestone`. Browser: `Features: 8` on `s`; `s-app-empty-lanes` lists `Tests run satisfied` with five event ids and `User-testing scenario passed unsatisfied`; clicking an event id opened the scrubbed raw line (`raw-line` text, `target/scratch/details-validation-criteria-raw.png`). Deviation: criteria are derived checkpoints (B-row). |
| VAL-METRIC-017 | `commits_tab_counts_equal_the_commit_created_events_of_the_milestone` (`s` 28, `m0` 14 equal the golden's `commit_created` per milestone) and `commits_tab_says_integrated_in_mission_and_never_merged`. Browser: `Commits: 28` on `s`, `merged` count in the tab text 0; `sh scripts/contract-checks/g1-integrated-wording.sh` prints `g1-integrated-wording ok VAL-METRIC-017` (no pending note). |
| VAL-METRIC-018 | `gate_tab_is_pending_with_empty_sections_and_two_unvalued_clocks_without_a_pr`. Browser: Gate on `s` reads `Gate state: Pending`, `Mission PR: none`, `reviews: 0`, empty checks, attestations and history, both clocks `no value`; `agent-browser console` and `errors` printed nothing. |
| VAL-METRIC-005 (behavioural part, from `m5-time-states-and-rework-core`) | `cost_tab_lists_an_unresolved_window_with_its_cause_lane`: an unresolved `finding.blocking` window lists `unresolved` with its cause lane name. Core only (the fixture has no window). |
| VAL-METRIC-007 (UI part, from `m5-cost-and-tokens-core`) | Per-feature cost with the price table date in the Cost tab: every `cost.price_table == "2026-10-22"` asserted per feature row; browser `Price table: 2026-10-22 (USD)` and per-feature costs. |
