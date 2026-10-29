# Staging: m4-kit-pending-fixes

Feature `m4-kit-pending-fixes`, worktree `observatory-wt/m4`, branch `wt/m4`. Leftovers of `m4-ui-kit-lint` (978bf5f).

## SPEC §9

- Row (F-NEW), owner Q4 ruling applied to `.card`: `ui/style.css:48` `.card` is back to the kit's `padding: 12px 14px; gap: 8px`
  (it was `6px 10px` and `4px`, found by `make check-kit` in `m4-ui-kit-lint`). Its two F-NEW rows are gone from
  `kit-departures.md`, which now lists the F-105 rows (`.observatory`, `.library`, `.details`) and the F-103/F-104 rows
  (`m4-gate-group-ui` reverts those). This closes the "approve or revert" question of the `m4-ui-kit-lint` row: reverted.
- Row (F-NEW), ten mono fixes (kit rule 2): `class="mono"` on `agents.rs` `agent-stream-rows` and `agent-stream-markers`,
  `attach.rs` session id `<code>`, `controls.rs` feature mark `<span>`, `launch.rs` `hosted-pid`, `hosted-session`,
  `<code>{ack.request_id}</code>` and `live-output-total`, `raw.rs` `raw-line-kind` and `raw-line-event`; their ten lines are
  deleted from `kit-pending.txt`, which keeps the five UT round 3 heading-count lines for `m4-gate-group-ui`.
  `make check-kit`: `check-kit ok VAL-UI-001 [5 departures recorded; pending: 5 known mono violations in kit-pending.txt]`.
  Claims (rule 30): none; VAL-UI-001 had already passed and is not in this feature's `fulfills`.

## Decisions

None.

## BACKLOG

None.

## Deferred clauses

- The launch view's `hosted-pid`, `hosted-session`, `live-output-total` render only in the Tauri host (the browser host shows
  "Launch not available in browser host"), so their mono face is proven by `make check-kit` and the class in the source, not
  by a browser screenshot.

## Library

None.
