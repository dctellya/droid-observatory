# Rule D, apply to the scrutiny rerun and every validator round after: per-claim reviews on Sonnet, Opus for the synthesis only

Source: owner ("can we enforce our ASAP findings?", 19:2x, 2026-10-29), relayed by pre-roci with the figures. Scrutiny round 1 (16:29Z to 16:45Z) cost about 27 USD-eq: the Opus validator 2.86 and 32 per-claim review subagents on Opus about 25 (M3's round 1: 18.39). Each review subagent read a claims list and its evidence paths and returned a verdict: mechanical work in the sense of owner item 5 and D-039.

1. The scrutiny validator stays on claude-opus-5-5 (owner item 12b, D-039) and writes the synthesis and the blocking verdicts itself.
2. Its per-claim review subagents run on the pinned mechanical Sonnet droid (.factory/droids/mechanical-sonnet.md), one batched task per feature (all the feature's claims in one subagent, not one subagent per claim), returning a verdict line per claim with the evidence path it checked. Opus re-reads only the claims the Sonnet pass marks failed or unsure.
3. The same for the user-testing validator's subagents (item 12b already pins the flow runner to Sonnet).
4. Expected: the rerun at about a third of round 1; the usage report's validation section shows subagent cost by model per round. Window for the verdict (FR-P9h): this rerun and the user-testing round; inverse: subagents back on Opus for the next round if the Sonnet pass misses a defect the Opus synthesis then finds.
