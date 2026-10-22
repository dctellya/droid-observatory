# D-005: Renovate with a 7-day minimum release age

**Date:** 2026-10-22
**Status:** accepted

## Context

Rule 3 wants versions verified and current; the readiness criterion "minimum dependency
release age" wants a delay before adopting new releases, which cuts exposure to
compromised or quickly-yanked releases.

## Decision

Renovate (`renovate.json`): `minimumReleaseAge: 7 days`, `internalChecksFilter: strict`,
Cargo updates grouped in one PR, GitHub Actions grouped with digests pinned, the Rust
toolchain tracked but labelled `type:decision` (a toolchain bump is a decision file),
security advisories bypass the delay. Weekly schedule, dashboard on.

## Alternatives

Dependabot with `cooldown`: works, but weaker grouping for Cargo and no digest pinning
helper. 14 days: safer, slower; 7 is the common compromise and can be raised by editing
one key.

## Consequences

Renovate must be installed on the repository (GitHub App) by the owner; until then the
config is inert. PRs land on `main` through the normal gate.
