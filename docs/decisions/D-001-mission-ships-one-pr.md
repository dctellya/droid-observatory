# D-001: A mission ships one business feature through one pull request

**Date:** 2026-10-22
**Status:** accepted

## Context

The PRD v0.1 implied one PR per feature. Reviewing it against how the work is actually
gated (attested CI on the PR into `main`, CD from `main`), the unit that ships is the
mission, not the feature.

## Decision

- A mission is a business feature: the unit that ships. It ends in one PR into `main`.
- Milestones are internal iterations. Each runs in its own worktree and integrates into
  `mission/<name>`, which carries fast checks only and is never shippable.
- The mission branch dies at the merge.
- CI red on the PR reopens the mission on its branch (`fix` cycle); it never opens a new PR.
- On the board: cards are milestones, the lane-8 gate group is the mission PR.

## Alternatives

- One PR per feature: more gates, more waiting-on-merge waste, and features inside a
  milestone are not independently shippable anyway.
- Long-lived integration branch (`dev`): rejected; the mission branch's death at merge is
  what prevents it.

## Consequences

PRD v0.2 §2, §8, FR-G; `.github/workflows/mission-branch.yml` targets `mission/**`;
`gate.yml` targets PRs to `main`; the `main` ruleset (D-007) requires a PR.
