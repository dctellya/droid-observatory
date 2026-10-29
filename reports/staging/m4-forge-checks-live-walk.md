# Staging: m4-forge-checks-live-walk

Feature `m4-forge-checks-live-walk` (VAL-GATE-012, -017, -020, -021), worktree `observatory-wt/m4`, branch `wt/m4`.
The live red-to-green walk was NOT run: Droid-Shield blocked the push of the demo branch (see SPEC §9).

## SPEC §9

- Row (F-NEW), live walk of `m4-forge-checks-live-walk`: not run, no figures. No run id, PR number or event id exists for it:
  nothing was pushed or opened. The push `git push origin HEAD:refs/heads/demo/forge-checks-1` from `da5e15d` was refused by
  Droid-Shield before it ran (2026-10-29, `/tmp/walk-shield.txt`): `tests/forge.rs:30`, `tests/secrets.rs:7`, `tests/secrets.rs:13`.
  Those line numbers are not in the tip: step 1 (`0da61cb`) builds the fakes at run time, and the tip holds no token-shaped
  literal. The shield scans the diff of the whole push range (`origin/wt/m4..HEAD`, 103 commits), and commits `3eeacc7`,
  `12d2c63` and `22cdb27` of the range still carry the old literals; history is forward-only (rule 12), so fixing the tip
  cannot clear the range. Nothing was worked around (no API-created branch, no retry).
- Evidence the walk's claims rest on instead: the recorded red and green runs of PR #24 replayed through the real source,
  store and view (`tests/forge.rs`), and the read-only live poll of the closed PR #24 (green side, `/tmp/live1.txt`, see
  `m4-forge-checks-and-fix-cycle.md`). The red-to-green transition on a live PR stays unproven until a demo branch is pushed.

## Decisions

None.

## BACKLOG

None (no B-row).

## Deferred clauses

- Live red-to-green walk on `demo/forge-checks-1` (label `demo:red` on, off, same PR number, same store file): blocked by the
  push range above. Ways to clear it, each the owner's or the orchestrator's call: (a) the owner pushes `demo/forge-checks-1`
  (or `wt/m4`) outside Droid; (b) a demo branch whose single commit sits on `origin/demo/base` and carries the tip tree (its
  diff against `demo/base` holds the clean tests only); (c) a recorded exception to rule 12 to rewrite the three commits. The
  walk itself (commands in the feature description) then takes about ten tool calls.
- `gate_changed` in the Tauri window: not in this feature (user-testing validator).

## Library

- Droid-Shield tool fact: it blocks a `git push` whose range diff, not only the tip, holds a token-shaped literal
  (`github_pat_` plus 22 or more characters and its kin). Fixing the tip is not enough once the literal is in a pushed-range
  commit: keep fakes out of every commit from the first one (build them at run time, `fake_token()` in `tests/forge.rs` and
  `tests/secrets.rs`). The message names `file:line` of the range's own commits, which is why lines 7, 13 and 30 are reported
  after the tip changed.
- `.markdownlint-cli2.yaml` ignores `docs/owner-feedback/inbox/**` (owner and delegate files, verbatim); with the untracked inbox
  folder present `markdownlint-cli2 "**/*.md"` lints 99 files and exits 0 (`/tmp/walk-mdlint.txt`), so the pre-commit markdown
  hook no longer fails on owner files.
