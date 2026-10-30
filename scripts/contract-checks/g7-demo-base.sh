#!/bin/sh
# VAL-GATE-001: demo/base exists on GitHub and descends from the mission-branch commit it was cut from (demo-base-cut.txt, which never moves;
# git-baseline.txt is the range base of the history checks and moves at each boundary, B-184).
. "$(dirname "$0")/lib.sh"
ids="VAL-GATE-001"
need_network "$ids"
name=$(gh api "repos/$REPO/branches/demo/base" --jq .name 2> /dev/null)
[ "$name" = "demo/base" ] || fail "gh api branches/demo/base did not return the branch (got '$name')"
base=$(cat "$(dirname "$0")/demo-base-cut.txt")
status=$(gh api "repos/$REPO/compare/$base...demo/base" --jq .status 2> /dev/null)
case "$status" in identical | ahead) ;; *) fail "demo/base is '$status' relative to $base, wanted ahead or identical" ;; esac
finish "$ids"
