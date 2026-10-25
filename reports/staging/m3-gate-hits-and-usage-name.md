# Staging: m3-gate-hits-and-usage-name

Feature `m3-gate-hits-and-usage-name` (fix after m3-gate-cache and m3-usage-target; VAL-TOOL-005, owner items 16
and 18), worktree `observatory-wt/m3`, branch `wt/m3`. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): the gate hit log. Each cache hit of `scripts/gate.sh` appends one tab-separated line to
  `reports/gate/hits.log`: RFC 3339 UTC time, name, tree, the stored `seconds` of the record, the session that was
  served, its `session_source`. A miss and a forced run append nothing. The minutes a hit saved are the stored
  seconds of the run it did not repeat. `.gitignore` stops ignoring the file (it ignored `*.log`) and
  `.gitattributes` merges it with `merge=union`, so two worktrees that each logged hits merge by keeping both sets
  of lines. `scripts/tests/gate_test.sh` covers the line, the miss, the failing miss, the forced run and `FORCE=1`.
- Row (F-NEW): `scripts/usage.py` is renamed `scripts/usage-report.py`, the name VAL-TOOL-005 and
  `library/tooling.md` use. `make usage M=<m>` and `make check-scripts` call the new name.
- Row (F-NEW): the Gates table of the usage report has one row per gate and one total row for gates run (records
  `reports/gate/*/*.json`: count, seconds, minutes, command, first and last time) and for gates served from cache
  (`hits.log`: count, minutes saved), each with its source file. The detailed run list is the `### Gate runs`
  table. The line `served from cache: not recorded` is gone; `gates served from cache: N, minutes saved: M` replaces
  it. Real run for the M3 window (`--from 2026-10-24T21:48:56Z`): 4 gates run (120.291 s), 1 served from cache
  (0.29 min saved), `re-issued waits: 0`.
- Row (F-NEW): reproducibility of the usage report. `Sidecar readings` states the number of sidecars read and their
  oldest and newest mtime, and the per-session table has a `Modified` column. `--help` states that a byte-for-byte
  rerun needs a `--to` at least 5 minutes before the generation time (log flush lag measured by m3-usage-target) and
  that Sidecar readings can move while a session is live. The `--to` default is unchanged.

## Decisions

- D-029 (amend, one phrase): the script is `scripts/usage-report.py` (already named so in the m3-usage-target
  staging file, which this feature corrected).

## BACKLOG

- Closed (do not file): the m3-usage-target row "`scripts/gate.sh` writes no record for a cache hit".
- B-NEW: a hit logs the serving session, the record keeps the proving session; if the scorecard wants hits by
  proving session too, add a seventh column. Reason: not asked.

## Deferred clauses

- m3-usage-target never committed `scripts/tests/fixtures/usage/logs/droid-log-single.log` (`*.log` was gitignored, so
  the tests passed in the worktree only). This feature un-ignores `scripts/tests/fixtures/usage/**/*.log` and commits it.
- `Command` of a hit row comes from the record `reports/gate/<tree>/<name>.json`; a hit whose record was replaced
  by a later run of another command on the same tree shows that command. Rare; not worth a command column in the log.

## Library

Lines for `library/tooling.md`, sections "Gate cache" and "Usage report":

- `reports/gate/hits.log`: `time<TAB>name<TAB>tree<TAB>stored seconds<TAB>session<TAB>session_source`, one line per
  hit, written by `scripts/gate.sh` after the cache check. The usage report reads it for gates served from cache and
  minutes saved. Committed with the records; merged by union.
- The script is `scripts/usage-report.py`. A byte-for-byte rerun needs `--to` at least 5 minutes before the
  generation time; Sidecar readings can move while a session is live and list the mtime of each sidecar read.
