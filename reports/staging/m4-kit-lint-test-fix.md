# Staging: m4-kit-lint-test-fix

Feature `m4-kit-lint-test-fix`, worktree `observatory-wt/m4`, branch `wt/m4`. Rule 30 claims: none to claim (`fulfills` is empty);
evidence is the red commit `08f81bb`, the green commit `0d094fd` and the runs below.

## SPEC §9

- Row (F-NEW), tooling and test policy (rule 26, measured on 2026-10-29). `scripts/tests/kit_lint_test.sh` failed 4 asserts
  (`pend1`, `pend2`) and so did `make check-scripts`. Measured with `scripts/tests/kit_lint_test.sh` in a throwaway
  `git worktree` per commit: `2b3c492^` (726493f) exit 1 with the 4 failures; `e821ff8^` (a5efd55) exit 1 with the same 4;
  `4723ad8^` exit 0; `4723ad8` (`feat(app): gate group, return arc, page head, kit fixes`, m4-gate-group-ui) exit 1. The
  breaking commit is `4723ad8`, not `e821ff8` (a staging-only commit): it fixed the five heading-count spans and emptied
  `kit-pending.txt` to comments, and the two test cases planted their violations through those five lines. The lint
  (`scripts/check-kit.py`) did not regress: it still fails an unrecorded violation (an unlisted sans number cell in panel.rs, an unlisted
  padding or width override, a stale departure row) and a listed line that no longer violates; the test was stale. The fix
  removes `pend1` and `pend2` and adds `pend3` (an unlisted planted violation is red and prints its pending key), `pend4` (the
  same violation listed in `kit-pending.txt` is `check-kit ok`), `pend5` (a listed line that no longer violates is red naming
  `kit-pending.txt`) and a check that the real `kit-pending.txt` holds comments only. `4723ad8` ran no `make check-scripts`;
  the gap lasted from it to the first worker that ran the target (ecf45169). **Alternatives considered:** restoring a
  pending line to keep `pend1`/`pend2` (rejected: the list is empty on purpose, owner answer Q4); changing the lint (rejected:
  it was right). Measured: `make check-scripts` exit 0 in 1m32.844s, `make check-contract` exit 0 in 5.012s (22 ok, 9 skipped,
  0 FAIL).

## Decisions

## BACKLOG

## Deferred clauses

## Library

- A feature that empties or edits `kit-pending.txt` or `kit-departures.md` runs `sh scripts/tests/kit_lint_test.sh` (2.4 s), part
  of `make check-scripts`, before handoff.
