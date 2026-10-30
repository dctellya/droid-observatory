#!/bin/sh
# VAL-PROC-020 (activity clause): GitHub's activity log for the mission branch and every pushed worktree branch lists
# only push, branch_creation and merge.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-020"
need_network "$ids"
for b in $(gh api "repos/$REPO/branches" --paginate --jq '.[].name' 2> /dev/null | grep -E '^(mission/observatory-v1|wt/)'); do
  bad=$(gh api "repos/$REPO/activity?ref=refs/heads/$b&per_page=100" --jq '.[].activity_type' 2> /dev/null | sort -u | grep -Ev '^(push|branch_creation|merge)$')
  [ -z "$bad" ] || fail "$b has activity '$(printf '%s' "$bad" | tr '\n' ' ')'"
done
finish "$ids"
