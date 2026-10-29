# m4-fix-wire-git-source

## SPEC §9

| kind | row |
|---|---|
| decision (rule 24) | The git thread takes the repository of a mission from the hosted `LaunchConfig.repo` when the app launched it (`Hosts::repo_of`), else `git rev-parse --show-toplevel` of the app's working directory (`ingest::git::toplevel`, read once at thread start). A mission with neither logs `git_skipped reason=no repository` once and writes nothing. No config key, no decision file, no store column. Alternatives: a `[mission].repository` key (decision file, example config row and the `Config::default() == example` test, and one repository for every mission), a `missions.repository` column (a migration for a value only hosted launches know). |
| fact | `git_sync` (core, `ingest/git/sync.rs`) reads the merges once per poll and stores `integration.merged` (source `git`), `gate_command.*` (source `github`) and `missions.branch`; the git thread polls at `[github].poll_interval_s`; the runs part runs only when `GITHUB_TOKEN` is in the environment. The runs come from `GET /actions/workflows/mission-branch.yml/runs?branch=<b>&per_page=100` (`GitHubClient::mission_branch_runs`), not from the per-head poll. |

Real-repo characterization (ignored test `the_real_repository_fills_a_store_copy`, store COPY `/tmp/store-copy.sqlite`, `GITHUB_TOKEN="$(gh auth token)"` in the invoking shell): 12 first-parent merges (the 6 of `c8524c2..mission/observatory-v1` with milestones s, m0, m0, m1, m2, m3; 6 older ones null), 21 runs, 53 events, second poll 0 new; `gh run list` gives 21 run ids, equal to the store's; 17 success, 3 failure, 1 cancelled (`.run` only). The machine was NOT quiet: `uptime` printed load averages 8,81 6,10 4,58 before and 17,52 8,17 5,35 during the run (system processes, no cargo, trunk or wdio of mine running); a rerun on a quiet machine is `GIT_WIRING_STORE=<copy> GITHUB_TOKEN="$(gh auth token)" cargo test -p observatory-core --test git_wiring -- --ignored --nocapture`.

## Decisions

None (see the SPEC §9 decision row above: no new key).

## BACKLOG

- B-new: the card shows the worktree name. Core (`view::milestone_worktrees`), `GitCache` and the `milestone_worktrees` Tauri command (payload `Vec<MilestoneWorktree{milestone,name,path}>`) are done; the Leptos card does not call it or render it yet (needs the kit class for a card sub-line, `make check-kit`, and a refresh on `events_appended`). VAL-GATE-007's "per-card worktree name" half stays open until it lands.
- B-new: a test for a run head that is not a full object name (short or non-hex) has no unit test; `milestone_integrated_last` returns none for it.

## Deferred clauses

- VAL-GATE-007, per-card worktree name (see BACKLOG above). The `missions.branch` half and the worktree list (`git worktree list --porcelain` against the snapshot) are proven by `the_git_poll_records_integration_and_gate_command_rows_in_the_store`.
- The `gate_changed pending` app-log half of VAL-GATE-007 is not part of this feature.

## Library

- `git_or_none` (cmd.rs) now yields none only for git's "bad object", "unknown revision" and "bad revision" (LC_ALL=C); any other git failure is an error.
- Non-blocking items of the m4-git-source-run-attribution review: all three done (`git_or_none` narrowed, `OBJECT_NAME_LEN` named, the ignored `git_source` test asserts that a run descending from a milestone merge has a milestone). Of the m4-git-source-core review: `mission_branches` takes the options (`[mission].branch_pattern`), `merges()` is computed once per poll; `github.rs:169` still writes `missions.branch` from the forge thread (same value, harmless).
