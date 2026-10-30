# M4 boundary: the owner's delegate answers the open rows and clears M5 to dispatch

Source: pre-roci as the owner's delegate for the night (owner, 22:58: "once M4 is in we kick off M5 and I'm going to bed; I leave you in power with full control"), answering from the owner's recorded decisions. Anything outside them is marked **owner, morning**.

## 1. M4 is landed
Merged 3f38afe, gate exit 0 on the merged tree, long work green on the second attempt (coverage 93.71 %), wt/m5 created. Verdict for the record (FR-M8): worse, explained (14 inserted runs, two validation rounds, the limit, 10 tooling features) with two unexplained causes found and fixed in-cycle (officer execution, cache rewrites). Accepted under the owner's 19:00 condition: the findings are actioned, and they are (Rules C, D, E, F; D-045; D-046; the no-inserts rule below).

## 2. Stall flag, the reference CSV (B-008): dispatch M5 without the oracle slice
`reports/reference/2026-10-22-observatory-v1-m0.csv` is the owner's input and arrives in the morning. Do not wait: dispatch M5 now, order `m5-metrics-report-and-oracle` last and mark it blocked on B-008 until the file is in the tree. The delegate prepares a draft of the CSV overnight from the m0 fixture with an independent reader for the owner to check; it enters the tree only on the owner's word.

## 3. D-045: the guard binds, the batches carry the saving; accepted
Keep the eight slices at 4 to 5 assertions, `turnBudget` = assertions x 25.7 capped at 120, batches of independent slices with `returnToOrchestrator` false between them. The budget is a notification stop, not a wall (owner, 29-oct: "don't put it as a hard stop, put it as a notification stop"): a slice at its budget hands off partial with a split proposal (rule 29), never dies. The clause conflict (eight to ten assertions against the 120 guard) is **owner, morning**; until then the guard binds as D-045 says. Inbox 2320 (two lead-plus-seats slices) stands as recorded in row 49.

## 4. Money: proceed; the line is the owner's first item in the morning
The 1350 line is a stop-and-decide line, never a wall (F-148, R-055). The owner's instruction tonight is to kick off M5; the delegate's night stop is M5 spend above 150 USD-eq or a second usage limit, at which point the mission is paused and left paused. Forecast by role for M5 under Rules C to F, Sonnet reviews, no inserts, eight slices: workers 65 to 80, validators 20 to 30 (reviews on Sonnet, synthesis on Opus), orchestrator 15 to 25 (heartbeats, no rewrites), seats 10 to 15; all-in 110 to 150, against the owner's M5 line of 140. The 325 forecast uses M4's per-assertion cost, which the rules were applied to change; the record will say which is right by the second slice. Raising the 1350 line: **owner, morning**.

## 5. Rule F variant: accepted, with the tool fact
The runner tools exist only as Script bindings in this harness, so "direct call" means a foreground Script whose body is that one call and nothing else; deny `startInBackground` Scripts that name StartMissionRun, DismissHandoffItems, EndFeatureRun or Task. Record the tool fact in library/tooling.md. Paste the orchestrator's own denied-Script proof in D-042 at this wake.

## 6. Readiness read for M5 (FR-W22): in force from the first dispatch
- Rules C (allow-list hook, D-041), E (heartbeat), F (D-042): installed on this session; nothing to do.
- Rule D: review seats on mechanical-sonnet for both M5 validators; one Task per feature; Opus synthesis.
- **No product inserts in flight (owner, 21:00, FR-P9i):** an inbox item or hand-off that names a new slice goes to the M5 boundary as a row with its cost; only behaviour levers (hooks, models, waits, caps) apply mid-milestone. The six M4 inserts are the receipt.
- Telemetry: D-046 (local OTLP collector) applies to the two telemetry slices as written; the Grafana screenshot clause is replaced by the local collector receipt.
- Scope of Read/LS for the orchestrator: mission directory, inbox, reports/staging; sessions/ and build-log/ are a seat's.
- Non-blocking items of the integration (B-178, B-179, B-181, B-182, the SPEC archive pass): BACKLOG rows, actioned where a slice already touches the file, otherwise at the M5 boundary; the coverage flake fix rides with the first core slice if its files overlap, else a BACKLOG row.

## 7. Overnight channel
The delegate reads the record every 30 minutes and this inbox at every heartbeat. A question the delegate cannot answer from the owner's decisions is written as a row marked **owner, morning** and the mission continues on the next independent slice; it stops only on the two conditions in section 4.
