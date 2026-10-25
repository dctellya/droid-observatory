#!/bin/sh
# Gate cache (rule 27): run a gate command once per (tree, tool, command) and replay its verdict after.
#   scripts/gate.sh [--force] <name> -- <command...>   run on a miss, replay on a hit, exit with the command's code
#   scripts/gate.sh --key [<name> -- <command...>]      print the tree id (and the key hash with a command)
# The tree is the worktree content including uncommitted and untracked files, build-log/ and reports/gate/
# excluded. Records live in reports/gate/<tree>/<name>.json. FORCE=1 is the same as --force.
set -u
usage() {
  echo "usage: gate.sh [--force] <name> -- <command...> | gate.sh --key [<name> -- <command...>]" >&2
  exit 2
}

force=0
[ -n "${FORCE:-}" ] && [ "${FORCE:-}" != 0 ] && force=1
key_only=0
while [ $# -gt 0 ]; do
  case "$1" in
    --force) force=1; shift ;;
    --key) key_only=1; shift ;;
    *) break ;;
  esac
done

name=
if [ $# -gt 0 ]; then
  name=$1
  shift
  [ "${1:-}" = "--" ] && shift
fi
if [ "$key_only" -eq 0 ] && { [ -z "$name" ] || [ $# -eq 0 ]; }; then
  usage
fi

cd "$(git rev-parse --show-toplevel)" || exit 2
command -v jq > /dev/null 2>&1 || { echo "gate: jq is required" >&2; exit 2; }

idx=$(mktemp "${TMPDIR:-/tmp}/gateidx.XXXXXX")
out=$(mktemp "${TMPDIR:-/tmp}/gateout.XXXXXX")
trap 'rm -f "$idx" "$out" "$out.code"' EXIT HUP INT TERM
rm -f "$idx"

# The record field is `fingerprint`, not `key`: gitleaks and Droid-Shield read a `key` field holding 40 hex
# characters as an API key and block the commit of the records.
# `git rm --cached` drops the committed copies of the excluded paths: the exclude pathspec only stops
# the working tree's changes, and HEAD's copy would otherwise make a commit of the records change the key.
tree=$(GIT_INDEX_FILE="$idx" sh -c 'git read-tree HEAD &&
  git rm -r -q --cached --ignore-unmatch -- build-log reports/gate &&
  git add -A -- . ":(exclude)build-log" ":(exclude)reports/gate" &&
  git write-tree') || { echo "gate: cannot compute the tree" >&2; exit 2; }

if [ "$key_only" -eq 1 ] && [ -z "$name" ]; then
  echo "$tree"
  exit 0
fi

case "$name" in
  check | test | lint | fmt-check | deny | machete | coverage | replay-fixtures)
    tool="$(rustc -V) | $(cargo -V)" ;;
  *) tool=none ;;
esac
cmd="$*"
key=$(printf '%s\n%s\n%s\n' "$tree" "$tool" "$cmd" | git hash-object --stdin)

if [ "$key_only" -eq 1 ]; then
  echo "$tree"
  echo "$key"
  exit 0
fi

dir="reports/gate/$tree"
file="$dir/$name.json"

if [ "$force" -eq 0 ] && [ -f "$file" ] && [ "$(jq -r '.fingerprint // empty' "$file" 2> /dev/null)" = "$key" ]; then
  echo "cached, proven at $(jq -r .finished_at "$file") by $(jq -r .session "$file")"
  jq -r '.summary' "$file"
  exit "$(jq -r .exit_code "$file")"
fi

session=${DROID_SESSION_ID:-}
session_source=DROID_SESSION_ID
if [ -z "$session" ]; then
  session=${FACTORY_SESSION_ID:-}
  session_source=FACTORY_SESSION_ID
fi
if [ -z "$session" ]; then
  newest=$(ls -t "${GATE_SESSIONS_DIR:-$HOME/.factory/sessions}"/*/*.settings.json 2> /dev/null | head -n 1)
  if [ -n "$newest" ]; then
    session=$(basename "$newest" .settings.json)
    session_source=newest-settings-mtime
  else
    session=unknown
    session_source=none
  fi
fi

# Decimal points in the duration must be dots whatever the user's locale, or jq rejects the number.
now() { LC_ALL=C perl -MTime::HiRes=time -e 'printf "%.3f\n", time' 2> /dev/null || date +%s; }
stamp() { date -u +%Y-%m-%dT%H:%M:%SZ; }

if [ "$force" -eq 1 ]; then
  echo "gate $name: forced $tree, running"
else
  echo "gate $name: miss $tree, running"
fi
started_at=$(stamp)
t0=$(now)
# The command's own exit code travels through a file: a pipe into tee would report tee's.
{ "$@"; echo $? > "$out.code"; } 2>&1 | tee "$out"
code=$(cat "$out.code" 2> /dev/null || echo 1)
t1=$(now)
finished_at=$(stamp)
seconds=$(LC_ALL=C awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f", b - a }')

summary=$(grep -E '^test result:|^TOTAL|generated [0-9]+ warning|^error(\[[A-Z0-9]+\])?:|FAILED|^failures:|could not compile' "$out" | tail -n 40)
[ -n "$summary" ] || summary=$(grep -v '^[[:space:]]*$' "$out" | tail -n 1)
[ -n "$summary" ] || summary="(no output)"

mkdir -p "$dir"
jq -n --arg name "$name" --arg tree "$tree" --arg fingerprint "$key" --arg tool "$tool" --arg command "$cmd" \
  --argjson exit_code "$code" --arg summary "$summary" --argjson seconds "$seconds" \
  --arg started_at "$started_at" --arg finished_at "$finished_at" \
  --arg session "$session" --arg session_source "$session_source" \
  '{name: $name, tree: $tree, fingerprint: $fingerprint, tool: $tool, command: $command, exit_code: $exit_code,
    summary: $summary, seconds: $seconds, started_at: $started_at, finished_at: $finished_at,
    session: $session, session_source: $session_source}' > "$file.tmp" && mv "$file.tmp" "$file"
exit "$code"
