# Stalled mission

**Symptom:** a feature shows no active time for longer than
`[intervention].stall_threshold_min` (default 20 minutes, `config/observatory.example.toml`),
or the orchestrator has been waiting on a question nobody saw.

## Confirm

Until M1 (attach): tail the orchestrator session file and look at the last event's timestamp.

```sh
ls -t ~/.factory/sessions/<cwd-slug>/*.jsonl | head -1 | xargs tail -n 3
```

From M1 the board shows the stall flag on the card; from M3 the card offers suggested
prompts.

## Causes and actions

| Last event seen | Meaning | Action |
|---|---|---|
| `question.asked` / `permission.requested` | Waiting on human | Answer in the terminal that runs the mission (M2 hosting: in the app) |
| `subagent.spawned` with no child file growing | Worker died or is blocked on a tool | Check the child session file; if silent, send a message to the orchestrator to retry or skip the task |
| `hook.blocked` repeated | An agent keeps hitting `guard-git.sh` or a git hook | Read `build-log/<day>/<session>.jsonl` for the command; the agent needs the forward-only instruction, not a bypass |
| `integration.conflict` | Two worktrees touched one file (rule 13 violated) | Orchestrator resolves at integration; if it loops, stop and split ownership in the spec |
| Nothing for a long time, process alive | Long tool call (build, tests) | Wait one more threshold; then `mission.paused` and inspect |

## Stop and involve the human when

- The suggested action is to change scope, model or threshold (decision file).
- The stall repeats on the same feature after one retry.
