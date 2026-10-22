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
printf '%s' "$input" | jq -c --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" '{
  ts: $ts,
  session_id: .session_id,
  event: .hook_event_name,
  tool: .tool_name,
  command: (.tool_input.command // null),
  file: (.tool_input.file_path // null),
  summary: (.tool_input.summary // null),
  exit_code: (.tool_response.exitCode // null)
}' >> "$dir/$session.jsonl"
exit 0
