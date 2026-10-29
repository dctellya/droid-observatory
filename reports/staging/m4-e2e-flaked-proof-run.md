# Staging: m4-e2e-flaked-proof-run

Feature `m4-e2e-flaked-proof-run`, worktree `observatory-wt/m4`, branch `wt/m4`. Split of `m4-e2e-keep-run-records`, whose one real
induced run ended `failed` on a loaded machine. Rule 30 claims: none to claim (`fulfills` is empty); evidence is the red commit
`33976bd`, the green commit `074d9f2` and the one real induced run below.

## SPEC §9

- Row (F-NEW), tooling proof (rule 26, 2026-10-29). The flaked outcome is proven on the real wdio reporter: one
  `E2E_INDUCE_FAILURE=panel make e2e` on a quiet machine (1-minute load 3.28, no cargo, trunk or wdio process) took `real 2m5.402s`
  and ended `e2e: flaked (first attempt red, rerun green): specs/panel.e2e.ts; rerun 6.15 s of 99.96 s`; run directory
  `reports/e2e/2026-10-29/162525/` (`record.json` outcome `flaked`, attempts 93.81 s then 6.15 s; `junit.xml` root `outcome="flaked"`,
  `grep -c '<failure'` = 0, parses with `/usr/bin/python3` minidom). `usage-report.py` prints
  `e2e flake: 2026-10-29/162525: specs/panel.e2e.ts; rerun 6.15 s (first attempt 93.81 s)`. The earlier failed run
  `reports/e2e/2026-10-29/161511/` (outcome `failed`) is whole beside it: two runs one day, nothing deleted. In `junit.xml` each
  suite replaced by its rerun carries `rerun_outcome` (`passed` or `failed`, read from the rerun suite itself) and
  `first_attempt_failures`; before, the property copied the run outcome, so a failed run labelled green-on-rerun suites `failed`.
  The `e2e.yml` artifact upload excludes `reports/e2e/latest/**` (the symlink would upload the newest run twice).
  `make check-contract`: 22 ok, 9 skipped, 0 FAIL in 5.8 s.

## Decisions

None.

## BACKLOG

None.

## Deferred clauses

None.

## Library

- A flaked proof needs a quiet machine: wait until the 1-minute load is under 4 (it fell from 8.83 to 3.28 in about two minutes after
  the earlier workers ended), then one `E2E_INDUCE_FAILURE=panel make e2e` is about 2 min wall (attempt 1 about 94 s, rerun about 6 s).
- junit.xml per-suite property is `rerun_outcome`, not `outcome`; the run outcome is only the root `outcome` attribute.
