# VAL-E2E-004 and VAL-PROC-019: what the attended record already holds

Source: owner (David), 31-oct-2026 00:0x, "document it please", relayed by pre-roci after the M6 close. Record only: no status change, no contract pass.

Both assertions are `handed_to_qa` (pass 43, rulings B and C; inbox 2129 item 5). That status does not mean they were never exercised.

## VAL-E2E-004 (suite on a locked display): exercised in part, attended, on M3 code
- The owner made two attended runs at the M3 boundary (25-oct; answer to Q1, SPEC F-160 in docs/spec/section-9-archive.md).
- Run 1 was unlocked: "Not evidence".
- Run 2: only the last spec (stall banner, 3 tests) ran while the display was locked; green, PNGs written while locked. The owner's reading then: "VAL-E2E-004 is not yet met".
- Earlier, the m3-e2e-harness session ran `make e2e` after `pmset displaysleepnow` (exit 0, 11 passing, `document.visibilityState = visible`) but could not confirm the display slept (reports/readiness/2026-10-25-m3.md).
- Item 18 (rule, cycle; blueprint R-023 under R-057, F-155) then moved every clause that needs a person to QA, never graded, never asked of the owner.
- Net: 3 of 11 tests proven on a locked display; 8 never run locked; nothing run locked on M4 to M6 code. The 11 tests pass unlocked in every gate run since M3. D-037: screenshots of a locked display are not evidence; the DOM assertions are.
- To close in full (optional, attended): `make e2e` once with the screen locked, paste `document.visibilityState` per worker.

## VAL-PROC-019 (owner screenshots per integration): a missing record, not a test
- Seven unlocked-display screenshots were expected under reports/screenshots/; none exists (reports/process/2026-10-30-process-record.md).
- The owner ruled they will not come (ruling B). board-browser-m0.png is the validator's capture, not the owner's (inbox 1230).
- The owner's attended window run of M2 (user-testing round 3, 16:15Z probe, launch 2036e3c5) is recorded in reports/usage/m2.md; no file came from it.

## Reading for the end-of-mission evidence
Neither assertion is a product defect. The product-behaviour evidence at the close is the 261 passed assertions, oracle parity 0 of 266, full make check green and PR #40 green on every check. The remaining gap is the carry-over rule itself (inbox 1319 item 3): earlier milestones' hand-graded assertions on engine paths M6 changed were carried as evidence, not re-run.
