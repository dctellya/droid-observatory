# m4-fix-card-worktree-name

## Claims (rule 30)

```json
[
  {
    "assertion": "VAL-GATE-007",
    "evidence": "Worktree half (per-card name): e2e case `the card names the worktree git lists for its milestone, the path as its title` in apps/observatory-app/e2e/specs/gate.e2e.ts, red on 6d199ef (10 passing, 1 failing; 'the card never named its worktree'), green on 44681ea in `make e2e` (record.json outcome passed, reports/e2e/2026-10-29/194840/, PNG reports/e2e/2026-10-29/194840/card-worktree-name.png shows `wt-e2e-gate` on card-e2e-gate); render at apps/observatory-app/ui/src/lanes.rs (card_view, `card-worktree`), refresh at ui/src/app.rs and ui/src/capture.rs (`refresh_worktrees`, on open and on `events_appended`); empty case: core test view::worktree::tests::a_milestone_without_a_worktree_has_no_name. Branch/worktree-list half against git: crates/observatory-core/tests/git_wiring.rs `a_card_names_the_worktree_git_lists_for_its_milestone` and the quiet re-take of `the_real_repository_fills_a_store_copy` (load 3,88 before, 3,71 after; `SELECT id, branch FROM missions` -> real | mission/observatory-v1; the six mission-range merge shas equal `git log --first-parent --merges c8524c2..mission/observatory-v1`, each with a milestone s, m0, m0, m1, m2, m3). NOT shown: the `gate_changed pending` app-log line (not this feature) and a `git_poll{...}` line from a running window (see Deferred clauses)."
  }
]
```

## SPEC §9

| kind | row |
|---|---|
| fact | The card names its worktree: `Capture` holds `worktrees` (`milestone_worktrees` command, asked when a mission opens and on every `events_appended` of the open mission); `card_view` draws `<p class="mono" data-testid="card-worktree" title=<path>>name</p>` for the card's milestone and nothing for a milestone git lists no worktree for. One kit class (`.mono`, kit rule 2: identifier text); no `style.css` rule, no `kit-departures.md` or `kit-pending.txt` row; `make check-kit` ok. |
| fact | The e2e build's `e2e_gate_poll` also puts one worktree (`wt-e2e-gate`, `/e2e/worktrees/wt-e2e-gate`) in `GitCache` for the mission, as a git poll would have cached it (the e2e config polls every 3600 s, so the real thread never fills it). |
| fact | `ingest::git::git_sync` is `#[cfg(feature = "store")]`: `observatory-ui` builds core without the store feature and `trunk build` had failed since 716f610 on `use crate::store` (E0432). The e2e build before this feature ran the stale `ui/dist`. Red: `cargo check -p observatory-ui --target wasm32-unknown-unknown` exit 101; green: exit 0 (d63f4cc). |
| fact | `github.rs` no longer writes `missions.branch` (the git thread's `git_sync` writes the same value; `cargo test -p observatory-app` 11 passed). |

## Decisions

None.

## BACKLOG

- B-new: `GitHubClient::mission_branch_runs` lists one page (`per_page=100`, `RUNS_PER_PAGE`, `ingest/github.rs:472`) of the workflow's runs of the branch and does not follow `Link: rel="next"`. 21 runs at the real mission today; a branch with more than 100 runs loses the oldest from the gate_command rows. Page it (or stop at the first page older than the mission's first merge) before a mission can reach 100 runs.
- B-new: an attached mission is named by its session id, so the git thread derives `mission/<session id>` (`[mission].branch_pattern`), which no repository has: with `cargo tauri dev` and no token the log has `git_started` and then, every `[github].poll_interval_s` (30 s), `git_poll_failed ... the branch mission/e194b9e3-... does not exist`; no `git_poll` line, no worktree on the cards of an attached mission whose branch is not named by its id. Name the branch from the mission's `mission.md` title or let the owner map it; log the failure once per mission like `git_skipped`.

## Deferred clauses

- VAL-GATE-007, `git_poll{...}` line from a running window: the `cargo tauri dev` run (GITHUB_TOKEN and FORGE_LIVE unset, 120 s, load 3,53 before) printed `git_started interval_s=30 pattern=mission/<name> runs=false` and `github.token source=off`, no keychain prompt, and two `git_poll_failed` lines for the live attached mission (see the second BACKLOG row). The successful poll is proven by `the_git_poll_records_integration_and_gate_command_rows_in_the_store` and the real-repository run, not by the window.
- The `gate_changed pending` app-log half of VAL-GATE-007 is not part of this feature.

## Library

- `make e2e` after a failed `trunk build` (cargo tauri build exits non-zero) must not be followed by the runner: a `;` chain ran the stale binary and printed a red for the wrong reason; chain the steps with `&&`.
- `cargo check -p observatory-ui --target wasm32-unknown-unknown` is the inner-loop proof that core's feature gates still hold for the UI; run it after any core change that adds an `ingest` or `store` import.
