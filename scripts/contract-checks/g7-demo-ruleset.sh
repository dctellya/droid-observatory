#!/bin/sh
# VAL-GATE-002 and VAL-GATE-031 (ruleset name): the demo-gate ruleset binds demo/base only, with a pull request rule,
# exactly one required context `demo-gate` and non_fast_forward.
. "$(dirname "$0")/lib.sh"
ids="VAL-GATE-002 VAL-GATE-031"
need_network "$ids"
need_tool jq
id=$(gh api "repos/$REPO/rulesets" --jq '.[] | select(.name == "demo-gate") | .id' 2> /dev/null)
[ -n "$id" ] || { fail "no ruleset named demo-gate"; finish "$ids"; }
body=$(gh api "repos/$REPO/rulesets/$id" 2> /dev/null)
chk() { # chk <label> <jq filter that prints true>
  [ "$(printf '%s' "$body" | jq -r "$2" 2> /dev/null)" = true ] || fail "ruleset $id: $1"
}
chk "conditions.ref_name.include is not exactly [refs/heads/demo/base]" '.conditions.ref_name.include == ["refs/heads/demo/base"]'
chk "no pull_request rule" '[.rules[].type] | index("pull_request") != null'
chk "non_fast_forward rule missing" '[.rules[].type] | index("non_fast_forward") != null'
chk "required_status_checks is not exactly one context demo-gate" '[.rules[] | select(.type == "required_status_checks") | .parameters.required_status_checks[].context] == ["demo-gate"]'
finish "$ids"
