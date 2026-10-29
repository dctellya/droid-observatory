#!/bin/sh
# VAL-PROC-006 (m4 row): wt/m4 does not touch crates/observatory-core/src/metrics (M5 owns it).
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-006"
[ "$(git rev-parse --abbrev-ref HEAD 2> /dev/null)" = "wt/m4" ] || skip "not on wt/m4" "$ids"
base=$(range_base) || skip "no mission branch ref" "$ids"
stat=$(git diff --stat "$base" "$(head_ref)" -- crates/observatory-core/src/metrics)
[ -z "$stat" ] || fail "metrics sources changed: $stat"
finish "$ids"
