# D-023: mapping v1 is frozen, and the PRD §11.1 open items are answered

**Date:** 2026-10-23
**Status:** accepted (M0 exit criterion; v1 frozen at the M0 merge commit f66a9f5, rules first committed in 86df619); amended at the M1 integration `c2fb42acf97883c1e1aa0458ebbc26fbc938bad2` (seven-phase lane column of D-024, `turn.usage` of D-029, golden refrozen; addendum at the end)

## Context

PRD §11.1 left open how worker child sessions, feature ids, milestone ids and validator roles are
identified in Droid's session files, and where `--tag` is recorded. M0 captured this mission's own
session directory (Droid 0.247.0; `fixtures/streams/2026-10-22-observatory-v1-m0.*`: 1 root and 55
children, 8735 records) and wrote `mapping::v1` against it. The result is pinned by the golden
`fixtures/streams/2026-10-22-observatory-v1-m0.events.json` (1279 events, 56 sessions) and by the
hard gate `unknown events: 0`. The rule table lives in
`crates/observatory-core/src/mapping/v1/kinds.rs`; this file reproduces it. Change both together,
and a changed rule is a new mapping version, not an edit of v1.

## Decision

### 1. The v1 rule table

**Amended 2026-10-23 by [D-024](D-024-phase-lanes.md) (F-028, `m1-view-models-core`):** the lane
column below is the phase column of the seven mission phases (1 Plan, 2 Contract, 3 Build,
4 Scrutiny, 5 User testing, 6 Integrate, 7 Gate). Kinds, pointers, roles and the sidecar are
unchanged. "Home phase" is `Lane::home(role)` (Orchestrator and Human: Plan; Worker and
Unresolved: Build; scrutiny validator: Scrutiny; user-testing validator: User testing; External:
Gate), except that a role-neutral kind of an integration session (feature id
`^(m[0-9]+|final)-integrate-`) is Integrate and one of a contract session (spawned by a `Task`
whose description matches `^Contract\b|[Vv]alidation contract`) is Contract. The full kind table
with the M0 counts is in D-024 §3. The rows below were written for the nine PRD lanes; where a
row says "home phase", the amended rule applies.

Raw shape of a Droid 0.247 session file, the PRD §7 kind it becomes, and the phase and role. An
event's `role` is always the role of its session (the capture header's role is the oracle); a
person's words are told apart by the kind, not by a `Human` role.

| Raw shape | Kind | Phase (D-024) / role |
|---|---|---|
| `session_start`, no `callingSessionId` | `intent.created` | Plan, overlay human, session role |
| `session_start` with `callingSessionId` | `subagent.spawned` | child's home phase (Build, Scrutiny, User testing; Contract or Integrate by rules 3 and 4) |
| root user text typed by the person (`userMessageSource: cli_tui`) | `intent.amended` | Plan, overlay human, session role |
| child user text that is a `Task` or runner assignment (first only) | `task.started` | home phase |
| assistant `tool_use` `Task` | `task.assigned`, plus `feature.defined` the first time a feature is named | `task.assigned`: home phase; `feature.defined`: Plan |
| assistant `tool_use` `ProposeMission` | `plan.created`, plus `milestone.defined` per `\| **Mn** ...` row | Plan |
| assistant `tool_use` `AskUser` | `question.asked` | Plan, overlay human |
| `tool_result` of an `AskUser` | `intent.answered` | Plan, overlay human, session role |
| `Execute` command that runs `git commit` (whole command words) | `commit.created` | home phase; the orchestrator's commits: Integrate |
| `Execute` command that runs `cargo test`, `make test` or `make check` | `tests.run` | home phase |
| `Execute` command that runs `make check` | `gate_command.run`; its result `gate_command.passed` / `gate_command.failed` by the exit status in the result text (`is_error` is a second failure signal) | Integrate for every role |
| assistant `tool_use` `Edit`, `Create`, `ApplyPatch` | `file.touched` | home phase |
| `tool_result` with `is_error` | `tool.error`; `hook.blocked` when the text names a hook | home phase |
| assistant `modelId` differs from the session's previous one | `model.changed` | home phase |
| `compaction_state` | `context.compacted` | home phase |
| `todo_state` whose todos differ from the previous one | `plan.revised` | Plan |
| `agent_turn_outcome` in a child: completed / cancelled / error or usage exhausted | `task.done` / `task.skipped` / `tool.error` | home phase |
| `agent_turn_outcome` in the root: cancelled / error or usage exhausted | `mission.paused` / `escalation.raised`; completed is quiet | Plan (`escalation.raised`; `mission.paused` also carries the human overlay) |
| child end of file | `subagent.finished` | home phase |
| sidecar `tokenUsage`, at end of file | `usage.reported` | home phase |
| scrutiny validator report text | `finding.blocking`, `finding.non_blocking`, `scrutiny.passed` | Scrutiny |
| user-testing validator report text | `qa.scenario.passed`, `qa.scenario.failed`, `qa.evidence` | User testing |
| thinking, plain text, empty user messages, `Script` (its inner `Execute` calls are their own records), other tool calls and their results | no event | none |
| any other `type`, content part type, tool name or `agent_turn_outcome` reason | `Unknown`, counted | `Lane::home(role)` |

