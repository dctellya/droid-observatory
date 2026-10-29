# Staging: m4-wire-dedupe-paused

Feature `m4-wire-dedupe-paused` (owner Q5, M3 boundary), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW), closes owner Q5 and BACKLOG B-093: `control::dedupe_paused` (F-072) is wired into the hosted pause path. A hosted
  Pause leaves two `mission.paused` rows, the ack-derived one (`control::ack_event`, id `<session>:<offset>:1073741824`) and
  the stream's own (the cancelled turn the runner reports after its response); the app's own action row (`app:<n>:0`, human
  overlay, and its `human_actions` row) is a third row of another kind and stays. The ack arrives first, so the rule cannot
  run at write time. It runs after a stream `mission.paused` is stored: `Store::retract_echoed_pauses(mission)` runs
  `dedupe_paused` over the mission's `mission.paused` and `mission.resumed` events (ack ids named by the new
  `control::is_ack`) and deletes the ack rows the stream reports before the next `mission.resumed`. The host pump
  (`host.rs` `store`) calls it, drops the ack from the batch it has not flushed, and tells the UI through the new
  `EventBatch.retracted_events` (serde default, so older payloads read) which `merge` applies with `ingest::drop_events`.
  Option chosen from B-093: the host-side retract, not a view-time filter: the store then holds one row, so replay,
  Details and the Panel counts agree. No store schema change, no migration, no threshold (so nothing in
  `config/observatory.example.toml`). Alternative considered: a view-time filter in the board fold (rejected: the store would
  still hold two rows and every reader would need the filter). A lone ack (the stream never reports the pause) is kept.
  Evidence: `crates/observatory-core/tests/hosted_pause.rs`, `tests/event_batches.rs`.

## Decisions

None.

## BACKLOG

- B-093 is closed by this feature (done, M4).
- New: `Store::retract_echoed_pauses` handles the order ack then stream, as `dedupe_paused` does. If a runner ever reports the
  cancelled turn before the response to the control request, the ack row stays; the live check (`live_check.rs`) shows
  the order on 0.247.0 and the harness has not seen the other.

## Deferred clauses

None.

## Library

- The hosted spool's events carry the session id of the init response; `Store::record_hosted` stores the launch id as the
  session row. A test that stores mapped hosted events must use the launch id as the `sessionId` of its lines, or the events
  row fails its foreign key.
