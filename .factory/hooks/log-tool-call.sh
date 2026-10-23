#!/bin/sh
# Factory PostToolUse hook: append one compact JSON line per tool call to build-log/
# (ai/RULES.md rule 17). Records what was called, never the tool response, so the
# log stays small and cannot leak tool output.
set -u
input=$(cat)
root=${FACTORY_PROJECT_DIR:-$(printf '%s' "$input" | jq -r '.cwd')}
day=$(date -u +%Y-%m-%d)
dir="$root/build-log/$day"
mkdir -p "$dir"
session=$(printf '%s' "$input" | jq -r '.session_id // "unknown"')
# `?` keeps jq from failing when tool_input or tool_response is a string or an array
# (Droid 0.247.0 sends tool_response as a string or an array); append only on success so a
# jq error cannot leave an empty file behind.
line=$(printf '%s' "$input" | jq -c --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" '{
  ts: $ts,
  session_id: .session_id,
  event: .hook_event_name,
  tool: .tool_name,
  command: (.tool_input.command? // null),
  file: (.tool_input.file_path? // null),
  summary: (.tool_input.summary? // null),
  script: (.tool_input.script? // null | if . == null then null else .[0:200] end),
  exit_code: (.tool_response.exitCode? // null)
}') && [ -n "$line" ] && printf '%s\n' "$line" >> "$dir/$session.jsonl"
exit 0