The droid-version range is `0.247.*`. The store row for v1 is `mapping_versions(version = 'v1',
droid_version_range = '0.247.*', frozen_at = '2026-10-23')`: the day commit 86df619 froze v1.

#### Shapes with no event

A shape the table lists as "no event" is understood and has no PRD kind. The architecture §5 row
"anything else gives `Unknown`" cannot hold with the gate at 0, because thinking, `Read`, `Grep`
and their results are everywhere. The table therefore names every quiet shape, and `Unknown` stays
for what v1 has not seen: an unseen record `type`, content part type, tool name or outcome reason,
and unparseable lines. They are counted and listed by `replay`, never dropped (rule 18). A line can
give several events and most give none, so the event count is smaller than the record count (M0:
1279 events over 8735 records; 474 for the root session; S fixture: 661).

#### Judgement calls

These readings are not in the PRD and are frozen with v1:

- Root `agent_turn_outcome` `completed` is quiet; `cancelled` becomes `mission.paused` (not observed
  in the capture); `error` and `model_usage_exhausted` become `escalation.raised`. In a child,
  `completed` becomes `task.done` (with the turn duration), `cancelled` becomes `task.skipped`, and
  `error` or `model_usage_exhausted` becomes `tool.error`.
- A `make check` command emits `tests.run` and `gate_command.run` at the call and
  `gate_command.passed` or `gate_command.failed` at its result. The verdict is the exit status in
  the result text: Droid 0.247 leaves `is_error` null on a non-zero exit and writes only
  `Command failed (exit code: N)` and/or a closing `[Process exited with code N]`. Non-zero is
  failed, zero is passed, and without a status in the text `is_error: true` is failed (it is also
  failed beside a zero status). Only the closing marker line and an opening `Command failed` line
  count, so a log that quotes `exit code: 2` in its middle does not fail a passing run. A failing
  gate result stays `gate_command.failed` (not `tool.error`) unless the text names a hook.
- Command matching is by whole command words in the `Execute` `command` text, never in a `Script`
  body or prose. A small shell lexer splits the line into simple commands at `;`, `&`, `|`,
  parentheses and newlines, keeps quoted text as one word, drops heredoc bodies and comments, and
  skips leading `NAME=value`, `time`, `{` and `!` words. A command counts when it starts with the
  phrase's words: `make check` is not `make check-todos`, `xmake check`, `echo "make check"`, or a
  line inside a commit-message heredoc. `make -C dir check` is not recognised. The same rule applies
  to `git commit`, `cargo test` and `make test`. `Script` is a quiet tool: the `Execute` calls it
  makes are separate records in the session file and map on their own.
- A feature id is the quoted id in the runner prompt `execute feature "<id>"`, or the first word of
  a `Task` description that starts with a milestone id from the root's `ProposeMission` table
  (rows `| **M0** ...`, lower-cased). A milestone is the first word of the feature id
  (`m0-mapping-v1`), or the last for the runner's per-milestone validator features
  (`scrutiny-validator-s`).
- The tool names `tests`, `discoveredIssues` and `skillFeedback` are not tools. They are keys of an
  `EndFeatureRun` call whose JSON arguments failed to parse (the capture holds one each), so they
  are quiet.
