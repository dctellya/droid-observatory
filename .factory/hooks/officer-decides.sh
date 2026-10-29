#!/bin/sh
# Factory PreToolUse hook: rule C, the officer decides (owner inbox 1845, 2026-10-29).
# Denies Execute, Edit, Create, Grep and TodoWrite for the sessions listed in
# officer-sessions.txt (the mission orchestrator); every other session is untouched. Exit 2 = block.
set -u
input=$(cat)
session=$(printf '%s' "$input" | jq -r '.session_id // empty')
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
[ -n "$session" ] && [ -n "$tool" ] || exit 0

list="$(dirname "$0")/officer-sessions.txt"
[ -f "$list" ] || exit 0
grep -Fxq "$session" "$list" || exit 0

case "$tool" in
  Execute | Edit | Create | Grep | TodoWrite)
    echo "execution goes to a seat (rule C)" >&2
    exit 2
    ;;
esac
exit 0
