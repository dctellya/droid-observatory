# D-006: Models per role for the mission

**Date:** 2026-10-22
**Status:** accepted

## Context

Rule 19: orchestrator, worker and validator models are declared in the spec and the
mission command; a change is a decision file. The owner set the roles on 2026-10-22.

## Decision

| Role | Model |
|---|---|
| Orchestrator | `claude-fable-5.1` |
| Workers | `claude-sonnet-5-5` |
| Validators (scrutiny and user testing) | `claude-opus-5-5` |

Declared in `.factory/settings.json` (`missionOrchestratorModel`,
`missionModelSettings.workerModel`, `missionModelSettings.validationWorkerModel`), the
README mission command, `config/observatory.example.toml` `[mission]`, and the price table
`config/prices/2026-10-22.toml`. Reasoning effort per role is set in `SPEC.md` §2.

**2026-10-22 correction:** Factory's catalog ID is `claude-fable-5.1`; `claude-fable-5-1` was a spelling error that caused fallback to GPT-6 Sol, not a change of intended model.
Source: [Factory model catalog](https://docs.factory.com/models.md).

## Alternatives

Same model for all roles (simpler, loses the cost/quality split the observatory is meant
to measure); cheaper validators (validation is where correctness is bought).

## Consequences

Lane 9 compares roles across these three models from the first mission. Changing any of
them: new decision file, update the four places above.
