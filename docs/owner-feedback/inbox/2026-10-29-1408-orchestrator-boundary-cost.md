# Two cycle rules, apply from the next boundary: the orchestrator's cost per feature boundary

Source: owner ("apply", 14:0x, 2026-10-29), relayed by pre-roci with the figures from the record. Reading at 14:00: 125 orchestrator turns for 16 feature boundaries (about 8 per boundary) at 0.39 USD-eq per turn, 177 k tokens re-read per turn (M3: 0.34 and 147 k); 47 hand-off items dismissed, each with its own written justification. About 4 USD-eq of orchestrator per feature against about 1.4 of worker. Target: 3 turns and about 1.5 USD-eq per boundary.

## Rule A. Three turns per boundary, dismissals batched (cycle rule, test for the rest of M4; KB candidate for M-0xx and R-067)
1. A feature boundary is three orchestrator turns: (1) read the hand-off; (2) one disposition turn for every hand-off item at once, one line per item (`dismissed: <reason in one clause>` or `replanned: <feature id>`); the long justification is written only for an item that is replanned or that changes a §9 row, and it goes in the staging file, not in the turn; (3) dispatch the next feature. Investigation of a discovered issue goes to a read-only Sonnet subagent, as already done at 13:07, never to the orchestrator's own turns.
2. Where the hand-off is long, a Sonnet triage subagent (the mechanical-sonnet droid) pre-digests it into the disposition list and the orchestrator confirms it in turn (2). The subagent reads the hand-off and the staging file only.
3. The usage report's "Lead context rewrites" section adds a line "orchestrator turns per feature boundary" so the next reading shows whether the rule held (target 3; M4 so far 8).

## Rule B. Compact at the boundary (rule 19 applied now)
Before dispatching the next feature, the orchestrator compacts its context once (the M4 rules, the current plan, the open §9 rows and the inbox pointer survive; the dispositions of landed features and the M3 boundary text do not, they are in the files). Expected: per-turn cost from 0.39 toward 0.30. Record the compaction as a `mission_resumed`-style line in the progress log so the reading can split before and after.

Neither rule changes a tree, a cap or a model. The slice size (item 21, five assertions) is not changed in M4; it is a card for the M5 plan at the boundary, with the ratio above as its evidence.
