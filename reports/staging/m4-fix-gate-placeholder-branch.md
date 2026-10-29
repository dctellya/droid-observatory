# m4-fix-gate-placeholder-branch

## SPEC §9

| kind | row |
|---|---|
| fact | The pre-gate placeholder names the mission's branch: `OpenedMission.branch` (`#[serde(default)]`, set by the shell in `open_replay`, `open_mission`, `attach`; core leaves it `None`) is `missions.branch` when git_sync recorded one, else `[mission].branch_pattern` resolved for the mission name (`commands::resolve_branch`); `Capture.branch` carries it to `Board`, which no longer builds it from `MissionConfig::default()`; the browser host (no shell, no config) keeps the default pattern. The `board` command and the `gate_changed` app-log line (`branch=<name>`, `announce_gate`) use the same value. No kit class, no `style.css` change. |
| fact | The e2e config sets `[mission].branch_pattern` to the literal `mission/observatory-v1` (`lib/env.ts` `E2E_BRANCH`), so every mission of the suite resolves to it; no other spec reads the branch (they match library rows by the launch form's branch name). |
| fact | `e2e_gate_poll` takes an optional `mission_id` and a `pending` scenario (a poll with no PR and no gate facts). Without `mission_id` it runs on the live mission that started last: the suite keeps one app for all specs, earlier hosted runs stay live, and the host's `HashMap` order picked the controls spec's mission in one run in two (`gate.e2e.ts` flaked, `reports/e2e/2026-10-29/211941/record.json`: poll hit e6b13589, the board showed 01091b34). Run 3 after the fix: outcome passed, no rerun. |
| fact | walk-2-policy waits up to 45 s for a `gate_changed` line with `merge_offered=false` (`browser.waitUntil`, `appLog()`), then keeps it in the walk's command log; the badge can read CI green from the first poll's events before the header's log line is written. The walk itself was not run (needs `gh` and the live demo pair). |

## Decisions

None.

## BACKLOG

None. The `mission/<session id>` row of `m4-fix-card-worktree-name` is addressed for the placeholder and the gate log (the board names `missions.branch` once the git thread recorded one); the git thread still derives its branch from the pattern and the mission name, so an attached mission under the default pattern still polls `mission/<session id>`.

## Deferred clauses

- VAL-GATE-007, `gate_changed ... badge="pending" ... branch=` from a running window: with GITHUB_TOKEN and FORGE_LIVE unset the github thread is off (`github.token source=off reason=no GITHUB_TOKEN and FORGE_LIVE unset`), so `cargo tauri dev` prints no `gate_changed`. The line is proven by `make e2e` (placeholder spec, `e2e_gate_poll` pending scenario through `apply_poll`, the path a real poll takes). A window line needs a token and FORGE_LIVE=1, the validators' live Gate run.
- `make e2e` of this session: the first run was `flaked` (gate.e2e.ts, the poll-target race above, fixed in 7c63ace), the second `failed` (placeholder spec passed the id as a free variable into the window script, fixed in 5309965), the third `passed`.

## Library

- The e2e app is one process for all spec files: hosted runs of earlier specs stay in `Hosts` (a `HashMap`, no order) and in `live_missions`. A shell command that acts on "the live mission" must name it or choose by start time.
- `browser.executeAsync` runs its function in the page: values from the spec's scope go in as arguments (`executeAsync((id, done) => ..., value)`), never as free variables.
- The 1-minute load average stays above 4 for several minutes after a `cargo test -p observatory-core` and `make check` and sometimes from system daemons (mediaanalysisd, XProtect); a loop on `sysctl -n vm.loadavg` (decimal comma in this locale: replace `,` by `.`) bounded by `perl -e 'alarm N'` returns when it passes.

**Claims (rule 30)**

```json
[
  {
    "assertion": "VAL-GATE-007",
    "evidence": "Behavioural half (placeholder and gate_changed payload name the mission branch): e2e spec apps/observatory-app/e2e/specs/placeholder.e2e.ts (attached S fixture, [mission].branch_pattern = the literal mission/observatory-v1 in lib/env.ts E2E_BRANCH), case `names the configured mission branch before any poll` reads `[aria-label=gate-branch]` = `mission branch: mission/observatory-v1`, case `names the same branch in the pending gate_changed line of the app log` finds the line `gate_changed mission_id=5e2e0000-0000-4000-8000-00000000a77a badge=\"pending\" branch=mission/observatory-v1 decided_by=None ruleset=\"\" requires_human=false merge_offered=false required_checks=[] ...` in reports/e2e/2026-10-29/215143/wdio-*.log; `make e2e` record reports/e2e/2026-10-29/215143/record.json outcome passed, 7 of 7 spec files. Red: 34cbcaf (cargo test -p observatory-app --lib an_opened_mission: does not compile, no field `branch` on OpenedMission, exit 101, /tmp/red.txt); green: 40a25f6 (2 passed; 0 failed). Code: crates/observatory-core/src/ingest/live.rs (OpenedMission.branch), apps/observatory-app/src-tauri/src/commands.rs (resolve_branch: missions.branch row, else branch_pattern resolved for the mission; used by open_replay, open_mission, attach and board), apps/observatory-app/src-tauri/src/github.rs (announce_gate `branch = %branch`), apps/observatory-app/ui/src/capture.rs and board.rs (the board takes capture.branch, no MissionConfig::default()). Window smoke: `cargo tauri dev` 120 s, GITHUB_TOKEN and FORGE_LIVE unset, literal branch_pattern config: `SELECT id, name, branch FROM missions` -> 5e2e0000-0000-4000-8000-0000000005a0|5e2e0000-0000-4000-8000-0000000005a0|mission/observatory-v1 and `git_poll mission_id=5e2e0000-0000-4000-8000-0000000005a0 branch=mission/observatory-v1 merges=12 runs=0 events=12 new=12` (/tmp/m4-smoke/tauri.log). The smoke has NO gate_changed line by design: with no token and FORGE_LIVE unset the github thread is off (`github.token source=off`), so the pending line is shown by the e2e run above, not the window. Gate: make check exit 0, `contract checks: 22 ok, 9 skipped, 0 FAIL in 5.4s` (/tmp/check.txt:2526), record reports/gate/d2d4b6e9d554606ef96e32bdb58ff1b1415ae13b/check.json."
  },
  {
    "assertion": "VAL-GATE-007",
    "evidence": "Mechanical half (missions.branch row and per-card worktree name against git): see reports/staging/m4-fix-card-worktree-name.md claim for VAL-GATE-007 (e2e case `the card names the worktree git lists for its milestone, the path as its title`, git_wiring.rs `a_card_names_the_worktree_git_lists_for_its_milestone`, the real-repository run `SELECT id, branch FROM missions` -> real | mission/observatory-v1 and the six mission-range merge shas equal `git log --first-parent --merges c8524c2..mission/observatory-v1`). Unchanged by this feature: gate.e2e.ts `the card names the worktree git lists for its milestone, the path as its title` passes in the same `make e2e` run (reports/e2e/2026-10-29/215143/record.json, outcome passed). New here: the shell now reads `missions.branch` (Store::mission_branch, written by git_sync) before the pattern, so the board names the branch the git source recorded: core/shell test apps/observatory-app/src-tauri/src/commands.rs `an_opened_mission_prefers_the_branch_the_store_recorded` (2 passed; 0 failed, /tmp/green.txt)."
  }
]
```
