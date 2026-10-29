#!/bin/sh
# VAL-PROC-036 and VAL-PROC-037 (mission end): exactly one PR from the mission branch into main, open, clean, not
# draft, unmerged, with the gate check passing. Before the PR exists the check skips.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-036 VAL-PROC-037"
need_network "$ids"
need_tool jq
n=$(gh pr list -R "$REPO" --base main --head mission/observatory-v1 --state all --json number --jq length 2> /dev/null)
[ "$n" != 0 ] || skip "no PR yet, mission-end clause" "$ids"
[ "$n" = 1 ] || { fail "'$n' PRs from mission/observatory-v1 into main, wanted 1"; finish "$ids"; }
num=$(gh pr list -R "$REPO" --base main --head mission/observatory-v1 --state all --json number --jq '.[0].number' 2> /dev/null)
pr=$(gh pr view "$num" -R "$REPO" --json state,mergeStateStatus,mergedBy,isDraft 2> /dev/null)
[ "$(printf '%s' "$pr" | jq -r '.state')" = OPEN ] || fail "PR $num state is not OPEN"
[ "$(printf '%s' "$pr" | jq -r '.mergeStateStatus')" = CLEAN ] || fail "PR $num mergeStateStatus is not CLEAN"
[ "$(printf '%s' "$pr" | jq -r '.mergedBy')" = null ] || fail "PR $num is merged"
[ "$(printf '%s' "$pr" | jq -r '.isDraft')" = false ] || fail "PR $num is a draft"
gh pr checks "$num" -R "$REPO" 2> /dev/null | grep -Eq '^gate[[:space:]]+pass' || fail "gate is not passing on PR $num"
finish "$ids"
