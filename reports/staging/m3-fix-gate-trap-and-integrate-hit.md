# Staging: m3-fix-gate-trap-and-integrate-hit

Feature `m3-fix-gate-trap-and-integrate-hit` (fix after scrutiny-validator-m3 round 1; VAL-TOOL-001, VAL-TOOL-006,
VAL-PROC-047), worktree `observatory-wt/m3`, branch `wt/m3`. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): an interrupted gate run is not a verdict. `scripts/gate.sh` handles HUP, INT and TERM by killing the
  command's process tree, removing its temp files and exiting 129, 130 or 143 before any record is written; the trap
  used to remove the temp files and let the script go on, so an interrupted run stored an exit 1, "(no output)" record
  that later runs replayed as a hit. The command now runs in the background and the script waits on it, because a
  shell defers a trap until its foreground pipeline ends. `scripts/tests/gate_test.sh` covers TERM and HUP (exit code,
  no record, command killed, no temp file left); it passes under `sh` and `dash`.
- Row (F-NEW): `scripts/integrate.sh` decides HIT or MISS from the check gate's own first line (`cached, proven at`
  against `gate check: miss|forced`), not from a grep over the whole `make check` output, which the inner lint and
  test gate hits also print. A real check run whose inner gates hit is now `gate: MISS` with a reason; with an empty
  diff the reason says that no check record exists for the tree. `scripts/tests/integrate_test.sh` has the inner-hit,
  outer-miss scenario.
- Row (F-NEW): `scripts/integrate-staging.py` files every bullet and table row under `## SPEC §9` (also `SPEC §9 rows`,
  `SPEC s9`), `## BACKLOG` and `## Decisions`. A SPEC bullet is a row (a `Row (F-NEW):` marker is dropped); a BACKLOG
  bullet is the next B-row (`B-NEW:` or `New:` optional) unless it is a closure (`Closed...`, `Close ...`,
  `B-NNN: done`), which goes to the report; a Decisions bullet naming a D-NNN is an addendum of that file (the text
  after the `D-NNN (...)` prefix), unless it says the decision is new or that there is no amendment, which goes to the
  report with the bullets that name no decision. Deferred clauses and Library are copied per feature. A bullet or
  paragraph under any other `##` section, or an addendum whose decision file does not exist, makes the script print
  `unfiled: N` with the items, exit 1 and write nothing, so the staging file is fixed and the run repeated.
- Row (F-NEW): `integrate-staging.py --dry-run DIR` merges into copies of SPEC.md, BACKLOG.md and docs/decisions under
  DIR, keeps the staging files and writes DIR/report.md; it refuses a DIR that would overwrite a shared file. Dry run
  over copies of the 11 pending M3 staging files (everything under `reports/staging/` except the merged
  `m3-boundary-docs-m2`): SPEC §9 rows 42 (F-071 to F-112), BACKLOG rows 27 (B-078 to B-104), closures 4, decision
  addenda 5 (D-029 twice, D-033, D-034, D-036), decision notes 6, prose notes 3, deferred clauses 20, library 30,
  `unfiled: 0`. The integrator numbers the rows again when this file joins the batch.

## Decisions

none

## BACKLOG

- B-NEW: the interrupted-run handler of `scripts/gate.sh` kills the command's descendants with `pgrep -P`; a command
  that double-forks away from its parent survives. Reason: no gate command does that; a process-group kill would also
  stop the `make` that started the gate.
- B-NEW: a SPEC §9 bullet becomes one table row however long it is (the M3 staging rows run to several hundred
  characters). Reason: shortening a row is the integrator's judgement; add a length warning if the table is to stay
  readable.
- B-NEW: a BACKLOG bullet with no `reason:` is split at its first sentence and the rest becomes the reason, or "no
  reason stated"; five of the 27 dry-run rows (B-078, B-079, B-087, B-088, B-103) have no stated reason. Reason: the staging
  convention (`B-NEW: item; reason: why`) is not enforced; the integrator may reword the rows by hand.

## Deferred clauses

- The real `make integrate M=m3` run is the first use of the new HIT/MISS decision and of the merger on the real
  staging files; here both ran on throwaway repositories (`scripts/tests/integrate_test.sh`) and on copies of the real
  staging files in a temporary directory (dry run).
- INT is handled in `gate.sh` (exit 130) but not exercised by the test: a background `sh` started by a script ignores
  SIGINT from the start, so only TERM and HUP are sent.

## Library

- `library/tooling.md`, "Gate cache": an interrupted run (HUP 129, INT 130, TERM 143) leaves no record and kills its
  command; a shell runs a trap only after its foreground pipeline ends, so `gate.sh` runs the command in the background
  and `wait`s.
- `library/tooling.md`, "Integrate target": the staging merge accepts the section names `SPEC §9`, `SPEC §9 rows`,
  `SPEC s9`, `Decisions`, `BACKLOG`, `Deferred clauses`, `Library`; run
  `python3 scripts/integrate-staging.py --milestone <m> --date <d> --staging <dir> --dry-run <tmp>` to see the counts
  and `unfiled: N` before the boundary; the repository files stay untouched.
