# D-027: The orchestrator never polls

**Date:** 2026-10-23
**Status:** accepted (owner rule F-032 during M1; `ai/RULES.md` rule 25)

## Context

On 2026-10-23 at 18:15Z the desktop app restarted and cancelled the test run of worker `f06338e6`
(`m1-attach-watcher-core`, verify-and-complete). The progress log records `worker_paused` and
`mission_paused` at 18:15:20Z. The orchestrator did not re-dispatch or fail the feature. It waited in
seven identical `WaitForScript` turns until 18:45Z, the mission paused again at 19:02Z (the owner's
figure; the progress log shows `mission_paused` at 19:09:55Z) and the 5-hour standard limit stopped the
mission at 19:10Z (`mission_paused` with `unrecoverable_usage_402` at 19:10:26Z). The last line of the
worker's session file was written at 19:10:26Z. The band from 18:15Z to 19:10Z produced no work, and the
orchestrator turns inside it spent tokens on the same answer (`reports/usage/m1.md`, "Tokens per role").

## Decision

Rule 25 of `ai/RULES.md`, verbatim: "**The orchestrator never polls.** Waiting is one blocking call with a
timeout equal to the feature's remaining cap, not a turn on a timer. If a worker's tool call is cancelled
or its run does not return, the orchestrator re-dispatches the feature or fails it at once, in the same
turn, and records a §9 row with the cause. A second identical tool call with identical arguments is a
deviation under rule 9, not a retry. While the mission is paused the orchestrator does nothing: no
inspection, no narration, no status checks, until the owner resumes it."

The band 18:15Z to 19:10Z is recorded as waste in `reports/usage/m1.md`, cause `waiting on validation`,
note "orchestrator polling".

## Alternatives considered

- **Timer polling as before (a wait turn every few minutes).** Rejected: every wake-up is a model turn that
  bills the whole context and returns the answer of the turn before; it also hides a dead run behind
  activity.
- **Fail the feature at the first cancelled call.** Rejected as the only rule: a cancelled call can be a
  platform event (an app restart), so re-dispatch is allowed. The rule fixes when it happens (at once, in
  the same turn) and that the cause is written down.
- **A longer polling interval.** Rejected: it lowers the cost per hour but keeps the pattern and does
  nothing for a run that will never return.

## Consequences

- The wait is one blocking call sized to the feature's remaining cap (SPEC §2), so a feature that ends
  earlier returns at once and one that never returns costs a single call.
- A cancelled call, a worker that stopped, or a run that does not return produces a re-dispatch or a failure
  in that turn and a SPEC §9 row with the cause. A repeated identical call is a rule 9 deviation.
- While the mission is paused the orchestrator does nothing. The paused time is `throttled` when the
  platform caused it ([D-028](D-028-throttled-waste-cause.md)).
- `ai/RULES.md` gains rule 25 (no renumbering); AGENTS.md "Mission setup" carries it in two sentences;
  SPEC §9 F-032.
- Rule 15, rule 20 and [D-018](D-018-mission-models-are-invariant.md) are unchanged.
