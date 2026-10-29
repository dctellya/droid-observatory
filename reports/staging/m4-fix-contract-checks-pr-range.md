# m4-fix-contract-checks-pr-range

## SPEC §9

- Rule 29 fix of the M4 scrutiny round 1 blocker on m4-contract-checks-gate: `lib.sh` `head_ref()` returns `HEAD^2` on a
  `pull_request` event when HEAD is the synthetic merge commit of GitHub, else `HEAD`; `range_base`, `g6-commit-format`,
  `g5-metrics-untouched` and `g6-shared-files` judge that ref. `g7-ci-run` lists only the `mission-branch.yml` push run.
  Self-test cases gc4 (merge over a clean PR head, ok), gc5 (PR head commit without the trailer, red) and gc6 (same merge
  without the event variable, red), plus the stubbed `gh` arguments case.
- `exceptions.txt`: feature ids of the rows for 253e95a and aa56596 corrected to `m4-demo-gate-workflow` and
  `m4-demo-branch-and-ruleset` (ids only; no row added or removed).

## Decisions

None.

## BACKLOG

- Persist the `make check-contract` output in the gate record.
- `yq` in `make doctor` and in SPEC `## Tooling`.
- g3-metrics-contract is `pending` until the metrics command lands (M5).
- `run-all.sh` prints `ok` for a check that exits non-zero.
- `g6-hooks-config` still ranges over `$base..HEAD`; on a pull_request checkout it only sees the merge commit.

## Deferred clauses

None.

## Library

- On a `pull_request` run `GITHUB_EVENT_NAME=pull_request` and HEAD has two parents; checks that range over commits use
  `$(head_ref)`. `g7-ci-run` on that checkout skips (the merge commit has no push run).
