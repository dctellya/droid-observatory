# D-034: A hosted mission is launched as an orchestrator session in mission mode, not with `--mission`

**Date:** 2026-10-24
**Status:** accepted and implemented in `m2-host-shell-and-launch` (`apps/observatory-app/src-tauri/src/{process,host}.rs`, core `view::{launch,hosted,live_output}`)

## Context

Architecture §4.5 and VAL-HOST-005 name the command `droid exec --mission --input-format stream-jsonrpc
--output-format stream-jsonrpc ...`. Droid 0.247.0 rejects it:

- `--mission` cannot be combined with `--input-format stream-jsonrpc`; `-f/--file` is an error in streaming mode.
- `--worker-model` and the other per-role flags need `--mission`, so they cannot be given either.
- In stream-jsonrpc mode `-m/--model`, `--auto` and `-r/--reasoning-effort` are validated but ignored (stderr:
  "these flags have no effect with --input-format stream-jsonrpc", seen in the hosted run's `.stderr.log`).

D-033 settled the wire format. This decision settles how a mission is started over it, from a live run on the
owner's machine (Haiku for all roles, scratch repo under `target/scratch`).

## Decision

1. The host spawns `droid exec --input-format stream-jsonrpc --output-format stream-jsonrpc --auto high --model
   <orchestrator model> --reasoning-effort <effort>` with the repository as working directory, as the leader of
   its own process group. The flags are kept because Droid validates them; they carry the form's values but the
   protocol carries the settings.
2. The host writes `droid.initialize_session` with `machineId`, `cwd`, a host-chosen `sessionId` (UUID),
   `modelId`, `reasoningEffort`, `autonomyLevel`, `interactionMode: "mission"`, `decompSessionType:
   "orchestrator"` and `missionSettings {workerModel, workerReasoningEffort, validationWorkerModel,
   validationWorkerReasoningEffort}`.
3. When the response arrives the host sends `droid.add_user_message` with the mission file's text, the line
   `Mission branch name: <name>` and, when set, the budget line.
4. The orchestrator then raises `droid.request_permission` for its `ProposeMission` tool. The request has no
   `confirmationType`; it is recognised by `toolUses[].toolUse.name == "ProposeMission"` (options
   `proceed_once`, `proceed_always`, `cancel`). The launch is the human's act of proposing, so the host answers
   it with `selectedOption: "proceed_once"` and logs `host_auto_approve`. Any other permission request stays open;
   the host control channel can approve or deny it, the launch screen has no button for it yet.
5. Controls map to JSON-RPC as follows, with string ids from a counter:
   pause is `droid.interrupt_session`; resume and message are `droid.add_user_message`; an answer is the
   response to the open `droid.ask_user` request with `{answers: [{index, question, answer}]}`; approve and deny
   are the response to the open `droid.request_permission` with `{selectedOption}`.

## Alternatives

- `droid exec --mission ... -f mission.md` as architecture §4.5 says: refused by Droid (above).
- Run `--mission` without stream-jsonrpc and attach to the session file: no controls, which is FR-L3.
- Auto-approve every permission request: hides the human's decisions; only the proposal is the launch itself.

## Consequences

- VAL-HOST-005 cannot be met literally. The process listing shows the command of point 1; the settings are in
  the logged initialize request (`host_control` line).
- A message sent while a turn runs is queued by Droid and discarded by a pause (`queued_messages_discarded`,
  chatter). The UI lists what was written and what Droid acknowledged, so the difference is visible.
- Stream notifications carry no timestamps; events mapped from them inherit the previous event's `at`
  (a pause or an answer has the same `at` as the line before it). Not fixed here.
- The process group is reaped on window close: SIGTERM goes to the group, then SIGKILL after
  `[mission].reap_grace_ms` if members remain; `host_reaped` is logged. The process is stopped by the host, not
  left to finish.
