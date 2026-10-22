# Runbooks

Short recovery procedures for the situations this repository and the mission can get into.
Each one states the symptom, how to confirm it, the fix, and when to stop and involve the
human (ai/RULES.md rule 15). Steps that need the observatory app are marked with the
milestone that delivers them; until then the equivalent CLI command is given.

| Runbook | When |
|---|---|
| [red-gate.md](red-gate.md) | The `gate` check on the mission PR is red |
| [fix-cycle.md](fix-cycle.md) | CI red after the orchestrator finished; reopen the mission on its branch |
| [stalled-mission.md](stalled-mission.md) | No progress on a feature for longer than `stall_threshold_min` |
| [hooks-disabled.md](hooks-disabled.md) | Git or Factory hooks are not running |
| [release.md](release.md) | Cutting a release from `main` |

Conventions: commands are the Makefile targets (rule 5); counts and timings are pasted from
the tool that produced them (rule 7); anything learned goes back into the runbook in the same
PR.
