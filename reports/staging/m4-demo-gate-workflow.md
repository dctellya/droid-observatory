# Staging: m4-demo-gate-workflow

Feature `m4-demo-gate-workflow` (slice of `m4-demo-branch-pair`, owner items 20 and 21), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW), `.github/workflows/demo-gate.yml` (commit `1d994af`): `pull_request` to `demo/base`, types `opened`, `synchronize`,
  `labeled`, `unlabeled`; `permissions: contents: read`; one job `demo-gate` (`timeout-minutes: 2`) that exits 1 when
  `github.event.pull_request.labels` holds `demo:red`; a concurrency group per PR number with `cancel-in-progress`, so the newest label
  change owns the check. Red test `084ad15` (`scripts/check-ci-jobs.sh` exits 1, workflow missing), green `1d994af` (exit 0).
- Row (F-NEW), smoke proof on throwaway PR #24 (`demo/smoke-1`, head `1d994af0`): labeled `demo:red` at 2026-10-29T09:58:31Z, run
  `40853828900` failure; unlabeled at 09:59:02Z, run `40853894595` success. `opened` run `40853828337` was cancelled by the concurrency
  group because the `labeled` event arrived in the same second. PR closed, `demo/smoke-1` deleted (ref API 404 afterwards); `demo/base`
  stays. `main` and `main-gate` untouched. Proof file: `reports/ci/2026-10-29-demo-gate-smoke.md`.
- The B-056 trim and the second warm measurement were already landed by `m4-gate-key-and-ci-paths` (`88ffd91`, run 40851767262
  attempt 2 = 144 s); nothing to redo here.

## Decisions

- None. D-040 (demo branch pair) already covers the workflow; no amendment needed.

## BACKLOG

- B-007: the workflow and smoke PR parts are done; with `m4-demo-branch-and-ruleset` the item can close.

## Deferred clauses

- `demo/base` still holds no `demo-gate.yml` (cut from the mission branch before the M4 worktree). A PR into `demo/base` runs the
  workflow from its own merge commit, so any head that carries the file satisfies the required check; a head without it can never go
  green. The real demo PRs (the gate-lane e2e) must branch from `wt/m4` or later, or `demo/base` must be advanced after the integration
  (a push by the owner or the orchestrator, not a worker).
- `statusCheckRollup` of a PR keeps every run of `demo-gate` (cancelled, failure, success) in order; the latest per name decides the
  merge gate. The app's gate lane must read the latest run per check name, not the rollup as a set.

## Library

- `gh run list` right after a label change can return the old runs only; the new run appears within seconds. A single `gh run list`
  followed by `gh run watch <id> --exit-status` (one blocking call) avoids polling.
- Creating a PR with `--label` fires `opened` and `labeled` in the same second; with the concurrency group the `opened` run ends
  `cancelled` and the `labeled` run carries the verdict.
