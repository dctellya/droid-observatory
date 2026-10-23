#!/bin/sh
# Compile time and its share of a wall-clock window, from the build-log written by
# .factory/hooks/log-tool-call.sh (D-021, D-025). Usage:
#   scripts/compile-share.sh <build-log dir> <from-ts> <to-ts>
# Timestamps are UTC, as in the log's `ts` field: 2026-10-23T10:00:00Z.
#
# Sums duration_ms of PostToolUse records in the window whose `command` has a segment (after the
# start or after ; & | and an optional `cd x &&`) that begins with `cargo `, `cargo-tauri`,
# `make check`, `make test` or `trunk `. Sessions that ran at the same time overlap, so the sum can
# exceed the window; the share is then of the window, as measured. A call made inside a Script
# call logs no end record of its own (only the whole Script call is timed), so those are reported
# on a second line and left out of the sum.
set -eu
if [ "$#" -ne 3 ]; then
  echo "usage: $0 <build-log dir> <from-ts> <to-ts>" >&2
  exit 2
fi
dir=$1 from=$2 to=$3
[ -d "$dir" ] || { echo "compile-share: no such directory: $dir" >&2; exit 2; }

files=$(find "$dir" -name '*.jsonl' -type f | sort)
[ -n "$files" ] || { echo "compile-share: no .jsonl files under $dir" >&2; exit 2; }

# shellcheck disable=SC2086
result=$(cat $files | jq -rs --arg from "$from" --arg to "$to" '
  def wall: (($to | fromdateiso8601) - ($from | fromdateiso8601));
  def seg: "(^|[;&|(])\\s*([A-Za-z_][A-Za-z0-9_]*=\\S*\\s+)*";
  def compile: test(seg + "(cargo |cargo-tauri|make (check|test)($|[\\s;&|>])|trunk )");
  if wall <= 0 then error("window end must be after start") else . end
  | [.[] | select(.event == "PostToolUse" and .ts >= $from and .ts <= $to)] as $win
  | [$win[] | select(.tool != "Script" and ((.command // "") | compile))] as $c
  | [$win[] | select(.tool == "Script" and ((.script // "") | test("cargo |cargo-tauri|make (check|test)|trunk ")))] as $s
  | [ ($c | map(.duration_ms // 0) | add // 0),
      wall,
      ($c | map(select(.duration_ms != null)) | length),
      ($c | map(select(.duration_ms == null)) | length),
      ($s | length),
      ($s | map(.duration_ms // 0) | add // 0) ] | @tsv') || exit 2

printf '%s\n' "$result" | LC_ALL=C awk -F '\t' '{
  printf "compile_seconds=%.3f wall_seconds=%d share=%.1f%% calls=%d calls_without_duration=%d\n",
    $1 / 1000, $2, $1 / 1000 / $2 * 100, $3, $4
  printf "script_calls_mentioning_compile=%d script_seconds=%.3f (not in the sum)\n", $5, $6 / 1000
}'
