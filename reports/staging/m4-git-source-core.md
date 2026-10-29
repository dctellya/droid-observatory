# Staging: m4-git-source-core

Feature `m4-git-source-core` (VAL-GATE-007, VAL-GATE-008, VAL-GATE-009), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW): `ingest::git::GitSource` (architecture §4.6) reads the repository with read-only `git` (no new dependency).
  `GitSource::mission_branches` is `for-each-ref refs/heads/mission/`; `snapshot()` returns the mission branch and head, every
  `git worktree list --porcelain` entry (with the milestone of a `wt/<m>` branch) and the `wt/*` branches with their tips;
  `integration_events()` yields one `integration.merged` per `git log --first-parent --merges` commit (role orchestrator,
  `SourcePtr::Git { repo, sha }`, id `git:<sha>:0`, `at` = committer date); `gate_command_events(&[FastCheckRun])` yields
  `gate_command.run` per run and `.passed` / `.failed` when the run has a verdict (role external, `SourcePtr::GitHub { url }` of
  the run, ids `github:<run_id>:0` and `:1`). Lanes are `template.lane_of(kind, role)`, no lane name or number in the code.
  `GitSource` does not implement the `Source` trait (its output is events, not raw lines); `SourceKind::Git` stays declared.
  `Store::set_mission_branch` and `Store::mission_branch` read and write `missions.branch` (column exists, no migration).
- Row (F-NEW), milestone attribution of a merge: the branch named in the subject (`Integrate m3: wt/m3`, git's `Merge branch
  'wt/m4'`), else the `wt/*` branch whose first-parent line holds the merged tip, nearest to its own tip first (a later
  milestone branched from the merge does not claim it). A merge no `wt/*` branch made (the pull requests merged on `main`
  before the mission) has `milestone_id = NULL`. On the real repository: 6 such merges, s, m0 (two), m1, m2, m3 attributed.
- Row (F-NEW), run attribution: exact match only, `milestone_id` of the merge whose sha is the run's `headSha`; a run on any
  other commit has `milestone_id = NULL`, because a nearest-ancestor guess would file the run in a milestone that did not
  integrate it. Cancelled, skipped and unfinished runs get `gate_command.run` and no verdict (`RunConclusion::from_github`:
  `success` passed; `failure`, `timed_out`, `startup_failure` failed).
- Evidence: `crates/observatory-core/tests/git_source.rs` (temp repository built by the test, the three named tests plus a
  store round trip with `unknown_count = 0`); the ignored `the_real_repository_yields_lane6_events_and_no_unknown` on this
  repository and `gh run list --workflow mission-branch` (21 runs, 53 lane-6 events, 0 unknown).

## Decisions

None.

## BACKLOG

- VAL-GATE-008 literally compares the `source='git'` lane-6 shas with `git log --first-parent --merges mission/observatory-v1`
  and demands a non-null `milestone_id` for each. The real log also holds six pull-request merges of `main` before the mission
  that no worktree branch made, so those six are unattributable. Either the contract query restricts to `main..mission/<name>`
  or the six rows stay NULL. Decision for the orchestrator at the boundary.
- VAL-GATE-009 attributes by exact head. Of 21 real `mission-branch` runs only 5 have a merge commit as head (m0 twice, m1, m2,
  s); the other heads are commits between merges and have `milestone_id = NULL`. If the owner wants those under the milestone
  integrated last, `gate_command_events` gains a nearest-first-parent-ancestor rule (one `merge-base --is-ancestor` per run).
- The wiring of `GitSource` into the app ingest loop (`apps/observatory-app/src-tauri/src/ingest.rs`), the `missions.branch`
  write on attach, the per-card worktree names and the `pending` placeholder payload (VAL-GATE-007 app-log part) belong to the
  forge and gate-group features; this feature delivers the core API and the store setters.
- `GitSource::merges` runs `rev-list --first-parent` once per `wt/*` branch per call; with a long-lived mission and many branches
  poll it at the `[github].poll_interval_s` rhythm, not per event.

## Deferred clauses

- VAL-GATE-007 app-log clause (`gate_changed` payload `pending`, per-card worktree name): view and shell, not core.
- VAL-GATE-009 `source='github'` rows from the live `gh`/API runs: this feature takes `FastCheckRun` as input; the fetch is
  `m4-forge-core-and-pr-detection` / `m4-forge-checks-and-fix-cycle`.

## Library

- Building a repository in a test: `git -c commit.gpgsign=false -c core.hooksPath=/dev/null` plus `GIT_AUTHOR_DATE` and
  `GIT_COMMITTER_DATE` per commit keep merge order deterministic; canonicalize the temp dir (macOS `/var` is `/private/var`)
  before comparing it with `git worktree list` paths.
- `gh run list --workflow mission-branch --branch <b> --json databaseId,conclusion,headSha,createdAt,updatedAt,url -L 100` is the
  input shape of `FastCheckRun` (url there is the HTML URL; it contains the run id).
