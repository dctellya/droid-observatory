# Staging: m4-demo-branch-and-ruleset

Feature `m4-demo-branch-and-ruleset` (slice of `m4-demo-branch-pair`, owner items 20 and 21), worktree `observatory-wt/m4`,
branch `wt/m4`.

## SPEC §9

- Row (F-NEW), the demo branch pair exists (SPEC §3 M4 test target, entry 19): `demo/base` was pushed on 2026-10-29 from
  `mission/observatory-v1` at `3ccf3aac518a327d3c73c03cbc9efdf77bbc1ce8` (`git merge-base --is-ancestor` exit 0 against
  `origin/demo/base`); label `demo:red` created; ruleset `demo-gate` id `26112925` (enforcement active, include
  `refs/heads/demo/base` only, no bypass actors; rules `pull_request`, `required_status_checks` with the single context
  `demo-gate`, `non_fast_forward`) created with `gh api` under the owner's login by the infra worker for the orchestrator.
  `main` and `main-gate` (id 25636994) were not touched. Decision file D-040.
- Row (F-NEW), the worktree took the mission branch's M3 boundary docs by a no-ff merge (`b82fac9`) so D-040 follows D-039.

## Decisions

- D-040 `docs/decisions/D-040-demo-branch-pair.md` (supersedes the private demo repository, B-007). The index is regenerated
  in the worktree; the integrator regenerates it again if the merge moves it.

## BACKLOG

- B-007 (demo branch pair in this repository): the `demo/base` and ruleset part is done; the workflow and the smoke PR belong to
  `m4-demo-gate-workflow`. Close the row when that feature lands.

## Deferred clauses

- Until `.github/workflows/demo-gate.yml` reaches a pull request head, no pull request into `demo/base` can satisfy the required
  check `demo-gate` (`m4-demo-gate-workflow`).
- `demo/base` was cut from the mission branch before the M4 worktree's commits; it holds no `demo-gate.yml`. The smoke PR's head
  carries the workflow file (`pull_request` runs read the workflow from the merge commit of the head).

## Library

- `gh api repos/dctellya/droid-observatory/rulesets` works with the owner's `gh` login (admin); the ruleset JSON used is the
  one in D-040's Decision section (parameters: `required_approving_review_count` 0, `strict_required_status_checks_policy` false).
  The API fills in `require_extra_approval_for_unattributed_changes: true` for `pull_request` by default.
