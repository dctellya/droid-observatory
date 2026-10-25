# Staging: m3-hosted-todo-state

Feature `m3-hosted-todo-state` (owner M2-boundary answer item 2), worktree `observatory-wt/m3`, branch `wt/m3`.
VAL-HOST-014 (restored wording). Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): VAL-HOST-014 reword reverted on owner instruction; fixed in the mapper. The hosted stream has no
  counterpart of Droid's `todo_state` record (round 2 of `m2-host-shell-and-launch`: hosted 18, attached 21 = 18 +
  `usage.reported` 1 + `plan.revised` 2), so `RpcMapper::with_session_files(&cfg.ingest)` also reads the hosted
  child's session file (`<factory_sessions_dir>/<cwd-slug>/<session-id>.jsonl`, found by the session id of the
  `droid.initialize_session` response) and maps its `todo_state` records through the same mapping v1 state as the
  stream. The events keep the session file's `Jsonl` pointers, so they have the ids the attach path gives the same
  lines. The contract keeps its wording: hosted equals attached minus `usage.reported`. Alternatives considered:
  exclude `plan.revised` in the contract (rejected by the owner); rebuild a `todo_state` record from the
  `TodoWrite` `tool_use` input in the envelope (rejected: id collision on sub 0 with the message's own event, and
  the file's `todos` shape was not probed); a separate `Mapper` for the file records (rejected: it loses the
  model and feature state, so the event would differ from the attached one).
- Row (F-NEW): ordering and dedupe of file records. A `todo_state` record is mapped before the first stream line whose
  own time is at or after the record's time (a line with no time of its own, a turn outcome, waits for the next one
  that has), and the rest at `finish`. Mapping v1 reports a plan only when it differs from the one before, so a plan
  carried by both the stream and the file, or a file read twice (Droid rewrites its first line in place and the tail
  reads the file again), maps once. Ids are `<session>:<file offset>:0`, equal to the attach path's, so the store
  holds the event once however many readers send it (`ON CONFLICT (id) DO NOTHING`).

## Decisions

- D-033 (amend, one line in Consequences): "The hosted mapper also reads `todo_state` from the child's session file
  (`RpcMapper::with_session_files`, `m3-hosted-todo-state`); the stream has no notification for it. The spool alone,
  without the session file, still lacks `plan.revised`."

## BACKLOG

- B-NEW: id spaces of the spool and the session file overlap. A stream event is `<session>:<spool offset>:<sub>`
  and a file-sourced `plan.revised` is `<session>:<file offset>:0`; the same numeric offset in both is a silent
  `ON CONFLICT DO NOTHING` drop of the later event. Not reproduced (the offsets differ by construction in every
  run so far); the same hazard exists when the attach watcher and the host read one session. Fix: put the pointer
  kind in the id, a migration of the id format and a golden regeneration, so not inside this feature.
- B-NEW: the idle gap. The `plan.revised` of the last turn appears at the next stream line that carries a time or at
  `finish`, not when the file is written, because the mapper runs only when a stream line arrives. A host timer
  calling a `RpcMapper::poll_session_file` would close it.
- B-NEW: other file-only records. The `rpc` tests note that `compaction_state` also has no known notification
  (`context.compacted`). Not touched and not probed; the same `SessionFile` reader could carry it after a probe of a
  hosted run that compacts.

## Deferred clauses

- VAL-HOST-014 asks that the assertion be re-graded from "a hosted scratch run whose orchestrator calls
  `TodoWrite`, with both replays pasted". This feature did not host a scratch mission (F-042, cost, and the shared
  daemon). The offline proof uses the round 2 evidence still on disk, scratch mission 386a27bd (spool and session
  file, both with two `todo_state` records), replayed by the CLI built from this tree:
  `observatory-cli replay ~/.droid-observatory/hosted/386a27bd-035c-4ffd-a981-dd2026e2349c.jsonl` prints 20 events
  and `unknown events: 0`; `observatory-cli replay --root-only ~/.factory/sessions/<cwd-slug>/386a27bd-....jsonl`
  prints 21 events and `unknown events: 0`. Per kind both have file.touched 10, intent.answered 1, intent.created 1,
  mission.paused 1, plan.created 2, plan.revised 2, question.asked 1, tool.error 2; the attached side has
  `usage.reported` 1 more. The validator's live re-grade runs a new hosted scratch mission.

## Library

Line for `library/hosted-jsonrpc.md`:

- The stream carries no `todo_state` (verified round 2 and again 2026-10-25 on the scratch spool 386a27bd: zero
  unknown rpc events and 2 `plan.revised` only in the session file). Since `m3-hosted-todo-state` the hosted
  `RpcMapper` built `.with_session_files(&cfg.ingest)` (host pump and `observatory-cli replay` of a spool) reads those
  records from `<factory_sessions_dir>/*/<session-id>.jsonl` and emits the attached path's `plan.revised` events with
  the file's pointers and ids. The host names the launch after the session, so launch id and session id are equal.
  `usage.reported` (sidecar) is still file-only: hosted equals attached minus `usage.reported`.
