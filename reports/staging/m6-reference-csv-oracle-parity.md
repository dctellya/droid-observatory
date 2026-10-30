# Staging: m6-reference-csv-oracle-parity (partial)

## SPEC §9

| F-next | 2026-10-30 | Row `core-worker` (kind: oracle parity, owner rule 2026-10-30 17:48): the 102 deviations of the engine against the owner's reference CSV are classified in `reports/readiness/2026-10-30-oracle-deviations.md` before any engine edit. Four engine defects fixed test first: E-1 milestone cycle starts at its first `feature.defined`; E-2 a question no feature owns is a `waiting_human_question` wait of every feature not active in it; E-3 a worker `task.skipped` is work done (waiting on validation reachable); E-4 a subagent whole-run span is not Active when its turns are measured. 35 deviations closed; 67 remain, none an engine defect: 28 reference errors (R), 36 owner questions (Q), 3 tolerance (T). **Alternatives considered:** changing the engine to the reference's derived numbers (rejected: the dismissal-as-claim pattern, inbox 1319 rule 4). | `reports/readiness/2026-10-30-oracle-deviations.md`, commits `d5c8b86`, `ae622fd`, `bc6b565`, `4599a85`, `ee62f7f` | partial: owner list open |

## Decisions

None written. Q-1 (token source of the CSV rows) and Q-2 (tool-error spans versus the frozen mapping) need a decision file once the owner rules.

## BACKLOG

- Owner list for the oracle (see the hand-off): R-1 six `s` feature rows (first_pass, waiting_validation) and R-2 milestone rows (union against the contract's sum), proposed corrected values are the engine's on the same tree; Q-1 tokens_input/tokens_output/cost_usd source (PRD 10.5 sidecar against F-034/D-029 `turn.usage`); Q-2 raw-file tool-error spans (frozen mapping v1); Q-3 rows m1 to m5 (drop or confirm); T-1 2 s on three `idle_queued` cells.
- The mission-level card (`None`) still pairs questions FIFO in `waiting_claims`; the unanswered question of 10:44:19.810Z is closed only in `mission_question_waits`, so the overhead's `waiting_human_question` for the M0 capture is mis-paired. Not read by any CSV column.

## Deferred clauses

VAL-METRIC-001, VAL-METRIC-003 and VAL-METRIC-026 are not proven: the oracle test still fails (67 deviations) and the METRIC-026 addenda to `reports/waste/m5.md` and `reports/usage/m5.md` are not written, because owner rule 5 grades them after the two files agree. The oracle test no longer skips: it panics when the CSV is unreadable (`bc6b565`).

## Library

- `metrics::state::mission_question_waits` pairs a mission-level `QuestionAsked` with the next answer, or with the `ToolError` whose span starts at the question (the refused AskUser call).
- Regenerating the m0 golden: `OBSERVATORY_UPDATE_GOLDEN=1 cargo test -p observatory-core --test metrics_gate_windows`.

### Claims (rule 30)

- VAL-METRIC-001: not claimed (tracked file and header exist; graded after the files agree).
- VAL-METRIC-003: not claimed. `cargo test -p observatory-core --test oracle metrics_match_reference_spreadsheet_within_tolerance` FAILED with 67 deviations (62 cells + 5 rows).
- VAL-METRIC-026: not claimed (addenda not written).
