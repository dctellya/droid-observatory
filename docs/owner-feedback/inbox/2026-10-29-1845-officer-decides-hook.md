# Rule C, apply at the next wake, as a mechanism: the orchestrator decides, every execution is a seat's

Source: owner ("can we enforce it now?", 18:4x, 2026-10-29), relayed by pre-roci with the figures from the record. Since the M4 dispatch the orchestrator made 513 tool calls: 74 decisions (DismissHandoffItems 33, StartMissionRun 33, Task 8) and 439 execution (Execute 133, Script 110, Read 55, Edit 46, Grep 35, TodoWrite 32, Create 27), each in its own turn re-reading about 170 k tokens; 0 of 596 turns batched, while workers batched 782 times in this mission, so the harness allows it.

## The rule
The orchestrator's turns carry decisions: read the hand-off, dispose of its items, raise or land a unit, answer a boundary. No grep, investigation read, file edit, row, to-do or shell call runs in its own context. Each goes to a seat as one batched task: a scribe subagent on claude-sonnet-5-5 for every file a boundary produces (AGENTS rule, boundary row, staging edits, to-do) from the orchestrator's one-line decision; the read-only Sonnet diagnostician for any investigation; the worker's own hand-off for its boundary row and claims list (rule 30). The one wait per unit (Script) stays.

## The mechanism, first thing at the wake
1. Install a pre-tool hook on the orchestrator's own session (Factory hooks; you know the schema) that denies `Execute`, `Edit`, `Create`, `Grep` and `TodoWrite` for the orchestrator session only, with the message "execution goes to a seat (rule C)". `Read`, `Script`, `Task`, `DismissHandoffItems`, `StartMissionRun` stay allowed. Workers and validators are untouched.
2. Prove it with one denied call recorded in the progress log or the boundary row, then continue.
3. If the harness cannot hook the orchestrator's own session, record the exact limit as a tool fact in library/tooling.md and boundary question 17, and apply the rule as practice until the boundary; the usage report's "orchestrator turns per feature boundary" line (m4-usage-boundary-turns) is the measure either way, with a new "single-call share" value.

## Scope
From the next wake through the M4 integration and all of M5. Not a model change; not a cap change; no tree other than the hooks config is touched. Cody ai/RULES.md delta 4 and kb candidate R-073 carry the same rule for every factory.
