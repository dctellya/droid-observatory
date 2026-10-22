# D-003: Two workflows, one required check, jobs that activate with the code

**Date:** 2026-10-22
**Status:** accepted

## Context

RULES.md rule 6 splits CI into light checks on the mission branch (under three minutes
warm) and a full attested run on the one PR into `main`. The `main` ruleset must name
required checks before any code exists, and a required check that never reports hangs the
final PR.

## Decision

- `mission-branch.yml` (on `mission/**`, also `workflow_call`): docs, links, workflows,
  secrets, rust, deny. Aggregate job **`mission-checks`**.
- `gate.yml` (on PRs to `main`): reuses the light workflow, adds Semgrep, Trivy fs, Checkov
  on the workflows, SBOM (syft), macOS Tauri bundle with `attest-build-provenance` and
  cosign keyless signatures. Aggregate job **`gate`**, the only check the ruleset requires.
- Aggregate jobs run `if: always()` and fail on any `failure`/`cancelled` in `needs`;
  skipped jobs count as not applicable. New jobs are added to `needs`, never to the ruleset.
- Rust, fixtures and Tauri jobs are gated by a `detect` job probing for `Cargo.toml`,
  `fixtures/streams/*.jsonl`, and `apps/observatory-app/src-tauri/tauri.conf.json`.
- Every action is pinned to a commit digest with a version comment; Renovate keeps them
  current (D-005). Versions verified against GitHub releases on 2026-10-22.
- Workflows call `make` targets so README commands and CI commands are the same thing.

## Alternatives

Listing each job as a required check: brittle, breaks when jobs are renamed or skipped.
Separate workflows per tool: more files, no shared `detect`.

## Consequences

The ruleset (D-007) requires exactly `gate`. The mission must keep the probed paths.
