#!/bin/sh
# VAL-PROC-020 (reflog clause): no amend, reset or rebase entry in the reflog of the mission branch or a worktree
# branch. The gh activity clause is g7-no-force-push.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-020"
for ref in $(git for-each-ref --format='%(refname:short)' refs/heads/mission refs/heads/wt 2> /dev/null); do
  hits=$(git reflog show "$ref" 2> /dev/null | grep -E 'commit \(amend\)|reset:|rebase')
  [ -z "$hits" ] || fail "$ref reflog: $hits"
done
finish "$ids"
