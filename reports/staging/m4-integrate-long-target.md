# Staging: m4-integrate-long-target

Feature `m4-integrate-long-target` (owner item 16c), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW), rule 28 amendment (owner item 16c): the integration's long work is one target, `make integrate-long M=<m>`
  (`scripts/integrate-long.sh`). Jobs run in lanes, lanes in parallel, the jobs of a lane in order: lane `cargo` is
  `coverage` (`make coverage`, so alone on the cargo lock and cached by `scripts/gate.sh`), then the warm timings
  (`cargo check -p observatory-core`, `cargo test -p observatory-core`, `trunk build` when the UI crate and trunk exist); lane
  `network` is `ci`, which finds the `mission-branch.yml` run of the head (or of an earlier commit with the same code tree, since a
  report- or docs-only push starts no run) and follows it with `gh run watch --exit-status`. One result file,
  `reports/integrate-long/<code_tree>.json` (schema in the script header): per job the exit code, wall time, log path and
  whether it was reused; the wall time of the run, the worst exit code, a fingerprint of code tree, toolchain and job set. The
  target exits with the worst code. A record with exit 0 and the same fingerprint is a HIT that runs nothing; after a red run
  only the failed jobs rerun; `FORCE=1` reruns all. It is started as a background job and its completion is the message (the
  result table and the last 20 lines of every failed job's log), so nothing is tailed or polled (item 16 (b), (c)). Evidence:
  `scripts/tests/integrate_long_test.sh` (red `5ff494a`, green `29277ad`; 59 checks: parallel lanes by overlapping timestamps, lane order,
  worst exit code, HIT under 2 s, docs commit keeps the HIT, code commit loses it, rerun of failed jobs only, interrupt writes
  no record, stub-`gh` CI cases).
- Row (F-NEW), owner item 16c: `make integrate` reads the result for the merged code tree into the boundary report (`## Long
  work`: the job table and the gate summary of each job that has one, e.g. the coverage `TOTAL` line) and into the summary
  line `long work: OK|RED|not run`; `INTEGRATE_LONG=1` runs it inside the target (branch pushed first); a pending result does
  not stop the boundary, a red one fails the target after the boundary commit; `reports/integrate-long/` is exempt from the
  clean-tree check so the second `make integrate` (the refresh after the background job) runs. Evidence:
  `scripts/tests/integrate_test.sh`, the four `long work` scenarios (red `53c4cf2`, green `46fe0c4`). Runbook:
  `docs/runbooks/integrate-long.md`.
- Row (F-NEW), fix to F-NEW (gate key, `m4-gate-key-and-ci-paths`): `scripts/gate.sh --diff <rev>` printed nothing and exited 2 when
  an excluded file (a docs file) differed between `<rev>` and HEAD, because `git rm --cached` refuses a path whose index copy
  differs from both the file and HEAD; an empty diff reads as "same code tree", so `make integrate`'s miss reason lost its file
  list and the new `ci` job would have accepted any run. The cached removals are forced (`-f`). Evidence: `scripts/tests/gate_test.sh`,
  scenario `--diff <rev> where an excluded file differs` (red `8532b69`, green `29277ad`).

## Decisions

None.

## BACKLOG

- Closed (do not file): owner item 16c, the long work of an integration as one background target with one result file keyed by
  the code tree (`m4-integrate-long-target`).

## Deferred clauses

- No live run of the default job set was made (rule 23: no `cargo llvm-cov` inside a feature). The default set, its lanes and the
  `ci` job against the real `gh` are covered by the fake-job and stub-`gh` self-test only; the first real run is the M4 integration.
  `gh run list --json databaseId,headSha,status,conclusion` and `gh run watch <id> --exit-status --interval N` are the two calls
  made; the field names are those of the installed `gh` (checked with `gh run list --help`).

## Library

- `scripts/integrate-long.sh --list` prints the job set as `lane<TAB>name<TAB>command`; `INTEGRATE_LONG_JOBS=<file>` replaces it
  (the self-test's fake job set). `--ci-wait` is the `ci` job on its own and can be run by hand after a push.
- The thresholds of the CI wait (`CI_FIND_SECONDS` 120, `CI_FIND_INTERVAL` 10, `CI_WATCH_INTERVAL` 15, `CI_LIST_LIMIT` 20,
  `CI_MATCH_LIMIT` 8) are variables at the top of the script, overridable from the environment.
- The result file is committed by the boundary commit (`git add -A`); the logs are `*.log` and git-ignored, so their paths in the
  JSON are local to the integrator's machine.
- Integrator flow: `make integrate`, push, `make integrate-long M=<m>` as a background Script, on the message `make integrate`
  again (gate HIT, merged branches skipped) to refresh the report; no turn is spent while it runs.
