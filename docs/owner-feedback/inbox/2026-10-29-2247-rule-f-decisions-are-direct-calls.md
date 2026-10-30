# Rule F, apply at this wake: decisions are direct tool calls; a background Script is for the heartbeat only

Source: owner ("let's do it", 22:47, 2026-10-29), relayed by pre-roci.

## Why
At 20:55 you declared your own session was running as two processes and stood down from dispatch. The host had one process. What you had met was your own dispatch returning as a stranger: a StartMissionRun issued inside a background Script returns only when the run returns (the 19:15 script completed after 1,869 s at 19:46), and the harness delivers that return as "a new asynchronous lifecycle event". Decisions wrapped in background Scripts come back out of order and unrecognised; direct calls return in the same turn.

## The rule
1. DismissHandoffItems, StartMissionRun, EndFeatureRun and Task are called directly in the turn, never inside a Script and never with startInBackground.
2. The only background Script is the Rule E heartbeat (sleep 240 s, two lines). A Script that waits on a run is not used; the harness wakes you on the worker's return.
3. A return you do not recognise in your own build-log is yours if the session id is yours; before claiming another actor, write the row and ask the owner's delegate, do not investigate in your own context (Rule C path scope).

## The mechanism (scribe, one commit with its decision file)
Extend `.factory/hooks/officer-decides.sh` for the officer session: when `tool_name` is `Script`, deny if `tool_input.script` matches `tools\.(StartMissionRun|DismissHandoffItems|EndFeatureRun|Task)\(` with the message "decisions are direct calls (rule F)"; allow otherwise. Proof: one deliberately denied Script in the progress log. Then continue with m5-integrate-m4.

## Measure and inverse
Measure: false-belief rows (claims about processes or sessions not confirmed by the host) and out-of-order returns per unit; target zero through M5. Inverse: remove the Script clause from the hook; the heartbeat is unaffected.
