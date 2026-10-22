# D-007: Ruleset on main

**Date:** 2026-10-22
**Status:** accepted

## Context

`main` is the gate the observatory watches. Its rules must be the ones the PRD's merge
policy reader (FR-G4) will find, and they must not hang the final PR.

## Decision

GitHub ruleset `main-gate`, enforcement active, target `refs/heads/main`:

- pull request required, **required approving reviews: 0** (the human merges by hand; the
  gate is the review), dismiss stale reviews on push, conversation resolution not required;
- required status check: **`gate`** (from `gate.yml`), strict (branch up to date);
- deletion blocked, non-fast-forward (force push) blocked;
- no bypass actors: admins included.

## Alternatives

Legacy branch protection (superseded by rulesets); admin bypass for the first PR (would
let the readiness PR skip the gate it introduces); requiring every job by name (D-003).

## Consequences

Applied with `gh api` on 2026-10-22 and recorded in `ai/session/`. Direct pushes to `main`
fail for everyone, including the owner; `.githooks/pre-push` and
`.factory/hooks/guard-git.sh` refuse them earlier.
