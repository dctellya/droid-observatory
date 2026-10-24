# D-033: The hosted lane speaks Droid's stream-jsonrpc; the spool is mapped by mapping v1

**Date:** 2026-10-24
**Status:** accepted and implemented in `m2-rpc-source-core` (`ingest::rpc`, feature `rpc`); merged at the M2 integration `7385b62`

## Context

Architecture §4.5 has the app host a mission with `droid exec --mission --input-format stream-jsonrpc
--output-format stream-jsonrpc`, spool stdout and write controls to stdin. It left the method names and the
shape of the stream open ("confirmed in M2 from `droid exec --help` and the first hosted capture"). It also
assumed ids are numbers and that the stream is the session file's lines.

Evidence gathered on 2026-10-24 with Droid 0.247.0:

- `droid exec --help` lists `-o, --output-format <format>` with `stream-jsonrpc`, `--input-format` with
  `stream-jsonrpc` ("controlled via JSON-RPC requests"), `--mission`, `--auto <level>`, `-m/--model`,
  `-r/--reasoning-effort`, `--worker-model`, `--worker-reasoning-effort`, `--validator-model`,
  `--validator-reasoning-effort`, `-f/--file`, `--cwd`, `--tag`. It does not name any JSON-RPC method. It also
  says that with `--input-format stream-jsonrpc` the flags `-m`, `--auto`, `-r`, `--tag`, `-s` and others "are
  still validated but have no effect" and a warning names them on stderr: session settings travel in the
  `droid.initialize_session` request (`modelId`, `reasoningEffort`, `autonomyLevel`, `interactionMode`,
  `missionSettings`, `tags`). `--mission` "Requires --auto high or --skip-permissions-unsafe". The launch flags
  of VAL-HOST-005 therefore stay on the command line (validated, and read by the process), and the host also
  sends them in the initialize request (app feature, not this one).
- Method and notification names were read from the `droid` binary (the zod schemas it ships) and tried against
  a live process: a process started with the two stream flags, one `droid.initialize_session` request, one
  `droid.add_user_message` request (Haiku, "Reply only OK", 3114 input and 8 output tokens) and one
  `droid.interrupt_session` request.
- A numeric `id`, or an envelope without `"type":"request"`, is answered
  `{"type":"response","id":null,"error":{"code":-32700,"message":"Invalid JSON-RPC message"}}`. The same
  request with `"id":"3"` is answered `{"id":"3","result":{}}`. Droid validates `id` as a string.
