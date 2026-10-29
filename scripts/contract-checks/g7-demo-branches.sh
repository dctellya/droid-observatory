#!/bin/sh
# VAL-GATE-030: between runs the only demo/* branch on GitHub is demo/base; the README entry for the test
# infrastructure lands with the M4 integration, so its absence is pending.
. "$(dirname "$0")/lib.sh"
ids="VAL-GATE-030"
need_network "$ids"
list=$(gh api "repos/$REPO/branches" --paginate --jq '.[].name' 2> /dev/null | grep '^demo/')
[ "$list" = "demo/base" ] || fail "demo branches on GitHub: '$(printf '%s' "$list" | tr '\n' ' ')', wanted exactly demo/base"
grep -q 'demo/base' README.md 2> /dev/null || pending "README.md entry for demo/base lands at the M4 integration"
finish "$ids"
