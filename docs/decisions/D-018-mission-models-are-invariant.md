# D-018: The models and reasoning efforts of a mission are invariant

**Date:** 2026-10-23
**Status:** accepted (owner decision 2026-10-23 at the S to M0 boundary)

## Context

Rule 19 says models per role are declared and a change is a decision file. D-006 declared them for
this mission. Facts from the S to M0 boundary:

- On 2026-10-22 at 19:55 local time the Factory usage overage switched the mission's
  `model-settings.json` to `kimi-k3` for workers and validators (`workerModel` and
  `validationWorkerModel`), without anyone asking for it.
- Worker session `82a61cf6` for `m0-integrate-s` started on `kimi-k3` at 2026-10-23T06:40:42Z and
  ended with an error at 07:13:30Z, with zero commits.
- The model of the resumed user-testing-validator-s session (`d0304e2e`, resumed at 06:40Z) is not
  recorded.
- After the owner restored the settings, the orchestrator restarted this feature on the approved
  models. This session is the second start of `m0-integrate-s`.

## Decision

INVARIANT: the models and reasoning efforts defined at mission approval (orchestrator
`claude-fable-5.1`, workers `claude-sonnet-5-5`, validators `claude-opus-5-5`, all `high`; D-006)
never change for the lifetime of the mission.

- When a Factory usage limit (5-hour, 7-day or monthly) is reached, the mission stops and waits for
  the owner. It never continues on a fallback or core model.
- The overage preference is "stop", never "switch".
- Before a resume the owner restores the mission model settings (`model-settings.json`: workers
  `claude-sonnet-5-5`, validators `claude-opus-5-5`, both `high`) and the orchestrator verifies them.
- A worker that started on another model is restarted from scratch, never resumed.
- Every such episode is recorded in the session log.

## Alternatives

Continue on the fallback model (rejected: it breaks rule 19 and makes usage and cost attribution
per model meaningless); continue on the fallback and note it in the report (the reports would
then mix models under a role, and a validator weaker than the approved one would pass work the
approved one might fail).

## Consequences

A usage limit costs wall time, not quality or attribution. `ai/RULES.md` rule 20 states the same
invariant for any mission. Each integration checks `.factory/settings.json` and the model ids in
`reports/usage/<m>.md` (VAL-PROC-027).

### Addendum: protection of `model-settings.json` (applied 2026-10-23)

- `model-settings.json` in the mission directory is protected with `chflags uchg` once it holds
  the correct models.
- A deliberate change is `chflags nouchg`, edit, `chflags uchg`, plus a decision file.
- The orchestrator checks `ls -lO` before every resume.
- Applied 2026-10-23; second start of `m0-integrate-s` is session `2c04bd42` on
  `claude-sonnet-5-5`.
