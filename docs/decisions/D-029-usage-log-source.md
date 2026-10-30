# D-029: The Droid CLI usage log is an input source

**Date:** 2026-10-23
**Status:** accepted and implemented in `m1-usage-log-source-core` (owner finding F-034 during M1; `ai/RULES.md` rule 7 amended); merged in `c2fb42acf97883c1e1aa0458ebbc26fbc938bad2`, addendum below

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

## Addendum 2026-10-24 (M1 integration, `m1-usage-log-source-core`, `b600c8c`)

- **Kind name:** `turn.usage` (`EventKind::TurnUsage`, serde `turn_usage`), one event per `[Agent] Streaming
  result` line, with `input`, `output` and `cache_read` tokens, the join source (`usage_log`) and the
  model id.
- **Join as implemented:** the nearest preceding same-second `[metrics_log_*]` line that carries a
  `sessionId`; the line's own `tags.sessionId` is the cross-check. On the M0 extract `join mismatches: 0`.
- **Lane of a turn:** its session's current phase, not the lane of the session's `subagent_spawned`. 105
  turns in 3 integration-feature sessions (2c04bd42 60, 36d06ed0 38, 82a61cf6 7) therefore sit in
  `integrate` while their `subagent_spawned` sits in `build`; the contract wording was changed to match.
- **Extract:** `fixtures/streams/2026-10-22-observatory-v1-m0.usage-log.jsonl`, written by
  `scripts/extract-usage-log.py`: 2230 lines (1115 turns + 1115 neighbouring metrics lines), first_ts
  `2026-10-22T10:18:19.200Z`, last_ts `2026-10-23T09:21:59.213Z`, SHA-256
  `c9b5e2747b23aba4a8cf09f2e0e85cf1b85ef0cdd8f39c671b230deade91637d`. Machine ids are zeroed to
  `00000000-0000-4000-8000-000000000000` (`usage_log.extra_redaction` in the M0 `.capture.json`). The early
  lines carry Droid 0.245.0 tags.
- **Golden:** 2394 events (1279 session events + 1115 `turn.usage`), SHA-256
  `25d91c53b580f8f5f3f1a62ecaa977a24dca0b61f5caf86e296182e8f23a5750`, refrozen at the M1 integration.
- **Rule 24 decision (orchestrator): `turn.usage` is Panel data.** It feeds tokens per role and per lane and is
  excluded from lane rows, cards, the tray and the Details timeline. Alternatives: show turns as rows (inflates
  the lane counts by 1115 on the M0 capture); a separate lane (D-024 has seven).
- **Store:** the `source` CHECK was widened in place in `0001_init.sql` (scratch stores from before `b600c8c`
  must be recreated). The store keeps `input`, `output` and `cache_read` of a turn only; `reasoning` and
  `total_input` read back as 0 (BACKLOG B-043).

## Addendum 2026-10-25 (m3 integration, m3-gate-hits-and-usage-name)

the script is `scripts/usage-report.py` (already named so in the m3-usage-target staging file, which this feature corrected).

## Addendum 2026-10-25 (m3 integration, m3-usage-target)

The per-role tables of a usage report are written by `scripts/usage-report.py` (`make usage M=<m>`) from the log, the sidecars and the progress log; `scripts/usage-from-log.sh` stays as its cross-check.

## Addendum 2026-10-30 (m6 integration, m6-reference-csv-oracle-parity)

PRD 10.5 sidecar-sum wording moves to D-029; the CSV token columns read turn.usage (F-034, METRIC-024). Owner ruling Q-1, 2026-10-30 ~17:25Z; the tokens of a CSV row are the sum of the `turn.usage` of its sessions, `tokens_cache_write` stays the sidecar's, the two agree on `cache_read` and `cache_write` on all 14 rows of the M0 capture.

## Addendum 2026-10-30 (m6 integration, m6-usage-report-slice)

No status change. The usage report's idle attribution is booked by cause with the router transitions and the owner's chat messages as evidence (`scripts/usage_idle.py`); the M5 recomputation is in `reports/usage/m5.md` addendum 2026-10-30.
