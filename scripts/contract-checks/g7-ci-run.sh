#!/bin/sh
# VAL-PROC-009: the CI run of HEAD succeeded and its machete and coverage jobs succeeded. A commit without a run (not
# pushed, or [skip ci]) skips.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-009"
need_network "$ids"
need_tool jq
sha=$(git rev-parse HEAD 2> /dev/null)
run=$(gh run list -R "$REPO" --commit "$sha" --json databaseId,conclusion,status --jq '.[0]' 2> /dev/null)
case "$run" in '' | null) skip "no CI run for $(printf '%s' "$sha" | cut -c1-7)" "$ids" ;; esac
[ "$(printf '%s' "$run" | jq -r .status)" = completed ] || skip "CI run in progress" "$ids"
[ "$(printf '%s' "$run" | jq -r .conclusion)" = success ] || fail "the run of $sha concluded '$(printf '%s' "$run" | jq -r .conclusion)'"
jobs=$(gh run view "$(printf '%s' "$run" | jq -r .databaseId)" -R "$REPO" --json jobs --jq '.jobs[] | "\(.name) \(.conclusion)"' 2> /dev/null)
for j in machete coverage; do
  printf '%s\n' "$jobs" | grep -qx "$j success" || fail "job $j is not success"
done
finish "$ids"
