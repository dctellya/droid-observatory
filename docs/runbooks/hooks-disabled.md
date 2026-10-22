# Hooks disabled

**Symptom:** a commit landed without the pre-commit output, a push to `main` was attempted,
`build-log/` has no line for a tool call, or an agent amended or rebased.

Two hook sets protect rule 12 and rule 17:

| Set | Where | Enabled by | Enforces |
|---|---|---|---|
| Git hooks | `.githooks/` | `make hooks` (sets `core.hooksPath`) | pre-commit checks, 72-char subject, no push to `main`, no non-fast-forward |
| Factory hooks | `.factory/hooks.json` | present in the checkout | `guard-git.sh` blocks amend/reset/rebase/stash/force/push-to-main (exit 2); `log-tool-call.sh` writes `build-log/` |

## Confirm

```sh
git config core.hooksPath            # must print .githooks
ls .githooks && test -x .githooks/pre-commit
jq . .factory/hooks.json             # both hooks listed
ls build-log/$(date +%F)/            # one file per session today
```

## Fix

- `core.hooksPath` empty or wrong: `make hooks`. Each worktree is its own checkout; run it
  there too.
- Hook files not executable: `chmod +x .githooks/* .factory/hooks/*.sh` and commit.
- `build-log/` silent: `jq` missing (`make doctor`), or the session runs outside the repo
  root. The hook needs the repository as the working directory.
- History was rewritten anyway: do not "fix" by rewriting again. Push the current state
  forward as new commits and record what happened in the session log and `reports/`.

## Stop and involve the human when

- A hook was edited or deleted by an agent: that is a rule 12 breach, logged and reviewed
  before anything else continues.
- `main` received a direct push (the ruleset should make this impossible; if it happened,
  the ruleset changed, which is a decision).
