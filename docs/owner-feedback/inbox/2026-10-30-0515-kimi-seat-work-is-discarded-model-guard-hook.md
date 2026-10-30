# User-testing round 1: the seat work after 04:43 ran on kimi-k3 and is discarded; a model-guard hook is the lever

Source: owner's delegate (pre-roci), 05:15, 2026-10-30, from the droid log.

## What happened
After the usage limit at 04:43 the Model-Router moved not only the orchestrator (which stopped itself under rule 31 at 04:45, correctly) but the user-testing flow seats, mid-run: sessions 7865a50a (22 turns on kimi-k3, noticed only at 05:11), 1fb15256 (13), 249bfd02 (6), 5ed1da29 (5), 8c9dc1c0 (5), b5e789a3 (3). About 54 seat turns of user testing ran on an undeclared model; cost about 2.5, trust zero. The Opus validator (364e8787) was not routed and is waiting on its Task groups.

## Do, when your model reads claude-fable-5.1 again
1. Treat every flow-group report produced by those sessions after 03:43:46Z as untrusted: the validator does not synthesise them. Re-run the affected groups on the declared models (Sonnet flow seats under the Opus validator); groups whose seat finished before 03:43:46Z stand.
2. Row the episode as rule 31's gap: the model check runs at a seat's start, the router switches models mid-run, so a seat can work for half an hour on the wrong model before it looks again. Nothing landed from those turns (validators do not commit), so no revert.

## Lever for M6 (apply at the M6 cut, not in flight): Rule G, a model guard on every tool call
A PreToolUse hook for every session of the mission (not only the officer): read the session's `.settings.json` sidecar (`model`), compare with the mission's declared model for that role (`model-settings.json`: orchestrator, worker, validation worker, custom droid), and deny the call with "undeclared model (rule G): <model>" when they differ. Then a mid-run switch stops the seat at its next call instead of its next self-check, and nothing is produced on the wrong model. Measure: turns on an undeclared model per unit (today 54 plus 6). Inverse: remove the hook. Record the tool fact: the router can switch a running session's model without the session being told until its next system reminder.