- Counts of the corrected rules on the M0 capture: `gate_command.run` 35 (`passed` 34, `failed` 1;
  before: 173, 167, 6), `tests.run` 83 (242), `commit.created` 73 (100), `tool.error` 59 (53).
  The 6 old `failed` were `Script` tool errors, now `tool.error`; the one new `failed` is the
  `make check` that exited 2 with `is_error` null (child `6eb88099`, line 41).
- A failed tool result is a hook block when its text contains `guard-git.sh` or `.factory/hooks`.
- Scrutiny findings are the numbered or bulleted items under a bold or `#` heading containing
  "Non-blocking" or "Blocking"; `scrutiny.passed` is emitted when a report holds no blocking item
  and does not say `Result: fail`. The format is Markdown the validators wrote by habit, not a
  schema.
- QA verdicts are table rows `| VAL-x | pass |` or bold lines `**VAL-x (pass): ...**`;
  `qa.evidence` is emitted when the report text names an `/evidence/` folder.

#### Corrections to architecture §5

- The root `session_start` is the single `intent.created`; every typed root prompt afterwards is
  `intent.amended`. The architecture's two rows conflicted; v1 reads them in that order.
- Events keep the session's role. Human prompts are Plan with the orchestrator role, not
  `Human`, and carry the human overlay; the app's own human-action rows are overlay events on a
  phase, never a lane of their own.
- `usage.reported` and `subagent.finished` come from the sidecar and end of file through
  `Mapper::finish` / `TreeMapper::finish`. A live source (M1 attach) must call `finish` when a file
  will not grow again and keep one `Mapper` per session; the stateless `mapping::map_record` sees
  every record as the first of its session.
- `session_start` has no timestamp: its event is held and timed by the next timestamped line.

### 2. The five PRD §11.1 open items

1. **Where `--tag` is recorded.** In the sidecar `<session-id>.settings.json`, key `tags`, not in
   the JSONL and not on its first line. `tags` is an array of objects `{ "name": <string>,
   "metadata": <object|null> }`. The M0 capture shows four shapes: `subagent` with
   `metadata.callingSessionId` / `callingToolUseId` (40 sessions, 3 of them also tagged
   `read-only`);
   `exec`, `mission-worker` and `mission-session` with `metadata.{role, missionId}` on the 15
   sessions the mission runner started; and `mission-session` with `metadata.{role, missionId}`
   alone on the orchestrator. The capture holds no tag chosen by the person beyond these, so a
   user `--tag` value would be one more entry of the same array; the app reads `tags[].name` and
   never assumes a fixed set. Detail: D-012.
2. **How child sessions link.** A child is a sibling file (`<id>.jsonl` next to the root, or under
   `<root>.children/` in the fixture). Its first line `session_start` carries `callingSessionId`
   (the parent's session id) and `callingToolUseId` (the `id` of the parent's `Task` `tool_use`
   part; `null` for sessions the mission runner starts). The sidecar `subagent` tag repeats both.
   Attachment is transitive, parents before children. All 56 sessions of the capture attach.
3. **How roles are identified.** In order: (1) sidecar tag `mission-session` with
   `metadata.role == "orchestrator"` gives the orchestrator; (2) no `callingSessionId` gives the
   orchestrator; (3) the parent's `Task` `tool_use` whose `id == callingToolUseId` has
   `input.subagent_type`, with the exact strings observed: `worker` (24 calls) and `explorer`
   (3) give Worker (an explorer is a read-only worker; the capture headers label it `worker`),
   `scrutiny-feature-reviewer` (6) gives the scrutiny validator, `user-testing-flow-validator` (7)
   gives the user-testing validator; (4) a runner-started session (tag `metadata.role == "worker"`,
   no spawning `tool_use`) is told apart by the `Skill` it invokes: `scrutiny-validator` gives the
   scrutiny validator, `user-testing-validator` the user-testing validator, otherwise Worker (the
   tag role `worker` is a hint, never final before the Skill check); (5) otherwise `Unresolved`
   (also for an unknown `subagent_type` or a `tool_use` missing from the parent). The capture
   headers are the oracle: 1 orchestrator, 39 worker, 7 scrutiny validator, 9 user-testing
   validator, zero `Unresolved`.
