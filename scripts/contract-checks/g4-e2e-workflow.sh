#!/bin/sh
# VAL-E2E-006 (mechanical clauses): e2e.yml has a macos-latest job that installs the trunk and tauri-cli versions of
# the SPEC Tooling rows, sets up node, builds through `make e2e` and uploads reports/e2e/**; mission-branch.yml does
# not mention e2e (the 3-minute budget); actionlint passes.
. "$(dirname "$0")/lib.sh"
ids="VAL-E2E-006"
need_tool yq actionlint
wf=.github/workflows/e2e.yml
[ -f "$wf" ] || { fail "$wf is missing"; finish "$ids"; }
[ "$(yq '[.jobs[] | select(."runs-on" == "macos-latest")] | length' "$wf")" -ge 1 ] || fail "no job runs on macos-latest"
tools=$(yq '.jobs[].steps[].with.tool | select(. != null)' "$wf" | tr ',' '\n')
for t in trunk tauri-cli; do
  want=$(grep -E "^\| \`$t\` \| [0-9.]+ \|" SPEC.md | head -n 1 | awk -F'|' '{gsub(/ /, "", $3); print $3}')
  have=$(printf '%s\n' "$tools" | sed -n "s/^$t@//p" | head -n 1)
  [ -n "$want" ] || fail "SPEC.md has no Tooling row for $t"
  [ "$want" = "$have" ] || fail "$t: workflow pins '$have', SPEC Tooling row says '$want'"
done
grep -q 'actions/setup-node' "$wf" || fail "no actions/setup-node step"
grep -Eq 'make e2e|npm ci' "$wf" || fail "no step that builds and runs the suite (make e2e)"
grep -q 'features e2e' Makefile 2> /dev/null || [ ! -f Makefile ] || fail "Makefile e2e target does not build --features e2e"
grep -q 'reports/e2e' "$wf" || fail "reports/e2e/** is not uploaded"
grep -q 'actions/upload-artifact' "$wf" || fail "no upload-artifact step"
grep -qi 'e2e' .github/workflows/mission-branch.yml 2> /dev/null && fail "mission-branch.yml mentions e2e"
actionlint "$wf" > /dev/null 2>&1 || fail "actionlint exits non-zero on $wf"
finish "$ids"
