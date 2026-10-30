# D-038: Money for M4 and M5: the mission line moves from 950 to 1350 USD-eq, caps M4 200 and M5 140

**Date:** 2026-10-29
**Status:** accepted; owner decision of 2026-10-25 (M3 boundary, item 11), received 2026-10-29, amended by the owner's go for M4 of 2026-10-29 (the 1350 line notifies and asks, it does not stop the mission)

## Context

The caps of F-055 (M3 110, M4 110, M5 60, orchestrator 30 per milestone, stop-and-decide at 950 USD-eq of the record) were
set at the M2 boundary. The M3 record, from `make usage M=m3` and the owner's reading (item 10 of the M3 boundary,
build window 08:59 to 19:53 local on 25-oct, every model priced, each subagent under the session that started it):

- M3 with its boundary: 216.88 USD-eq against the cap 110 (build 188.81, like-for-like 171.51, boundary before the build 28.07).
  M3 crossed its cap at 178.46.
- Mission to date: 1007.73 USD-eq against the 950 line. The mission crossed 950 at 957.57, two features before the boundary.
- Cost per assertion, like-for-like: M3 4.75 with its boundary (4.08 build only) against M1 6.48 and M2 12.57.

With M3 at about 200 including its boundary, the old M4 and M5 caps (110 and 60) would stop each milestone mid-plan.

## Decision

- The mission line moves from 950 to **1350 USD-eq**, all roles.
- Milestone caps: **M4 200**, **M5 140**. The orchestrator cap stays **30 per milestone**.
- The caps and the line stay **stop-and-decide lines under R-055, never walls**. A milestone that will need more than its cap
  says so before dispatch (the allowance statement of the boundary report).
- Amendment by the owner's go for M4 (2026-10-29, verbatim in `owner-feedback/2026-10-29-m3-boundary-items.md`, last section):
  1350 USD-eq is a **notify-and-ask line, not a hard stop**. At 1350 USD-eq mission-to-date, and at a usage-limit stop, the
  orchestrator reports the state and the figures and asks whether to proceed; it never ends the mission on its own. The runner
  still pauses by itself on a 402 (F-057). The orchestrator's forecast for M4 (162 USD-eq) is a forecast, not a gate; the weekly
  allowance is the owner's to manage and the orchestrator does not check it.

## Alternatives

1. **Keep 950, M4 110, M5 60.** Rejected: every milestone would end in a stop mid-plan; M3 alone used 216.88.
2. **A hard stop at 1350.** Offered by the owner's first wording ("hard stop"), replaced by the notify-and-ask line in the go
   for M4: the owner prefers to decide with the figures in front of them over a mission that ends by itself.
3. **Remove the caps.** Rejected: the lines are what makes the orchestrator state the need before dispatch (R-055).
4. **Raise the orchestrator cap.** Not asked; the orchestrator's 68.81 in M3 against 30 is a deviation to be reported, and the
   levers of items 5, 15 and 19 aim at it.

## Consequences

- SPEC §2 (usage table and "Caps and stops") keeps the old figures as history and states the new ones; README's status
  paragraph says the mission continues into M4 under the 1350 line.
- The M4 and M5 boundary reports compare each milestone with 200 and 140, the orchestrator with 30, and the mission with 1350.
- The mission is judged against the record from `make usage`, not against the Mission Control gauge (F-067).
- The M4 boundary reports whether the cost plan (item 15, target about 2 USD-eq per assertion) worked.
