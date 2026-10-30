# D-044: Rule D, per-claim review and flow subagents on Sonnet; Opus for the synthesis only

**Date:** 2026-10-29
**Status:** accepted; owner inbox 2026-10-29-1926 (19:2x), relayed by pre-roci; refines D-039

## Context

Scrutiny round 1 of M4 (16:29Z to 16:45Z) cost 35.38 USD-eq in `reports/usage/m4.md` (the owner's reading at the time, about 27):
the Opus validator 2.86 and 32 per-claim review subagents on Opus 32.52 (M3 round 1: 18.39). Each subagent read a claims list and
its evidence paths and returned a verdict, which is mechanical work in the sense of owner item 5 and D-039.

## Decision

Verbatim from the owner:

1. The scrutiny validator stays on claude-opus-5-5 (owner item 12b, D-039) and writes the synthesis and the blocking verdicts itself.
2. Its per-claim review subagents run on the pinned mechanical Sonnet droid (`.factory/droids/mechanical-sonnet.md`), one batched task per feature (all the feature's claims in one subagent, not one subagent per claim), returning a verdict line per claim with the evidence path it checked. Opus re-reads only the claims the Sonnet pass marks failed or unsure.
3. The same for the user-testing validator's subagents (item 12b already pins the flow runner to Sonnet).
4. Expected: the rerun at about a third of round 1; the usage report's validation section shows subagent cost by model per round.

Applied here: the repo `AGENTS.md` model table (Subagents row: `claude-sonnet-5-5` for mechanical work, flow runners and per-claim reviews;
`claude-opus-5-5` for the validators' synthesis and verdicts) and its `## Mission setup` wording.

Measure (window for the verdict, FR-P9h): the M4 scrutiny rerun and the user-testing round, then every validator round after.
Recorded in `reports/usage/m4.md` (Subagent classes, User testing rounds): scrutiny round 2 (`e31d7c6b`) 1.61 root + 3 Sonnet
subagents 1.07; round 3 (`f07cd47d`) 1.28 + 2 Sonnet 0.53 against round 1 35.38; user testing round 2 (`0aceb2ca`) 2.63, 19 min,
against M3 round 2 16.10, 46 min.

**Inverse:** subagents go back to Opus for the next round if the Sonnet pass misses a defect that the Opus synthesis then finds.
Read from `validation/m4/scrutiny/synthesis*.json` at the M4 boundary: round 1 (Opus subagents, before the rule) found 2 blocking
issues in 2 features (`m4-contract-checks-gate`, `m4-git-source-core`; M3 round 1: 6 in 4); round 2 (Sonnet subagents) passed with Opus
re-reads of the claims the Sonnet pass marked failed, unsure or "for you to judge"; round 3 passed and the synthesis states "No Sonnet verdict
was overturned". No miss that would trigger the inverse is recorded; the first M5 validator round reads the same files again.

## Alternatives

- **All subagents on Opus, as in round 1:** 35.38 USD-eq for one round; rejected by the owner.
- **Sonnet for the validators too:** rejected; item 12b keeps the verdicts on Opus.

## Consequences

- The Task `complexity` tier still routes nothing here (D-039); the model is the droid file's.
- A defect that a Sonnet pass misses and the synthesis finds is a SPEC §9 row and triggers the inverse.
