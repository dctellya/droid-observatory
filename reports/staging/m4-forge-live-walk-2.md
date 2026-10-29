# Staging: m4-forge-live-walk-2

Feature `m4-forge-live-walk-2` (VAL-GATE-012, -015, -016, -017, -020, -021, -027, -028), worktree `observatory-wt/m4`,
branch `wt/m4`. Raw outputs: `/tmp/walk2-red-live.txt`, `/tmp/walk2-green-live.txt`, `/tmp/walk2-merge-live.txt`,
`/tmp/walk2-policy-live.txt`, `/tmp/walk2-unit.txt`; store `/tmp/observatory-forge-walk.sqlite`.

## SPEC §9

Route read from `docs/owner-feedback/inbox/2026-10-29-1524-question-16-closed.md`: route (a), the owner pushed
`wt/m4` from a plain shell (`origin/wt/m4` a5efd55); no history rewrite, no demo-branch workaround. `git log
origin/wt/m4..HEAD` held 5 commits (a2eb508, 319ec1a, d4d3194, ecb193d, e821ff8); `git diff origin/wt/m4..HEAD`
matched no `github_pat_`, `ghp_` or `gho_` literal; Droid-Shield accepted the push.

| Step | Figure |
|---|---|
| Pushed head | `f4de0fe` = empty `git commit-tree` child of the tip a2eb508 (tip carries `[skip ci]`, which would skip the `pull_request` run; the child message has no marker), `demo/forge-checks-1`, push exit 0 |
| PR | #29 `demo/forge-checks-1` -> `demo/base`, created with `--label demo:red` 2026-10-29T14:47:06Z |
| Red run | 40894949421 failure (created 14:47:09Z, updated 14:47:17Z); a second run 40894948984 for the same sha was `cancelled` by the workflow's `cancel-in-progress` (opened and labeled fire together); `gh run watch 40894949421 --exit-status` returned the failure at once (exit 1) |
| Red live test | `badge ci_red`; `prs` 29 open; `checks` demo-gate actions_run failed; `check.failed` id `github:check:29:40894949421:failed`; `rework_windows` 1 m4 gate opened_by that id, closed_by NULL, pr 29; `missions` live `fixing` |
| Label removed | 2026-10-29T14:47:45Z; no new head needed (`unlabeled` triggers the workflow) |
| Green run | 40895054822 success (created 14:47:47Z, updated 14:47:53Z); `gh run watch --exit-status` exit 0 |
| Green live test (same store) | `badge ci_green`; `checks` passed; `check.passed` id `github:check:29:40895054822:passed`; window 1 closed_by `github:check:29:40895054822:passed`; `missions` `at_gate`; same PR 29 |
| Merge | `gh pr merge 29 --squash --delete-branch` by dctellya, mergedAt 2026-10-29T14:48:12Z, merge commit `36f287b84fe4d9a1deea3b081bb96ccee114b152`; the branch is gone on origin (`git ls-remote` empty) |
| Merge probe | `badge human_merged decided_by Ruleset { id: 26112925, name: "demo-gate" } requires_human true merge_offered false`; `prs` 29 merged merged_by dctellya; one `handed_to_cd` (id `github:pr:29:handed_to_cd`, role human, lane gate, sha 36f287b, url `.../pulls/29`, at 2026-10-29T14:48:12.000Z); `missions` live complete, completed_at 2026-10-29T14:48:12.000Z |
| Policy probe | separate store, same merged PR: `decided_by ConfigOverride("policy") requires_human false merge_offered false` |
| Counts | `SELECT COUNT(*) FROM prs WHERE head='demo/forge-checks-1'` 1; `gh pr list --head demo/forge-checks-1 --state all` one number (29); `human_actions` kind merge 0; `unknown.event` from git/github 0; attestations 0 and reviews 0 (the demo-gate run uploads no attestation artifact, so the `handed_to_cd` event has an empty attestation list) |
| Unit rerun | `cargo test -p observatory-core --test forge --test forge_merge`: 16 passed (forge), 13 passed (forge_merge), exit 0, real 7.036 s |

