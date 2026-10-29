#!/bin/sh
# VAL-PROC-022 (git clauses): core.hooksPath is .githooks, and the hook files changed since the baseline only in
# commits a decision file names (by short sha) or exceptions.txt lists.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-022"
path=$(git config core.hooksPath 2> /dev/null)
if [ -z "$path" ] && [ -n "${CI:-}" ]; then
  pending "core.hooksPath is not set on a CI checkout"
elif [ "$path" != ".githooks" ]; then
  fail "core.hooksPath is '$path', wanted .githooks"
fi
base=c8524c2
git rev-parse --verify -q "$base^{commit}" > /dev/null 2>&1 || base=$(git rev-list --max-parents=0 HEAD | tail -n 1)
for h in $(git log --format=%h "$base..HEAD" -- .factory/hooks.json .factory/hooks/ .githooks/ 2> /dev/null); do
  excepted "$h" && continue
  grep -rqs "$h" docs/decisions || fail "$h changed a hook file and no decision file names it"
done
finish "$ids"
