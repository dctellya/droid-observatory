#!/bin/sh
# Gate cache (rule 27): run a gate command once per (tree, tool, command) and replay its verdict after.
#   scripts/gate.sh [--force] <name> -- <command...>   run on a miss, replay on a hit, exit with the command's code
#   scripts/gate.sh --key [<name> -- <command...>]      print the code tree (and the key hash with a command)
#   scripts/gate.sh --diff <rev>                        git diff --stat of the code paths, <rev>'s code tree to now
# The key is the CODE tree: the worktree content including uncommitted and untracked files, minus the excluded set
# below. The record also carries `tree`, the same content minus build-log/ and reports/gate/ only, so a HIT can say
# that non-code paths moved. Records live in reports/gate/<code_tree>/<name>.json. FORCE=1 is the same as --force.
# Only a green record (exit_code 0) is a hit; a red one is a miss that reruns the command and replaces the record.
# Each hit appends a tab-separated line to reports/gate/hits.log (time, name, code tree, stored seconds, session,
# session_source); misses and forced runs append nothing.
# shellcheck disable=SC2329,SC2012,SC2100 # trap-only functions; ls -t picks the newest file; SC2100 misreads a plain word
set -u

# The excluded set, in one place (config/observatory.example.toml lists it with the reason). These paths prove nothing
# a gate checks about the code, and every integration, usage report and log commit writes them, so keying on them made
# a documentation commit a full rebuild. Markdown is code only under GATE_CODE_ROOTS (crates, apps, scripts, config and
# the workflows carry include_str!, doc tests and markdownlint targets).
GATE_EXCLUDE_DIRS="build-log reports docs"
GATE_CODE_ROOTS="crates apps scripts config .github"

usage() {
  echo "usage: gate.sh [--force] <name> -- <command...> | gate.sh --key [<name> -- <command...>] | gate.sh --diff <rev>" >&2
  exit 2
}

force=0
[ -n "${FORCE:-}" ] && [ "${FORCE:-}" != 0 ] && force=1
key_only=0
diff_rev=
while [ $# -gt 0 ]; do
  case "$1" in
    --force) force=1; shift ;;
    --key) key_only=1; shift ;;
    --diff) [ $# -ge 2 ] || usage; diff_rev=$2; shift 2 ;;
    *) break ;;
  esac
done

