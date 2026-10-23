#!/bin/sh
# Test for .factory/hooks/log-tool-call.sh: a PreToolUse payload writes a start record and the
# matching PostToolUse record carries ts_ms and duration_ms (end minus start). The payloads carry no
# tool_use_id (Droid 0.247.x), so the hook matches on session, tool and tool_input.
# LOG_TOOL_CALL_NOW_MS pins the hook clock so the differences are exact.
set -u
root=$(cd "$(dirname "$0")/../.." && pwd)
hook="$root/.factory/hooks/log-tool-call.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0

fail() {
  echo "FAIL: $1"
  failures=$((failures + 1))
}

# fire <now_ms> <event> <tool> <tool_input json> [session]
fire() {
  now=$1 event=$2 tool=$3 input=$4 session=${5:-sess-a}
  payload=$(jq -nc --arg e "$event" --arg t "$tool" --arg s "$session" --argjson i "$input" \
    '{session_id: $s, cwd: "/tmp", hook_event_name: $e, tool_name: $t, tool_input: $i}
     + (if $e == "PostToolUse" then {tool_response: {exitCode: 0}} else {} end)')
  printf '%s' "$payload" | FACTORY_PROJECT_DIR="$tmp" LOG_TOOL_CALL_NOW_MS="$now" sh "$hook"
  rc=$?
  [ "$rc" -eq 0 ] || fail "hook exit $rc for $event $tool"
}

log() { cat "$tmp"/build-log/*/"${1:-sess-a}".jsonl; }
last() { log "${1:-sess-a}" | tail -1; }
expect() { # expect <label> <jq expr on last line> <expected>
  got=$(last | jq -r "$2")
  [ "$got" = "$3" ] || fail "$1: $2 = $got, expected $3"
}

cargo='{"command":"cargo test -p observatory-core","summary":"Run tests"}'

fire 1000000 PreToolUse Execute "$cargo"
expect "pre record" '.event' PreToolUse
expect "pre record" '.ts_ms' 1000000
expect "pre record" '.command' "cargo test -p observatory-core"
expect "pre record" '.duration_ms? // "absent"' absent
fire 3500000 PostToolUse Execute "$cargo"
expect "post duration is end minus start" '.duration_ms' 2500000
expect "post ts_ms" '.ts_ms' 3500000
expect "post exit code kept" '.exit_code' 0
pre_id=$(log | head -1 | jq -r '.call_id')
post_id=$(last | jq -r '.call_id')
[ -n "$pre_id" ] && [ "$pre_id" != null ] && [ "$pre_id" = "$post_id" ] || fail "call_id differs: $pre_id vs $post_id"

# existing fields keep their names and order; new fields are appended after them
keys=$(last | jq -r 'keys_unsorted | join(",")')
case "$keys" in
  ts,session_id,event,tool,command,file,summary,script,exit_code,*) ;;
  *) fail "existing field order changed: $keys" ;;
esac

fire 4000000 PostToolUse Execute '{"command":"never started"}'
expect "post without a start" '.duration_ms' null

# the same call twice in a row pairs each post with its own start
fire 10000000 PreToolUse Execute "$cargo"
fire 10000200 PostToolUse Execute "$cargo"
expect "repeat of the same call" '.duration_ms' 200

# a start with no end (nested Script call) must not be paired with a later identical call
orphan='{"command":"cargo check"}'
fire 20000000 PreToolUse Execute "$orphan"
fire 30000000 PreToolUse Execute "$orphan"
fire 30000050 PostToolUse Execute "$orphan"
expect "orphan start is skipped" '.duration_ms' 50

# a different session never pairs with this session's start
fire 40000000 PreToolUse Execute "$cargo" sess-b
fire 40000900 PostToolUse Execute "$cargo" sess-a # sess-a has no unclaimed start of its own
expect "other session ignored" '.duration_ms' null
log sess-b >/dev/null || fail "no sess-b log"

# Script calls keep the 200 character script field on both events
long=$(printf 'a%.0s' $(seq 300))
fire 50000000 PreToolUse Script "{\"script\":\"$long\"}"
expect "pre script length" '.script | length' 200
fire 50000100 PostToolUse Script "{\"script\":\"$long\"}"
expect "post script length" '.script | length' 200
expect "script duration" '.duration_ms' 100

# without the pinned clock the hook uses real time
printf '%s' '{"session_id":"sess-c","hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"/x"}}' |
  FACTORY_PROJECT_DIR="$tmp" sh "$hook" || fail "hook exit non-zero without pinned clock"
ms=$(last sess-c | jq -r '.ts_ms')
[ "$ms" -gt 1700000000000 ] 2>/dev/null || fail "real ts_ms looks wrong: $ms"

if [ "$failures" -ne 0 ]; then
  echo "$failures failure(s)"
  exit 1
fi
echo "log-tool-call-duration: ok"
