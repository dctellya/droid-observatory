# D-050: Rule G, a PreToolUse model guard on every mission session

**Date:** 2026-10-30
**Status:** accepted; owner, inbox 1350 item 5 (`docs/owner-feedback/inbox/2026-10-30-1350-owner-m5-feedback-long-form.md`) and inbox 0515, applied at the M6 cut (SPEC §9 row 95)

## Context

At 04:43 on 2026-10-30 the Model-Router moved the orchestrator and several running sessions from their declared models to
`kimi-k3` without telling them. Rule 31 held the orchestrator, but the model check runs at a session's start, so six
user-testing seats worked on `kimi-k3` until one noticed (inbox 0515); their reports were discarded and the groups re-run.
Nothing in the harness stops a session from calling tools on a model the mission did not declare.

## Decision

`.factory/hooks/model-guard.sh` is a PreToolUse hook registered for every tool (`matcher: "*"`) in `.factory/hooks.json`, where
the other mission hooks are registered. It reads the session's own `model` from
`~/.factory/sessions/<project>/<session-id>.settings.json` and compares it with the models declared for the session's role
(`model-settings.json`, `.factory/settings.json`, AGENTS.md "Mission setup"):

| Role (sidecar tag) | Declared models |
|---|---|
| orchestrator (`mission-session` role `orchestrator`) | `claude-fable-5.1` |
| mission worker (`mission-session` role `worker`: workers, seats, scribes, validators) | `claude-sonnet-5-5`, `claude-opus-5-5` |
| subagent (`subagent`) | `claude-sonnet-5-5`, `claude-opus-5-5` |
| scratch (`scratch`) | `claude-haiku-4-5-20251001` |

On a mismatch it exits 2 with `undeclared model (rule G): <model>` on stderr and appends one `ModelGuardDeny` line (tool,
command, file, summary, script, `model`, `role`, `reason`) to `build-log/<day>/<session>.jsonl`, the file the log-tool-call
hook writes. Every other case passes: no sidecar, no `model`, no mission or subagent tag (the owner's own sessions). The guard
fails open because it must never stop a session it cannot read.

The test is `scripts/tests/model-guard.sh` (in `make test`). The hook and its registration landed in `b36cd08` (the red test is
`2456492`); VAL-PROC-022 requires a decision file to name a commit that changes a hook file.

## Alternatives

1. **A per-seat model table.** The inbox asks for the declared model per seat. The sidecar of a worker-tagged session does not say
   whether it is a worker (Sonnet) or a validator (Opus), and a subagent's sidecar carries only the calling session, so the
   hook accepts the union of the two declared models for those roles. A session on the wrong one of the two is not caught; the
   router's `kimi-k3`, and any model outside the declared set, always is.
2. **A SessionStart check only.** Rejected: the switch happens mid-run; this is exactly rule 31's gap.
3. **Guard only the officer, as rule C does.** Rejected: the seats are where the 54 turns ran.
4. **Fail closed on an unreadable sidecar.** Rejected: a read error would stop the mission for a fault that is not a model drift.

## Consequences

- A session routed away from its declared model stops at its next tool call, with the model named, instead of at its next
  self-check. The measure is the count of `ModelGuardDeny` lines per unit against M5's 54 seat turns plus 7 orchestrator turns.
- A subagent that inherits the orchestrator's model (a built-in explorer under `claude-fable-5.1`) is denied; the pinned droid
  files (`mechanical-sonnet`, `user-testing-flow-validator`) are the declared route (D-039, D-044).
- The guard reads the sidecar on every tool call: one `jq` read of a small file, inside the 10 s hook timeout.
- The report line "1077 turns on non-declared models" in `reports/usage/m5.md` counts Sonnet validator seats and orchestrator
  subagents against their parent role's model, which Rule D (D-044) puts on Sonnet by design; of the 1077 turns, 70 ran on
  `kimi-k3` (staging file `reports/staging/m6-model-guard-hook.md`). The usage report keeps its figure as written.
- Inverse: remove the `model-guard.sh` entry from `.factory/hooks.json`.
