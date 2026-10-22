# D-009: Releases are tags on main, built by the gate's bundle workflow

**Date:** 2026-10-22
**Status:** accepted

## Context

A mission ends in one PR into `main` (D-001). The gate on that PR already builds the
macOS bundle with provenance and a cosign signature (D-003). There was no step that turned
a merged mission into something a user can download, and no release notes.

## Decision

- A release is a semver tag `vMAJOR.MINOR.PATCH` pushed by the human to a commit on
  `main`, after the mission PR merges. Tags are not pushed by agents.
- `.github/workflows/release.yml` verifies the tag is on `main` and is semver, builds the
  bundle through the same reusable `bundle.yml` the gate uses, and creates the GitHub
  release with `gh release create --generate-notes`.
- Release notes come from GitHub's generator configured in `.github/release.yml`: PRs
  grouped by this repository's labels (`type:feature`, `type:bug` and `fix-cycle`,
  `type:security`, `type:decision`, `area:deps`, other). The PR title is the note; write it
  for the reader.
- No deployment, update server or package registry. The release page is the distribution.

## Alternatives

- `release-plz` or `cargo-release`: version bumps and changelogs from commits; too much
  machinery for one PR per mission, and it would need `Cargo.toml` to exist.
- `git-cliff` (B-004 as first written): a second notes format next to GitHub's; dropped.
- Release on every merge to `main`: a mission PR is already the unit; the human decides
  when a merge is a release.

## Consequences

- `gate.yml` and `release.yml` share `bundle.yml`; a change to how the bundle is built lands
  once. The gate's `bundle` job keeps its name, so the ruleset is unchanged.
- Until the Tauri app exists, a tag produces a release with notes and no assets.
- Version source of truth, once code exists: `Cargo.toml` workspace version, equal to the
  tag without `v`. The release workflow does not check this yet; add the check with the
  skeleton (TODO is in `BACKLOG.md` as B-016).
