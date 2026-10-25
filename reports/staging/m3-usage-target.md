# Staging: m3-usage-target

Feature `m3-usage-target` (owner M2-boundary item 18), worktree `observatory-wt/m3`, branch `wt/m3`.
VAL-TOOL-005. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): `make usage M=<milestone>` runs `scripts/usage.py` (python3 stdlib only) and writes the tables of
  `reports/usage/<milestone>.md` between `<!-- usage:begin -->` and `<!-- usage:end -->`; prose outside the markers
  is kept, a file without markers gets the block appended, a new file gets a title and the block. The script is
  named `usage.py` as the feature text says; the contract (VAL-TOOL-005) and `library/tooling.md` say
  `usage-report.py`, so those two names change to `usage.py`. Sources: `~/.factory/logs/droid-log-single.log*`, the
  session sidecars, the orchestrator transcript, `progress_log.jsonl` (`OBSERVATORY_MISSION_DIR`, default the mission
  directory), `config/prices/<newest>.toml`, `reports/gate/*/*.json`. Window `--from` (default: first
  `worker_started` of `<milestone>-*`) to `--to` (default: generation time minus 1 s), compared at second
  resolution as `scripts/usage-from-log.sh` does. Caps are passed, not typed in code:
  `make usage M=m3 CAPS="orchestrator=30 worker=110"`.
- Row (F-NEW): role classification is deterministic from the sidecar tags and `progress_log.jsonl`: orchestrator =
  `mission-session` role `orchestrator` of this mission; worker or validator = `mission-session` role `worker` of
  this mission, validator when the `worker_started` feature matches `^(scrutiny|user-testing)-validator-`;
  subagent = `subagent` tag, role of the caller's root (an orchestrator's subagent is `orchestrator subagent`;
  a validator's reviewers and flow validators count as validators, as `m2.md` did); self-check = a
  `mission-session` of another mission id (the scratch missions) and its subagents; everything else is `other`.
- Row (F-NEW): the orchestrator split joins each log turn of the orchestrator to the nearest top-level assistant
  record of the transcript within 3 s (the m2 method; 251 of 251 matched for M2). Class `waits`: the turn's tools are
  only `WaitForScript`, or only `StartMissionRun` straight after a wait turn; `decisions and boundary reports`: any
  of Edit, Create, Task, TaskStop, StartMissionRun, DismissHandoffItems, AskUser, ProposeMission, ExitSpecMode,
  ApplyPatch, a background Script, a shell call whose text writes (redirect, tee, git add/commit/merge/push, mv, cp,
  rm, mkdir, touch, sed -i), or no tool call at all (text to the owner); `narration`: the rest; `unmatched`: no
  transcript record. Tools inside a `Script` are read from its `tools.<Name>(` calls. The stated line is
  `re-issued waits: N`; `WaitForScript calls in the window` and `waits returning in under 60 s` follow it.
- Row (F-NEW): reproducibility. Every table is bounded at `--to` (log lines, progress records, transcript records,
  gate records), so two runs with the same `--to` differ only in the `generated:` line, with one exception: the last
  section, `Sidecar readings` (credits by role and sidecar totals per session), reads lifetime totals of sessions that
  may still be running (the orchestrator, the integrator), so it can move between runs. The contract's "byte for byte"
  diff excludes that section (or runs when no session is live). A second limit: the Droid log is flushed late
  (a turn timestamped 11:26:55Z was absent when the script read the live log at about 11:27:15Z and present in the
  `usage-from-log.sh` pass seconds later), so a window ending within some tens of seconds of generation can grow on a
  re-run and the cross-check can print `cross-check: DIFFERENT` for the live session; use a `--to` older than that
  (the M2 acceptance below is stable).
- Row (F-NEW): acceptance against `reports/usage/m2.md` (not edited). Same window `2026-10-24T04:27:54Z` to
  `2026-10-24T19:36:40Z`: orchestrator 251 turns / 188317 / 151058 / 17784767 (input / output / cache read) equals the
  m2 row, and `118.84` USD-eq equals the m2 log split. Summed over the session lists printed in `m2.md`, workers
  (9 sessions) 720 / 1504 / 615393 / 91429803 and validators (22) 412 / 908 / 503976 / 31990959 equal the m2 rows
  exactly. The script's own role rows differ from the m2 rows by two sessions groups, both stated in `m2.md`: the M1
  integrator `cbdcd8ef` (39 turns in the window; `m2.md` says its M2-window turns belong to `m1.md`) is a worker
  here, and the `model fallback (void)` sessions (the `kimi-k3` reviewers and validators, the void worker
  `f4033628`) are in their roles here and listed again in the `Model fallback` table, where `m2.md` kept them in a
  separate line. `scripts/usage-from-log.sh --embedded` over the same window gives identical rows for all 97
  sessions (`cross-check: identical`).

## Decisions

- D-029 (amend, one line in Consequences): "The per-role tables of a usage report are written by `scripts/usage.py`
  (`make usage M=<m>`) from the log, the sidecars and the progress log; `scripts/usage-from-log.sh` stays as its
  cross-check."

## BACKLOG

- B-NEW: `scripts/gate.sh` writes no record for a cache hit, so the usage report cannot state "gates served from
  cache" or the minutes saved (the Gates section prints `served from cache: not recorded`). Fix: append one line per
  hit (time, name, tree, the stored `seconds`) to `reports/gate/hits.log`, then add a `Hits` table to `usage.py`.
- B-NEW: `usage.py` does not read `features.json` (the contract lists it); the milestone window comes from the
  `worker_started` entries and the feature id prefix. Add it when a table needs a feature title or a cap per feature.
- B-NEW: `build()` in `scripts/usage.py` is one long function that emits the tables in order; split it per table when
  a second report format (JSON for the app) is wanted.
- B-NEW: `reports/usage/m2.md` has no markers. `make usage M=m2` appends a generated block at its end; the integrator
  decides whether to wrap the hand-made tables of `m0.md` to `m2.md` or leave them.

## Deferred clauses

- VAL-TOOL-005: "gates served from cache with the minutes saved" has no source (see the first BACKLOG row); the
  table prints the runs and the sentence `served from cache: not recorded`.
- VAL-TOOL-005: `features.json` is not a source (second BACKLOG row).
- VAL-TOOL-005: the `Credits` of a role come from the sidecars (lifetime, not windowed); the log has no credits.

## Library

- `library/tooling.md`, row "Usage report": command `make usage M=<m> [CAPS='role=USD ...'] [USAGE_ARGS='--from <ts>
  --to <ts> --out <path>']`; script `scripts/usage.py`; stdlib only; about 14 s over the 9 log files of 2026-10-25
  (`time make usage M=m3`); the unit test is `python3 scripts/tests/usage_test.py` (also `make check-scripts`).
