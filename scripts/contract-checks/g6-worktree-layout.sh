#!/bin/sh
# VAL-PROC-001: the active milestone lives in observatory-wt/<m> on branch wt/<m>, git lists it as a worktree on that
# branch, and observatory-wt/current resolves to it. A checkout outside observatory-wt skips.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-001"
top=$(git rev-parse --show-toplevel 2> /dev/null)
parent=$(dirname "$top")
[ "$(basename "$parent")" = "observatory-wt" ] || skip "not an observatory-wt worktree" "$ids"
m=$(basename "$top")
[ "$(git rev-parse --abbrev-ref HEAD)" = "wt/$m" ] || fail "HEAD is not wt/$m"
git worktree list --porcelain | awk -v p="$top" '$1 == "worktree" { on = ($2 == p) } on && $1 == "branch" { print $2 }' | grep -qx "refs/heads/wt/$m" ||
  fail "git worktree list has no block for $top on refs/heads/wt/$m"
if [ -L "$parent/current" ]; then
  [ "$(cd "$parent/current" 2> /dev/null && pwd -P)" = "$(cd "$top" && pwd -P)" ] || fail "observatory-wt/current does not resolve to $top"
else
  fail "observatory-wt/current is not a symlink"
fi
git branch --list "wt/$m" | grep -q . || fail "branch wt/$m is not listed"
finish "$ids"
