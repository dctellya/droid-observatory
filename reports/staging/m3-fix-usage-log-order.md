# Staging: m3-fix-usage-log-order

Feature `m3-fix-usage-log-order` (fix after scrutiny-validator-m3 round 1, `blockingIssues`), worktree
`observatory-wt/m3`, branch `wt/m3`. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): `scripts/usage-report.py` filters every log line by its own timestamp instead of stopping at the first
  line past `--to`. `~/.factory/logs/droid-log-single.log` is not time-ordered (lines up to 48 s behind an earlier
  one), so the old stop dropped in-window turns and the cross-check against `scripts/usage-from-log.sh` read DIFFERENT
  (scrutiny reproduced 3 against 6 turns on session 2036e3c5). A line past `--to` is now skipped; the scan stops only at
  a line more than `LOG_REORDER_MARGIN_S` (600 s) past it. A line without a timestamp counts with the timestamped line
  before it. Real M2 window (`--from 2026-10-24T04:27:54Z --to 2026-10-24T19:36:40Z`): `cross-check: identical (97
  sessions ...)`, orchestrator row unchanged (251 turns, 188317 input, 151058 output, 17784767 cache read, 118.84
  USD-eq). Real M3 window (`--from 2026-10-24T21:48:56Z --to 2026-10-25T14:10:37Z`): `cross-check: identical (33
  sessions ...)`, `re-issued waits: 0`. Test `test_a_line_logged_out_of_order_at_the_window_end_is_still_counted` over
  `scripts/tests/fixtures/usage/logs-out-of-order/`.

## Decisions

none

## BACKLOG

- B-NEW: the 600 s stop margin of `usage-report.py` is a guess at the log's flush lag; the largest backwards step
  measured is 48 s. Reason: a line more than 10 minutes behind the window end would still be dropped; drop the early
  stop if a full scan of the rotated logs stays fast (the M2 window took 15.8 s with it).

## Deferred clauses

none

## Library

- `library/tooling.md`, "Usage report": the Droid log is not time-ordered; both `usage-report.py` and `usage-from-log.sh`
  filter per line by timestamp, so a window total never depends on line order. Cross-check after any change to either.