- The stream on stdout is not the session file's lines. Every line is an envelope
  `{"jsonrpc":"2.0","type":"notification|response|request","factoryApiVersion":"1.0.0",
  "factoryProtocolVersion":"1.274.0",...}`. The messages of the session are
  `droid.session_notification` with `params.notification.type = "create_message"`, whose `message` equals the
  `message` of the file's `message` line plus `id`, `parentId`, `seq`, `createdAt`, `updatedAt`; a turn ends
  with `agent_turn_completed {reason, turnId}`, the file's `agent_turn_outcome`; the response to
  `droid.initialize_session` carries `result.sessionId`, the file's `session_start` id. The rest is chatter
  (`mcp_status_changed`, `settings_updated`, `droid_working_state_changed`, `session_token_usage_changed`,
  `session_title_updated`, `assistant_text_delta`, `assistant_text_complete`). Mapped both ways through mapping
  v1, the one-turn capture gives the same events (`intent.created`, Plan, orchestrator) from the spool and the
  session file; their times differ by 2 ms (`createdAt` against the file's own `timestamp`).

## Decision

Method names the host writes (`HostControl`, `ingest::rpc::method`), all with the envelope
`{"type":"request","jsonrpc":"2.0","factoryApiVersion":"1.0.0","id":"<n>","method":...,"params":...}` and a
string id taken from a counter:

| `HostControl` | Written as | Method / result | Acknowledged by |
|---|---|---|---|
| `Pause` | request | `droid.interrupt_session`, params `{}` | response with the same id (`result: {}`) |
| `Resume` | request | `droid.add_user_message`, params `{"text": RESUME_TEXT}` (an interrupted session continues on its next user message) | response with the same id |
| `Message { text }` | request | `droid.add_user_message`, params `{"text": text}` | response with the same id |
| `Answer { question_id, text }` | response to Droid's `droid.ask_user` request | `result: {"answers":[{"index","question","answer"}]}`, `id` = Droid's request id | none (Droid continues the turn) |
| `Approve { request_id }` | response to `droid.request_permission` | `result: {"selectedOption":"proceed_once"}` | none |
| `Deny { request_id }` | response to `droid.request_permission` | `result: {"selectedOption":"cancel"}` | none |

Notifications and requests read: `droid.session_notification` (all events), `droid.ask_user` (params
`toolCallId`, `questions[{index, topic, question, options}]`), `droid.request_permission` (params `toolUses`,
`options[{value}]`).

1. `Answer.question_id` is the id of the `QuestionAsked` event. The mapper binds that event to the `AskUser`
   `tool_use` id of its message, and the `droid.ask_user` request carries the same id in `toolCallId`, so the
   answer is routed to the request id of that question and to no other. A question whose tool call already has a
   result (answered here or in a terminal) or that was answered is closed: a second answer is `NotOpen`.
2. Ids are strings holding the decimal counter. This replaces architecture §4.5's `id: u64`, and VAL-HOST-007's
   "numeric id" is read as "an id from a numeric counter".
3. The spool is the stdout verbatim (`Spool::copy_from` byte for byte, `append_line` adds the newline). The
   events point at it with `SourcePtr::Rpc`. `RpcSource` is a `JsonlSource` over the spool with the pointer
   rewritten to `rpc`.
4. `RpcMapper` turns the envelope lines above back into the file's records in memory and maps them with
   `mapping::v1` (`Mapper`), so there is one set of rules and `mapping_version` is `v1`. A line that is not an
   envelope (a replayed session record) is mapped as it is. Chatter in the list above yields no event and is
   counted by name (`protocol_counts`). Any other envelope is an `Unknown` event named `rpc/<type>` (rule 18).
   This means the spool has more lines than the store has distinct pointers; VAL-HOST-006's
   `wc -l = COUNT(DISTINCT byte_offset)` holds only for a stream without chatter and must compare against the
   lines that yield events (reported to the orchestrator).
5. No lane is named in `ingest::rpc`; lanes come out of mapping v1 only (F-041).

## Alternatives

- Numeric ids as in architecture §4.5: rejected, Droid answers -32700.
- One `Unknown` event per chatter line so every spool line has a store row: rejected, it fills the Unknown
  count (SPEC M2 gate: zero `unknown.event` on the hosted capture) with lines that are not events.
- A second mapping for the stream: rejected, two rule sets drift (invariant: one mapping per version).
- Pause as `droid.update_session_settings` or as killing the process group: `interrupt_session` is the
  documented interrupt and is acknowledged.

## Consequences

- The launch flags are validated by Droid but ignored in stream mode; the host app (next feature) must send the
  models, efforts and autonomy in `droid.initialize_session` (and `missionSettings`) as well.
- Notification types not in the chatter list (for example `mission_paused`) surface as `Unknown` until a
  hosted mission shows what they carry; that count is the signal to extend the list or the mapping.
- `agent_turn_completed.reason` is passed through; reasons mapping v1 does not know are `Unknown`.
- Verified only on a one-turn session without tools, questions or permissions; the question and permission
  shapes come from the binary's schemas and are covered by tests on hand-made lines.
- Probe sessions created by the capture sit in `~/.factory/sessions/-private-tmp-rpc-probe` (three sessions, no
  secrets); they are not fixtures and are not committed.
