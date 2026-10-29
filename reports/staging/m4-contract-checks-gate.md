# Staging: m4-contract-checks-gate

Feature `m4-contract-checks-gate` (owner item 14b, first half), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW), rule 28 amendment (owner item 14b): the mechanical clauses of the pending contract assertions are gate checks.
  `scripts/contract-checks/g<N>-<slug>.sh`, one per proposed check of `contract-work/pass26-mechanical.md`, each printing one
  line `<check> <ok|FAIL|skip (<why>)> <assertion ids> [pending: ...]` and exiting 0 or 1; `make check-contract` runs the 31
  scripts in parallel (`run-all.sh`, 4.4 s without `GH_TOKEN`, 4.7 s with it), `make check` runs it after the cached gate with
  `CONTRACT_REQUIRE_TESTLOG=1`, and the `contract` job of `mission-branch.yml` (in `mission-checks.needs`, required by
  `check-ci-jobs.sh`) runs it in CI. Validators read the check's line instead of grading a `Kind: mechanical` clause. A clause
  whose code has not landed is `pending`, never red: `g2-tests.tsv` rows carry `landed=no` until the test is written, then flip
  and the test must match its body patterns and read `ok` in the gate's test log. `scripts/gate.sh` now writes
  `reports/gate/<code tree>/test.tests` beside the `test` record (`R <binary>`, then `ok|FAILED|ignored <name>`, 463 lines on
  this tree). G7 (`gh`) checks print `skip (no network)` without `GH_TOKEN` or `GITHUB_TOKEN`. G8 and G10 are gate-record stages,
  documented in `scripts/contract-checks/README.md`, not scripted. Evidence: `scripts/tests/contract_checks_test.sh` (every
  check green on the tree and red on a planted violation; red commits `fee904e`, `81aebaa`, `6fc0bc4`, `8b02edd`, `021ff58`,
  `3a4b66d`, `c6fe9f4`; green `245b0a5`, `b429a66`, `27399d7`, `3735379`, `dcb6cbe`, `9cf927f`, `4c8fa50`).
- Row (F-NEW), fix to the contract text of VAL-PROC-008: the clause's `grep -vc` over `git log --format='%H %(trailers:...)'`
  also counts the blank lines of multi-line trailer output (366 false hits on this branch); `g6-commit-format` reads the trailer
  per commit instead.

## Decisions

None.

## BACKLOG

- Closed (do not file): owner item 14b, first half (mechanical clauses become `make check` scripts); the second half (the
  UI-kit lint `make check-kit`) is `m4-ui-kit-lint-gate`.
- New: a boundary script `scripts/check-boundary-files.sh M=<m>` for the G10 clauses (reports, shared-file fields at an
  integration commit), run by `make integrate`; the checks are specified in `scripts/contract-checks/README.md` and the
  pass-26 table.
- New: a post-run script for the G8 clauses (app log greps and scratch-store counts), owned by the recorded live harness
  (owner item 12c).

## Deferred clauses

Assertions covered by the gate (check, ids). Validators read the check's line for these clauses:

