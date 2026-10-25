# Staging: m3-controls-ui-and-stall

Feature `m3-controls-ui-and-stall`, worktree `observatory-wt/m3`, branch `wt/m3`.
VAL-CTRL-002 to 010, 013, 015, 016, 017 and VAL-UI-001. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): the six controls are one Tauri command, `human_action(mission_id, launch_id?, action, text?)`. With a
  `launch_id` (hosted) `Hosts::human_action` writes the control to the process and then records the `human_actions` row
  and its human-overlay event; a control the process cannot take (a request already answered) leaves no row. Without it
  (attached) the event is recorded and the window shows `TERMINAL_INSTRUCTION` plus the text to paste
  (`view::ControlBar::attached`, `view::terminal_text`); no `host_control` line is written. Mark complete reaches no host
  (the row and the event only); mark skipped is a `droid.add_user_message` carrying the feature id and the reason.
- Row (F-NEW): acknowledgement pointers (contract pass 24). The host feeds `control::ack_event` with the spool line that
  answers the control: for pause, resume, message and skip the response line whose `id` equals the request id of the
  `host_control` it wrote; for approve and deny the `permission_resolved` notification line
  (`params.notification.type = "permission_resolved"`, `requestId` = the JSON-RPC id of Droid's
  `droid.request_permission`); for an answer the `tool_result` line of the AskUser tool call (`call:<toolCallId>`).
  The ack event id is `<session>:<byte_offset>:1073741824`; kinds seen live: `intent.answered`, `intent.amended`
  (message and skip), `mission.paused`, `mission.resumed`, `permission.denied`, `permission.granted`.
  `ingest::rpc::resolution_of` classifies the two resolving shapes.
- Row (F-NEW): the host synthesises the `permission.requested` event that approve and deny point at
  (`control::permission_requested_event`, id `<session>:<offset>:1073741825`, a human overlay on the lane of the latest
  non-overlay event of the stream, else the template's first lane). Closes B-075 for Approve, Deny and Resume (the
  acknowledgement events above are the `permission.granted`, `permission.denied`, `mission.resumed` rows).
- Row (F-NEW): the stall flag has a feature id `"mission"` (`stall::MISSION_LEVEL`) when the mission itself waits on the
  human (an unanswered question or permission request with no feature). `stall::activity_of` flags it after the threshold
  like any idle feature; the prompt is the matching `RunbookPrompt` ("Waiting on human: answer it", `#waiting-on-human`).
- Row (F-NEW): `[intervention].stall_poll_s` (default 15, `config/observatory.example.toml`): how often the app
  evaluates `stall::evaluate` over the live missions. The app logs `stall_flag` (mission_id, feature_id, idle_for,
  prompt, anchor) and emits `stall_flag` to the window only when the set of flags changes; nothing under the threshold
  is ever reported.
- Row (F-NEW): VAL-UI-001 applied as the owner's answer 6 asked: `ui/theme.css` is a byte copy of
  `ai/design/ui-kit-v2/theme.css`; `ui/style.css` is token-only (no colour literal, no `font-family`; greps: 0 and 0);
  the kit classes `kpi`, `lib`, `chip`, `switch`, `card` are used by the views; no new navigation, no brand work; a
  theme switch (System, Dark, Light) is the kit's `switch`. The departures from the kit's "Eight rules" (rule 3 board page width, rule 4 page head) are the
  two proposed rows in `reports/staging/m3-fix-ut-ui-kit-logs-and-attached-e2e.md`; numeric cells (rule 2) and the `.lib`
  boxes (rule 6) are met there. The `donut.js` component is not
  used: this feature draws no share-of-total chart.
- Row (F-NEW): the live self-check harness (`live_check.rs`) takes every control through `Hosts::human_action`, runs the
  stall poller and installs a tracing subscriber (closes B-067); `OBSERVATORY_LIVE_HOLD_S` keeps the question
  unanswered for N seconds to show the stall flag.

## Decisions

- D-034 (hosted launch): the approve and deny acknowledgement is the `permission_resolved` notification, not a response
  to the host's reply (Droid sends none). Amend the line "the host's own responses are not in the stream" to say the
  host now maps `permission_resolved` to `permission.granted` or `permission.denied` (B-075).

## BACKLOG

- Close B-067 (tracing subscriber in the live harness) and B-075 (host-synthesised overlay events for Approve, Deny,
  Resume): done here.
- New: the controls spec does not cover mark complete and mark skipped (the fake droid has no feature list); the live
  self-check covers them. Reason: the e2e fake droid would need a `ProposeMission` with features first. Milestone M4.
- New: `stall_flag` is a window event only; no system notification or sound. Reason: scope, PRD FR-H5 asks for a flag.
  Later.

## Deferred clauses

- None of the contract clauses of the feature is deferred. Window checks are e2e specs (`controls.e2e.ts`,
  `stall.e2e.ts`); the live self-check ran in the mock runtime, no display (as m2's).

## Library

- `OBSERVATORY_LIVE_HOLD_S=<s>` plus `OBSERVATORY_LIVE_CONFIG` with `[intervention] stall_threshold_min = 1,
  stall_poll_s = 5` shows the flag in about 65 s; `cargo test -p observatory-app --lib live_check -- --ignored
  --nocapture` run without `timeout` (macOS has none). Each run takes about 145 s on Haiku at `low`.
- Scratch dirs of this check: `observatory-scratch/m3-controls-a` (threshold 1) and `m3-controls-b`
  (default), each with its own store and an empty sessions dir.
