# Staging: m6-reference-csv-oracle-parity

## SPEC §9

| F-next | 2026-10-30 | Row `core-worker` (kind: oracle parity, owner rule 2026-10-30 17:48 and rulings ~17:25Z): the 102 deviations of the engine against the owner's reference CSV were classified in `reports/readiness/2026-10-30-oracle-deviations.md` before any engine edit. Four engine defects fixed test first: E-1 milestone cycle starts at its first `feature.defined`; E-2 a question no feature owns is a `waiting_human_question` wait of every feature not active in it; E-3 a worker `task.skipped` is work done (waiting on validation reachable); E-4 a subagent whole-run span is not Active when its turns are measured. The owner ruled R-1, R-2, Q-1, Q-2 and T-1: the engine's values are the corrected reference, tolerance unchanged; Q-3: the five rows m1 to m5 are dropped. The corrected reference is the proposal `reports/readiness/2026-10-30-oracle-reference-proposal.csv` (the reference header and bytes, 79 cells set to the engine's values, 5 rows dropped); the CSV in `reports/reference/` is copied from it by `m6-close-integrate` on the owner's word, in one commit. Against the proposal the engine has 0 deviations in 266 compared cells, and the oracle test copied onto the proposal passes. **Alternatives considered:** changing the engine to the reference's derived numbers (rejected: the dismissal-as-claim pattern, inbox 1319 rule 4); recomputing the CSV token columns from the sidecar (PRD 10.5; rejected by the owner's Q-1 ruling). | `reports/readiness/2026-10-30-oracle-deviations.md`, `reports/readiness/2026-10-30-oracle-reference-proposal.csv`, commits `d5c8b86`, `ae622fd`, `bc6b565`, `4599a85`, `ee62f7f` | proven against the proposal; graded after the close copy |

## Decisions

- D-029 (usage-log source): PRD 10.5 sidecar-sum wording moves to D-029; the CSV token columns read turn.usage (F-034, METRIC-024). Owner ruling Q-1, 2026-10-30 ~17:25Z; the tokens of a CSV row are the sum of the `turn.usage` of its sessions, `tokens_cache_write` stays the sidecar's, the two agree on `cache_read` and `cache_write` on all 14 rows of the M0 capture.

## BACKLOG

- New: mapping v2: tool_error spans carry durations (Q-2). Reason: mapping v1 puts a `duration_ms` on 2 of the 59 `tool_error` events of the M0 capture and is frozen (VAL-METRIC-022), so the raw-file spans of `is_error` tool calls (METHOD 6.7: 12 s in `s-app-empty-lanes`, 18 s in `scrutiny-validator-s`, 338 s in `user-testing-validator-s`, 2 s in `m0-fixture-redaction`) are invisible to the engine; the owner ruled that the engine's values stand and the mapping change goes to a later version with the golden refrozen.
- New: `waiting_idle_queued_s` +2 s on `s-cli-replay`, `s-core-event-model` and `s-workspace-scaffold` (T-1) has no isolated cause. Reason: the owner ruled the engine's value stands with the tolerance unchanged; the other six `s` rows agree within 1 s and `idle_queued` + `idle_unscheduled` agrees per row; the likely cause is the reference rounding float sums of its segments (METHOD 4) against the engine's rounding of each interval in milliseconds, not shown.
- New: `waiting_claims` pairs a mission-level question with the next answer FIFO (FIFO pairing of mission-level questions). Reason: the unanswered question of 2026-10-23T10:44:19.810Z shifts every later pair on the mission-level card, so the overhead's `waiting_human_question` is mis-paired; `mission_question_waits` (state.rs) already pairs by answer or by the refused AskUser error and could be reused; no CSV column reads the overhead line.

## Deferred clauses

Owner rulings, 2026-10-30 ~17:25Z, verbatim: "Oracle rulings. R-1, R-2, Q-1, Q-2, T-1: the engine's values are the corrected reference; tolerance unchanged. Q-3: drop the five m1 to m5 rows. PRD 10.5 wording moves to D-029 at the boundary. Tool-error spans in the mapping go to BACKLOG as mapping v2. The parity worker writes the corrected CSV as a proposal beside the deviations report; the close integrator copies it to reports/reference/ in one commit on this word; then METRIC-001/003/026 are graded."

VAL-METRIC-001, VAL-METRIC-003 and VAL-METRIC-026 are graded after `m6-close-integrate` copies the proposal to `reports/reference/` (owner ruling 2026-10-30 ~17:25Z). Until then `metrics_match_reference_spreadsheet_within_tolerance` stays red against the owner's draft CSV (67 deviations: R, Q and T items, all ruled to the engine's values); the pre-push gate and the `coverage` job fail on it and on nothing else.

The M5 waste figures (`reports/waste/m5.md`, pasted at `448696e`) predate the engine fixes E-1 to E-4: four causes and `first_pass_active_s` are outside the tolerance against the proposal as committed. The dated addendum in each report pastes the comparison and the re-run figures; the reports' bodies are not edited (forward only).

## Library

- `metrics::state::mission_question_waits` pairs a mission-level `QuestionAsked` with the next answer, or with the `ToolError` whose span starts at the question (the refused AskUser call).
- Regenerating the m0 golden: `OBSERVATORY_UPDATE_GOLDEN=1 cargo test -p observatory-core --test metrics_gate_windows`.
- Running the oracle test against another reference without touching the test: copy `tests/oracle.rs` to a scratch `tests/oracle_<x>.rs` with the path replaced, run it, delete it (the test has no env override).

### Claims (rule 30)

- VAL-METRIC-001: `git ls-files reports/reference/` lists `2026-10-22-observatory-v1-m0.csv` (e57aaed); its header is the 22-name header (the proposal's first line is byte-identical, `cmp` exit 0); `throttled_minutes` is an integer on all 14 rows of the proposal. Graded after the close copy.
- VAL-METRIC-003: proven against the proposal, graded after the close copy: `python3 cmp.py proposal engine.csv` prints `rows reference=14 engine=14 cells compared=266 deviations=0`; a scratch copy of the oracle test with the proposal as the reference prints `test result: ok. 6 passed`. Against the owner's draft the committed test is still red (67 deviations).
- VAL-METRIC-026: proven against the proposal, graded after the close copy: the dated addenda of `reports/waste/m5.md` and `reports/usage/m5.md` paste the per-row comparison, deltas and tolerances; the engine's CSV is inside every tolerance; the M5 waste figures as committed are stale against the proposal on four causes and `first_pass_active_s` (stated in the addendum).
