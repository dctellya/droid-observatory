# Staging: m4-tooling-fixes

Feature `m4-tooling-fixes` (owner Q8 of the M3 boundary; tooling quirks of `MD/library/tooling.md`), worktree `observatory-wt/m4`,
branch `wt/m4`.

## SPEC §9

- Row (F-NEW), tooling change (rule 26, re-verified on 2026-10-29 by running the two scripts' own self-tests): the three quirks seen at
  the M3 integration are fixed, test first. (1) `scripts/integrate-staging.py` validates every staging file before it plans or writes
  anything: each `##` heading that is not one of the five sections is reported as `file:line` for all files in one pass and the script
  exits 2; `--check` validates only and `--rev REV` reads the files of a git revision, which `scripts/integrate.sh` runs on every
  branch still to merge before the first `git merge` (exit 2, nothing merged, no gate run). The comma marker forms `Row (F-NEW),`
  and `B-NEW,` are dropped like the colon form. (2) The `scripts/usage-report.py` Gates table takes a run when `started_at` < `--to` or
  `finished_at` <= `--to` and it did not finish before the window start (a run that straddles either edge is in); the table's caption
  `Gate window rule:` states it. (3) `re-issued waits` counts `WaitForScript` calls (any `action` but `kill`) on a `toolCallId` already
  waited on; kills are the separate row `worker pauses (kills)`; both rows carry their definition. Real M3 window (`--from
  2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z`): `re-issued waits: 0`, `worker pauses (kills): 1`, `WaitForScript calls in the window: 1`
  (the 15:35:59Z kill); the same window ended at 19:55:00Z reads kills 2 (the owner's "2 in the log") and lists the integrator's two full
  gate runs (`0c7b6f23` 195.592 s, `a3adb9b3` 85.676 s; 19 gate runs against 13). Evidence: `scripts/tests/integrate_test.sh` (red
  `c81d76e`: 15 failed; green `141bfab`: all passed, `make check-integrate` 18.051 s) and `scripts/tests/usage_test.py` (red `51dfa38`:
  failures 3, errors 3; green `5f23cbc`: 38 tests OK, 1.557 s). **Alternatives considered:** keep exit 1 with `unfiled: N` for a stray
  heading (rejected: it printed no line number and the merge had already run); reject only headings that hold bullets (rejected: an
  empty stray heading is still a mis-filed section).

## Decisions

None.

## BACKLOG

- Closed: the `scripts/integrate-staging.py` quirk "a stray `##` heading inside a staging file stops the merge after earlier files were
  merged (`unfiled: N`)" (`MD/library/tooling.md`, integration quirks of the M3 boundary): every file is validated before any merge step,
  exit 2 with `file:line`.
- Closed: the `Row (F-NEW),` comma marker quirk of `scripts/integrate-staging.py` (the marker stayed in the SPEC row text).
- Closed: the usage report quirk "the Gates table counts records with `started_at <= --to`; the integrator's own gate runs are not in the
  boundary report's table": the table follows the overlap rule in its caption; a run that started at or after `--to` is still outside, so
  the integrator passes no `--to` earlier than its own gate (`scripts/integrate.sh` passes none, the default is the generation time).
- Closed: the usage report quirk "`re-issued waits` counts every WaitForScript tool_use (a `kill` counts)": repeats and kills are two rows.

## Deferred clauses

- The orchestrator's brief says a gate record "whose started_at is after the window end but finished within it" must appear; a record
  cannot finish before it starts, so no such record exists. The rule is implemented as written (`started_at` < `--to` or `finished_at` <=
  `--to`, plus the window-start guard), which includes the straddling runs and a record without `started_at`; the integrator's gate runs
  of M3 appear only when `--to` is at or after their start (shown above with `--to 2026-10-25T19:55:00Z`).

## Library

- `tooling.md`, integration quirks section: replace the three M3 quirk bullets with the as-built facts: `python3 scripts/integrate-staging.py
  --check [--rev <branch>]` (exit 0 valid, exit 2 with `file:line`); the marker forms accepted are `Row (F-NEW):`, `Row (F-NEW),`, `(F-NEW)`,
  `F-NEW` and `B-NEW:`/`B-NEW,`/`New:`; the usage report's wait rows are `re-issued waits` (repeat on one `toolCallId`) and `worker pauses
  (kills)`, both with their definitions on the line.
- The usage fixture transcript `scripts/tests/fixtures/usage/sessions/-home-user-proj/11111111-....jsonl` now ends with one `kill` call
  (`toolu_k1`, 10:08:30Z) so the fixture reads re-issued waits 1, kills 1, calls 3.
