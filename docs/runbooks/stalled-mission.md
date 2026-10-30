# Stalled mission

**Symptom:** a feature shows no active time for longer than
`[intervention].stall_threshold_min` (default 20 minutes, `config/observatory.example.toml`),
or the orchestrator has been waiting on a question nobody saw.

## Confirm

1. Open the mission on the board. A stalled feature carries the stall flag on its card
   (`stalled`), and the stall banner above the controls names the feature, how long it has been
   idle and the prompt below that matches what it last did.
2. Follow the prompt to its section of this runbook: [Waiting on human](#waiting-on-human),
   [Worker silent](#worker-silent), [Hook blocked](#hook-blocked),
   [Integration conflict](#integration-conflict) or [Long tool call](#long-tool-call).
3. The app polls every `[intervention].stall_poll_s` seconds and writes a `stall_flag` line to its
   log when the flags of a mission change. Nothing under the threshold is flagged.

If the app is not running, open the orchestrator's session file and read the timestamp of its
last event; the section to follow is the one for that event's kind.

## Waiting on human

**Prompt:** "Waiting on human: answer it". The last event is `question.asked` or
`permission.requested`.

- Hosted mission (started from the app): answer in the AskUser sheet of the control bar, or press
  Approve or Deny on the permission request. The control is written to the mission and recorded
  as a human overlay event.
- Attached mission (watched from its session file): answer in the terminal that runs the mission.
  The app shows the text to copy to the terminal and still records the event.

## Worker silent

**Prompt:** "Worker silent: check the child file, retry or skip". The last event is
`subagent.spawned` and the child's session file does not grow.

Check the child session file. If it is silent, send a message to the orchestrator from the
control bar (hosted) or the terminal (attached) asking it to retry the task, or press Skip on the
feature's card with a reason: the orchestrator receives the skip as an instruction.

## Hook blocked

**Prompt:** "Agent hits the write guard: forward-only instruction". The last events are
`hook.blocked`, repeated.

Read `build-log/<day>/<session>.jsonl` for the command. The agent needs the forward-only
instruction, not a bypass: send it as a message.

## Integration conflict

**Prompt:** "Two worktrees touched one file". The last event is `integration.conflict`
(rule 13 violated).

The orchestrator resolves it at integration. If it loops, stop and split ownership in the spec.

## Long tool call

**Prompt:** "Long tool call: wait one more threshold, then pause and inspect". Nothing for a long
time and the process is alive: a build or a test run.

Wait one more threshold. Then press Pause in the control bar, inspect the process and press
Resume.

## Stop and involve the human when

- The suggested action is to change scope, model or threshold (decision file).
- The stall repeats on the same feature after one retry.
