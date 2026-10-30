#!/bin/sh
# Test for the operational summary in AGENTS.md and the ignore list (VAL-PROC-052, owner item 6, F-056): rule 27 is the
# gate cache and rule 28 the gate flow, the staging-file line stays under F-066, the wait wording is "background Script,
# end the turn, no WaitForScript", and the user-data and bytecode paths are ignored.
set -u
root=$(cd "$(dirname "$0")/../.." && pwd)
failures=0
fail() {
  echo "FAIL: $1"
  failures=$((failures + 1))
}
ok() { echo "ok: $1"; }
check() { # check <description> <command...>
  d=$1
  shift
  if "$@"; then ok "$d"; else fail "$d"; fi
}
agents="$root/AGENTS.md"
flat=$(tr '\n' ' ' < "$agents" | tr -s ' ')
has() { printf '%s' "$flat" | grep -q -- "$1"; }
lacks() { ! printf '%s' "$flat" | grep -q -- "$1"; }

check "rule 27 is named as the gate cache by tree hash" has 'gate cache by tree hash[^;]*(27)'
check "rule 27 is not named as the staging-file rule" lacks 'merges files (27)'
check "rule 28 is named as the gate flow" has 'gate flow[^;]*(28)'
check "the staging-file line stays under F-066" has 'the integrator merges the files (F-066)'
readme_lacks() { ! grep -q 'rule 27, F-066' "$root/reports/README.md"; }
check "reports/README.md does not label the staging file as rule 27" readme_lacks
check "the wait line says the run starts in a background Script" has 'background Script'
check "the wait line says the turn ends" has 'ends its turn'
check "the wait line says there is no WaitForScript call" has 'no `WaitForScript` call'
check "no sentence allows a blocking wait" lacks 'in one blocking call'
check "no sentence allows a re-issued wait" lacks 're-issued only'
check "rule 25 line does not describe a blocking call" lacks 'waits in one blocking call'

ignored() { git -C "$root" check-ignore -q "$1"; }
check "vscode_target/ is ignored" ignored vscode_target/some/file
check "__pycache__/ is ignored" ignored scripts/__pycache__/x.pyc
check "*.pyc is ignored" ignored scripts/x.pyc

if [ "$failures" -ne 0 ]; then
  echo "agents_md_test: $failures failed"
  exit 1
fi
echo "agents_md_test: all passed"
