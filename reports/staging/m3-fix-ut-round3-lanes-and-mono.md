# Staging: m3-fix-ut-round3-lanes-and-mono

Feature `m3-fix-ut-round3-lanes-and-mono` (round-3 fix, owner feedback 2026-10-25 item 2), worktree
`observatory-wt/m3`, branch `wt/m3`. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): VAL-CYCLE-003: `crates/observatory-core/tests/cycle_template.rs` asserts the `four_step` template's lane
  count once (`template.lanes.len() == 4`, the template content check of VAL-CYCLE-001, in
  `a_second_template_with_four_lanes_maps_every_event_into_one_of_them`). Every board, digest, lane_events,
  lane_cards and lane-count comparison now uses `template.lanes.len()`. Beyond the three lines named in the
  feedback, `:291` (`counts.len() == 4`) and `:405` (`counts.len(), 4`) had the same literal.
- Row (F-NEW): VAL-UI-001 rule 2 (ids, event kinds and timestamps in mono): closed. The kit's `.mono` class
  (`theme.css:33`) is on the Details timestamp, event kind and model id cells (`details.rs`), the Panel declared and
  observed model cells (`panel.rs`), the Agents feature id, model and declared-model cells (`agents.rs`) and the
  Library date cell (`library.rs`). No CSS changed; `theme.css` is byte-identical to the kit.
- Row (F-NEW): proposed, owner approval at the M3 boundary: ui-kit-v2 rule 6 (table density, the kit's
  `.lib td { padding: 10px }`). Departure 1: `ui/style.css:9`, `.library th, .library td { padding: 6px 8px }`.
  Screen: Library. Reason: row density, the library lists every opened capture and the seven-lane board sits one
  click away; 10 px rows halve how many captures show without scrolling. Alternative: remove the override and take
  the kit's 10 px (rejected by the owner for now; the override stays). Cost: none.
- Row (F-NEW): proposed, owner approval at the M3 boundary: ui-kit-v2 rule 6, departure 2: `ui/style.css:35`,
  `.panel th, .panel td, .agents th, .agents td, .details th, .details td { padding: 3px 8px }`. Screens: Panel,
  Agents, Details. Reason: row density on the seven-lane board page, where the Panel, the agents table and the
  Details timeline (hundreds of rows) share one viewport with the lanes; 10 px rows would push the lanes below the
  fold. Alternative: remove the override (rejected by the owner for now). Cost: none.

## Decisions

None.

## BACKLOG

None.

## Deferred clauses

- Kit rule 7 (minor): the ThemeSwitch (`ui/src/controls.rs:428-432`) does not mark the active theme with the kit's
  `.switch button.on` class (`theme.css:91`). The three buttons render without an active marker. Deferred to the
  next UI feature that touches `controls.rs` (needs a signal holding the chosen theme; `set` currently only writes
  `data-theme` on the root).

## Library

None.

### Browser-host evidence (S fixture, trunk on 3141, computed `font-family`)

- Details timestamp `2026-10-22T15:30:13.542Z`, event id, event kind `milestone.defined`, model id
  `claude-fable-5.1`: `"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, monospace`.
- Panel declared model `claude-fable-5.1` and observed models: same mono stack.
- Agents model cells `claude-fable-5.1` (observed and declared): same mono stack; the feature id cell is empty on the
  S fixture and carries `class="mono"`.
- Library date `2026-10-22`: same mono stack; the name cell stays `"IBM Plex Sans", ...`.
- Screenshots: `target/scratch/m3-fix-round3-mono-details.png`, `target/scratch/m3-fix-round3-mono-library.png`.
