# Red gate

**Symptom:** the `gate` check on the pull request into `main` is red, so the ruleset blocks
the merge.

## Confirm

```sh
gh pr checks <pr> --watch=false
gh run view <run-id> --log-failed
```

`gate` is an aggregate; the failing job is one of `light / *`, `sast`, `sast-upload`,
`fs-scan`, `workflow-policy`, `sbom`, `bundle`.

## Fix by job

| Job | Local reproduction | Typical cause |
|---|---|---|
| `light / docs` | `make check-docs check-file-size check-todos check-decisions-index` | markdownlint, stale decisions index, bare `TODO` |
| `light / links` | `make check-links` | dead URL; if the site is flaky, add it to `lychee.toml` `exclude` with a comment |
| `light / workflows` | `make check-workflows` | actionlint |
| `light / secrets` | `make check-secrets` | a leaked value: rotate it first, then remove it with a new commit (never rewrite history, rule 12) |
| `light / rust`, `light / deny` | `make fmt-check lint test deny` | warnings are errors; a new dependency fails `deny` until its licence or advisory is handled |
| `sast` | run the container: `docker run --rm -v "$PWD:/src" semgrep/semgrep:<version> semgrep scan --config p/default --config p/rust --error` | findings show on the PR via code scanning; fix or add an inline `nosemgrep` with a reason |
| `fs-scan` | `trivy fs --severity HIGH,CRITICAL .` | vulnerable dependency or misconfiguration; findings also in the Security tab |
| `workflow-policy` | `checkov -d .github/workflows --framework github_actions` | unpinned action, missing permissions block |
| `bundle` | `make app-build` on macOS | Trunk or Tauri toolchain; see D-002 go/no-go if it recurs |

Push the fix as a new commit on the mission branch. The gate reruns on the PR.

## Stop and involve the human when

- The fix needs a change to `rust-toolchain.toml`, a workflow permission, or the ruleset
  (each is a decision file).
- `secrets` found a real credential (rotation is the owner's).
- The same job has failed three times on the same cause (rule 15 stall).
