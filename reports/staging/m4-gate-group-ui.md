# Staging: m4-gate-group-ui

Feature `m4-gate-group-ui`, worktree `observatory-wt/m4`, branch `wt/m4`. Slice of `m4-gate-ui-and-merge` (owner items 20/21).
Red commit `5a2919b`, green commit `9481d87` (core), shell `0249217`, UI `4723ad8`, e2e `a5efd55`.

## SPEC §9

- Row (F-NEW), the Gate group on the Gate lane (the template's final lane) and in Details: `GateGroup::view(&template)` in
  `crates/observatory-core/src/view/gate.rs` returns `GateGroupView` (badge text, PR, required checks with their conclusion,
  `reviews: N`, `ruleset: <name>` or `decided by: ...`, `attestations: N`, the merge offer, the `handed to CD` reading
  `handed to CD: <sha7> · PR #<n> · attestations <ids|none> · merged by human|policy`, and one line per clock of the final lane:
  `closed at <final event>` for the clock with an end, `starts at <sha7>; not computed here` for the platform's clock).
  `ui/src/gate.rs` renders it as is. Counts of 0 are shown (`reviews: 0`, `attestations: 0`).
- Row (F-NEW), the fix cycle's return arc: `Card::return_arc(&template)` (core, `card.rs`) is `from` the template's final lane
  `to` `template.home(Role::Worker)` with `cycle` and `cycle N`, `active` while no green check has come since the red one. The card
  keeps `cycle N` after the green check (the arc turns from amber to muted). Nothing names a lane: `LaneId`s and
  `template.name_of`, no `7`, no enum path.
- Row (F-NEW), Merge visibility is `GateGroupView.merge` (`Some` only when `GateGroup.merge_offered`, which `with_facts` already
  computed: PR open, a human decides, every required check passed, badge `CI green`). `GateGroup::with_polled` overlays the poll on
  the events a scrubber keeps and withholds Merge unless those events are green too. The button is drawn and has no handler: the
  confirmation, the press and the 403 are `m4-merge-ui-and-403`.
- Row (F-NEW), `gate_changed` now carries `GateChanged { mission_id, gate }` (was the bare `GateGroup`) and the shell keeps each
  mission's last header in `GateCache`, read by the new command `gate_group(mission_id)`, so a board opened after the change
  still shows the group. The `gate_changed` log line lists `required_checks` (`name=conclusion`), `reviews`, `ruleset`,
  `attestations` and `attestation_ids` (VAL-GATE-014). Architecture §7 command list gains `gate_group` (orchestrator to
  record).
- Row (F-NEW), e2e: `--features e2e` builds the command `e2e_gate_poll(scenario_name)` (`red`, `green`, `merged`) that feeds a
  scenario of PR #24 through `apply_poll` (store `record_poll`, `events_appended`, `gate_changed`) on the first live mission.
  The release build has no such command. `lib/env.ts` sets `[github] poll_interval_s = 3600` so a real poll cannot race it.
  This is a stand-in for the GitHub transport, not a recorded GitHub answer: the live walk (`m4-forge-live-walk-2`) proves the
  real poll, this proves the window.
- Row (F-NEW), kit work: heading count spans `agent-count`, `overlay-human-count`, `tray-count`, `details-milestone`,
  `details-row-count` take `class="mono"` (their five lines are gone from `kit-pending.txt`, now comments only); ThemeSwitch
  marks the active choice with `.on` (`current` signal, kit rule 7); every page (library, board) starts with the kit's page head
  (`.pagehead`, `.eyebrow` breadcrumb, h1 title, mono muted subtitle; `ui/src/pagehead.rs`, F-106); the two padding overrides
  at `style.css` (`.library th, .library td` 6px 8px and the Panel/Agents/Details `th, td` 3px 8px) are back to the kit's
  `10px` (F-103/F-104 reverted, their two rows deleted from `kit-departures.md`). `make check-kit`:
  `check-kit ok VAL-UI-001 [3 departures recorded]`.

## Decisions

None. Proposal (owner Q4, for the integrator to put to the owner): one kit variant class, not a local override. If a denser
table is wanted, add to the kit's `theme.css`:

```css
.lib.dense th, .lib.dense td { padding: 6px 10px; }
```

and put `dense` on every `.lib` that lists rows of events or sessions (Library, Panel roles, Agents, Details timeline), never on
KPI or permission tables. Reason: at the kit's 10 px cells fewer rows fit before the Panel, Agents table (capped at 28vh) and Details
timeline scroll; the variant gives the owner the denser reading as a kit choice. Not measured here. Cost: one rule in
the kit file and a kit version bump (`ui-kit-v3`), the kit's README rule 6 gains "or `.dense`", and `check-kit` learns the
selector. Not applied here.

## BACKLOG

- Merge has no handler yet: `m4-merge-ui-and-403` owns the confirmation naming PR and target, the press and the 403 message.
  The button's `data-testid="merge"` and `aria-label="merge-pr-<n>"` are what that slice's cases click.
- The browser host never receives `gate_changed` (no shell), so a browser-host board shows the pending placeholder; the group is
  proven in the Tauri window (`gate.e2e.ts`) and by `crates/observatory-core/tests/gate_view.rs`.

## Deferred clauses

- VAL-GATE-018's rework-window row (`rework_windows` open, `missions.state = 'fixing'`) is the store's `m4-forge-checks-and-fix-cycle`
  test (`red_required_check_opens_a_fix_cycle_on_the_affected_card`); this feature supplies only the e2e case and PNG
  `card-lane3-cycle-1`. The e2e poll passes no `WindowChange`, so it does not write `rework_windows`.

## Library

- `e2e_gate_poll` and `[github] poll_interval_s = 3600` in `apps/observatory-app/e2e/lib/env.ts` are how window cases get PR data
  without GitHub. Run one spec: `cargo tauri build --debug --no-bundle --features e2e` from the repository root (from the app
  directory trunk cannot find `ui/Trunk.toml`), then `cd apps/observatory-app/e2e && E2E_RUN_DIR=<dir> npx wdio run wdio.conf.ts --spec specs/gate.e2e.ts`.

**Claims (rule 30)**

- VAL-GATE-014: `crates/observatory-core/tests/gate_view.rs::the_gate_group_lists_checks_reviews_ruleset_and_attestations` (7 passed);
  `apps/observatory-app/src-tauri/src/github.rs` `announce_gate` logs `required_checks`, `reviews`, `ruleset`, `attestations`,
  `attestation_ids` in `gate_changed`; the window shows them in `gate.e2e.ts` "the gate group lists the required check, reviews,
  ruleset and attestations". The sqlite counts and `gh` comparison are the live walk's (`m4-forge-live-walk-2`).
- VAL-GATE-018 (not in this feature's claim list, e2e case written here as the Pass 27 note asks): `gate.e2e.ts` "red check moves
  the affected card to the build lane with cycle 1", PNG `card-lane3-cycle-1.png`.
- VAL-GATE-019: `gate.e2e.ts` "red check draws the return arc with cycle 1", PNG `return-arc-cycle-1.png`;
  `gate_view.rs::red_required_check_draws_a_return_arc_from_the_final_lane_to_the_build_lane`;
  `tests/forge.rs::red_required_check_opens_a_fix_cycle_on_the_affected_card` is the store-side one.
- VAL-GATE-022: `gate.e2e.ts` "merge button appears only when the checks are green and a human is required", PNG `merge-button.png`;
  `gate_view.rs::the_merge_offer_follows_the_policy_and_the_checks_through_the_view`;
  `tests/forge_merge.rs::merge_button_shows_only_when_policy_is_human_and_checks_are_green`.
