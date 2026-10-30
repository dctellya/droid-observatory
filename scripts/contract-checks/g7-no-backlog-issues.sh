#!/bin/sh
# VAL-PROC-026: the backlog lives in BACKLOG.md: no issue or label named backlog on GitHub, no gen-backlog.sh tracked.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-026"
need_network "$ids"
n=$(gh issue list -R "$REPO" --state all --label backlog --json number --jq length 2> /dev/null)
[ "$n" = 0 ] || fail "'$n' issue(s) carry the backlog label"
l=$(gh label list -R "$REPO" --json name --jq '[.[] | select(.name == "backlog")] | length' 2> /dev/null)
[ "$l" = 0 ] || fail "a backlog label exists"
[ -z "$(git ls-files scripts/gen-backlog.sh 2> /dev/null)" ] || fail "scripts/gen-backlog.sh is tracked"
finish "$ids"
