# D-048: Money for M5: the mission line moves from 1350 to 1500 USD-eq, the M5 cap is 190

**Date:** 2026-10-30
**Status:** accepted; owner decision of 2026-10-29 23:40 (M4 boundary, answers relayed in inbox `2026-10-29-2345-owner-answers-row-50`), the M5 cap amended from 150 to 190 by the owner's chat message of 2026-10-30 09:55Z

## Context

D-038 set the mission notify-and-ask line at 1350 USD-eq and the milestone caps at M4 200 and M5 140. M4 landed at 253.73 USD-eq
(D-043, accepted by the owner on condition that the findings are actioned), and the mission-to-date record at the M4 boundary was
1260.74 USD-eq against the 1350 line (the base `reports/usage/m5.md` carries forward, because the log files before 2026-10-22
have rotated out of `~/.factory/logs`). An M5 at the old 140 would have stopped mid-plan: the forecast for M5 under Rules C to F
with Sonnet reviews and no inserts was workers 65 to 80, validators 20 to 30, orchestrator 15 to 25 and seats 10 to 15 USD-eq
(inbox `2026-10-29-2345-owner-answers-row-50`, item 2),.

## Decision

- The mission line moves from 1350 to **1500 USD-eq**, all roles. It stays a **notify-and-ask line under R-055, never a wall**: at
  1500 mission-to-date and at a usage-limit stop the orchestrator reports the state and the figures and asks whether to proceed.
- The **M5 cap is 150** (owner, 2026-10-29 23:40, inbox `2026-10-29-2345`), raised to **190** by the owner on 2026-10-30 at 09:55Z
  (chat; the orchestrator's note to the integrator is the record, no inbox file exists for it). The orchestrator cap stays 30
  per milestone. The overnight delegate's stops were M5 above 150 and a second usage limit (AGENTS.md "M5 rules").
- Both lines are **stop-and-decide lines under R-055, never walls**; 190 is the M5 line now in force.
- The verdict condition for M5 (item 14) is re-based by contract pass 36: cost per assertion on the pinned definition of inbox
  `2026-10-30-0145` (window, w, coordination, validation, tooling, integration; `scripts/usage_ratios.py`), M5 beside M4's 7.22 and
  w 0.56, from the same script. `reports/usage/m5.md` prints both.

## Alternatives

1. **Keep 1350 and M5 140.** Rejected: the mission was already at 1260.74 before M5 started, so the line would stop M5 in its first
   slices.
2. **Remove the caps.** Rejected for the reason of D-038: the lines make the orchestrator state the need before dispatch (R-055).
3. **Hold M5 at 150.** Not kept: the owner raised it to 190 on 2026-10-30 at 09:55Z.

## Consequences

- `AGENTS.md` (repo) "Money" bullet states 1500, M5 190 and points here; the mission `AGENTS.md` M5 rules already read the same.
- `reports/usage/m5.md` compares the M5 window with 190 and the mission with 1500: mission-to-date is the recorded M4 base 1260.74
  plus the M5 window total, because the rotated logs cannot regenerate the earlier months (BACKLOG row from
  `m5-fix-usage-ratios-kind-weight`).
- The final pull request is paid out of the remainder under 1500; a mission that crosses it asks, it does not stop by itself.
