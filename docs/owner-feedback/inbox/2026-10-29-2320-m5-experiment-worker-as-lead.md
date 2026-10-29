# M5 experiment, apply at the M5 cut: two slices run as a lead worker with parallel seats

Source: owner ("let's do it, as an experiment; it can tell us if time per assertion really goes down and how we are doing with cost", 23:20, 2026-10-29), relayed by pre-roci. Boundary item for the M5 re-cut, not an in-flight change.

## The experiment
Two M5 slices with natural, file-disjoint parts run as **a lead worker with two to three parallel seats**: `m5-details-tabs-ui` and `m5-panel-charts-and-library-ui`. The other seven M5 slices run as today, single worker, so the comparison is inside one milestone.

## The pattern (write it into the two feature descriptions and the worker skill)
1. The lead reads the slice, cuts it into two or three parts that own disjoint files (rule 13: no shared-file edits; parts that share a file or a test fixture are not parallel).
2. The lead dispatches the parts as Task seats on `mechanical-sonnet` in the same worktree, in one turn, and waits once. Each seat returns at most ten lines: files touched, tests added, anything blocked. No seat commits.
3. The lead runs the tests and the local gate once on the combined tree, records the gate, writes the staging file and the hand-off with the claims list (rule 30), and hands off partial with a split proposal before the cap (rule 29) as today.
4. The lead's own turns count against the slice budget; the seats' turns are their own sessions and are priced to the slice by Roci from the record.

## Measure (Roci reads it from the record; the orchestrator records nothing extra)
- Wall time per slice against its plan turn budget, and landed assertions per hour, for the two lead slices against the seven single-worker slices of the same milestone and against M4.
- Cost per landed assertion, raw and weighted (10.3b), lead plus seats, against the single-worker slices.
- Persistence: both slices landed. Inverse: the pattern is dropped from the feature description; nothing else changes.

## Why
M4 ran 37 worker runs strictly one at a time, 486 of 627 wall minutes with one worker busy; Factory runs features serially by design and parallel inside a feature (the validators already run as lead plus seats, scrutiny 27 to 3). If the two slices land faster and not dearer per assertion, the pattern goes to the Cody worker brief (owner: "if it works we need to add it to Cody") and to the blueprint as a candidate.
