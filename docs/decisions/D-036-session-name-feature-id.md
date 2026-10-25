# D-036: A session is named by its feature id when its files carry one

**Date:** 2026-10-25
**Status:** accepted and implemented in `m3-session-name-feature-id` (F-038, owner decision 28 of 2026-10-24, B-041)

## Context

FR-L1 named a session by its first user sidecar tag, else its id (`ingest::sessions::display_name`, M1). The
tags of a mission's worker sessions are all runner tags (`exec`, `mission-worker`, `mission-session`), so every
worker was shown by its session id, which says nothing about the work. The owner decided (F-038, rule 9) that a
session is named by its feature id when one is present, by its user tag otherwise, through the one shared naming
function. The app reads session files only: the mission's `features.json` is not an input.

## Evidence: where the feature id is recoverable

Surveyed over the M0 capture (`fixtures/streams/2026-10-22-observatory-v1-m0.jsonl` and its 55 child
files), the S capture and the live session files of this mission on 2026-10-25.

- **Sidecar `<id>.settings.json`: no.** Keys are `allowSubagentsInScripts, apiProviderLock, assistantActiveTimeMs,
  autonomyLevel, autonomyMode, childInclusiveTokenUsageBySessionId, compactionThresholdCheckEnabled,
  inclusiveTokenUsage, interactionMode, lastCallTokenUsage, model, providerLock, providerLockTimestamp,
  reasoningEffort, tags, tokenUsage, toolExecutionMode, toolExecutionModeModelId`. `title` and `agentName` are absent.
  `tags` of a worker is `[{"name":"exec"},{"name":"mission-worker"},{"name":"mission-session","metadata":
  {"role":"worker","missionId":"<mission id>"}}]`: role and mission, no feature.
- **`session_start` line: no.** `title` is free text (`"Integrate S to M0 boundary"`, or `"Start new session"` on
  0.247 live sessions); keys are `callingSessionId, cwd, hostId, id, isSessionTitleManuallySet, organizationId,
  owner, sessionTitleAutoStage, title, type, version`.
- **The runner's first user message: yes.** The third record of a worker session (a `type: "message"` record whose
  `message.role` is `"user"`; the first two are the `context-*` system reminders) carries, inside a
  `<system-reminder>` text part, the sentence `You are a worker assigned to execute feature "<feature id>".` It is
  written before the first assistant message. Field path:

  ```text
  message.content[].text   (type "text", message.role "user", before the first assistant message)
    matches  assigned to execute feature "<feature id>"
  ```

  Example (record 3 of `2c04bd42-00fc-4c8b-a8bc-00ff515bac46`, redacted to the sentence):

  ```json
  {"type":"message", ..., "message":{"role":"user","content":[{"type":"text","text":"<system-reminder>\nYou are a worker assigned to execute feature \"m0-integrate-s\".\n\n## Worker Session ..."}]}}
  ```

Oracle on the M0 capture, `grep -o 'assigned to execute feature \\"[^\\]*\\"'` over the root and every child file:
**15 sessions of 56** carry a feature id (the root and 40 `subagent`/`read-only` children carry none):
`m0-integrate-s` (3 sessions), `user-testing-validator-s` (2), and one each of `m0-fixture-freeze`, `s-ci-wiring`,
`scrutiny-validator-s`, `s-cli-replay`, `s-fixture-snapshot`, `s-core-event-model`, `s-app-empty-lanes`,
`s-workspace-scaffold`, `m0-ci-rust-job-linux`, `m0-fixture-redaction`. The S capture carries the same sentence in
2 sessions (`s-fixture-snapshot`, `s-workspace-scaffold`). The sentence occurs in no other session of either capture.

## Decision

1. `ingest::sessions::feature_id_in(lines)` reads the id: the user messages that precede the first assistant message
   of the file, the first text part matching `assigned to execute feature "<id>"`, with `<id>` of `[A-Za-z0-9._:-]`
   only. No assistant message, tool result or later user message counts, so a transcript that quotes the sentence is
   not renamed.
2. `display_name(feature_id, tags, id, runner_tags)` is the one rule: feature id, else `user_tag`, else the session id.
   `Session` stores the name computed through it; the watcher (`session_name`, used by discovery and the attached
   list), the store recorder (`record_tree`) and the memory replay (`session_infos`) take the name from there.
3. The file head is read up to `[ingest].tail_chunk_kb` (the existing chunk bound), line by line, until the first
   assistant message: no new threshold. A session whose id is not readable yet is named by tag or id and renamed once
   when the line arrives: the live list re-reads names every `jsonl_poll_ms` and announces a change as a second
   `session_discovered`. A name never changes back, because the user messages of a file are never rewritten.
4. The `sessions.id` column and the card id prefix are unchanged. The display name is stored in the existing
   `sessions.tag` column (no `name` column exists; no migration): its value changes, its meaning is the display name
   since M1. `Event` and the golden `.events.json` do not carry a session name, so the golden is unchanged.

## Alternatives

- **`features.json` of the mission** (rejected): the app reads session files only, and a replay of a copied capture has
  no mission directory.
- **`session_start.title`** (rejected): free text, set by the runner from the first message, not an identifier.
- **The `Task` description of the spawning call** (rejected in F-038): free text; reviewers it spawns have none.
- **The `EndFeatureRun` `featureId` argument** (rejected): written at the end of the session, not when it starts.
- **A new `sessions.name` column** (rejected): a migration for a value the `tag` column already holds; the column was
  repurposed as the display name at M1.

## Consequences

- Three worker sessions of one feature share a name (`m0-integrate-s`), as sessions that retry a feature should; the
  id column tells them apart.
- A session whose runner changes the sentence is named by tag or id again (the failure is a name, not an error).
  The sentence is the runner's wording in Droid 0.247; a test over the M0 capture pins it.
- Sessions read through `Task` (reviewers, explorers) have no feature id and keep their tag or id.
