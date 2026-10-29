# D-039: Subagents are a role with a declared model, pinned by droid files

**Date:** 2026-10-29
**Status:** accepted; owner items 5 and 12b of the M3 boundary (dated 25-oct, received 2026-10-29)

## Context

In M3's build (read 17:49 on 25-oct, each subagent counted under the session that started it) subagents cost 57.92 USD-eq on
Fable and Opus with no declared model: 47.88 started by the validators and 10.04 by the orchestrator, more than the
orchestrator's own session (44.12) and twice the workers (28.20). Item 5 asks that every subagent declare its model and that
mechanical work run on the worker model; item 12b asks that user-testing flow groups run on the worker model, with one Opus
pass for the verdict.

The harness does not offer a model choice on the built-in `worker` and `explorer` droids: they inherit the parent's model, and the
Task `complexity` tier routes nothing on this machine. Receipt: the two subagents of the M3 boundary (rules propagation
`c312afac` and contract pass 26 `c1d0aec9`) ran on `claude-fable-5.1`, the orchestrator's model. A custom droid with a `model:`
line does pin the model: pass 27 (`7e16efd6`) ran on `claude-sonnet-5-5` through `mechanical-sonnet`.

## Decision

- A subagent is a role with a declared model. Every subagent the orchestrator or a validator starts names its model and its
  purpose; every subagent report starts with its `model = <value>` line and the parent pastes it.
- **Mechanical work and flow runners run on `claude-sonnet-5-5`:** contract passes, triage, document assembly, report grading,
  coverage checks (`mechanical-sonnet`) and user-testing flow groups (`user-testing-flow-validator`, was `model: inherit`).
- **Scrutiny reviewers and the verdicts run on `claude-opus-5-5`:** `scrutiny-feature-reviewer` stays `inherit` under the
  Opus validator that starts it; the user-testing verdict is one Opus pass over the recorded evidence.
- The Task `complexity` tier routes nothing here and is not used to choose a model. Pinning is done by a droid file.
- The droid files are kept in the repository at `.factory/droids/` (verbatim copies of `~/.factory/droids/mechanical-sonnet.md`
  and `~/.factory/droids/user-testing-flow-validator.md`), so the pin is reviewable and the home copies can be restored from
  them. The usage report has a row per subagent class (`m4-usage-token-classes`).

## Alternatives

1. **Rely on the Task `complexity` tier.** Rejected: it routed nothing (both boundary subagents ran on `claude-fable-5.1`).
2. **Pass a model id in the Task call.** The tool has no such parameter.
3. **Let subagents inherit and read the model afterwards.** Kept only for `scrutiny-feature-reviewer` (judgement, Opus); for
   everything else it is the 57.92 USD-eq in the context.
4. **Keep the droid files only in the home directory.** Rejected: the pin would be invisible to the repository and lost with the
   machine.

## Consequences

- The models table of AGENTS.md and SPEC §2 gains a row for subagents; SPEC `## Tooling` lists the two droid files and the two
  harness limits as tool facts (rule 26).
- A change to a pinned model in a droid file is a decision file (rule 19), and the repository copy and the home copy are changed
  together.
- Target for M4: orchestrator subagents about 3 USD-eq (item 15), the saving reported at the M4 boundary or "did not work".
