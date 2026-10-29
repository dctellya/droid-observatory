# Staging: m4-usage-idle-host-and-rewrites

Feature `m4-usage-idle-host-and-rewrites` (owner items 6 second half, 7, 8, 12 and 19c of the M3 boundary), worktree `observatory-wt/m4`,
branch `wt/m4`.

## SPEC §9

- Row (F-NEW), tooling change (rule 26, verified on 2026-10-29 by the scripts' own tests and a regeneration of the M3 window on the real
  logs): `scripts/usage-report.py` gains six sections, in three new modules (`usage_idle.py`, `usage_testing.py`, `usage_rewrites.py`,
  standard library only). (1) `Owner interventions` (item 6): the `mission_resumed` records of `progress_log.jsonl`, and from the
  orchestrator transcript the answer to each `AskUser` call and each `StartMissionRun` with a `messageToWorker`, each with its time and the
  feature it unblocked (the session it resumes, else the next `worker_started`). (2) `Idle time by cause` and `Idle gaps` (item 8): a gap
  between two consecutive events (a model turn of any session in the Droid log, a `progress_log.jsonl` record) over 15 minutes is idle
  time, with the cause `usage limit` (throttled pause, or `[LLM] Chat route failure` `paymentRequired`), `host` (ECONNREFUSED,
  EADDRNOTAVAIL, ENOBUFS, ENETDOWN, ENETUNREACH, or a route failure with another reason), `owner` (owner pause or intervention) or
  `unexplained` (listed with its bounds). (3) `Host health` and `Model call faults` (item 12): `sysctl kern.boottime` pasted, uptime at
  generation and at the window end against the 49.7-day macOS TCP clock limit (new `--boottime` option for tests), the count of stalled or
  retried model calls (`[LLM] Chat route failure`, `"retried":true`, `"providerRotated":true`, usage limit excluded) and one `host verdict:`
  line. (4) `User testing rounds` and `User testing flow groups`: cost per round (validator session plus every subagent under it) and per
  flow group (a Task of the validator, matched by `callingToolUseId` to its `description`), one `user testing:` line. (5) `Lead context
  rewrites` and `Lead context rewrites per milestone` (item 19c): the lead's turns whose cache write exceeds the cache read by at least
  the context size (median nonzero cache read of the lead's turns in the window, measured), with tokens, USD-eq and the share of the lead's
  cost. (6) The `weekly allowance share:` line (item 7): the window's cost per assertion times the assertions the next milestone claims in
  `fulfills` (`--next-milestone`, default the next `m<N>` of `features.json`; its order is not chronological), for the orchestrator to
  state before it dispatches the next milestone. Real M3 window (`--from 2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z`): 7 owner
  interventions (5 `mission_resumed`, 1 AskUser answer, 1 `messageToWorker`); host gap 2026-10-25T09:12:16Z to 10:18:07Z (65.85 min,
  EADDRNOTAVAIL 1, ECONNREFUSED 5) is `host`, 10 stalled or retried calls, verdict `inflated`; 17 flow groups in 4 user-testing rounds,
  56.47 USD-eq (validators 16.71, subagents 39.76); 8 rewrites of 194 lead turns (context 132 984 tokens), 20.44 USD-eq, 29.7 % of the
  lead's 68.81; `weekly allowance share: m4 claims 39 assertions x 5.55 USD-eq per assertion (m3 window, 38 fulfilled) = 216.53 USD-eq`,
  which is above the M4 cap of 200 (D-038). Evidence: `scripts/tests/usage_idle_test.py`, `usage_testing_test.py`, `usage_rewrites_test.py`
  and `usage_test.py`, red then green per addition (commits in the handoff). **Alternatives considered:** count only `progress_log.jsonl`
  and turn events and leave the failure lines out of the cause (rejected: the 23 October 16:11 to 18:36 local gap would read `unexplained`
  although the Droid log holds two `paymentRequired` failures in it; the table keeps the evidence so the owner can see either reading); a
  fixed 136 000-token context for the rewrites (rejected: the context differs by window and model; it is measured); the next milestone as
  the next block in `features.json` (rejected: the file lists m4, m5, final, s, m0 to m3 in that order).

## Decisions

None.

## BACKLOG

- Closed: owner items 6 (second half), 7, 8, 12 and 19c for the report (interventions, idle time by cause, host health, user testing
  cost per round and group, the lead's rewrites, the weekly allowance share line). Item 19c's target (under 5 USD-eq per M3-size
  milestone) is a measure for M4, not met by the tool alone.
- B-NEW: the rewrite count on M3 is 8 turns and 20.44 USD-eq, not the owner's 16 turns and 24.16 USD-eq | The owner's figures came from a
  narrower turn set (104 lead turns, 32.07 USD-eq in total); on the report's window of 194 lead turns no cache-write-over-read threshold
  between 30 000 and 100 000 tokens gives 16 turns (27 turns 48.33 at 30 000, 19 turns 39.47 at 66 000, 13 turns 30.19 at 100 000). The
  literal rule (exceed by the context size) gives 8; the owner decides whether 16 means a different rule (for example cache write over
  three times the cache read, which gives 16 turns and 32.35 USD-eq on the build window 07:59Z to 18:53Z). Every rewrite but one reads
  38 251 tokens of cache (the fixed prefix): the lead's context fell to the prefix and was rebuilt, which is a compaction or an expiry. | M4
- B-NEW: the 23 October 16:11 to 18:36 local gap (14:11:20Z to 16:36:30Z, 145.17 min) is not `unexplained` | The owner expected it as
  `unexplained` in M2's report; it belongs to M1's window (before 2026-10-24T04:39:58Z), and the Droid log has two `[LLM] Chat route failure`
  `paymentRequired` lines in it (resets in 2h 24min), so the report says `usage limit`. It reads `unexplained` only if the failure lines
  are ignored as evidence, which this feature does not do. M2's own window shows two unexplained gaps (17.82 and 16.32 min on 24 October,
  20:34Z and 21:11Z). | owner
- B-NEW: the host verdict cannot see the uptime before a reboot | `kern.boottime` is the current boot (2026-10-25T10:51:46Z), 33 minutes
  after the host fault of 09:12Z to 10:18Z; the window-end uptime of 0.31 days is the new boot's. The TCP limit check is valid only for a
  report generated before a reboot; the verdict for M3 rests on the host-fault idle time and 10 stalled or retried calls instead. | unscheduled

## Deferred clauses

- The gate on the final tree (item 17b) is not recorded here: this feature is not the last code change before the integration.
- `reports/usage/m3.md` is not regenerated here (`reports/` is shared, rule 13); the real-window figures above are from a scratch output.
- The feature has no `fulfills`, so it claims no assertions (rule 30).

## Library

- `tooling.md`, usage report: new sections `Owner interventions`, `Idle time by cause`, `Idle gaps`, `Host health`, `Model call faults`,
  `User testing rounds`, `User testing flow groups`, `Lead context rewrites`, `Lead context rewrites per milestone`; summary lines
  `host verdict:`, `user testing:`, `context rewrites:` and `weekly allowance share:`; new options `--boottime <text|unavailable>` and
  `--next-milestone <m<N>>`; new modules `scripts/usage_idle.py`, `usage_testing.py`, `usage_rewrites.py`; `make check-scripts` also
  runs `usage_idle_test.py`, `usage_testing_test.py` and `usage_rewrites_test.py` (about 35 s in all).
- Test helper `scripts/tests/usage_scratch.py` (`Scratch`) builds a scratch copy of the usage fixture on 2026-10-26 with `turn`, `progress`,
  sidecar, `transcript`, `features` and `run` helpers; new tests use it instead of editing the shared fixture.
