# D-012: Droid session schema and the input of the mapping

**Date:** 2026-10-23
**Status:** accepted (owner decision 2026-10-22, entry 26; recorded at the S integration)

## Context

PRD §11.1 left open where Droid records its version, tags, child sessions and usage. The
dependency readiness report (`reports/readiness/2026-10-22-dependency-readiness.md`) and the S
fixture (`fixtures/streams/2026-10-22-observatory-v1-s.*`, Droid 0.247.0) settled it by
inspection of the session directory:

- No session file carries the Droid version. `session_start.version` is the numeric schema
  version (`2`), not the Droid release.
- Usage is not in the JSONL. There are no `usage.reported` lines; token counts live in the
  sibling `<session-id>.settings.json`.
- `tags` in the sidecar is an array of objects `{ "name": <string>, "metadata": <object|null> }`,
  not strings. Seen: `mission-session` with `metadata.role`, `subagent` with
  `metadata.callingSessionId` and `callingToolUseId`, `mission-worker`, `exec`, `read-only`.
- Child sessions are sibling files. A child's `session_start.callingSessionId` names its parent;
  `callingToolUseId` is null for workers the mission runner starts.

SPEC §3 M0 and §5.2 said the mapping version was selected from "the Droid version in the session
`.settings.json`". That cannot work, because the file has no such field (rule 8).

## Decision

- The mapping version is selected from `droid_version` in the fixture header
  `<capture>.capture.json` (written at capture time from `droid --version`; a live attach records
  `droid --version` into the `captures` row). `0.247.x` selects `mapping::v1`. An unknown Droid
  version uses the latest mapping and raises a one-line warning.
- Usage, model, reasoning effort, tags and child links come from the sibling `.settings.json`
  (`tokenUsage`, `inclusiveTokenUsage`, `childInclusiveTokenUsageBySessionId`, `lastCallTokenUsage`,
  `assistantActiveTimeMs`, `tags`, `model`, `reasoningEffort`, `interactionMode`, `autonomyLevel`).
  The mapper emits `UsageReported` from the sidecar, once at the end of the stream or when it changes.
- Roles come from `tags[].metadata.role` (orchestrator, worker) and from the `Task` input
  `subagent_type` for sessions a `Task` call started. Parent linkage comes from
  `session_start.callingSessionId`.
- SPEC §3 M0 and §5.2 are amended as-built in the same integration.

## Alternatives

Read a version from the JSONL (there is none); guess the version from line shapes (a silent
mis-selection is exactly what the versioned mapper exists to prevent, F-007); require users to
pass the version on the command line (a captured file would stop replaying by itself).

## Consequences

Every fixture needs a `.capture.json` with `droid_version` and a sidecar per session, and a
fixture is never replayed without its header. A Droid release that changes either file shape
shows up as a rising unknown-event count against a known version. M0 confirms the remaining open
items of PRD §11.1 (exact validator `subagent_type` strings, feature and milestone ids) in
`docs/decisions/D-0NN-mapping-v1.md`.