4. **Feature ids.** From the runner prompt `execute feature "<id>"` in the child's first user
   message, or from the `Task` input `description` of the parent: the first word that is kebab-case
   and starts with a milestone id (`m0-mapping-v1`).
5. **Milestone ids.** From the `ProposeMission` input: the rows `| **M0** ... |` of its plan table,
   lower-cased, give the milestone ids and one `milestone.defined` each. A feature belongs to the
   milestone named by the first word of its id, or the last word for the runner's
   per-milestone validator features.

### 3. Defined, not observed

PRD §7 defined 61 kinds (5 + 12 + 14 + 5 + 3 + 6 + 5 + 11). D-024 removes `deploy.completed`,
`pr.merged_by_human` and `pr.merged_by_policy` and adds `handed_to_cd { merge_sha, pr_number,
attestation_ids }`, so `EventKind::KNOWN` has 59 entries. The M0 capture shows 30 of them:
`commit.created`, `context.compacted`, `escalation.raised`, `feature.defined`, `file.touched`,
`finding.non_blocking`, `gate_command.failed`, `gate_command.passed`, `gate_command.run`,
`hook.blocked`, `intent.amended`, `intent.answered`, `intent.created`, `milestone.defined`,
`model.changed`, `plan.created`, `plan.revised`, `qa.evidence`, `qa.scenario.passed`,
`question.asked`, `scrutiny.passed`, `subagent.finished`, `subagent.spawned`, `task.assigned`,
`task.done`, `task.skipped`, `task.started`, `tests.run`, `tool.error`, `usage.reported`.

The other 29 (31 at the M0 freeze) are defined by the PRD and the model, and v1 has no rule for
them because no raw shape in the capture carries them (several come from Git, GitHub or the app,
not from session files). `every_prd_kind_present_in_capture_is_mapped` asserts that the observed
set is exactly the 30 above and that none is `Unknown`. The 29:

`approval.given`, `approval.requested`, `attestation.recorded`, `attestation.verified`,
`check.failed`, `check.passed`, `check.required_missing`, `check.started`, `finding.blocking`,
`handed_to_cd`, `integration.conflict`, `integration.conflict_resolved`, `integration.merged`,
`milestone.completed`, `mission.completed`, `mission.paused`, `mission.resumed`,
`permission.denied`, `permission.granted`, `permission.requested`, `pr.opened`,
`pr.review_requested`, `qa.scenario.failed`, `question.answered`, `review.outcome`,
`rework.requested`, `rework.resolved`, `ruleset.blocked`, `spec.versioned`.

`Overlay::Commander` is likewise declared with no producer. `mission.paused` and `finding.blocking`
and `qa.scenario.failed` do have v1 rules (see the table) that the capture does not trigger. A new
observation of any of the 29 is added in a later mapping version, with a fixture, and removed from
this list in the same change.

### 4. Droid version to mapping version

- The mapping version is selected from `droid_version` in the fixture header
  `<capture>.capture.json`; a live attach records the output of `droid --version` in the store's
  `captures` row (D-012). No session file carries the Droid release; `session_start.version`
  (`2`) is the schema number.
- `0.247.*` selects v1. Any other version, or none, selects the latest mapping (v1 today) and
  raises one `Warning::UnknownDroidVersion { seen, used }`, printed once by the CLI and shown once
  in the UI status bar. Showing events with the newest rules beats showing none.
- A Droid release whose files break v1 gets a new module `mapping::v2` and a new row in
  `mapping_versions`; v1 and its golden never change after the M0 integration. A `.events.json`
  golden is regenerated only with `OBSERVATORY_UPDATE_GOLDEN=1` and a reviewer read.

### 5. As-built notes from the M0 stages

- **Roles for runner-started sessions** are resolved by their `Skill` invocation, as in item 3
  above; this amends architecture §5 step (3), which assumed a `role:<x>` tag.
- **`.capture.json` `source_dir`** is stored in its redacted form
  (`~/.factory/sessions/-home-user-code-...`) under rule 22. The real directory is documented only
  outside the fixture (the capture skill and the D-020 addendum).
