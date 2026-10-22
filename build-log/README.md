# build-log

One JSON line per tool call made by a Droid session in this repository, written by
`.factory/hooks/log-tool-call.sh` (PostToolUse). Files are `YYYY-MM-DD/<session-id>.jsonl`.

Each line: `ts`, `session_id`, `event`, `tool`, `command`, `file`, `summary`, `exit_code`.
Tool responses are never recorded here; the session transcript in `~/.factory/sessions/`
holds them and is linked from `ai/session/`.

This directory is a deliverable (ai/RULES.md rules 16 and 17) and the first dataset the
observatory replays against itself.
