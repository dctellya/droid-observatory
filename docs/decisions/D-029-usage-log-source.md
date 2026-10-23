# D-029: The Droid CLI usage log is an input source

**Date:** 2026-10-23
**Status:** accepted (owner finding F-034 during M1; `ai/RULES.md` rule 7 amended)

## Context

Session files and sidecars carry no per-turn token usage. The sidecar `tokenUsage` is cumulative per
session, with no turn, no lane and no time. The Droid CLI log `~/.factory/logs/droid-log-single.log`
writes one `[Agent] Streaming result | Context: {...}` line per model turn with `inputTokens`,
`cacheReadInputTokens`, `totalInputTokens`, `reasoningTokens` and `outputTokens`. On 2026-10-23 the current
file began at 17:39:24Z because the previous day's file `droid-log-single.log.2026-10-23` had been rotated
mid-day; the rotated files are `droid-log-single.log.<YYYY-MM-DD>`.

The owner's note says the `sessionId` and `modelId` sit on the metrics lines written at the same timestamp
(`[metrics_log_droid_chat_client_success_count]`). Measured with `scripts/usage-from-log.sh` over the two
2026-10-23 files (1583 turns), Droid 0.247.0 also writes `tags.sessionId` and `tags.modelId` on the
Streaming result line itself, and the two joins agree on every turn (0 disagreements, 0 unjoined).

## Decision

The log is an input source beside the session files (`ingest::usage_log`; tailed in attach at M1, replayed
in history, shown in the usage panel at M5). Its line formats are recorded as v1 heuristics:

- **Join:** a Streaming result line belongs to the `sessionId` of the nearest preceding
  `[metrics_log_*]` line that has one, when both lines carry the same second. The line's own `tags.sessionId`
  is read as a cross-check and counted when it disagrees; where it is the only id present it is used.
- **Rotation:** read the rotated files oldest first, then the current file; reset the join state at each
  file start; report the first and last line timestamps used with every table, and mark a table "partial,
  log starts <first ts>" when the oldest file read begins after the milestone started.
- **Fixtures:** extracts keep only the lines of the capture's session ids, pass the PII rules (rule 22) and
  gitleaks like any fixture; the log is never copied whole into fixtures or the store.
- **Sidecar stays:** the `usage_reported` event from the sidecar remains the per-session total; the log
  gives the per-turn detail. The two are reported side by side, never merged.
- **Per-turn event kind:** the name is fixed by `m1-usage-log-source-core` and recorded in this file as an
  addendum.

`scripts/usage-from-log.sh <log...> --session <id>... [--from ts] [--to ts] [--embedded]` is the reference
extraction; `reports/usage/<milestone>.md` carries its per-role table from M1 on (rule 7).

## Alternatives considered

- **Sidecar `tokenUsage` only.** Rejected: no per-turn data, no lane attribution, no way to say how many
  tokens were burnt inside a waste band.
- **Provider billing export.** Rejected: not available to the owner for this account.
- **Join by `tags.sessionId` alone.** Kept as the cross-check, not the rule: the owner's finding names the
  metrics lines, and an older or newer Droid may not write the tag; the heuristic is versioned with the
  mapping.

## Consequences

- The M1 usage tables and the M5 panel report input, output and cache-read tokens per role, per lane and per
  turn; "tokens burnt in waste" becomes a sum over a waste window.
- The log is rotated and can start mid-day: coverage gaps are reported, not hidden (BACKLOG B-037). Two
  sessions logging in the same second can be mis-attributed by the metrics join (BACKLOG B-036).
- `cacheCreationTokens` (cache write) is not in the Streaming result line as a count of its own
  (`cachedTokensWritten` is); the table sums `inputTokens`, `outputTokens` and `cacheReadInputTokens` only.
- Reading the log is read-only; its lines carry session ids, local paths and host identifiers, so it stays out of
  the repository except as filtered, redacted extracts.
