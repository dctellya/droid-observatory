#!/bin/sh
# Factory Pre/PostToolUse hook: append one compact JSON line per tool event to build-log/
# (ai/RULES.md rule 17, D-021, D-027). Records what was called, never the tool response, so the
# log stays small and cannot leak tool output.
#
# The payload has no tool_use_id (Droid 0.247.x: cwd, hook_event_name, permission_mode, session_id,
# tool_input, tool_name, tool_response, transcript_path), so `call_id` is a hash of tool name and
# tool input. A PostToolUse record pairs with the latest PreToolUse record of the same session and
# `call_id` that no later PostToolUse has claimed; `duration_ms` is null when there is none (a
# nested Execute inside a Script call logs a start and never an end, so such starts are skipped
# by the "latest unclaimed" rule). Two identical calls running in parallel pair imperfectly.
# LOG_TOOL_CALL_NOW_MS pins the clock for scripts/tests/log-tool-call-duration.sh.
set -u
input=$(cat)
root=${FACTORY_PROJECT_DIR:-$(printf '%s' "$input" | jq -r '.cwd')}
day=$(date -u +%Y-%m-%d)
dir="$root/build-log/$day"
mkdir -p "$dir"
session=$(printf '%s' "$input" | jq -r '.session_id // "unknown"')
file="$dir/$session.jsonl"

now_ms() {
  ns=$(date -u +%s%N)
  case "$ns" in
    *[!0-9]*) echo $(($(date -u +%s) * 1000)) ;; # date without %N
    *) echo $((ns / 1000000)) ;;
  esac
}
sha() { shasum -a 256 2>/dev/null || sha256sum; }

ms=${LOG_TOOL_CALL_NOW_MS:-$(now_ms)}
event=$(printf '%s' "$input" | jq -r '.hook_event_name // ""')
call_id=$(printf '%s' "$input" | jq -cS '[.tool_name, .tool_input]' | sha | cut -c1-16)

duration=null
if [ "$event" = PostToolUse ] && [ -f "$file" ]; then
  duration=$(grep -F "\"call_id\":\"$call_id\"" "$file" | jq -s --argjson now "$ms" '
    (map(.event == "PreToolUse") | rindex(true)) as $pre
    | (map(.event == "PostToolUse") | rindex(true)) as $post
    | if $pre != null and ($post == null or $pre > $post) then $now - .[$pre].ts_ms else null end
  ' 2>/dev/null) || duration=null
  [ -n "$duration" ] || duration=null
fi

# `?` keeps jq from failing when tool_input or tool_response is a string or an array
# (Droid 0.247.0 sends tool_response as a string or an array); append only on success so a
# jq error cannot leave an empty file behind. The first nine fields are D-021's; new ones go last.
line=$(printf '%s' "$input" | jq -c --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg id "$call_id" --argjson ms "$ms" --argjson dur "$duration" '{
  ts: $ts,
  session_id: .session_id,
  event: .hook_event_name,
  tool: .tool_name,
  command: (.tool_input.command? // null),
  file: (.tool_input.file_path? // null),
  summary: (.tool_input.summary? // null),
  script: (.tool_input.script? // null | if . == null then null else .[0:200] end),
  exit_code: (.tool_response.exitCode? // null),
  call_id: $id,
  ts_ms: $ms
} + (if .hook_event_name == "PostToolUse" then {duration_ms: $dur} else {} end)') && [ -n "$line" ] && printf '%s\n' "$line" >> "$file"
exit 0
