#!/bin/sh
# VAL-GATE-020 (gh clause): the walk's run window holds exactly one PR from demo/red-1 into demo/base, however many red
# runs it saw. Every walk reuses the head, so earlier closed or merged PRs are history, not a second PR: only PRs created
# at or after the run's start count. The start (UTC, YYYY-MM-DDTHH:MM:SSZ) is, first hit wins:
#   CONTRACT_RUN_START;
#   the earliest pr.opened row of the walk's store (CONTRACT_WALK_STORE, a sqlite file);
#   the creation time of the PR the walk recorded in its PR file (CONTRACT_WALK_PR_FILE, written by e2e/lib/walk.ts).
# Without a window or a demo run in it the check skips; the scratch-store count is a gate-record stage (G8).
. "$(dirname "$0")/lib.sh"
ids="VAL-GATE-020"
# gh lists 30 PRs by default; a reused head can outgrow that.
PR_LIMIT=200
WALK_PR_FILE_DEFAULT=/tmp/m4-walk-pr.txt
need_network "$ids"
start=${CONTRACT_RUN_START:-}
store=${CONTRACT_WALK_STORE:-}
if [ -z "$start" ] && [ -f "$store" ]; then
  need_tool sqlite3
  # The store keeps milliseconds (…:30.000Z); gh prints whole seconds, and the two compare as strings.
  start=$(sqlite3 "$store" "SELECT MIN(at) FROM events WHERE kind = 'pr.opened'" 2> /dev/null | sed 's/\.[0-9]*Z$/Z/')
fi
prfile=${CONTRACT_WALK_PR_FILE:-$WALK_PR_FILE_DEFAULT}
if [ -z "$start" ] && [ -f "$prfile" ]; then
  start=$(gh pr view "$(cat "$prfile")" -R "$REPO" --json createdAt --jq .createdAt 2> /dev/null)
fi
[ -n "$start" ] || skip "no run window (no CONTRACT_RUN_START, walk store row or walk PR file)" "$ids"
created=$(gh pr list -R "$REPO" --base demo/base --head demo/red-1 --state all --limit "$PR_LIMIT" --json createdAt --jq '.[].createdAt' 2> /dev/null)
n=$(printf '%s\n' "$created" | awk -v s="$start" 'NF && $0 >= s' | wc -l | tr -d ' ')
[ "$n" != 0 ] || skip "no demo/red-1 PR since $start" "$ids"
[ "$n" = 1 ] || fail "$n PRs for demo/red-1 created since $start, wanted 1"
finish "$ids"
