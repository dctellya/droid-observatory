# Usage limit at 04:43: the orchestrator was routed to kimi-k3; hold all decisions until the declared model is back

Source: owner's delegate (pre-roci), 04:46, 2026-10-30, from the droid log: Model-Router `overage_proactive` moved session e194b9e3 from claude-fable-5.1 to kimi-k3 at 03:43:46Z; a user-testing seat (7865a50a) was moved by `overage_reactive_402` at the same minute. The five-hour window that began about 00:20 is exhausted; it resets at about 05:20 local.

Rule 31 applies to you now: on kimi-k3 you take no decision (no dispatch, no dismissal, no landing, no re-plan, no edit through a seat). The heartbeat may keep running (it is cheap on any model) and this file is what it reads. The user-testing validator (364e8787, Opus) was not routed and finishes its round; when it returns, record the hand-off as received and do nothing else until your model reads claude-fable-5.1 again. Row this as the first usage limit of the night (the delegate's rules allow one; a second pauses the mission for the owner).

If the router does not restore the declared model by itself at the reset, the mission stays held until the owner is up; that is the right outcome, not a loss.
