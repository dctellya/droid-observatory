# D-042: Rule F, decisions are direct calls; a background Script is for the heartbeat only

**Date:** 2026-10-29
**Status:** accepted; owner inbox 2026-10-29-2247 ("let's do it", 22:47, relayed by pre-roci), applied as the faithful variant by the orchestrator

## Context

Owner inbox 2247, verbatim:

> At 20:55 you declared your own session was running as two processes and stood down from dispatch. The host had one process. What you had met was your own dispatch returning as a stranger: a StartMissionRun issued inside a background Script returns only when the run returns (the 19:15 script completed after 1,869 s at 19:46), and the harness delivers that return as "a new asynchronous lifecycle event". Decisions wrapped in background Scripts come back out of order and unrecognised; direct calls return in the same turn.

The rule, verbatim:

> 1. DismissHandoffItems, StartMissionRun, EndFeatureRun and Task are called directly in the turn, never inside a Script and never with startInBackground.
> 2. The only background Script is the Rule E heartbeat (sleep 240 s, two lines). A Script that waits on a run is not used; the harness wakes you on the worker's return.
> 3. A return you do not recognise in your own build-log is yours if the session id is yours; before claiming another actor, write the row and ask the owner's delegate, do not investigate in your own context (Rule C path scope).

The mechanism asked for, verbatim:

> Extend `.factory/hooks/officer-decides.sh` for the officer session: when `tool_name` is `Script`, deny if `tool_input.script` matches `tools\.(StartMissionRun|DismissHandoffItems|EndFeatureRun|Task)\(` with the message "decisions are direct calls (rule F)"; allow otherwise. Proof: one deliberately denied Script in the progress log. Then continue with m5-integrate-m4.

Measure and inverse, verbatim:

> Measure: false-belief rows (claims about processes or sessions not confirmed by the host) and out-of-order returns per unit; target zero through M5. Inverse: remove the Script clause from the hook; the heartbeat is unaffected.

Tooling fact: in this harness StartMissionRun, DismissHandoffItems and EndFeatureRun have no direct call form; they exist only as
Script bindings (`tools.StartMissionRun` etc.); `Task` alone is a direct tool.

## Decision

The orchestrator's decision, verbatim:

> Rule F applied as the faithful variant. Tooling fact: in this harness StartMissionRun, DismissHandoffItems and EndFeatureRun have no direct call form; they exist only as Script bindings (tools.StartMissionRun etc.), Task alone is a direct tool. A hook denying every Script that names them would stop all dispatch. The hook therefore denies those calls only when the Script carries the startInBackground directive; decisions run in foreground Scripts, which return in the same turn (DismissHandoffItems in under a second; StartMissionRun blocks the turn and the harness wakes the orchestrator on the worker's return). Question for the owner's delegate: accept the variant, or prefer the literal clause with its consequence.

- **The clause** (`.factory/hooks/officer-decides.sh`, officer sessions only): for `tool_name` `Script`, when `tool_input.script` matches
  `startInBackground` and also matches `tools\.(StartMissionRun|DismissHandoffItems|EndFeatureRun|Task)\(`, the hook prints
  `decisions are direct calls (rule F)` to stderr and exits 2. Any other Script keeps the rule C behaviour (allowed; its inner tool
  calls pass through the allow-list). `hooks.json` is unchanged: the rule C entry already has matcher `*`.
- Supersedes the background-Script dispatch of F-056/D-027 for decision calls; the heartbeat (rule E) stays a background Script.

Proof, crafted stdin on the officer session id `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c` (`sh officer-decides.sh`, output pasted):

```text
a Script bg + StartMissionRun: exit=2 stderr=[decisions are direct calls (rule F)]
b Script fg + StartMissionRun: exit=0 stderr=[]
c heartbeat bg LS+Read: exit=0 stderr=[]
d Grep: exit=2 stderr=[not an officer tool (rule C): Grep]
e non-officer, case a: exit=0 stderr=[]
```

Case a is a Script with `// @script: {"startInBackground": true}` and `await tools.StartMissionRun({})`; b is the same without the
directive; c is a background Script with `tools.LS` and `tools.Read` only; e is case a with a session id not in
`officer-sessions.txt`.

## Alternatives

- **The literal clause** (deny every Script that names the four tools): in this harness three of them exist only as Script
  bindings, so every dispatch, dismissal and feature end would be denied; the orchestrator could only call `Task`. Consequence if the
  owner prefers it: the orchestrator cannot start or end a run. Left to the owner's delegate.
- **No hook, practice only:** rejected for the reason in D-041 (a rule stated in prose does not hold under load).

## Consequences

- A decision call runs in a foreground Script. A foreground Script returns after 5 min with status `running` when the inner call is
  still open (`StartMissionRun` blocks until the worker returns); the harness wakes the orchestrator on completion.
- A `Script` that merely mentions `startInBackground` and a decision tool (for example in a comment) is denied; the match is a text
  match, not a parse.
- Open question for the owner's delegate: variant accepted, or the literal clause with its consequence.
- A commit cannot name its own sha: the commit that adds this clause is named here by short sha in the docs-only commit that follows
  it (as D-041 did for 060698a). That commit is 7fef46e (`chore(hooks): rule F, decisions are direct calls, D-042`), named at the
  boundary integration `m5-integrate-m4`; `g6-hooks-config` matches hook-file commits against `docs/decisions`.
- The decision files the integrator numbers from the inbox are numbered from D-043 on (D-042 is this file).
- 2026-10-29 22:5xZ proof: the orchestrator's deliberate background Script containing tools.DismissHandoffItems was denied with `decisions are direct calls (rule F)`; owner 23:40: the faithful variant is accepted.
