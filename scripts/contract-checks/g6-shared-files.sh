#!/bin/sh
# VAL-PROC-033: a worktree branch leaves the integrator's files alone (README.md, BACKLOG.md, the decisions index,
# the usage, readiness and CI reports) and adds no SPEC section 9 or 10 table row. Not run on the mission branch itself.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-033"
base=$(range_base) || skip "no mission branch ref" "$ids"
[ "$base" != "$(git rev-parse HEAD)" ] || skip "no commits beyond the mission branch" "$ids"
git rev-parse --abbrev-ref HEAD | grep -q '^wt/' || skip "not a worktree branch" "$ids"
for h in $(git log --format=%h "$base..HEAD" -- README.md BACKLOG.md docs/decisions/README.md reports/usage reports/readiness reports/ci); do
  excepted "$h" || fail "$h touched a file the integrator writes"
done
rows=$(git diff "$base" HEAD -- SPEC.md | grep -E '^[+-]\| (F-[0-9]{3}|S |M[0-5] )')
[ -z "$rows" ] || fail "SPEC.md table rows changed on the branch: $rows"
finish "$ids"
