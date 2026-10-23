# D-014: Read-only fine-grained PAT, check results from Actions runs and commit statuses

**Date:** 2026-10-23
**Status:** accepted (owner decision 2026-10-22, entry 26; recorded at the S integration)

## Context

The gate lane needs the state of CI for a branch and a pull request. The token in the keychain
(`droid-observatory/github-token`) is a fine-grained personal access token for owner `dctellya`,
repositories `droid-observatory` and `artstudio`, read-only on Contents, Issues, Pull requests,
Actions, Commit statuses and Metadata. Fine-grained tokens have no Checks permission, so
`/repos/{o}/{r}/commits/{sha}/check-runs` is not available (entry 26).

## Decision

- Read check results from Actions runs (`GET /repos/{o}/{r}/actions/runs?head_sha=<sha>`) and
  commit statuses (`GET /repos/{o}/{r}/commits/{sha}/status`). Never call the Checks API.
- The token stays read-only. The one outward write, merge, is not attempted by default: the app
  shows the Merge button when policy allows, surfaces the `403` cleanly (message naming the PR,
  the target and the missing permission, no crash, no retry storm), and the owner merges in the
  GitHub UI or API; the badge updates from the poll (owner decision 2026-10-22, planning note 7).
- Whether to give the app a write-capable token is asked at the M3 to M4 boundary.

## Alternatives

A classic token with `repo` scope (broad, can write everywhere); a GitHub App (setup cost for a
single-user desktop tool); the `gh` CLI as the transport (an external process in the hot path, and
the token would live in the CLI's store, not in the app's keychain item).

## Consequences

Required-check names come from the ruleset and workflow job names, not from check runs. A check
that reports only through the Checks API and not as an Actions run or a status is invisible to the
app. SPEC §3 M4 acceptance is amended as-built in the same integration.