name=
if [ $# -gt 0 ]; then
  name=$1
  shift
  [ "${1:-}" = "--" ] && shift
fi
if [ "$key_only" -eq 0 ] && [ -z "$diff_rev" ] && { [ -z "$name" ] || [ $# -eq 0 ]; }; then
  usage
fi

cd "$(git rev-parse --show-toplevel)" || exit 2
command -v jq > /dev/null 2>&1 || { echo "gate: jq is required" >&2; exit 2; }

stamp() { date -u +%Y-%m-%dT%H:%M:%SZ; }
# Decimal points in the duration must be dots whatever the user's locale, or jq rejects the number.
now() { LC_ALL=C perl -MTime::HiRes=time -e 'printf "%.3f\n", time' 2> /dev/null || date +%s; }
# The clock starts at entry: the tree computation (0.5 s on this repository) and the tool-version calls are part
# of the wall time a caller measures around the script, so `seconds` must include them.
started_at=$(stamp)
t0=$(now)

idx=
out=$(mktemp "${TMPDIR:-/tmp}/gateout.XXXXXX")
cleanup() { rm -f "$idx" "$out" "$out.code"; }
# An interrupted run is not a verdict: the handlers stop the command and exit without writing a record, or a later
# run would replay the half-run as a hit. The command runs in the background and the script waits on it, because a
# shell defers a trap until its foreground pipeline ends.
killtree() {
  for c in $(pgrep -P "$1" 2> /dev/null); do killtree "$c"; done
  kill "$1" 2> /dev/null
}
interrupted() {
  trap '' HUP INT TERM
  for c in $(pgrep -P $$ 2> /dev/null); do killtree "$c"; done
  cleanup
  exit "$1"
}
trap cleanup EXIT
trap 'interrupted 129' HUP
trap 'interrupted 130' INT
trap 'interrupted 143' TERM

# The record field is `fingerprint`, not `key`: gitleaks and Droid-Shield read a `key` field holding 40 hex
# characters as an API key and block the commit of the records.
# `git rm --cached` drops the committed copies of the excluded paths: the exclude pathspec only stops
# the working tree's changes, and HEAD's copy would otherwise make a commit of the records change the key.
# Prints two lines: the tree (build-log/ and reports/gate/ out) and the code tree (the excluded set out as well).
# With a revision the index starts from that commit and the working tree is not read; the removals are forced because
# git refuses a path whose index copy differs from both the file and HEAD, which a revision's docs often do.
compute_trees() { # compute_trees [<rev>]
  idx=$(mktemp "${TMPDIR:-/tmp}/gateidx.XXXXXX")
  rm -f "$idx"
  # shellcheck disable=SC2086 # the excluded set is a word list
  (
    GIT_INDEX_FILE=$idx
    export GIT_INDEX_FILE
    git read-tree "${1:-HEAD}" &&
      git rm -r -q --cached --ignore-unmatch -- build-log reports/gate &&
      { [ -n "${1:-}" ] || git add -A -- . ":(exclude)build-log" ":(exclude)reports/gate"; } &&
      git write-tree &&
      git rm -r -q -f --cached --ignore-unmatch -- $GATE_EXCLUDE_DIRS &&
      md=$(git ls-files -- '*.md' | awk -v roots="$GATE_CODE_ROOTS" '
        BEGIN { n = split(roots, r, " ") }
        { keep = 0; for (i = 1; i <= n; i++) if (index($0, r[i] "/") == 1) keep = 1; if (!keep) print }') &&
      { [ -z "$md" ] || printf '%s\n' "$md" | GIT_LITERAL_PATHSPECS=1 git rm -q -f --cached --ignore-unmatch --pathspec-from-file=-; } &&
      git write-tree
  )
  rc=$?
  rm -f "$idx"
  return $rc
}

if [ -n "$diff_rev" ]; then
  git rev-parse --verify -q "$diff_rev^{commit}" > /dev/null || { echo "gate: unknown revision $diff_rev" >&2; exit 2; }
  then_code=$(compute_trees "$diff_rev" | sed -n 2p)
  now_code=$(compute_trees | sed -n 2p)
  [ -n "$then_code" ] && [ -n "$now_code" ] || { echo "gate: cannot compute the code trees" >&2; exit 2; }
  git diff --stat "$then_code" "$now_code"
  exit 0
fi

trees=$(compute_trees) || { echo "gate: cannot compute the tree" >&2; exit 2; }
tree=$(printf '%s\n' "$trees" | sed -n 1p)
code_tree=$(printf '%s\n' "$trees" | sed -n 2p)
[ -n "$tree" ] && [ -n "$code_tree" ] || { echo "gate: cannot compute the tree" >&2; exit 2; }

if [ "$key_only" -eq 1 ] && [ -z "$name" ]; then
  echo "$code_tree"
  exit 0
fi

case "$name" in
  check | test | lint | fmt-check | deny | machete | coverage | replay-fixtures)
    tool="$(rustc -V) | $(cargo -V)" ;;
  *) tool=none ;;
esac
cmd="$*"
key=$(printf '%s\n%s\n%s\n' "$code_tree" "$tool" "$cmd" | git hash-object --stdin)

if [ "$key_only" -eq 1 ]; then
  echo "$code_tree"
  echo "$key"
  exit 0
fi

# The code files that differ from the newest record of this gate: both sides are code trees, so `git diff --stat`
# between them is already restricted to the code paths.
miss_files() { # miss_files <name> <code_tree>
  newest=$(jq -r 'select(.code_tree) | "\(.finished_at)\t\(.code_tree)"' reports/gate/*/"$1.json" 2> /dev/null | sort | tail -n 1)
  prev=$(printf '%s' "$newest" | cut -f2)
  if [ -z "$prev" ]; then
    echo "gate $1: MISS, no earlier record to compare with"
  elif [ "$prev" = "$2" ]; then
    echo "gate $1: MISS, the code paths equal the newest record ($prev); the toolchain or the command differs"
  elif ! git cat-file -e "$prev^{tree}" 2> /dev/null; then
    echo "gate $1: MISS, the newest record's code tree $prev is not in this repository, no file list"
  else
    echo "gate $1: MISS, code files that differ from the newest record ($prev, proven $(printf '%s' "$newest" | cut -f1)):"
    git diff --stat "$prev" "$2"
  fi
}

dir="reports/gate/$code_tree"
file="$dir/$name.json"

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

red_code=
if [ -f "$file" ] && [ "$(jq -r '.fingerprint // empty' "$file" 2> /dev/null)" = "$key" ]; then
  stored=$(jq -r '.exit_code // empty' "$file" 2> /dev/null)
  [ "$stored" = 0 ] || red_code=${stored:-unreadable}
fi

# A red record is evidence of a failure, not of a proof: replaying it made a flaky run a permanent verdict for the tree
# (B-182), so only a green record is a hit and the run that follows a red one replaces it.
if [ "$force" -eq 0 ] && [ -z "$red_code" ] && [ -f "$file" ] && [ "$(jq -r '.fingerprint // empty' "$file" 2> /dev/null)" = "$key" ]; then
  # One line per served hit, for the usage report: the stored seconds are the run the hit saved, the session is
  # the one that was served (the record keeps the one that proved it).
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$(stamp)" "$name" "$code_tree" "$(jq -r .seconds "$file")" "$session" "$session_source" \
    >> reports/gate/hits.log
  echo "cached, proven at $(jq -r .finished_at "$file") by $(jq -r .session "$file")"
  recorded_tree=$(jq -r '.tree // empty' "$file")
  if [ -n "$recorded_tree" ] && [ "$recorded_tree" != "$tree" ]; then
    echo "gate $name: HIT code tree $code_tree; non-code paths changed since the proof (tree $recorded_tree -> $tree)"
  else
    echo "gate $name: HIT code tree $code_tree"
  fi
  jq -r '.summary' "$file"
  exit "$(jq -r .exit_code "$file")"
fi

if [ "$force" -eq 1 ]; then
  echo "gate $name: forced $code_tree, running"
elif [ -n "$red_code" ]; then
  echo "gate $name: miss $code_tree, running; the earlier record for this tree exited $red_code and a red record is never replayed"
else
  echo "gate $name: miss $code_tree, running"
  miss_files "$name" "$code_tree"
fi
# The command's own exit code travels through a file: a pipe into tee would report tee's.
{ "$@"; echo $? > "$out.code"; } 2>&1 | tee "$out" &
wait $!
code=$(cat "$out.code" 2> /dev/null || echo 1)
t1=$(now)
finished_at=$(stamp)
seconds=$(LC_ALL=C awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f", b - a }')

summary=$(grep -E '^test result:|^TOTAL|generated [0-9]+ warning|^error(\[[A-Z0-9]+\])?:|FAILED|^failures:|could not compile' "$out" | tail -n 40)
[ -n "$summary" ] || summary=$(grep -v '^[[:space:]]*$' "$out" | tail -n 1)
[ -n "$summary" ] || summary="(no output)"

mkdir -p "$dir"
jq -n --arg name "$name" --arg tree "$tree" --arg code_tree "$code_tree" --arg fingerprint "$key" --arg tool "$tool" --arg command "$cmd" \
  --argjson exit_code "$code" --arg summary "$summary" --argjson seconds "$seconds" \
  --arg started_at "$started_at" --arg finished_at "$finished_at" \
  --arg session "$session" --arg session_source "$session_source" \
  '{name: $name, tree: $tree, code_tree: $code_tree, fingerprint: $fingerprint, tool: $tool, command: $command, exit_code: $exit_code,
    summary: $summary, seconds: $seconds, started_at: $started_at, finished_at: $finished_at,
    session: $session, session_source: $session_source}' > "$file.tmp" && mv "$file.tmp" "$file"
# A `test` record keeps the name and verdict of every test beside it (`R <binary>`, then `<status> <name>`), which the
# contract checks read to prove named tests ran and passed; the record's summary holds only the counts.
if [ "$name" = test ]; then
  LC_ALL=C awk '
    /^ +Running / { n = $NF; sub(/^\(.*\//, "", n); sub(/\)$/, "", n); sub(/-[0-9a-f]+$/, "", n); print "R " n; next }
    /^ +Doc-tests / { print "R doc-" $2; next }
    /^test .* \.\.\. / {
      i = index($0, " ... "); split(substr($0, i + 5), a, ",")
      if (a[1] == "ok" || a[1] == "FAILED" || a[1] == "ignored") print a[1] " " substr($0, 6, i - 6)
    }' "$out" > "$dir/$name.tests"
fi
exit "$code"
