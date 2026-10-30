#!/bin/sh
# Test for .factory/hooks/model-guard.sh (Rule G, D-050): a PreToolUse call from a mission session whose sidecar `model`
# is not declared for its role is denied (exit 2, "undeclared model (rule G): <model>") and the denial is appended to
# build-log/<day>/<session>.jsonl; a declared model, an unguarded session and an unreadable sidecar pass (exit 0).
# MODEL_GUARD_SESSIONS_DIR points the hook at a scratch sessions directory.
set -u
root=$(cd "$(dirname "$0")/../.." && pwd)
hook="$root/.factory/hooks/model-guard.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
sessions="$tmp/sessions"
mkdir -p "$sessions"
failures=0

fail() {
  echo "FAIL: $1"
  failures=$((failures + 1))
}

# sidecar <session> <model> <tags json>
sidecar() {
  jq -nc --arg m "$2" --argjson t "$3" '{model: $m, tags: $t, tokenUsage: {}}' >"$sessions/$1.settings.json"
}

# fire <session> [tool]: runs the hook, leaves the exit code in $rc and stderr in $err
fire() {
  payload=$(jq -nc --arg s "$1" --arg t "${2:-Execute}" \
    '{session_id: $s, cwd: "/tmp", hook_event_name: "PreToolUse", tool_name: $t, tool_input: {command: "cargo test -p observatory-core", summary: "Run tests"}}')
  err=$(printf '%s' "$payload" | FACTORY_PROJECT_DIR="$tmp" MODEL_GUARD_SESSIONS_DIR="$sessions" sh "$hook" 2>&1 >/dev/null)
  rc=$?
}

expect_allow() { # expect_allow <label> <session>
  fire "$2"
  [ "$rc" -eq 0 ] || fail "$1: exit $rc, expected 0 ($err)"
}
expect_deny() { # expect_deny <label> <session> <model>
  fire "$2"
  [ "$rc" -eq 2 ] || fail "$1: exit $rc, expected 2"
  [ "$err" = "undeclared model (rule G): $3" ] || fail "$1: stderr '$err'"
}

orch='[{"name":"mission-session","metadata":{"role":"orchestrator","missionId":"m"}}]'
worker='[{"name":"exec"},{"name":"mission-worker"},{"name":"mission-session","metadata":{"role":"worker","missionId":"m"}}]'
sub='[{"name":"subagent","metadata":{"callingSessionId":"x"}}]'
scratch='[{"name":"scratch"}]'

sidecar orch-ok claude-fable-5.1 "$orch"
expect_allow "orchestrator on its declared model" orch-ok
sidecar orch-bad kimi-k3 "$orch"
expect_deny "orchestrator routed to kimi-k3" orch-bad kimi-k3
sidecar orch-sonnet claude-sonnet-5-5 "$orch"
expect_deny "orchestrator on the worker model" orch-sonnet claude-sonnet-5-5

sidecar worker-sonnet claude-sonnet-5-5 "$worker"
expect_allow "worker on sonnet" worker-sonnet
sidecar worker-opus claude-opus-5-5 "$worker"
expect_allow "validator seat on opus" worker-opus
sidecar worker-bad kimi-k3 "$worker"
expect_deny "worker routed to kimi-k3" worker-bad kimi-k3
sidecar worker-haiku claude-haiku-4-5-20251001 "$worker"
expect_deny "worker on the scratch model" worker-haiku claude-haiku-4-5-20251001
sidecar worker-fable claude-fable-5.1 "$worker"
expect_deny "worker on the orchestrator model" worker-fable claude-fable-5.1

sidecar sub-sonnet claude-sonnet-5-5 "$sub"
expect_allow "subagent on sonnet" sub-sonnet
sidecar sub-opus claude-opus-5-5 "$sub"
expect_allow "subagent on opus" sub-opus
sidecar sub-bad kimi-k3 "$sub"
expect_deny "subagent routed to kimi-k3" sub-bad kimi-k3
sidecar sub-fable claude-fable-5.1 "$sub"
expect_deny "subagent inheriting the orchestrator model" sub-fable claude-fable-5.1

sidecar scratch-ok claude-haiku-4-5-20251001 "$scratch"
expect_allow "scratch mission on haiku" scratch-ok
sidecar scratch-bad kimi-k3 "$scratch"
expect_deny "scratch mission routed to kimi-k3" scratch-bad kimi-k3

# a session with no mission or subagent tag is the owner's own and is not guarded
sidecar owner kimi-k3 '[]'
expect_allow "untagged session" owner

# fail open: no sidecar, no model, no session id, no jq payload
expect_allow "no sidecar" no-such-session
printf '%s' '{"tags":[{"name":"subagent"}]}' >"$sessions/no-model.settings.json"
expect_allow "sidecar without a model" no-model
printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Read"}' |
  FACTORY_PROJECT_DIR="$tmp" MODEL_GUARD_SESSIONS_DIR="$sessions" sh "$hook" || fail "payload without session_id must pass"

# the sidecar sits beside the transcript when the payload names it
mkdir -p "$tmp/proj"
jq -nc --arg m kimi-k3 --argjson t "$worker" '{model: $m, tags: $t}' >"$tmp/proj/by-transcript.settings.json"
printf '%s' "{\"session_id\":\"by-transcript\",\"transcript_path\":\"$tmp/proj/by-transcript.jsonl\",\"tool_name\":\"Read\",\"tool_input\":{\"file_path\":\"/x\"}}" |
  FACTORY_PROJECT_DIR="$tmp" sh "$hook" >/dev/null 2>&1
[ $? -eq 2 ] || fail "sidecar next to transcript_path was not read"

# the denial is logged; an allowed call is not
log="$tmp/build-log/$(date -u +%Y-%m-%d)"
[ -f "$log/worker-bad.jsonl" ] || fail "no denial log for worker-bad"
line=$(tail -1 "$log/worker-bad.jsonl")
for want in '.event == "ModelGuardDeny"' '.session_id == "worker-bad"' '.tool == "Execute"' '.model == "kimi-k3"' \
  '.reason == "undeclared model (rule G): kimi-k3"' '.command == "cargo test -p observatory-core"' '(.ts | length) == 20'; do
  printf '%s' "$line" | jq -e "$want" >/dev/null || fail "denial record lacks $want: $line"
done
[ ! -e "$log/worker-sonnet.jsonl" ] || fail "an allowed call was logged"
[ "$(wc -l <"$log/worker-bad.jsonl" | tr -d ' ')" -eq 1 ] || fail "one denial must write one line"

# the hook is registered for every tool before the generic logger
reg=$(jq -r '.PreToolUse | map(select(.hooks[0].command | test("model-guard.sh"))) | map(.matcher) | join(",")' "$root/.factory/hooks.json")
[ "$reg" = "*" ] || fail "model-guard.sh is not registered once with matcher * in .factory/hooks.json (got '$reg')"

if [ "$failures" -ne 0 ]; then
  echo "$failures failure(s)"
  exit 1
fi
echo "model-guard: ok"
