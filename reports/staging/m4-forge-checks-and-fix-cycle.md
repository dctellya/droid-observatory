# Staging: m4-forge-checks-and-fix-cycle

Feature `m4-forge-checks-and-fix-cycle` (VAL-GATE-012, -017, -020, -021; VAL-GATE-018 store and log rows, its e2e PNG is
`m4-gate-group-ui`), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW): a finished Actions run or commit status is one event, `check.failed` or `check.passed`, role `external`, lane
  from `template.lane_of` (Gate in the default template), `at` = the run's or status's own `updated_at` (the poll time when the
  forge gives none), pointer `SourcePtr::GitHub { url }` = the run's `html_url` (a status without `target_url`: its status API
  URL), so the run id is in `events.url`. The id names the run (`github:check:<pr>:<run_id>:<failed|passed>`) or the status
  context on the head (`github:check:<pr>:<sha>:<context>:<verb>`): a restarted source records the same event, never a new one.
  Running and queued results make no event (`check.started` is not emitted here). `Poll.windows` carries the window changes.
- Row (F-NEW), migration `0003_gate_windows.sql`: `rework_windows` gains `pr_number` and `check_name` (nullable, other causes
  leave them empty) and a partial index on the open ones. `Store::record_poll` applies `WindowChange::Open` (a window per
  failing check, none when one is open for that PR and check or this event opened one already) and `Close` (the green run of
  the same check sets `closed_by`) after the events they name; `missions.state` reads `fixing` while a gate window of the
  mission is open and `at_gate` after the last closes. `rework_windows.feature_id` of a gate window holds the card (the
  milestone) the fix cycle returns to: the check names no feature. The window needs a card: `GitHubSource::set_affected_milestone`
  (the shell sets it before each poll from `Store::affected_milestone`: the milestone of the latest `integration.merged`, else
  of the latest event that names one); with none the check events still land in Gate and no window opens.
- Row (F-NEW), view: `Card.cycle` (serde default 0) counts fix cycles. A `check.failed` on a card that is not fixing sends it to
  `template.home(Role::Worker)` (the lane the template gives the worker role, Build in the default template; no lane name in
  code) and adds one; a second red while fixing is the same cycle; the next `check.passed` of a fixing card puts it back in the
  template's final lane. `GateGroup::of(events)` is the Gate header of a list of events. The badge states were already
  there (`CiRed`, `CiGreen`).
- Row (F-NEW), shell: the GitHub poll thread logs `gate_changed{mission_id, badge, pr}` and emits the `gate_changed` window
  event (payload `GateGroup`) when the badge or PR number changed since the last poll, and once at the first poll (`pending`
  before a PR). This is the app-log line VAL-GATE-017 and -021 read (and VAL-GATE-007 for `pending`).
- Evidence (live, 2026-10-29, read-only, `GITHUB_TOKEN="$(gh auth token)"`, ignored test
  `the_live_fix_cycle_is_recorded_in_a_store_file` against PR #24 `demo/smoke-1 -> demo/base`, closed): request lines
  `pulls?head=dctellya:demo/smoke-1&base=demo/base&state=all`, `actions/runs?head_sha=1d994af0...`, `commits/1d994af0.../status`,
  all `status=200`; store `checks`: `demo-gate | actions_run | passed | .../actions/runs/40853894595`; event
  `github:check:24:40853894595:passed | check.passed | gate | github`; badge `ci_green`; `gh api .../actions/runs?head_sha=` lists
  run 40853894595 `success`, 40853828900 `failure`, 40853828337 `cancelled`. The red-to-green transition on a live PR was NOT run
  (see Deferred clauses).

## Decisions

None.

## BACKLOG

- Droid-Shield blocks every `git push` from the worktree: it flags `crates/observatory-core/tests/forge.rs:33` (`const TOKEN`),
  `crates/observatory-core/tests/secrets.rs:7` and `:13` (`github_pat_` fake values). The values were committed by
  `m4-forge-core-and-pr-detection` and `m4-forge-token-env-first`; the fix (compose the fake at run time, or a placeholder the
  scanner accepts) needs the owner's go. Until then no demo branch can be pushed from this worktree, which blocks the live walk
  of VAL-GATE-017/-020/-021 here and for anyone who pushes `demo/*` from it.
- A gate window is per PR and check, but the view has no check name: two checks red at once count as one cycle and the first
  green clears `fixing`. The required-checks slice (`m4-forge-rulesets-and-merge`) owns required versus optional checks and can
  carry the name into the events if the board needs it.
- `rework_windows.feature_id` holds a milestone for `cause = 'gate'`; M5 may want a `milestone_id` column when it reads the table.
- A card that has no `integration.merged` yet and no events gets no window (no affected milestone); the check events still show
  the badge. A PR that spans several milestones is attributed to the one integrated last.

## Deferred clauses

- Live red-to-green walk (label `demo:red`, unlabel, same PR number throughout) on a pushed demo branch: blocked by the
  Droid-Shield push block above. The unit tests replay the recorded red and green runs of PR #24 (the same head, run 2 failed,
  run 3 passed) through the real source, store and view.
- `gate_changed` in the Tauri window and the app log of a hosted mission was not run (the window was not started); the
  line is covered by `GateGroup::of` and by the shell compiling and linting.
- VAL-GATE-018 e2e PNG `card-lane3-cycle-1` is `m4-gate-group-ui`; the store rows (`rework_windows` one open row,
  `missions.state = 'fixing'`) and the board `cycle` it reads are here.
- Coverage of `observatory-core` (80 %) runs once at the integration.

## Library

- `GitHubSource` memory is only the set of announced event ids; the store makes every write idempotent, so a restart re-announces
  the current state (a failing run opens no second window, a green one closes the open one).
- The recorded PR #24 (`fixtures/github/demo-smoke-1/runs.json`) is a red-to-green pair on one head: dropping run 3 from the
  body gives the red poll, the whole body the green one (`runs_while_red` in `tests/forge.rs`).
- `scripts/contract-checks/g2-tests.tsv`: rows for `red_required_check_opens_a_fix_cycle_on_the_affected_card` and
  `fix_cycle_keeps_the_same_pr` flipped to `yes`; the other three named gate tests stay `no`.
