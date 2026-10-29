#!/bin/sh
# VAL-GATE-020 (gh clause): the demo/red-1 branch has exactly one PR into demo/base, however many red runs it saw.
# Without a demo run the check skips; the scratch-store count is a gate-record stage (G8).
. "$(dirname "$0")/lib.sh"
ids="VAL-GATE-020"
need_network "$ids"
n=$(gh pr list -R "$REPO" --base demo/base --head demo/red-1 --state all --json number --jq length 2> /dev/null)
[ "$n" != 0 ] || skip "no demo/red-1 run" "$ids"
[ "$n" = 1 ] || fail "'$n' PRs for demo/red-1, wanted 1"
finish "$ids"
