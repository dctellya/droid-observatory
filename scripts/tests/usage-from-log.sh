#!/bin/sh
# Test for scripts/usage-from-log.sh on a hand-written 10-line sample (no real log lines).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
script="$here/../usage-from-log.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

A=aaaaaaaa-0000-4000-8000-000000000001
B=bbbbbbbb-0000-4000-8000-000000000002
C=cccccccc-0000-4000-8000-000000000003

cat >"$tmp/sample.log" <<EOF
[2026-10-23T10:00:00.100Z] INFO: [metrics_log_droid_chat_client_success_count] | Context: {"modelId":"m","sessionId":"$A"}
[2026-10-23T10:00:00.200Z] INFO: [Agent] Streaming result | Context: {"count":2,"inputTokens":2,"cacheReadInputTokens":1000,"totalInputTokens":1200,"outputTokens":50,"tags":{"sessionId":"$A"}}
[2026-10-23T10:00:05.100Z] INFO: [metrics_log_droid_chat_client_success_count] | Context: {"modelId":"m","sessionId":"$B"}
[2026-10-23T10:00:05.300Z] INFO: [Agent] Streaming result | Context: {"count":2,"inputTokens":3,"cacheReadInputTokens":2000,"totalInputTokens":2300,"outputTokens":70,"tags":{"sessionId":"$B"}}
[2026-10-23T10:00:06.000Z] INFO: [Daemon] Droid working state changed | Context: {"state":"idle"}
[2026-10-23T10:00:09.100Z] INFO: [metrics_log_droid_chat_client_success_count] | Context: {"modelId":"m","sessionId":"$A"}
[2026-10-23T10:00:09.900Z] INFO: [Agent] Streaming result | Context: {"count":2,"inputTokens":4,"cacheReadInputTokens":500,"totalInputTokens":700,"outputTokens":10,"tags":{"sessionId":"$A"}}
[2026-10-23T10:00:11.000Z] INFO: [Agent] Streaming result | Context: {"count":2,"inputTokens":9,"cacheReadInputTokens":90,"totalInputTokens":99,"outputTokens":9,"tags":{"sessionId":"$A"}}
[2026-10-23T10:00:20.100Z] INFO: [metrics_log_droid_chat_client_success_count] | Context: {"modelId":"m","sessionId":"$C"}
[2026-10-23T10:00:20.400Z] INFO: [Agent] Streaming result | Context: {"count":2,"inputTokens":5,"cacheReadInputTokens":50,"totalInputTokens":60,"outputTokens":5,"tags":{"sessionId":"$C"}}
EOF

fail=0
check() {
  name=$1 want=$2 got=$3
  if [ "$want" = "$got" ]; then
    echo "ok   $name"
  else
    echo "FAIL $name"
    echo "  want: $want"
    echo "  got:  $got"
    fail=1
  fi
}

tab=$(printf '\t')
rows() { grep -v '^#' | tr "$tab" ' '; }

got=$(sh "$script" "$tmp/sample.log" --session "$A" "$B" | rows)
want="$A 2026-10-23T10:00:00.200Z 2026-10-23T10:00:09.900Z 2 6 60 1500
$B 2026-10-23T10:00:05.300Z 2026-10-23T10:00:05.300Z 1 3 70 2000"
check "metrics join sums per session and skips the turn with no metrics line in its second" "$want" "$got"

got=$(sh "$script" "$tmp/sample.log" --embedded --session "$A" | rows)
want="$A 2026-10-23T10:00:00.200Z 2026-10-23T10:00:11.000Z 3 15 69 1590"
check "embedded join counts the turn the metrics join cannot attribute" "$want" "$got"

got=$(sh "$script" "$tmp/sample.log" --from 2026-10-23T10:00:05Z --to 2026-10-23T10:00:10Z --session "$A" "$B" | rows)
want="$A 2026-10-23T10:00:09.900Z 2026-10-23T10:00:09.900Z 1 4 10 500
$B 2026-10-23T10:00:05.300Z 2026-10-23T10:00:05.300Z 1 3 70 2000"
check "from/to keeps only turns inside the band" "$want" "$got"

got=$(sh "$script" "$tmp/sample.log" --session dddddddd-0000-4000-8000-000000000004 | rows)
check "unknown session prints a zero row" "dddddddd-0000-4000-8000-000000000004 - - 0 0 0 0" "$got"

got=$(sh "$script" "$tmp/sample.log" --session "$A" | grep '^#')
want="# logs first_line=2026-10-23T10:00:00.100Z last_line=2026-10-23T10:00:20.400Z lines=10 streaming_result_lines=5
# join=metrics unjoined=1 embedded_vs_metrics_disagree=0"
check "coverage lines state first and last line, line count and unjoined turns" "$want" "$got"

exit "$fail"
