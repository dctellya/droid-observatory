# D-043: The M4 slip is accepted on one condition; caps not raised, the slip is recorded as data

**Date:** 2026-10-29
**Status:** accepted; owner inbox 2026-10-29-1900 (18:5x), applied at the M4 boundary `m5-integrate-m4`

## Context

M4 landed over its 200 USD-eq cap and over M3 on cost per assertion. `reports/usage/m4.md` (window 2026-10-25T18:19:15Z to
2026-10-29T22:05:24Z): 253.73 USD-eq against the cap 200 (over by 53.73), 7.93 USD-eq per assertion fulfilled against M3's 4.75
(F-147), the orchestrator 111.86 against 30, 10.3 orchestrator turns per feature boundary against the target 3. The cause the
owner names: tooling ordered by the M3 boundary, more boundaries from the five-assertion slice cap (item 21), a behavioural-heavy
mix, and one unexplained cause found and fixed in-cycle, the orchestrator's turns per boundary (7.9 before, 4.1 after 14:23).

The owner, verbatim:

> As CEO of Dalimaray, I'm accepting the M4 slip with the condition that we action our findings ASAP (either now if we can, or in the next milestone). What I'm impressed by is that we detected the root cause and we can action over it.

## Decision

- Verdict "worse, explained": the slip is accepted as data. The caps are not raised (M5 140, orchestrator 30, mission line 1350, D-038).
  The slip is recorded and not tuned away (the "D-158 kept" of the owner's note is shorthand: no file D-158 exists in this
  repository; the rule it points to is F-158, owner item 21, rolled back by D-045).
- The condition is a checklist that the boundary report answers item by item (the report and the session log entry carry the
  answers):
  - Applied in M4, now: rule A three turns per boundary; forge off and no keychain from any worker process (F-218, F-219, F-231);
    e2e first-failure evidence and one rerun (F-169); the token-literal rule; the owner inbox as the channel
    (`docs/owner-feedback/inbox/`); rule C, the officer decides (D-041, hook `a4b4646`, allow-list `060698a`).
  - Applied in the M5 plan before dispatch: slices at eight to ten assertions under the turn budget and batches of independent
    slices (D-045); a scribe seat and a diagnostician seat for the officer (D-041); the forecast by role with the three allowance
    windows; the readiness read with the expected figures written as the verdict condition; review subagents on Sonnet with Opus
    for the synthesis (D-044); `/compact` by the owner at the boundary.
  - Raised to Factory as product feedback: a one-hour cache TTL or a wait under the TTL; a "wait for reset" option for "when
    limit is reached"; a decision protocol for the orchestrator.

## Alternatives

- **Raise the M4 or M5 cap to the measured figure:** rejected by the owner; a cap that follows the spend measures nothing.
- **Hold the milestone until the findings are fixed:** rejected; the owner allows "now or in the next milestone".

## Consequences

- The M4 boundary report states the verdict and the checklist answers; the M5 boundary reads the same figures again.
- The weekly allowance share for M5 (41 assertions x 7.93 = 325.10 USD-eq) is stated before dispatch (D-038, F-144); it is a
  forecast for the owner to manage, and the mission is past 93 % of the 1350 line, so the orchestrator stops and asks before
  dispatch.
