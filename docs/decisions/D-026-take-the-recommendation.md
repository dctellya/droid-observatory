# D-026: The orchestrator takes a recommendation that changes neither scope, money nor models

**Date:** 2026-10-23
**Status:** accepted (owner rule F-030 at the M0 to M1 boundary; `ai/RULES.md` rule 24)

## Context

Rule 15 limits what reaches the human: milestone boundaries and the stall flag. At the M0 to M1
boundary several questions arrived with a recommendation attached and no effect on scope, usage caps or
the models per role, for example the wording of the CI budget or the record of JSON-RPC method names.
Asking the owner each time costs the owner's attention and stalls the mission without changing the
outcome.

## Decision

When a decision comes with a recommendation and changes neither scope, nor usage caps, nor the models
per role, the orchestrator takes the recommendation, records it as a SPEC §9 row with the alternatives,
and reports it at the next boundary. The owner is asked only for scope, money or model changes and for
rule 9 deviations.

This narrows the boundary table "What the human answers at each boundary": the rows that are
recommendations with no such effect are decided, recorded and reported, not asked.

## Alternatives considered

- **Ask the owner for every decision with a recommendation (the S and M0 practice).** Rejected by the
  owner: each ask costs attention and a stall for a decision that moves neither scope, money nor models.
- **Decide silently without a record.** Rejected: rule 9 and rule 13 want the decision visible, so each
  one is a §9 row with the alternatives, reported at the next boundary.

## Consequences

- `ai/RULES.md` gains rule 24; AGENTS.md "Mission setup" carries it in one sentence; SPEC §9 F-030.
- Scope, money (caps, spend beyond a cap) and models still need the owner. Rule 20 and D-018 are
  unchanged: no model or effort change without a decision file approved by the owner.
- Rule 21 and [D-019](D-019-communication-rule-for-the-human.md) still govern how a request that does
  need the owner is phrased.
