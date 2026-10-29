# D-041: Rule C, the officer seat has an allow-list of tools, enforced by a PreToolUse hook

**Date:** 2026-10-29
**Status:** accepted; owner inbox 2026-10-29-1845 (the hook) and 2026-10-29-2049 (the allow-list), a4b4646 and the commit that turns it into an allow-list

## Context

Since the M4 dispatch the orchestrator made 513 tool calls: 74 decisions (`DismissHandoffItems`, `StartMissionRun`, `Task`) and 439
execution calls (`Execute`, `Script`, `Read`, `Edit`, `Grep`, `TodoWrite`, `Create`), each in its own turn re-reading about 170 k tokens.
Owner inbox 1845 made the orchestrator's turns carry decisions only and sent every execution to a seat (a scribe on
`claude-sonnet-5-5`, a read-only diagnostician, the worker's own hand-off). Commit a4b4646 installed the first mechanism: a
PreToolUse hook (`.factory/hooks/officer-decides.sh`, session ids in `.factory/hooks/officer-sessions.txt`, entry in
`.factory/hooks.json`) that denied `Execute`, `Edit`, `Create`, `Grep` and `TodoWrite` for the orchestrator session. It was proven
live at 2026-10-29 ~17:45Z (a `Grep` inside a Script was rejected with `execution goes to a seat (rule C)`).

Owner inbox 2049 found the hole in that design: the deny-list was built from the tools the orchestrator had used, not from
what the seat is for, so `FetchUrl` (loaded at 20:47 local) was not on it. A deny-list built from observation has a hole for every
tool not yet seen. Scrutiny round 2 of M4 also found that a4b4646 had no decision file, which `g6-hooks-config` requires of every
commit that touches a hook file.

## Decision

- **The officer's tools are an allow-list built from its activities.** For a session listed in `officer-sessions.txt`:
  read the record (`Read`, `LS`), wait (`Script`), delegate (`Task`), dispose, raise and land (`DismissHandoffItems`,
  `StartMissionRun`, `EndFeatureRun`). Every other tool is denied with `not an officer tool (rule C): <tool>` (exit 2): `Execute`,
  `Edit`, `Create`, `ApplyPatch`, `Grep`, `Glob`, `TodoWrite`, `FetchUrl`, `WebSearch`, `GenerateImage`, `Skill`, any MCP tool and
  any tool the harness adds later. An activity the list does not cover is a boundary row for the owner, not a new tool.
- **Read and LS have a path scope** (owner's delegate, inbox 2103): `Read` passes only when `tool_input.file_path`, and `LS` only when
  `tool_input.directory_path`, starts with the mission directory `/Users/dct/.factory/missions/e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/`, the
  worktree's `docs/owner-feedback/inbox`, or its `reports/staging`. The sessions directory, `build-log/` and the repository tree are
  investigation and go to a seat. A path with a `..` segment and an `LS` without `directory_path` are denied
  (`outside the officer's read scope (rule C): <path>`).
- **Mechanism:** the rule C entry in `.factory/hooks.json` has matcher `*` (the git guard and the logger are unchanged); the script
  reads `session_id`, `tool_name` and `tool_input` from the hook's stdin JSON. A session not listed in
  `officer-sessions.txt` exits 0 untouched: workers, validators and subagents are not affected. The matcher is honoured for tools
  invoked inside a Script as nested calls.
- **Commits:** a4b4646 (the deny-list hook) and 060698a (the allow-list and this file). Both are named here by short
  sha, as `scripts/contract-checks/g6-hooks-config.sh` matches hook-file commits against `docs/decisions`; a commit cannot name
  its own sha, so 060698a is named by the docs-only commit that follows it.
- **Scope:** from the 2026-10-29 wake through the M4 integration and all of M5. Not a model change, not a cap change.

## Alternatives

- **Keep the deny-list and add `FetchUrl`:** the next unseen tool is the next hole; rejected by owner inbox 2049.
- **No hook, practice only (the first reading of rule C):** 513 calls in the record show that a rule stated in prose does not hold under
  load; a hook turns it into a check.
- **Scope Read and LS to nothing (decisions only, no reads):** the officer must read hand-offs, the inbox and its own mission files to
  decide; rejected. Unscoped Read: at 20:53 local the orchestrator read about one hundred session files into its own context
  (132 k new tokens) to answer a process question a seat answers in three lines; rejected by inbox 2103.

## Consequences

- Investigation reads (sessions, `build-log/`, the repository tree) go to a diagnostician seat; the officer receives the lines.
- A resumed session with a new session id must be added to `officer-sessions.txt`, or the hook stops applying to it; the
  orchestrator reads that file at every wake. The hook keys on the session id and cannot tell two processes of one session apart.
- The harness still has no per-role tool set (feedback item 21); the hook is the workaround, one `jq` call per tool call in every
  session because the matcher is `*`.
- A change to the hook files is a commit that a decision file names by short sha, written in a docs-only commit after it.
- 2026-10-29 amendment: the worktree read scope is a pattern over observatory-wt/<name>/ (inbox and reports/staging), because the integrator replaces the worktree at every boundary; commit <sha named by the follow-up>.
