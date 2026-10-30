#!/bin/sh
# Test for scripts/compile-share.sh: sums duration_ms of compile commands inside a window.
set -u
root=$(cd "$(dirname "$0")/../.." && pwd)
tool="$root/scripts/compile-share.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0
fail() {
  echo "FAIL: $1"
  failures=$((failures + 1))
}

mkdir -p "$tmp/2026-10-23"
rec() { # rec <ts> <event> <tool> <command|null> <duration|null> [script]
  jq -nc --arg ts "$1" --arg ev "$2" --arg tool "$3" --arg cmd "$4" --argjson d "$5" --arg sc "${6:-}" \
    '{ts: $ts, session_id: "s", event: $ev, tool: $tool,
      command: (if $cmd == "null" then null else $cmd end),
      script: (if $sc == "" then null else $sc end), exit_code: 0, ts_ms: 0}
     + (if $ev == "PostToolUse" then {duration_ms: $d} else {} end)'
}
{
  rec 2026-10-23T10:00:10Z PostToolUse Execute "cargo test -p observatory-core" 20000
  rec 2026-10-23T10:00:20Z PostToolUse Execute "cd /w && cargo clippy -p x -- -D warnings" 10000
  rec 2026-10-23T10:00:30Z PostToolUse Execute "OBSERVATORY_UPDATE_GOLDEN=1 cargo test" 5000
  rec 2026-10-23T10:00:40Z PostToolUse Execute "make check" 30000
  rec 2026-10-23T10:00:50Z PostToolUse Execute "trunk build" 4000
  rec 2026-10-23T10:00:55Z PostToolUse Execute "cargo-tauri build" 1000
  rec 2026-10-23T10:01:00Z PostToolUse Execute "ls cargo-dir" 99999
  rec 2026-10-23T10:01:05Z PostToolUse Execute "grep -E 'cargo|trunk' log" 99999
  rec 2026-10-23T10:01:10Z PostToolUse Execute "cargo test" null
  rec 2026-10-23T10:01:20Z PreToolUse Execute "cargo test" null
  rec 2026-10-23T10:01:30Z PostToolUse Execute "make test" 40000
  rec 2026-10-23T09:59:00Z PostToolUse Execute "cargo build" 77777
  rec 2026-10-23T10:09:00Z PostToolUse Execute "cargo build" 88888
  rec 2026-10-23T10:00:15Z PostToolUse Script null 3000 "cargo test -p x"
  rec 2026-10-23T10:00:16Z PostToolUse Script null 8000 "echo hi"
} > "$tmp/2026-10-23/s.jsonl"

out=$(sh "$tool" "$tmp" 2026-10-23T10:00:00Z 2026-10-23T10:05:00Z)
rc=$?
[ "$rc" -eq 0 ] || fail "exit $rc"
# 20+10+5+30+4+1+40 = 110 s of 300 s
case "$out" in
  *"compile_seconds=110.000"*) ;;
  *) fail "compile seconds: $out" ;;
esac
case "$out" in *"wall_seconds=300"*) ;; *) fail "wall seconds: $out" ;; esac
case "$out" in *"share=36.7%"*) ;; *) fail "share: $out" ;; esac
case "$out" in *"calls=7"*) ;; *) fail "call count: $out" ;; esac
case "$out" in *"calls_without_duration=1"*) ;; *) fail "missing durations: $out" ;; esac
case "$out" in *"script_calls_mentioning_compile=1"*"script_seconds=3.000"*) ;; *) fail "script line: $out" ;; esac

sh "$tool" "$tmp" 2026-10-23T10:00:00Z >/dev/null 2>&1 && fail "missing argument accepted"
sh "$tool" "$tmp/none" 2026-10-23T10:00:00Z 2026-10-23T10:05:00Z >/dev/null 2>&1 && fail "missing dir accepted"
sh "$tool" "$tmp" 2026-10-23T10:05:00Z 2026-10-23T10:00:00Z >/dev/null 2>&1 && fail "reversed window accepted"

if [ "$failures" -ne 0 ]; then
  echo "$failures failure(s)"
  exit 1
fi
echo "compile-share: ok"
