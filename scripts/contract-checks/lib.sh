# shellcheck shell=sh
# Shared helpers of the contract checks (owner item 14b). Sourced by every check; a check prints exactly one line
#   <check> <ok|FAIL|skip (<why>)> <assertion ids> [note]
# and exits 0 (ok, skip) or 1 (FAIL). Reasons for a FAIL go to stderr, one line each.
# CONTRACT_ROOT points a check at a scratch tree (the self-test plants violations there).
set -u
CHECK=$(basename "$0" .sh)
ROOT=${CONTRACT_ROOT:-$(git rev-parse --show-toplevel 2> /dev/null || pwd)}
cd "$ROOT" || exit 2
FAILS=
PENDING=
fail() {
  FAILS="$FAILS$*
"
}
# A clause whose code has not landed yet is `pending`, not red: the check stays green and names what it waits for, so a
# validator reading the gate record sees which assertions the gate does not cover yet.
pending() { PENDING="${PENDING:+$PENDING; }$*"; }
finish() { # finish <assertion ids>
  if [ -n "$FAILS" ]; then
    printf '%s FAIL %s\n' "$CHECK" "$1"
    printf '%s' "$FAILS" | sed "s/^/  $CHECK: /" >&2
    exit 1
  fi
  printf '%s ok %s%s\n' "$CHECK" "$1" "${PENDING:+ [pending: $PENDING]}"
  exit 0
}
skip() { # skip <why> <assertion ids>
  printf '%s skip (%s) %s\n' "$CHECK" "$1" "$2"
  exit 0
}
need_tool() { # need_tool <tool>...: a missing tool is a FAIL locally; a CI runner without it skips (its own job covers it)
  for t in "$@"; do
    command -v "$t" > /dev/null 2>&1 && continue
    [ -z "${CI:-}" ] || skip "$t is not installed on this CI runner" "${ids:--}"
    fail "tool $t is not installed"
    finish "${ids:--}"
  done
}
mission_ref() {
  for r in mission/observatory-v1 origin/mission/observatory-v1; do
    if git rev-parse --verify -q "$r^{commit}" > /dev/null 2>&1; then
      echo "$r"
      return 0
    fi
  done
  return 1
}
# The GitHub token the gh checks use; without one they skip (no network in the sandbox, no secret in CI).
need_network() { # need_network <assertion ids>
  GH_TOKEN=${GH_TOKEN:-${GITHUB_TOKEN:-}}
  [ -n "$GH_TOKEN" ] || skip "no network" "$1"
  export GH_TOKEN
  REPO=${CONTRACT_REPO:-dctellya/droid-observatory}
}
# exceptions.txt: `<check> <sha7> <reason>`, a commit that predates its check and was reviewed; the list only shrinks.
excepted() { # excepted <sha7>
  grep -q "^$CHECK $1 " "$(dirname "$0")/exceptions.txt" 2> /dev/null
}
# The commit a check judges. GitHub checks out a pull_request as a synthetic merge commit (refs/pull/N/merge) that
# carries no trailer and hides the branch behind it; the branch is its second parent.
head_ref() {
  if [ "${GITHUB_EVENT_NAME:-}" = pull_request ] && git rev-parse --verify -q 'HEAD^2' > /dev/null 2>&1; then
    git rev-parse 'HEAD^2'
  else
    git rev-parse HEAD
  fi
}
# The commits a branch adds: from the merge base with the mission branch, or from the pinned baseline when the head is
# the mission branch itself.
range_base() {
  m=$(mission_ref) || return 1
  h=$(head_ref)
  b=$(git merge-base "$m" "$h" 2> /dev/null) || return 1
  if [ "$b" = "$h" ]; then
    pin=$(cat "$(dirname "$0")/git-baseline.txt" 2> /dev/null)
    if [ -n "$pin" ] && git merge-base --is-ancestor "$pin" "$h" 2> /dev/null; then
      b=$pin
    fi
  fi
  echo "$b"
}
