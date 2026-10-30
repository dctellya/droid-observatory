# D-028: `throttled` is a waste cause of its own

**Date:** 2026-10-23
**Status:** accepted (owner rule F-033 during M1; SPEC §5.6, PRD §10.1)

## Context

The waste causes of SPEC §5.6 (eight, shown as five groups, F-028) all describe time in which someone
could have acted. A mission also stops because the platform stops it: the 5-hour standard limit on
2026-10-23 (`mission_paused` with `unrecoverable_usage_402` at 19:10:26Z, `mission_resumed` at 21:43:09Z in
the progress log) and the overnight stop of the M0 capture, about 755 minutes by the owner's figure. Put
into idle or into waiting on a human, those hours would drown the signals the Panel exists to show:
nobody was asked anything and nothing was scheduled and blocked.

## Decision

`throttled` is the time the platform stopped the work: a usage window exhausted or a platform pause. It is
measured from `mission_paused` to `mission_resumed`, with the reason recorded (`unrecoverable_usage_402`
and the like). It is not idle and not waiting on a human. It is reported per milestone and per run,
separately from the eight causes and from the five display groups. PRD §10.1 time states gain the row
"Throttled", marked "Added by F-033 (2026-10-23)"; the M5 engine and Panel compute it, the engine pastes the
measured figure for the M0 capture.

## Alternatives considered

- **Fold it into idle.** Rejected: idle (queued or unscheduled) is a scheduling signal and the bottleneck
  metric reads it; a platform stop is neither.
- **Fold it into waiting on a human.** Rejected: no question or approval was open, and every usage reset
  would inflate the human-wait causes the owner acts on.
- **Drop it from the waste total and say nothing.** Rejected: rule 10 asks for honest time; the stop is
  part of the elapsed time of the milestone and of the run.

## Consequences

- The eight causes and the five groups keep their meaning; `throttled` sits beside them in the Panel and
  the CSV, per milestone and per run.
- `reports/usage/m1.md` carries a Throttled row from `mission_paused` to `mission_resumed`.
- A pause the orchestrator caused by waiting (the 18:15Z to 19:10Z band of [D-027](D-027-orchestrator-never-polls.md))
  is not throttled while work could run; it is recorded as `waiting on validation` with the note
  "orchestrator polling".
- The engine needs `mission_paused` / `mission_resumed` and the pause reason from the progress log or the
  session stream; where a source has neither, the cause is not computed and the report says so (rule 18).
