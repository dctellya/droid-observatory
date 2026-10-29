# Staging: m4-git-source-run-attribution

Feature `m4-git-source-run-attribution` (follow-up of `m4-git-source-core`, orchestrator ruling 2026-10-29, rule 24), worktree
`observatory-wt/m4`, branch `wt/m4`. Claims `VAL-GATE-009`.

## SPEC §9

- Row (F-NEW), orchestrator ruling under rule 24 (2026-10-29, recorded at the next boundary): `GitSource::gate_command_events` attributes a
  fast-check run to the milestone whose `wt/<m>` first-parent merge is the run's head or the nearest first-parent ancestor of the head, the
  milestone integrated last (one `git rev-list --first-parent --merges <head>` per distinct run head, the first merge a milestone made wins;
  no per-event git call). A run whose head has no `wt/<m>` merge behind it, is not a full object name, or is not in the clone keeps
  `milestone_id` null. Real repository (`GIT_SOURCE_RUNS_JSON=/tmp/runs.json cargo test -p observatory-core --test git_source -- --ignored
  --nocapture`, 21 runs of `gh run list --workflow mission-branch --branch mission/observatory-v1`): 21 of 21 runs carry a milestone (s 3, m0
  7, m1 4, m2 4, m3 3, counted from the pasted output, one m3 run has no verdict); the exact-head rule gave a milestone only to the 5 runs whose head is a
  merge commit (38e56174, cc05aa55, f66a9f50, c2fb42ac, 7385b629).
  Evidence: `runs_on_commits_after_a_merge_are_attributed_to_the_milestone_integrated_last` (red 6e1590b, green 6419db9), and run 104 of
  `gate_command_runs_are_lane6_events_attributed_to_the_milestone` now expects the milestone of the last merge (changed in the red commit).
  **Alternatives considered:** attribute only a run whose head is the merge commit (rejected: the first version; 16 of the 21 runs sit on commits after a
  merge, so the Integrate lane showed them under no milestone); one `git merge-base --is-ancestor` per run and
  merge (rejected: runs times merges git calls, where one first-parent walk per head answers the nearest merge); attribute by commit date
  (rejected: a clock, not the history; rebased or backdated commits land in the wrong section).

## Decisions

## BACKLOG

## Deferred clauses

## Library