| Check | Assertions covered |
|---|---|
| `g1-no-literal-main` | VAL-GATE-011 (mechanical half; pending until `ingest/github.rs` and `forge.rs` exist) |
| `g1-no-check-runs` | VAL-GATE-013 (source half) |
| `g1-integrated-wording` | VAL-METRIC-017 (wording; pending until the Details string exists) |
| `g1-no-python-deps` | VAL-TOOL-005 (grep half) |
| `g2-gate-tests` | VAL-GATE-006, VAL-GATE-019 and VAL-GATE-022 (test halves; all pending) |
| `g2-metric-tests` | VAL-METRIC-003, -005, -006, -007, -023, -024 (pending), VAL-METRIC-022 (mapping binary, live) |
| `g2-telemetry-tests` | VAL-TELEM-001, -002, -003, -006 (pending), VAL-TELEM-005 (live) |
| `g3-metrics-contract` | VAL-METRIC-001 (header), -002, -004, -006, -007, -009 (pending until `observatory-cli metrics`) |
| `g4-demo-gate-workflow` | VAL-GATE-003 |
| `g4-e2e-workflow` | VAL-E2E-006 (workflow clauses) |
| `g4-hooks-json` | VAL-PROC-022 (hooks.json clause) |
| `g5-frozen-fixtures` | VAL-METRIC-022 (hashes, mapping sources), VAL-PROC-006 (m5 row) |
| `g5-metrics-untouched` | VAL-PROC-006 (m4 row) |
| `g5-fixture-secrets` | VAL-PROC-007 (fixtures clause) |
| `g6-commit-format` | VAL-PROC-008 |
| `g6-no-history-rewrite` | VAL-PROC-020 (reflog clause) |
| `g6-hooks-config` | VAL-PROC-022 (git clauses) |
| `g6-worktree-layout` | VAL-PROC-001 |
| `g6-merge-shape` | VAL-PROC-010 |
| `g6-shared-files` | VAL-PROC-033 |
| `g7-demo-base` | VAL-GATE-001 |
| `g7-demo-ruleset` | VAL-GATE-002, VAL-GATE-031 (ruleset name) |
| `g7-demo-red-pr` | VAL-GATE-020 (gh clause; skips without a demo run) |
| `g7-demo-branches` | VAL-GATE-030 (README half pending until the M4 integration) |
| `g7-main-untouched` | VAL-PROC-021, VAL-PROC-006 (main-gate byte identity against `main-gate.json`) |
| `g7-no-force-push` | VAL-PROC-020 (activity clause) |
| `g7-no-backlog-issues` | VAL-PROC-026 |
| `g7-ci-run` | VAL-PROC-009 (skips when HEAD has no run) |
| `g7-mission-pr` | VAL-PROC-036, VAL-PROC-037 (skips until the PR exists) |
| `g9-check-target-list` | VAL-PROC-003 (target list) |
| `g9-make-targets` | VAL-PROC-017, VAL-GATE-031 (index), VAL-PROC-024, VAL-PROC-025 |

Clauses of the pass-26 table not covered here, each with its reason (the validator or the integrator keeps them):

- Unnamed unit tests, no name to pin: VAL-METRIC-008, -009 (enum lists), -010, -020, VAL-TELEM-004, -007, -009. Add rows to
  `g2-tests.tsv` when the worker names the tests.
- Need data the CLI does not produce yet: VAL-METRIC-011 (hand recomputation), VAL-METRIC-021 (re-run diff),
  VAL-METRIC-023 sum rule and `$16` integer check beyond the column rule, VAL-METRIC-024 sums, VAL-CROSS-004, VAL-CROSS-005
  (`jq` over the recorded runs, gate record). The column rules of VAL-METRIC-023 (`throttled_minutes` an integer) are in
  `g3-metrics-contract`.
- VAL-PROC-002 (test-first order and the `FAILED` line in `build-log/`): needs each worker's named-test list; not scripted.
- VAL-PROC-023, VAL-TOOL-006 (1) to (4), VAL-PROC-047 to -050: integration-time fields (G6 rows at the integration commit, G10).
- G8 and G10 as a whole: gate-record stages, see `scripts/contract-checks/README.md`.
- VAL-PROC-003's `exit_code` and VAL-PROC-004 (`TOTAL` line) are read from `reports/gate/` and `reports/integrate-long/` records.
- VAL-PROC-006 (s, m0, m1, m2, m3 rows) are historical and already integrated.
- VAL-PROC-008 contract text: the `grep -vc` trailer form miscounts blank lines (see the SPEC row); the check reads trailers
  per commit. VAL-PROC-033 is red on `wt/m4` as the contract is written (six commits touch `reports/usage`, `reports/ci`,
  `BACKLOG.md`, `README.md` or the decisions index by orchestrator direction); `exceptions.txt` lists the six and the four hook
  commits of VAL-PROC-022 with reasons. The orchestrator ratifies or removes them.

## Library

- Run the checks in the inner loop with `make check-contract`; add `GH_TOKEN=$(gh auth token)` for the `g7-*` checks.
- `scripts/contract-checks/lib.sh` is the shared helper (`fail`, `pending`, `skip`, `finish`, `need_tool`, `need_network`,
  `range_base`, `excepted`); `CONTRACT_ROOT` points any check at a scratch tree.
- A new named test for a pending assertion: flip its `landed` column in `g2-tests.tsv` in the same commit that adds it.
- `git-baseline.txt` is the mission-branch tip the m4 range starts from (`3ccf3aa`); move it at a boundary. `main-gate.json` is the
  pinned `gh api repos/dctellya/droid-observatory/rulesets/25636994` read; `frozen-hashes.txt` is the M0 fixture and mapping pin.
- In CI the checks run with `CONTRACT_TESTLOG=none` (presence only), no token (gh checks skip) and `CI` set (a tool the runner
  lacks, such as `actionlint` or `gitleaks`, skips the check; the `workflows` and `secrets` jobs cover them).
