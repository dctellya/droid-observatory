# Rule C amended, apply at this wake: the officer's tools are an allow-list built from its activities, everything else is denied

Source: owner ("add it; we need to construct the harness; this is a policy rule for Roci", 20:49, 2026-10-29), after pre-roci saw the orchestrator load a FetchUrl binding at 20:47. FetchUrl was not on the deny list because the list was built from what the orchestrator had used, not from what it is for. A deny-list built from observation has a hole for every tool not yet seen.

## The rule
The officer's seat has exactly the tools its activities need, and no other:
- read the record: `Read`, `LS`
- wait: `Script` (a sleep that returns the two heartbeat lines, or the wait on a run; no inner `tools.Execute`, `tools.Grep`, `tools.Edit`, `tools.Create`, `tools.FetchUrl`; the hook already rejects inner calls)
- delegate: `Task`
- dispose, raise, land: `DismissHandoffItems`, `StartMissionRun`, `EndFeatureRun`
Everything else is denied for the officer's session: `Execute`, `Edit`, `Create`, `ApplyPatch`, `Grep`, `Glob`, `TodoWrite`, `FetchUrl`, `WebSearch`, `GenerateImage`, `Skill`, any MCP tool, and any tool added to the harness later. If a decision needs a page read or a search, a seat reads it and returns the lines. If you find an activity the list does not cover, do not add the tool: write it as a boundary row (the activity, the tool, why a seat cannot do it) and the owner decides whether the policy grows.

## The mechanism, first thing (scribe seat, one commit with its decision file)
1. `.factory/hooks.json`: the rule C entry's matcher becomes `*` (keep the git guard and the logger as they are).
2. `.factory/hooks/officer-decides.sh`: replace the deny `case` with an allow `case`: `Read | LS | Script | Task | DismissHandoffItems | StartMissionRun | EndFeatureRun) exit 0 ;;` and `*) echo "not an officer tool (rule C): $tool" >&2; exit 2 ;;`. Sessions not in officer-sessions.txt stay untouched.
3. The commit carries its decision file (the a4b4646 gap found by scrutiny round 2: a hook commit without a decision fails g6-hooks-config at integration); `make decisions-index`.
4. Proof: one `FetchUrl` from the orchestrator session, denied, in the progress log. Then continue with the user-testing hand-off.

## Scope
From this wake through M5. Boundary row and tool fact: Factory's hook input gives `tool_name` and `session_id`, which is enough for a per-session allow-list; the harness still lacks a per-role tool set (feedback item 21).
