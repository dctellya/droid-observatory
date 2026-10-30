# D-049: The mission notify-and-ask line moves from 1500 to 1700 USD-eq at the M5 boundary

**Date:** 2026-10-30
**Status:** accepted; owner steering in chat, 2026-10-30 12:52Z (the orchestrator's note to the scribe is the record, no inbox file exists for it)

## Context

D-048 set the mission notify-and-ask line at 1500 USD-eq and the M5 cap at 190 (the cap in force at the boundary is 200).
The M5 cap moved 190 → 200 by the owner's chat at about 12:15Z on 2026-10-30 (mission record row 88), before this decision; D-048
stays at 190 as written. M5 closed at 186.13 USD-eq in the window
(measured 2026-10-30 12:15:25Z, `--from 2026-10-29T22:30:00Z`: orchestrator 81.83, orchestrator subagents 26.33, workers 36.88,
validators 39.52, self-check 1.43, other 0.14), under the cap of 200. Mission-to-date is 1446.87 USD-eq, 253.13 under 1700 and
53.13 under the old line. The final pull request, PR #40, opened on 2026-10-30 with head `72d0412`, is red (run 41052611163: `sast`
8 Semgrep findings, `fs-scan` 2 Trivy findings on a `ghp_fake` placeholder in a build-log file); fixing it and re-running
`final-open-pr` is paid out of the remainder, and M6 follows.

## Decision

- The mission line moves from 1500 to **1700 USD-eq**, all roles. It stays a **notify-and-ask line under R-055, never a wall**: at
  1700 mission-to-date and at a usage-limit stop the orchestrator reports the state and the figures and asks whether to proceed.
- The **M5 cap stays 200** (140, 150 and 190 before; D-048 records the path to 190). M5 is closed at **186.13**; the orchestrator
  figure, 81.83 against 30, is the measured one (the committed record `reports/usage/m5.md` keeps 71.27 as written).
- The **M6 cap is set at dispatch**, priced by role, with validation priced by rounds from M5's record (not a fixed number in
  advance).
- The **carry-over slice is booked first in M6**: feature `m6-carryover-fix-pr-gate`, then `final-open-pr` re-run, as the slice
  "M5 carry-over: release gate". VAL-PROC-036 and 037 are validated at M6 as a carry-over.

## Alternatives

1. **Keep 1500.** Rejected: mission-to-date is 1446.87, so the line would stop the carry-over fix and M6 in their first slices.
2. **Remove the line.** Rejected for the reason of D-038: the line makes the orchestrator state the need before dispatch (R-055).
3. **Fix M6's cap now.** Not kept: the owner ruled it is set at dispatch, priced by role from M5's record.

## Consequences

- `AGENTS.md` (repo) "Money" bullet states 1700 and M5 200 and points here.
- The mission `AGENTS.md` and `reports/usage/m6.md` compare the mission with 1700 from M6 on; mission-to-date stays the recorded
  M4 base plus the window totals, because the rotated logs cannot regenerate the earlier months (B-254).
- The record stays as written: `reports/usage/m5.md` prints 71.27 for the orchestrator and 163.57 for the window; the figures of
  this decision are the 12:15:25Z measurement (SPEC §9 F-346).
