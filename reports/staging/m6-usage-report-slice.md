# Staging: m6-usage-report-slice

## SPEC §9

| F-next | 2026-10-30 | Row `infra-worker` (kind: report rule, inbox 1350 items 1, 12 and 15; owner rows 91, 102, 105): `scripts/usage-report.py` splits the orchestrator turns into heartbeat, boundary and unmatched turns (`scripts/usage_heartbeat.py`, detector in the report header with a matched and an unmatched example), marks the recorded Ratios rows `(recorded), not comparable` and recomputes the previous milestone on the pinned bases with `--previous <m>`, and books idle time by cause (usage limit from the router's `detector_recovery` overage transitions until the next `manual` one, owner from owner chat messages and interventions, runner idle from a pause nobody answered) with the three clocks. M5 recomputed: usage limit 134.00 min (recorded as owner), owner 31.52 (recorded as unexplained); `reports/usage/m5.md` carries the dated addendum. | owner inbox `2026-10-30-1350-owner-m5-feedback-long-form.md` items 1, 12, 15 | applied |

## Decisions

- D-029 (usage is Panel data, the usage-log source): amend with one line, no status change: the usage report's idle attribution is booked by cause with the router transitions and the owner's chat messages as evidence (`scripts/usage_idle.py`); the M5 recomputation is in `reports/usage/m5.md` addendum 2026-10-30.

## BACKLOG

- New: the clocks use the report window, not the owner's dispatch-to-landing times (the m5 report window starts at 22:30Z, the owner's wall at 23:47 local). Reason: the tool has no landing marker; a `--clock-from/--clock-to` pair or the first `worker_started` and the last `worker_completed` would give the owner's reading.
- New: the previous milestone's assertion count is the current `validation-state.json` (`validatedAtMilestone`), so a later re-validation at another milestone moves it. Reason: the state file carries no timestamps.

## Deferred clauses

None.

## Library

- `make usage M=m6 USAGE_ARGS='--previous m5'` prints the m5 window recomputed (Ratios row, Idle time by cause beside the recorded figures, Clocks).

### Claims (rule 30)

model = claude-sonnet-5-5 (read from `~/.factory/sessions/-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/45ea27cf-b0b9-4bba-bd97-1c88a1eeee29.settings.json`; `model-settings.json` `workerModel` is `claude-sonnet-5-5`, match)

- Item 1 (heartbeat split): command `python3 scripts/usage-report.py --milestone m5 --from 2026-10-29T22:30:00Z --to 2026-10-30T11:12:26Z --out /tmp/m5-recomputed.md --now 2026-10-30T20:00:00Z`; output rows `heartbeat 122 turns 31.93 USD-eq 23.1 %`, `boundary 400 turns 39.16 USD-eq 75.6 %`, `unmatched 7`, `total 529 turns 71.27`, line `boundary turns per feature boundary: 400 / 23 = 17.4`, the detector line and one matched example (`2026-10-29T22:32:36.079Z ... setTimeout(r, 240000)`) and one unmatched (`2026-10-30T06:45:14.594Z ... (2 tool calls)`).
- Item 12 (pinned bases): command `python3 scripts/usage-report.py --milestone m6 --previous m5 --previous-report reports/usage/m5.md --to 2026-10-30T19:30:00Z --out /tmp/m6-run.md`; the Ratios table reads `m4 (recorded), not comparable` and `m5 (recomputed, pinned bases) | 39 | 0.54 | 4.48 | 1.68 | 0.41 | 0.15 | 4.08 | 4.19`, with the `m5 bases:` line; pasted in `reports/usage/m5.md` addendum 2026-10-30.
- Item 15 (idle by cause, clocks): same command; `Idle time by cause, m5 recorded and recomputed` reads usage limit 0.00 -> 134.00, owner 134.00 -> 31.52, unexplained 31.52 -> 0.00, total 165.51 both; `Clocks, m5 recomputed`: wall 762.43, held 134.00, cycle 628.44, owner wait 31.52, system 596.92.
- Tests, red then green: `python3 scripts/tests/usage_heartbeat_test.py` and `usage_clocks_test.py` failed (exit 1, `FAILED (failures=3, errors=1)` and `FAILED (failures=5, errors=2)`) at ba8aed4 and pass (`OK`, 4 and 9 tests) at 63a7f10.
