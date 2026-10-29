#!/bin/sh
# VAL-GATE-013 (mechanical half): the read-only PAT has no Checks permission, so no source line outside a comment
# names the check-runs or check-suites endpoint.
. "$(dirname "$0")/lib.sh"
ids="VAL-GATE-013"
hits=$(grep -rn -E 'check-runs|check-suites' crates/observatory-core/src/ 2> /dev/null | grep -Ev '^[^:]+:[0-9]+:[[:space:]]*//')
[ -z "$hits" ] || fail "check-runs or check-suites outside a comment: $hits"
finish "$ids"