- **Redactor fix.** The email rule of `scripts/redact-fixture.sh` had turned the JSON escape
  `\n128x128@2x.png` into `\user@example.invalid`, which is invalid JSON (9 lines: S 3, M0 6). The
  rule now refuses to start an address right after a backslash; 6 M0 and 2 S children were
  re-redacted from the originals and their `redacted_sha256` updated in both headers. Line counts
  and original hashes are unchanged. Hashes and the reason are in the [D-020 addendum](D-020-pii-in-fixtures-is-redacted-at-capture.md).
- **`inspect`** prints the raw line without a trailing newline, so `cmp` against a `dd` slice of
  `byte_len` bytes at `byte_offset` of the source file is exact.
- **`replay` writes no store** unless `--store` is given or a config `[store].path` is in effect.
- **Gate rule limits found by scrutiny (accepted v1 behaviour, golden frozen at the merge).** The
  gate verdict reads the exit code of the Execute result. (1) A command that ends `; echo exit=$?`
  or is piped into another command masks the real exit, so four failing `make check` runs map to
  `gate_command.passed` (root:516, 9d874ce8:83, bfc3f3f7:71, fef523bb:62 in the M0 capture). (2) A
  `fireAndForget` launch whose result text is `Background process started` carries no exit code and
  maps to `passed` (49847d4f:32). Both are fixed in the next mapping version (`BACKLOG.md` B-030,
  B-031). Further limits not seen in the capture: B-028 (non-gate non-zero exits map to no event)
  and B-029 (command lexer forms).

## Alternatives

- A Droid version field read from each session file: none exists (D-012).
- Read validator roles from `description` text or the child's system prompt: fragile, and the
  system prompt is never stored. The `subagent_type` string and the invoked `Skill` are exact.
- Map every unknown shape to `Unknown`, including `Read` and thinking: makes the `unknown events: 0`
  gate unreachable and buries real surprises.
- One event per record: keeps event count equal to line count but invents events with no PRD kind.
- Edit v1 when a new shape appears: breaks the golden and every stored event's `mapping_version`.

## Consequences

- Replay of the M0 capture ends `unknown events: 0`; a new unseen shape in a later capture fails
  that gate and the test that pins the 30 observed kinds, so the change is a deliberate new version.
- The S-era wording "event count equals line count" no longer holds; checks compare with the
  golden or the record count instead.
- The 29 unobserved kinds are not testable from session files; later milestones produce them from
  Git, GitHub, the host channel or the app.
- Validator report parsing depends on habitual Markdown; a change of validator style is a new
  mapping version.

## Addendum 2026-10-24 (M1 integration)

- **Lane column:** the kinds table in §1 matches the doc table in `crates/observatory-core/src/mapping/v1/kinds.rs`
  line for line, in the seven-phase column of D-024 (checked at the M1 integration: `diff` of the two
  26-line tables is empty, after `m1-fix-session-name-user-tag`). The rule did not change; `kinds.rs` stays the source.
- **Kinds:** 60 known kinds (59 of D-024 plus `turn.usage`) and 31 observed on the M0 capture (30 session-file
  kinds plus `turn.usage`). `turn.usage` is not a session-file shape and has no row in §1; it comes from the
  Droid CLI log and is specified in [D-029](D-029-usage-log-source.md) (Panel data, not a lane row). The 29
  defined-not-observed kinds of §3 are unchanged.
- **Golden at the M1 tip:** `fixtures/streams/2026-10-22-observatory-v1-m0.events.json`, 2394 events (1279
  session events + 1115 `turn.usage`), SHA-256 `25d91c53b580f8f5f3f1a62ecaa977a24dca0b61f5caf86e296182e8f23a5750`.
  It supersedes `f055657a…` of the M0 freeze and the interim `c10b27ae…` of `m1-view-models-core`. The
  `.jsonl` (`ab23da55…a58c`) and `.settings.json` (`efef27eb…76ec`) hashes are unchanged.
- **Session names:** a session is named by its `user` tag when there is one, else by its id
  (`m1-fix-session-name-user-tag`, `m1-fix-session-name-everywhere`); the first sidecar tag is not unique in the
  M0 capture (BACKLOG B-041).
