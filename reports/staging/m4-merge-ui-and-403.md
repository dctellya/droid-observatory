# Staging: m4-merge-ui-and-403

Feature `m4-merge-ui-and-403`, worktree `observatory-wt/m4`, branch `wt/m4`. Slice of `m4-gate-ui-and-merge` (owner items 20/21).
Core red `ec6a0cf`, green `261037a`; shell red `160bea3`, green `f52bc0f`; UI and e2e cases `df26b36`; live walk specs and this file in the
closing commit.

## SPEC §9

- Row (F-NEW), the Merge press: `HumanAction::Merge { pr_number, target }` (store kind `merge`, state `requested` then `merged`,
  `refused` or `failed`, `target` text `pr:<n>`, event `approval.given`, overlay human, lane = the template's final lane). One press
  is one `human_actions` row, one `approval.given` event and one request. `control::merge::press(forge, pr, allowed)` (core) makes the
  request, picks the method from the policy's allowed set (squash, merge, rebase) and maps a 403 to `MergeState::Refused` with the
  architecture §4.6 text (`Merge of PR #n into <target> refused: token lacks pull_requests:write. Merge in GitHub; the badge updates
  from the next poll.`); no retry, no extra request, the poll continues at its interval. `Store::set_human_action_state` moves the row.
- Row (F-NEW), `GateFacts.target` (from `pr.base`), `GateGroup.target`, `MergeOffer { pr_number, target, label }` and
  `MergeConfirmation` (`Merge PR #n into <target>?`, `Confirm merge`, `Cancel`). A header that does not know its target offers no Merge.
- Row (F-NEW), config `[github].api_base` (default `https://api.github.com`, in `observatory.example.toml` and `DOCUMENTED_KEYS`);
  `GitHubClient::with_token` applies it. Needed for the e2e stub and the live walk proxy; production leaves it unset.
- Row (F-NEW), architecture §7 `merge` command: payload `{ mission_id, pr_number, target, confirm }`, answer `MergePressed
  { state, message }` (was `PrMergedByHuman` in the parent text). `confirm` is the dialog's answer: the first click only opens the confirmation in the
  window (no command call), and a call with `confirm: false` is refused without a request or a row. Shell: `merge.rs` `MergeDesk` (the open PR and allowed methods per mission,
  noted by the poll thread; the token is set once by the poll thread, no second keychain read), `press_merge` logs `merge_requested` and
  `merge_result` and records the overlay row through `actions::record`.
- Row (F-NEW), e2e: `lib/forge.ts` `startForgeStub` (loopback forge on 3147 answering the merge PUT 403) and `startForgeProxy` (reads go
  to api.github.com, the merge PUT gets the 403); `E2E_CONFIG_FILE` in `lib/env.ts` replaces the generated config; `specs/walk-*.walk.ts`
  are the live walk, not matched by the `.e2e.ts` glob. 3142-3149 are the mission's scratch ports; 3147 is now taken by the stub.

## Claims (rule 30)

- VAL-GATE-023: `gate.e2e.ts` "merge click opens a confirmation naming the PR and the target", PNG `confirm-dialog.png`; no request before
  the confirmation (`forge.requests == []`, no `PUT` in the app log); live: `live-confirm-dialog.png`, text `Merge PR #32 into demo/base?`;
  core `gate_view.rs::the_merge_confirmation_names_the_pr_and_the_target`.
- VAL-GATE-024: `gate.e2e.ts` "confirming the merge with the read-only token shows the refusal message", PNG `403-message.png` (exact
  text asserted, Merge button still shown, app alive); core `merge_press.rs::a_forbidden_merge_is_refused_with_the_exact_message_and_one_request`;
  live: `live-403-message.png`, one `PUT ... pulls/32/merge status=403` line.
- VAL-GATE-025: `gate.e2e.ts` "two presses of Merge after the 403 send two requests", PNG `two-presses-refused.png` (two `PUT` lines
  `status=403`, two `human_actions` rows kind merge state refused, two `approval.given` human-overlay events on lane `gate`); shell
  `merge.rs` test `two_presses_send_two_requests_and_leave_two_refused_rows`; live below (30 s cadence: polls at 16:03:48.770 and
  16:04:22.945, 34.2 s apart, after the two presses).
- VAL-GATE-026: live walk run 3, PR #32 merged by `dctellya` with `gh pr merge 32 --squash --delete-branch` at 16:03:57.7Z (GitHub
  `mergedAt` 16:03:55Z); next poll at 16:04:22.945Z logged `gate_changed ... badge="human_merged"` and the window read `handed to CD`
  30.8 s after the merge command started (interval 30 s, poll cycle 34 s). `live-handed-to-cd.png`: `handed to CD: c90d973 · PR #32 ·
  attestations none · merged by human`, Gate header `engineering cycle time: closed at handed_to_cd` and `lead time to production:
  delivery platform's clock: starts at c90d973; not computed here`. Store: `events` row `github:pr:32:handed_to_cd`, role human, lane
  `gate`, sha `c90d9734ba7f046b0b9e3a2b5203701d7bbfa977` = `gh pr view 32 --json mergeCommit`; `prs` row state merged, merged_by `dctellya`.
  The merger role is `human` (login is not the app's PAT); the External role rests on `tests/forge_merge.rs` (one login on this machine).
- VAL-GATE-033: run ids and PR numbers below.

## Live walk (2026-10-29, demo pair `demo/base` at 36f287b)

Setup, stated honestly: the app read the real GitHub through a loopback proxy (`[github].api_base`, `GITHUB_TOKEN="$(gh auth token)"`), and
the proxy answers the merge PUT with 403. The owner's read-only PAT is in the keychain, which no worker may open, so the 403 is the
proxy's, not GitHub's; one real read-only-PAT 403 is owner-run. The mission is the S fixture session attached (it has milestones, so a
fix cycle has a card to return to) with `branch_pattern = "demo/red-1"`; a hosted fake-droid mission has no milestone and so no card.
`demo-gate` uploads no attestation artifact, so the attestation list is empty (`attestations: 0`, `attestations none`).

- Pull requests: #30 (closed, aborted: spec selector), #31 (closed, aborted: hosted mission has no card), **#32** (the walk; merged,
  squash `c90d9734ba7f046b0b9e3a2b5203701d7bbfa977`, head `dddb459ccf5b90d895e636241b9506df623f157b`, branch `demo/red-1` deleted,
  `git ls-remote --heads origin 'demo/*'` lists only `demo/base`). #30 and #31 ran their own red and green runs (not listed by id here
  beyond the table below).
