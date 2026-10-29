# Integrate: the long work

**When:** a milestone boundary. `make integrate` lands the merge, the gate, usage and the staging
files. The slow checks that follow, line coverage, the warm build timings and the CI run on the
pushed head, are one target: `make integrate-long`.

## Steps

1. `make integrate M=<m> FROM=<branch>` (README "Running a boundary"). The summary ends with
   `long work: not run ...` and the boundary report has a `## Long work` section saying it is
   pending.
2. Push the mission branch. The `ci` job waits for the run of the pushed head and starts none.
3. Start the long work as a background job and end the turn. Do not tail the log, `sleep` or
   check on it: the completion arrives as a message with the result table and the last 20 lines
   of every failed job.

   ```sh
   make integrate-long M=<m>      # as a background Script (startInBackground)
   ```

4. When the message arrives, run `make integrate M=<m> FROM=<branch>` again. The merged branch is
   skipped, the gate is a hit, and the report and the summary read the result file. The refresh
   commits `reports/integrate-long/<code_tree>.json`.

`INTEGRATE_LONG=1 make integrate ...` runs the long work inside the integrate target; use it only
when the branch is already pushed, since it blocks for the length of the coverage run.

## What runs

`scripts/integrate-long.sh --list` prints the job set.

| Lane | Job | Command |
|---|---|---|
| `cargo` | `coverage` | `make coverage` (cargo llvm-cov of observatory-core, alone on the cargo lock) |
| `cargo` | `timing-check`, `timing-test` | `cargo check -p observatory-core`, `cargo test -p observatory-core`, after coverage |
| `cargo` | `timing-trunk` | `trunk build`, only when the UI crate and trunk exist |
| `network` | `ci` | `gh run watch` on the CI run that proves the head's code tree |

Lanes run in parallel and the jobs of a lane run in order. The `ci` job takes the newest run whose
head is `HEAD`, else the newest run whose head has the same code tree (a push of reports or docs
only starts no run). It exits 3 when there is none: push, then start the target again.

## The result file

`reports/integrate-long/<code_tree>.json`, keyed by the code tree of `scripts/gate.sh --key`: the
exit code, wall time, log path and whether it was reused, per job, and the worst exit code, which
is also the exit code of the target. The schema is in the header of `scripts/integrate-long.sh`.
Logs are in `reports/integrate-long/<code_tree>.logs/` and stay local (`*.log` is ignored).

An unchanged code tree is a HIT: nothing runs and the target returns the stored exit code at once.
After a red run only the failed jobs rerun. `make integrate-long M=<m> FORCE=1` reruns all.

## Red

| Job | First step |
|---|---|
| `coverage` | the `TOTAL` line is in the message; the floor is `COVERAGE_MIN` in the Makefile; see [red-gate.md](red-gate.md) |
| `timing-*` | a compile or test failure on the merged tree: fix with a new commit on the mission branch |
| `ci` | `gh run view <id> --log-failed`; see [red-gate.md](red-gate.md) and [fix-cycle.md](fix-cycle.md) |

A red result fails `make integrate` after the boundary commit; it never stops the boundary
report. Fix with a new commit (forward only) and start the target again.
