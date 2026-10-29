#!/bin/sh
# VAL-PROC-010: a worktree branch that has been integrated is the second parent of exactly one first-parent merge of
# the mission branch (no fast-forward, no squash). A branch with no commits of its own, or not integrated yet, is skipped.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-010"
m=$(mission_ref) || skip "no mission branch ref" "$ids"
chain=$(git rev-list --first-parent "$m")
for b in $(git for-each-ref --format='%(refname:short)' refs/heads/wt 2> /dev/null); do
  tip=$(git rev-parse "$b")
  git merge-base --is-ancestor "$tip" "$m" 2> /dev/null || continue
  n=$(git log --first-parent --merges --format=%P "$m" | awk -v t="$tip" '$2 == t' | wc -l | tr -d ' ')
  if printf '%s\n' "$chain" | grep -qx "$tip"; then
    # On the first-parent chain: either a branch that never had a commit, or one that was fast-forwarded in.
    git reflog show "$b" 2> /dev/null | grep -q ': commit' && fail "$b was fast-forwarded into $m (no merge commit has it as second parent)"
  elif [ "$n" -ne 1 ]; then
    fail "$b is the second parent of $n merge commits of $m, wanted 1"
  fi
done
finish "$ids"
