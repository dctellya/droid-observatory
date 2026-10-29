# demo-gate smoke proof (2026-10-29)

Source: `gh run list`, `gh run view`, `gh pr view`, `gh api`, pasted. Repository `dctellya/droid-observatory`, PR #24
(`demo/smoke-1` into `demo/base`), head sha `1d994af00fb141466e46021e9c6f8adbf314db55`, workflow `.github/workflows/demo-gate.yml`.

| Step | Time (UTC) | Run id | Event | Conclusion |
|---|---|---|---|---|
| PR created with `--label demo:red` (`opened`) | 09:58:34 | 40853828337 | pull_request | cancelled (superseded by the `labeled` run, same concurrency group) |
| `labeled` demo:red | 09:58:31 (label event) | 40853828900 | pull_request | **failure** (log: `labels: ["demo:red"]`, `demo:red is present: demo-gate is red`) |
| `gh pr edit 24 --remove-label demo:red` (`unlabeled`) | 09:59:02 (label event) | 40853894595 | pull_request | **success** (log: `labels: []`, `no demo:red label: demo-gate is green`) |

Check runs on the head sha (`gh api repos/dctellya/droid-observatory/commits/<sha>/check-runs`): `123379916616` success
(completed 09:59:11Z), `123379690904` failure (09:58:40Z), `123379682724` cancelled (09:58:35Z).
`gh pr view 24 --json statusCheckRollup` after the unlabel: FAILURE, CANCELLED, SUCCESS (three runs of `demo-gate`, latest SUCCESS).

Clean-up: `gh pr close 24`, `gh api -X DELETE repos/dctellya/droid-observatory/git/refs/heads/demo/smoke-1` exit 0, then
`gh api repos/dctellya/droid-observatory/git/refs/heads/demo/smoke-1` returns 404 Not Found; `demo/base` is the only `demo/*` branch.
