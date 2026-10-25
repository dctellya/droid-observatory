# m3-control-core staging

## SPEC §9

- F-028 amendment applied: every control is exactly one event with `overlay = Human`, `role = Human`,
  `source = App { action_id }` on the template's first lane (pause, resume, message, answer, mark complete, mark
  skipped) or the request's lane (approve, deny). Tests renamed as the amendment says. Merge (M4) is not a
  `HumanAction` variant yet; it takes `template.final_lane()` when it lands.
- Host acks (orchestrator note after m2-host-shell-and-launch): `control::ack_event` builds the overlay event of the
  host's acknowledgement (`SourcePtr::Rpc` of the ack line of the spool, arrival-time `at`, id
  `<session>:<offset>:2^30`). Kinds: pause `mission.paused`, resume `mission.resumed`, message and mark skipped
  `intent.amended`, answer `intent.answered`, approve `permission.granted`, deny `permission.denied`; mark complete has
  no host ack. `control::dedupe_paused` drops the ack-derived `mission.paused` when the stream carries its own before
  the next `mission.resumed`. The `intent.amended` choice for message and skip is mine: no new kind is invented
  (architecture §8) and the PRD names no kind for a message.
- The contract pass can reword VAL-CTRL-003/006/007 to the ack events above (ids from `ack_event`, `at` = arrival
  time of the ack).
- `FeatureActivity` gained `repeated` and `child_growing` (the architecture shows three fields): the runbook table
  needs "HookBlocked repeated" and "SubagentSpawned with no child growth", which the last kind alone cannot say.
  `stall::activity_of` fills `repeated` and leaves `child_growing = false` (a growing child has events of its own,
  so the spawn is not the last event); the UI feature may feed the child-file growth from the watcher.
- `stall::activity_of` treats a feature as finished (never idle) when its last event is `task.done`, `task.skipped`
  or an app event naming the feature (mark complete or skipped).

## Decisions

none

## BACKLOG

- Wire `control::relay` for answer, approve and deny: they need the open request's JSON-RPC id from
  `OpenRequests`; `relay` returns `None` for them and the host layer builds the `HostControl` (m3-controls-ui-and-stall).
- `docs/runbooks/stalled-mission.md` has no per-prompt headings yet; the five anchors in `RunbookPrompt`
  (`#waiting-on-human`, `#worker-silent`, `#hook-blocked`, `#integration-conflict`, `#long-tool-call`) resolve only
  after VAL-CTRL-017 (m3-controls-ui-and-stall).

## Deferred clauses

- Live checks VAL-CTRL-002 to -010, -013, -015, -016 need the Tauri command `human_action` and the stall emitter
  (m3-controls-ui-and-stall, m3-e2e-harness); this feature is core only.
- State transitions of `human_actions` for merge (`requested` to `merged` or `refused`) are M4.

## Library

- `core::control`: `record` (event of a control), `ack_event` (event of the host's ack), `dedupe_paused`, `relay`
  (feature `rpc`), `stall::{threshold, evaluate, activity_of}`. `Store::record_human_action` writes the row and the
  event in one transaction; `Store::human_actions` reads rows. `human_actions` has no text column.
