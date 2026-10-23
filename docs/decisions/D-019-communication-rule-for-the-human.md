# D-019: Think as much as needed, say little to the human

**Date:** 2026-10-23
**Status:** accepted (owner decision 2026-10-23 at the S to M0 boundary)

## Context

During S and the S to M0 boundary the orchestrator narrated its reasoning, restated plans and
sent long status messages. The owner has to read them between milestones and asked for a fixed,
short format. Rule 15 already limits when the human is addressed; nothing limited how.

## Decision

Think as much as you need, say little. To the human: one line of status per milestone event, a
decision request in three lines (what, options, your recommendation) when rule 15 requires one,
and nothing else. No narration of reasoning, no restating the plan; explanations go to the
session log and the mission files. Workers and validators report to the orchestrator, not to the
human.

- This is `ai/RULES.md` rule 21.
- The project output style `.factory/output-styles/mission-terse.md` encodes it for the
  orchestrator. Droid output styles apply to interactive CLI sessions only (not to `droid exec`,
  Missions or subagents), so the owner selects it with `/settings` (saved as `outputStyle` in
  `~/.factory/settings.json`); the built-in `Concise` style is the fallback. Workers and
  validators get the rule from the mission `AGENTS.md` and their skills.

## Alternatives

Leave it to the orchestrator's judgement (rejected: that is the behaviour the owner corrected);
use only the built-in `Concise` style (rejected: it shortens text but does not fix the status and
decision-request formats); a hook that truncates messages (rejected: it changes content, not the
habit).

## Consequences

Status lines and decision requests are short; the reasoning that used to be sent to the human is
written to the session log. A worker handoff addressed to the human is a rule 21 violation and is
caught at scrutiny. The output style takes effect only after the owner selects it, because styles
cannot be forced from the repository.
