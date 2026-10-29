# Rule E, apply at this wake, as a lever with an inverse: a heartbeat keeps the orchestrator's cache warm

Source: owner ("yes yes, apply", 19:59, 2026-10-29), relayed by pre-roci with the figures from the record. Today the orchestrator (claude-fable-5.1) made 198 turns from dispatch to the 18:08 pause for 77.8 USD-eq; 67 % of that was cache writes: 25 full rewrites of 108k to 248k tokens (1.5 to 3.3 each, about 45 in total) for 31 worker completions. Cause: the orchestrator has no wait tool; it dispatches in the background and is woken when a worker returns, every worker runs longer than the harness's 5-minute prompt-cache TTL, so nearly every wake-up re-writes the whole context. Rule C (hook a4b4646) does not touch this. Since the hook: 7 turns, 0.43, 0.06 per turn, no rewrite.

## The lever
Every orchestrator turn ends by starting one background Script that sleeps 240 seconds and then returns two lines: the inbox listing (file names only, `ls -1` of `/Users/dct/code/EllyaOrg/projects/dalimaray/observatory-wt/m4/docs/owner-feedback/inbox`) and the count of entries in the mission progress log. Its return wakes the orchestrator inside the TTL; that turn is a cache read (about 0.06 to 0.08 at 250k) and does exactly one of two things: nothing new in the two lines, start the next heartbeat; a new inbox file or a hand-off, handle it under Rule C, then start the next heartbeat. No other tool call in a heartbeat turn. When a worker returns, the heartbeat already running is left to expire (its return is a no-op turn).

Expected: 15 heartbeats an hour, about 1.2 an hour, against 5 to 8 an hour in rewrites today. Side effect wanted: owner inbox files are seen within four minutes instead of at the next worker return (boundary item 12).

## Measure, persistence, inverse
- Measure: full rewrites (cachedTokensWritten above 50k) per worker completion, read from the droid log by pre-roci; today 25 of 31. Target: under 1 in 4. Heartbeat turns are counted and priced separately so the lever pays for itself on the record.
- Persistence: three worker completions before the verdict.
- Inverse: stop starting the heartbeat. Trigger: rewrites per completion do not fall, or the heartbeat turns cost more than the rewrites they avoid, or the orchestrator does anything but the two actions above in a heartbeat turn (that is a Rule C breach, and the hook will show it).

## Proof, first thing
Start the first heartbeat now. If its return does not produce an orchestrator turn (the harness does not wake the session on a background Script's completion), record the exact limit as a tool fact in library/tooling.md and boundary question 17, and stop; the lever then becomes a Factory product ask (a cache TTL of one hour, or a wake under the TTL).

## Scope
From this wake through the M4 integration and all of M5. Not a model change; not a cap change; no tree touched. Candidate for cody ai/RULES.md (delta 5) and kb R-075 (the officer keeps its cache warm) once the three-completion reading is in. Blueprint owner note from 28-oct: "check your TTL of the cache and put a wake-up script so you don't have to re-fetch the entire context."
