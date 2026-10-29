# Contract checks

The mechanical clauses of the pending validation-contract assertions (`Kind: mechanical`, or the `mechanical:` part of a
`Kind: mixed` line) run here as shell checks. Source of the clause list: `contract-work/pass26-mechanical.md` in the mission
directory (69 clauses, groups G1 to G10). `make check-contract` runs every `g<N>-*.sh` in parallel (`run-all.sh`) and prints
one line per check plus a summary; `make check` runs it after the cached gate, and the `contract` job of
`mission-branch.yml` runs it in CI.

## Line format

```text
<check> <ok|FAIL|skip (<why>)> <assertion ids> [pending: <what the check waits for>]
```

A check exits 0 for `ok` and `skip`, 1 for `FAIL`; the reasons for a FAIL go to stderr. A validator reads the line for a
mechanical clause instead of grading it (owner item 14).

- `pending`: the clause's code has not landed (a test not written, a CLI command not built). The check stays green and says
  what it waits for. `g2-tests.tsv` carries a `landed` column: when the test appears, a `landed=no` row turns the check red
  until the row is flipped to `yes`, after which the test must match its body patterns and read `ok` in the gate log.
- `skip (no network)`: the `g7-*` checks need `GH_TOKEN` or `GITHUB_TOKEN`. Locally use
  `GH_TOKEN=$(gh auth token) make check-contract`; CI passes none.
- `exceptions.txt`: reviewed commits that predate a git-history check (`<check> <sha7> <reason>`); the list only shrinks.

## Environment

| Variable | Effect |
|---|---|
| `CONTRACT_ROOT` | Check this tree instead of the repository root (the self-test plants violations in scratch trees) |
| `CONTRACT_SKIP` | Space-separated check names to skip |
| `CONTRACT_TESTLOG` | `none` checks test names by presence only (CI); a path replaces `reports/gate/<code tree>/test.tests` |
| `CONTRACT_REQUIRE_TESTLOG` | `1` makes a missing test log a FAIL (`make check` sets it after `make test`) |
| `CONTRACT_REPO` | GitHub repository for the `g7-*` checks (default `dctellya/droid-observatory`) |
| `CI` | A tool the runner lacks skips the check instead of failing it |

## Checks and the assertions they cover

| Check | Assertions | What it reads |
|---|---|---|
| `g1-no-literal-main` | VAL-GATE-011 | no `"main"` literal in `ingest/github.rs` and `ingest/forge.rs` |
| `g1-no-check-runs` | VAL-GATE-013 | no `check-runs` or `check-suites` outside comments in the core sources |
| `g1-integrated-wording` | VAL-METRIC-017 | no `merged` in Details strings of the integration lane |
| `g1-no-python-deps` | VAL-TOOL-005 | `usage-report.py` imports no `requests`, `pandas` or `numpy` |
| `g2-gate-tests` | VAL-GATE-006, VAL-GATE-019, VAL-GATE-022 | the eight SPEC-named M4 tests and two view-level bodies |
| `g2-metric-tests` | VAL-METRIC-003, -005, -006, -007, -022, -023, -024 | the M5 engine tests and the frozen `mapping` test binary |
| `g2-telemetry-tests` | VAL-TELEM-001, -002, -003, -005, -006 | the telemetry tests |
| `g3-metrics-contract` | VAL-METRIC-001, -002, -004, -006, -007, -009 | `metrics --csv` and `--json` over the M0 fixture |
| `g4-demo-gate-workflow` | VAL-GATE-003 | `demo-gate.yml` fields and actionlint |
| `g4-e2e-workflow` | VAL-E2E-006 | `e2e.yml` job, pinned versions against the SPEC Tooling rows, steps |
| `g4-hooks-json` | VAL-PROC-022 | `.factory/hooks.json` names `guard-git.sh` and `log-tool-call.sh` |
| `g5-frozen-fixtures` | VAL-METRIC-022, VAL-PROC-006 | hashes in `frozen-hashes.txt` for the M0 fixture and the mapping sources |
| `g5-metrics-untouched` | VAL-PROC-006 | `wt/m4` leaves `crates/observatory-core/src/metrics` alone |
| `g5-fixture-secrets` | VAL-PROC-007 | `gitleaks detect --no-git` over `fixtures/streams` |
| `g6-commit-format` | VAL-PROC-008 | subject length, WIP markers and trailer of every commit the branch adds |
| `g6-no-history-rewrite` | VAL-PROC-020 | reflogs of the mission and worktree branches |
| `g6-hooks-config` | VAL-PROC-022 | `core.hooksPath` and hook-file history against decision files |
| `g6-worktree-layout` | VAL-PROC-001 | `observatory-wt/<m>`, branch `wt/<m>`, the `current` link |
| `g6-merge-shape` | VAL-PROC-010 | integrated `wt/*` tips are second parents of one first-parent merge |
| `g6-shared-files` | VAL-PROC-033 | a worktree branch leaves the integrator's files and SPEC rows alone |
| `g7-demo-base` | VAL-GATE-001 | `demo/base` exists and descends from the baseline |
| `g7-demo-ruleset` | VAL-GATE-002, VAL-GATE-031 | the `demo-gate` ruleset fields |
| `g7-demo-red-pr` | VAL-GATE-020 | exactly one PR for `demo/red-1` |
| `g7-demo-branches` | VAL-GATE-030 | `demo/base` is the only `demo/*` branch |
| `g7-main-untouched` | VAL-PROC-021, VAL-PROC-006 | `main` unchanged, `main-gate` ruleset equals `main-gate.json` |
| `g7-no-force-push` | VAL-PROC-020 | the GitHub activity log lists no `force_push` |
| `g7-no-backlog-issues` | VAL-PROC-026 | no backlog issue, label or `gen-backlog.sh` |
| `g7-ci-run` | VAL-PROC-009 | the CI run of HEAD, its `machete` and `coverage` jobs |
| `g7-mission-pr` | VAL-PROC-036, VAL-PROC-037 | the mission PR at the mission's end |
| `g9-check-target-list` | VAL-PROC-003 | `check-all` lists every named target |
| `g9-make-targets` | VAL-PROC-017, VAL-GATE-031, VAL-PROC-024, VAL-PROC-025 | exit codes of the three quick targets |

Already covered by an existing target, so not duplicated: VAL-PROC-025 (`make check-todos`), VAL-PROC-024
(`make check-file-size`), VAL-METRIC-022 replay line (`make replay-fixtures`), VAL-GATE-031 index (`make check-decisions-index`).

## Gate-record stages (not scripted here)

- **G8: counts over a recorded live run.** Greps and `sqlite3` counts over the app log and scratch store that the recorded
  live harness (owner item 12c) leaves on disk: VAL-GATE-007, -013 (log half), -014, -020 (store half), -025, -028, -029, -032,
  VAL-TELEM-007 (log half), VAL-PROC-005, VAL-PROC-006 (m1, `lsof`). The post-run script belongs to the harness that
  produces the run; its result is read from the gate record keyed by the tree.
- **G10: report and shared-file fields at an integration commit.** Read by `make integrate` from the committed reports
  and shared files: VAL-METRIC-001 (reference CSV tracked), -021, -023 (report halves), VAL-GATE-031 (decision file), -033,
  VAL-PROC-011 to -016, -018, -019, -023, -027 to -037, -046 to -050, VAL-CYCLE-005, VAL-TOOL-005 (re-run diff), VAL-TOOL-006.
  A boundary script reading them is future work (`scripts/check-boundary-files.sh`), see the staging file's deferred clauses.
