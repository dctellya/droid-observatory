#!/bin/sh
# VAL-GATE-003: demo-gate.yml triggers only on pull requests into demo/base with the four types, reads contents only,
# has the single job demo-gate, and passes actionlint.
. "$(dirname "$0")/lib.sh"
ids="VAL-GATE-003"
need_tool yq actionlint
wf=.github/workflows/demo-gate.yml
[ -f "$wf" ] || { fail "$wf is missing"; finish "$ids"; }
eq() { # eq <label> <yq expression> <wanted>
  got=$(yq -o=json -I=0 "$2" "$wf" 2> /dev/null)
  [ "$got" = "$3" ] || fail "$1: got $got, wanted $3"
}
eq "on.pull_request.branches" '.on.pull_request.branches' '["demo/base"]'
eq "on.pull_request.types" '.on.pull_request.types' '["opened","synchronize","labeled","unlabeled"]'
eq "permissions" '.permissions' '{"contents":"read"}'
eq "jobs" '.jobs | keys' '["demo-gate"]'
eq "triggers" '.on | keys' '["pull_request"]'
actionlint "$wf" > /dev/null 2>&1 || fail "actionlint exits non-zero on $wf"
finish "$ids"