Side effect to know: the squash merge put the whole `wt/m4` tree (as of a2eb508) on `origin/demo/base`, a test branch
that no ruleset ties to `main`. The merge was the feature's own instruction.

External merger role: this machine holds only the login dctellya, so the live merge reads `role = human`. The
`external` role stays with the unit test `policy_merged_detected_without_human_action` (forge_merge.rs:469); VAL-GATE-027
is claimed on that test plus the live merge's `handed_to_cd`, not on a live external merge.

### Claims (rule 30)

| Assertion | Evidence |
|---|---|
| VAL-GATE-012 | Unit: `checks_come_from_actions_runs_and_commit_statuses` tests/forge.rs:426. Live: `/tmp/walk2-red-live.txt` request lines `GET .../actions/runs?head_sha=f4de0fe...` and `GET .../commits/f4de0fe.../status`; `checks` row demo-gate actions_run failed; `gh api .../actions/runs?head_sha=f4de0fe...` returned 40894949421 failure (and 40894948984 cancelled), the stored conclusion agrees with the failure run |
| VAL-GATE-017 | Unit: `red_required_check_opens_a_fix_cycle_on_the_affected_card` tests/forge.rs:767. Live: `check.failed` lane gate source github url `.../actions/runs/40894949421`, badge `ci_red` |
| VAL-GATE-020 | Unit: `fix_cycle_keeps_the_same_pr` tests/forge.rs:904. Live: PR 29 across red and green, `prs` count 1, `gh pr list` one number |
| VAL-GATE-021 | Unit: `green_check_closes_the_rework_window_and_the_badge_turns_green` tests/forge.rs:854. Live: window closed_by `github:check:29:40895054822:passed`, badge `ci_green`; the second poll ran 8 s after the run finished (updatedAt 14:47:53Z, poll 14:48:01Z), inside the 30 s bound |
| VAL-GATE-015 | Unit: `rule_attribution_names_the_ruleset` tests/forge_merge.rs:339. Live: `decided_by Ruleset { id: 26112925, name: "demo-gate" } requires_human true`, id equal to `gh api repos/dctellya/droid-observatory/rulesets` demo-gate |
| VAL-GATE-016 | Unit: `merge_policy_override_hides_merge_and_names_the_config` tests/forge_merge.rs:417. Live: policy probe `decided_by ConfigOverride("policy") requires_human false merge_offered false`; run on the merged PR, so the open-and-green button state is unit evidence only |
| VAL-GATE-027 | Unit: `policy_merged_detected_without_human_action` tests/forge_merge.rs:469, `handed_to_cd_is_emitted_once_with_merge_sha_pr_and_attestations` tests/forge_merge.rs:523. Live: one `handed_to_cd`, 0 `human_actions` merge rows; role is `human` not `external` (one login on this machine) |
| VAL-GATE-028 | Unit: `a_merge_completes_the_mission_at_the_hand_over` tests/forge_merge.rs:618. Live: `missions` complete, completed_at 2026-10-29T14:48:12.000Z equals the `handed_to_cd` at |

The badge line in the merge probe reads `human_merged` (the probe prints the Gate header after the merge); the app-log
`gate_changed` payload lines the contract names come from the app run, not from this probe.

## Decisions

None.

## BACKLOG

- The probe prints no attestation ids from the `handed_to_cd` event itself (the event row has no payload column); a run
  that uploads an attestation artifact would be needed to show a non-empty list live (reason: demo-gate uploads none).

## Deferred clauses

- VAL-GATE-027 `role = external` live (needs a second GitHub login).
- VAL-GATE-016 live with an open green PR (the merged PR was probed under `policy`).

## Library

- `gh pr create --label demo:red` fires `opened` and `labeled` together: the first demo-gate run is cancelled by the
  concurrency group and the second carries the result. Take the run id from `actions/runs?head_sha=` after both exist.
- A `[skip ci]` in the pushed head skips the `pull_request` run; give the demo branch a head commit without it.
