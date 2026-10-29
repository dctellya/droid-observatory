# D-040: The demo target is a branch pair in this repository, not a private demo repository

**Date:** 2026-10-29
**Status:** accepted; built in `m4-demo-branch-and-ruleset` and `m4-demo-gate-workflow` (SPEC §3 M4, B-007, SPEC §9 entry 19)

## Context

M4 watches a real pull request: a red required check, a fix on the same PR, a green check, a merge decided by a
ruleset. The observatory needs a repository where red and green are deterministic and where nothing can reach
`main`. The PRD first proposed a private demo repository. SPEC §9 entry 19 and BACKLOG B-007 replaced it with a
branch pair in this repository, so the app reads one repository, one token (read-only, D-014) and one set of
rulesets, and the mission creates and removes the test pull requests with the owner's `gh` login.

## Decision

This decision supersedes the private demo repository (B-007).

- **`demo/base`:** a long-lived branch created from the mission branch `mission/observatory-v1` at
  `3ccf3aac518a327d3c73c03cbc9efdf77bbc1ce8` on 2026-10-29 with
  `git push origin <sha>:refs/heads/demo/base`. It stays as test infrastructure.
- **`demo/*`:** short-lived branches, one per run (`demo/red-1`, `demo/smoke-1`), opened as pull requests into
  `demo/base` and deleted after the run, by `gh`, as cleanup. The app does not observe the deletion (F-028).
- **Ruleset `demo-gate`:** id `26112925`, target `branch`, enforcement `active`, no bypass actors, one include ref
  `refs/heads/demo/base`, no exclusions. Rules: `pull_request` (0 required approving reviews, the human merges by
  hand), `required_status_checks` with exactly one context `demo-gate` (not strict), and `non_fast_forward` (no
  force push). It was created on 2026-10-29 by the infra worker session of `m4-demo-branch-and-ruleset`, acting for the
  orchestrator, with `gh api repos/dctellya/droid-observatory/rulesets` under the owner's `gh` login. The owner did not
  have to create it at the M3 to M4 boundary: the login had admin.
- **Workflow `.github/workflows/demo-gate.yml`:** on `pull_request` to `demo/base`, types `opened, synchronize,
  labeled, unlabeled`, one job `demo-gate` that fails when the label `demo:red` is on the pull request and passes
  otherwise. Written by `m4-demo-gate-workflow`.
- **Label `demo:red`:** created on 2026-10-29 (colour `b60205`). Adding it turns the check red, removing it turns
  it green.
- **Untouched:** `main` and the ruleset `main-gate` (D-007). Merging a demo pull request into `demo/base` is the
  owner's, by hand: the GitHub token stays read-only (owner decision of the M3 boundary, 2026-10-25) and the app
  shows the 403 on Merge.

## Alternatives

- **A private demo repository** (the PRD's first proposal): a second repository, a second token or a wider one, and
  a ruleset the app could not be tested against without owner admin there. Rejected in SPEC §9 entry 19.
- **Pull requests into `main` with a red check:** touches the one branch the mission must not touch, and a red
  `gate` on `main` blocks the mission's own pull request.
- **A mocked GitHub API only:** deterministic, but it proves nothing about the real ruleset attribution
  (`decided_by`) or the 403 on Merge.
- **Legacy branch protection on `demo/base`:** superseded by rulesets, and the app reads rulesets (FR-G4).

## Consequences

- The `demo-gate` ruleset is the one the gate lane attributes the merge decision to in the M4 live checks
  (`decided_by` names `demo-gate` and id `26112925`).
- `demo/base` needs no CI of its own: the only required check is the one the workflow on each pull request
  produces. Until the workflow file reaches the pull request head, a pull request into `demo/base` cannot satisfy
  the required check.
- The ruleset can be inspected with `gh api repos/dctellya/droid-observatory/rulesets/26112925`. Editing it
  reopens VAL-GATE-002.
- `demo/*` branches left behind by an aborted run are the next run's first cleanup.