- Runs on the head sha of #32 (`gh api repos/dctellya/droid-observatory/actions/runs?head_sha=dddb459...`): `40907437311` (opened,
  cancelled by the label event), `40907442437` (labeled `demo:red`, **red**, failure), `40907509800` (after `--remove-label demo:red`,
  **green**, success). Aborted attempts: #30 runs `40906454250` failure, `40906454746` cancelled, `40906539262` success; #31 runs
  `40906727540` failure, `40906727779` cancelled, `40906965445` success.
- Run 1 (`/tmp/m4-walk1-report-keep/walk-commands.log`): PR opened 16:01:28Z with `demo:red`; `gate_changed ... badge="ci_red" pr=32
  ... requires_human=true merge_offered=false required_checks=["demo-gate=failed"]` at 16:01:49.580Z; card `cycle 1` and the return arc
  asserted; label removed 16:01:53Z; `gate_changed ... badge="ci_green" ... merge_offered=true required_checks=["demo-gate=passed"]` at
  16:02:22.989Z; Merge shown; `human_actions` empty and every proxy request a GET. PNGs `live-red-cycle-1.png`, `live-green-merge.png`.
- Run 2 (`merge_policy = "policy"`, PR open and green): `gate_changed ... badge="ci_green" pr=32 decided_by=Some(ConfigOverride("policy"))
  requires_human=false merge_offered=false`; no Merge button (`live-policy-no-merge.png`). This is VAL-GATE-016's live evidence.
- Run 3: two presses of Merge, each answered `Merge of PR #32 into demo/base refused: token lacks pull_requests:write...`; app log:
  `human_action ... action_id=1 kind="merge"`, `merge_requested ... action_id=1`, `github.request method="PUT" ... pulls/32/merge status=403`,
  `merge_result ... state="refused"`, then the same for `action_id=2`; store `human_actions`: `1|merge|refused|pr:32`, `2|merge|refused|pr:32`;
  `events`: `app:1:0|approval.given|human|gate|human|app`, `app:2:0|...`. Then the owner merge above.
- Commands: `E2E_RUN_DIR=/tmp/m4-walkN E2E_REPORT_DIR=/tmp/m4-walkN-report E2E_CONFIG_FILE=/tmp/walk/config-{auto,policy}.toml npx wdio run
  wdio.conf.ts --spec specs/walk-N-*.walk.ts --mochaOpts.timeout 600000` (wall 1m10.6s, 36.6s, 1m12.8s).

## Decisions

None.

## BACKLOG

- No UI lists `human_actions`; a refused merge row is visible only in the store and the Gate lane's overlay events.
- The merge answer (`merge-result`) is held in window memory (board context); a reload shows the Merge button without it.
- The UI and its e2e cases landed in one commit (`df26b36`) with the e2e spec as the executable test, not red first; the core and the
  shell halves have separate red commits.
- Contract text for VAL-GATE-025 reads `lane=7`; the store keeps lane ids since migration 0002, so the row reads `lane='gate'`.

## Deferred clauses

- A real 403 from the read-only PAT (keychain, off-limits to workers): the proxy stands in; owner-run.
- External merger role live (one login on this machine): unit test only.
- Attestation ids on the badge: the demo workflow uploads none; the list is empty and says so.

## Library

- The app log is the newest `wdio-*.log` of the report directory (`captureBackendLogs`); `appLog()` reads it.
- A hosted fake-droid mission cannot show a fix cycle (no milestone event, so no card); attach the S fixture session and name the head
  branch literally in `branch_pattern` for a live Gate walk.
- `gh pr create --label demo:red` opens two runs (opened, labeled); the first is cancelled by the concurrency group.
