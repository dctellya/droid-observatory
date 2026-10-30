#!/bin/sh
# Factory PreToolUse hook: rule G, the model guard (owner inbox 1350 item 5; D-050).
# The Model-Router can switch a running session's model without telling it (04:43, 2026-10-30: six seats ran 54 turns on
# kimi-k3). This hook reads the session's own `model` from its sidecar, <sessions dir>/<session-id>.settings.json, and denies
# the call when that model is not declared for the session's role. Exit 2 = block.
#
# Role comes from the sidecar tags (the same `mission-session` role tag the other hooks' sessions carry):
#   orchestrator       claude-fable-5.1
#   mission worker     claude-sonnet-5-5 (workers, seats) or claude-opus-5-5 (validators); the sidecar does not say which
#   subagent           claude-sonnet-5-5 (mechanical, flow seats) or claude-opus-5-5 (validator synthesis)
#   scratch            claude-haiku-4-5-20251001
# A session with none of these tags is the owner's own and passes. A missing or unreadable sidecar passes: the guard
# must never stop a session it cannot read. Each denial appends one line to build-log/<day>/<session>.jsonl.
# MODEL_GUARD_SESSIONS_DIR overrides the sessions directory for scripts/tests/model-guard.sh.
set -u
input=$(cat)
session=$(printf '%s' "$input" | jq -r '.session_id // empty' 2>/dev/null)
[ -n "$session" ] || exit 0

root=${FACTORY_PROJECT_DIR:-$(printf '%s' "$input" | jq -r '.cwd // empty')}
transcript=$(printf '%s' "$input" | jq -r '.transcript_path // empty')
sidecar=""
for dir in "${MODEL_GUARD_SESSIONS_DIR:-}" "${transcript:+$(dirname "$transcript")}" "$HOME/.factory/sessions/$(printf '%s' "$root" | tr / -)"; do
  [ -n "$dir" ] && [ -f "$dir/$session.settings.json" ] && sidecar="$dir/$session.settings.json" && break
done
[ -n "$sidecar" ] || exit 0

model=$(jq -r '.model // empty' "$sidecar" 2>/dev/null)
[ -n "$model" ] || exit 0

role=$(jq -r '
  [.tags[]? | if .name == "mission-session" then (.metadata.role // "worker") elif .name == "subagent" then "subagent"
   elif .name == "scratch" then "scratch" else empty end]
  | (map(select(. == "orchestrator")) + map(select(. == "scratch")) + map(select(. == "subagent")) + map(select(. == "worker")))[0] // empty
' "$sidecar" 2>/dev/null)

case "$role" in
  orchestrator) allowed="claude-fable-5.1" ;;
  worker | subagent) allowed="claude-sonnet-5-5 claude-opus-5-5" ;;
  scratch) allowed="claude-haiku-4-5-20251001" ;;
  *) exit 0 ;;
esac

for m in $allowed; do
  [ "$model" = "$m" ] && exit 0
done

reason="undeclared model (rule G): $model"
day=$(date -u +%Y-%m-%d)
dir="$root/build-log/$day"
mkdir -p "$dir" 2>/dev/null
printf '%s' "$input" | jq -c --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg model "$model" --arg role "$role" --arg reason "$reason" '{
  ts: $ts,
  session_id: .session_id,
  event: "ModelGuardDeny",
  tool: .tool_name,
  command: (.tool_input.command? // null),
  file: (.tool_input.file_path? // null),
  summary: (.tool_input.summary? // null),
  script: (.tool_input.script? // null | if . == null then null else .[0:200] end),
  model: $model,
  role: $role,
  reason: $reason
}' >>"$dir/$session.jsonl" 2>/dev/null
echo "$reason" >&2
exit 2
