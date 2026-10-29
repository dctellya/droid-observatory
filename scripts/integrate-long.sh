#!/bin/sh
# The long work of an integration as one target (owner item 16c): `make integrate-long M=<milestone>`.
#   scripts/integrate-long.sh [--force] <milestone>   run the job set, write the result file, exit with the worst exit code
#   scripts/integrate-long.sh --list                  print the job set (lane, name, command; tab separated)
#   scripts/integrate-long.sh --ci-wait               the `ci` job: wait for the CI run that proves the head's code tree
# Start it as a background job and end the turn: the completion is delivered as a message that carries the result table and
# the tail of every failed job's log, so nobody tails or polls it (rule 25, item 16). FORCE=1 is the same as --force.
#
# Jobs and lanes. The default set is coverage (`make coverage`, cargo llvm-cov of observatory-core, itself cached by
# scripts/gate.sh), the warm timings (`cargo check -p observatory-core`, `cargo test -p observatory-core`, and the trunk
# build when apps/observatory-app/ui/Trunk.toml and trunk exist) and `ci`. Lanes run in parallel; the jobs of a lane run in
# order. Coverage and the timings share the `cargo` lane, coverage first, so coverage has the cargo lock alone and the
# timings are measured on an idle machine (AGENTS.md "Runtime limits"); `ci` is the `network` lane and touches no lock.
# INTEGRATE_LONG_JOBS names a file of `lane<TAB>name<TAB>command` lines (`#` lines and blank lines ignored) that replaces the
# default set; the self-test (scripts/tests/integrate_long_test.sh) uses it for a fake job set.
#
# The CI job. `gh run list` for the current branch and the mission workflow (CI_WORKFLOW) picks the newest run whose head is
# HEAD, else the newest run whose head has the same CODE tree as the working tree (scripts/gate.sh --diff is empty): a push
# that only adds reports or docs starts no run (paths-ignore), and the run of the code commit proves it. A completed run
# gives its conclusion; a run in progress is followed with `gh run watch --exit-status`. No run after CI_FIND_SECONDS exits 3
# (the head was not pushed, or the code commit was never pushed). Push before starting the target.
#
# Result file: reports/integrate-long/<code_tree>.json, keyed by the code tree of scripts/gate.sh --key (build-log/,
# reports/ and docs/ never move it). Logs: reports/integrate-long/<code_tree>.logs/<job>.log (`*.log` is git-ignored, so the
# paths are local; the JSON is committed with the boundary files). Schema:
#   milestone     string   the milestone the run was started for
#   code_tree     string   the gate key of the working tree
#   fingerprint   string   hash of code tree, toolchain and job set; a record with another fingerprint is never reused
#   tool          string   `rustc -V | cargo -V`
#   started_at, finished_at   UTC ISO times of this invocation
#   seconds       number   wall time of this invocation (lanes overlap, so it is below the sum of the jobs)
#   exit_code     number   the worst (highest) exit code of any job
#   jobs[]        {name, lane, command, exit_code, seconds, log, hit}; `hit` is true when the job passed in an earlier
#                 invocation on this code tree and was not run again (seconds and log are then the earlier run's)
# A record with exit_code 0 and the same fingerprint is a HIT: nothing runs, the exit code is 0 and the wall time is the tree
# hash (about half a second). After a red run only the failed jobs rerun. --force reruns everything.
# Exit codes: the worst job exit code; 2 for a usage or job-set error; 143, 130 or 129 when interrupted (no result file).
# shellcheck disable=SC2329 # the cleanup and signal functions run from traps
set -u
LC_ALL=C
export LC_ALL

# Every threshold in one place. CI_FIND_*: how long the `ci` job waits for GitHub to create the run of a fresh push (the
# run appears within seconds; two minutes is far past that); CI_WATCH_INTERVAL is the refresh of `gh run watch`;
# CI_LIST_LIMIT and CI_MATCH_LIMIT bound the runs listed and the runs whose code tree is compared (each comparison hashes
# two trees, half a second).
CI_WORKFLOW=${CI_WORKFLOW:-mission-branch.yml}
CI_FIND_SECONDS=${CI_FIND_SECONDS:-120}
CI_FIND_INTERVAL=${CI_FIND_INTERVAL:-10}
CI_WATCH_INTERVAL=${CI_WATCH_INTERVAL:-15}
CI_LIST_LIMIT=${CI_LIST_LIMIT:-20}
CI_MATCH_LIMIT=${CI_MATCH_LIMIT:-8}
TAIL_LINES=20

usage() {
  echo "usage: integrate-long.sh [--force] <milestone> | integrate-long.sh --list | integrate-long.sh --ci-wait   (make integrate-long M=<milestone>)" >&2
  exit 2
}

force=0
[ -n "${FORCE:-}" ] && [ "${FORCE:-}" != 0 ] && force=1
mode=run
while [ $# -gt 0 ]; do
  case "$1" in
    --force) force=1; shift ;;
    --list) mode=list; shift ;;
    --ci-wait) mode=ci; shift ;;
    -*) usage ;;
    *) break ;;
  esac
done
m=${1:-}
[ "$mode" = run ] && [ -z "$m" ] && usage

cd "$(git rev-parse --show-toplevel)" || exit 2
command -v jq > /dev/null 2>&1 || { echo "integrate-long: jq is required" >&2; exit 2; }
tab=$(printf '\t')

stamp() { date -u +%Y-%m-%dT%H:%M:%SZ; }
now() { perl -MTime::HiRes=time -e 'printf "%.3f\n", time' 2> /dev/null || date +%s; }

job_set() {
  if [ -n "${INTEGRATE_LONG_JOBS:-}" ]; then
    cat "$INTEGRATE_LONG_JOBS"
    return
  fi
  printf 'cargo\tcoverage\tmake --no-print-directory coverage\n'
  printf 'cargo\ttiming-check\tcargo check -p observatory-core\n'
  printf 'cargo\ttiming-test\tcargo test -p observatory-core\n'
  if [ -f apps/observatory-app/ui/Trunk.toml ] && command -v trunk > /dev/null 2>&1; then
    printf 'cargo\ttiming-trunk\ttrunk build --config apps/observatory-app/ui/Trunk.toml\n'
  fi
  printf 'network\tci\tsh scripts/integrate-long.sh --ci-wait\n'
}

if [ "$mode" = list ]; then
  job_set | grep -v '^[[:space:]]*\(#\|$\)'
  exit 0
fi

ci_pick() { # ci_pick <runs json>: prints "<id> <status> <conclusion> <sha>" of the run that proves the head, or nothing
  rows=$(printf '%s' "$1" | jq -r '.[] | [.databaseId, .headSha, .status, .conclusion] | @tsv' 2> /dev/null) || return 0
  [ -n "$rows" ] || return 0
  hit=$(printf '%s\n' "$rows" | awk -F '\t' -v h="$head" '$2 == h { print $1 " " $3 " " ($4 == "" ? "-" : $4) " " $2; exit }')
  if [ -n "$hit" ]; then
    echo "$hit"
    return 0
  fi
  n=0
  printf '%s\n' "$rows" | while IFS="$tab" read -r id sha status conclusion; do
    n=$((n + 1))
    [ "$n" -le "$CI_MATCH_LIMIT" ] || break
    git cat-file -e "$sha^{commit}" 2> /dev/null || continue
    # An empty diff is "same code tree" only when the command succeeded: a failed tree computation also prints nothing.
    if d=$(sh scripts/gate.sh --diff "$sha" 2> /dev/null) && [ -z "$d" ]; then
      echo "$id $status ${conclusion:--} $sha"
      break
    fi
  done
}

ci_wait() {
  command -v gh > /dev/null 2>&1 || { echo "ci: gh is required"; return 3; }
  branch=$(git rev-parse --abbrev-ref HEAD)
  head=$(git rev-parse HEAD)
  waited=0
  while :; do
    list=$(gh run list --branch "$branch" --workflow "$CI_WORKFLOW" --limit "$CI_LIST_LIMIT" \
      --json databaseId,headSha,status,conclusion 2>&1) || { echo "ci: gh run list failed: $list"; return 3; }
    picked=$(ci_pick "$list")
    [ -n "$picked" ] && break
    if [ "$waited" -ge "$CI_FIND_SECONDS" ]; then
      echo "ci: no $CI_WORKFLOW run on $branch for $head or for a commit with the same code tree; push the branch first (a push of reports or docs only starts no run)"
      return 3
    fi
    sleep "$CI_FIND_INTERVAL"
    waited=$((waited + CI_FIND_INTERVAL))
  done
  # shellcheck disable=SC2086 # four words, none with spaces
  set -- $picked
  run_id=$1 status=$2 conclusion=$3 sha=$4
  if [ "$sha" = "$head" ]; then
    echo "ci: run $run_id of $branch at $sha ($status)"
  else
    echo "ci: run $run_id of $branch at $sha proves the same code tree as $head ($status)"
  fi
  if [ "$status" = completed ]; then
    echo "ci: run $run_id concluded $conclusion"
    [ "$conclusion" = success ] && return 0
    return 1
  fi
  gh run watch "$run_id" --exit-status --interval "$CI_WATCH_INTERVAL"
}

if [ "$mode" = ci ]; then
  ci_wait
  exit $?
fi

# --- run mode ---
t0=$(now)
started_at=$(stamp)
tree=$(sh scripts/gate.sh --key | sed -n 1p)
[ -n "$tree" ] || { echo "integrate-long: cannot compute the code tree" >&2; exit 2; }
tool="$(rustc -V 2> /dev/null) | $(cargo -V 2> /dev/null)"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/integrate-long.XXXXXX")
cleanup() { rm -rf "$tmp"; }
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

job_set | grep -v '^[[:space:]]*\(#\|$\)' > "$tmp/jobs.tsv"
[ -s "$tmp/jobs.tsv" ] || { echo "integrate-long: the job set is empty" >&2; exit 2; }
total=0
while IFS="$tab" read -r lane name cmd; do
  case "$name" in
    "" | *[!A-Za-z0-9_-]*) echo "integrate-long: bad job name '$name' (letters, digits, - and _)" >&2; exit 2 ;;
  esac
  case "$lane" in
    "" | *[!A-Za-z0-9_-]*) echo "integrate-long: bad lane '$lane' for job $name" >&2; exit 2 ;;
  esac
  [ -n "$cmd" ] || { echo "integrate-long: job $name has no command (lane<TAB>name<TAB>command)" >&2; exit 2; }
  total=$((total + 1))
done < "$tmp/jobs.tsv"

fingerprint=$(printf '%s\n%s\n%s\n' "$tree" "$tool" "$(cat "$tmp/jobs.tsv")" | git hash-object --stdin)
dir=reports/integrate-long
res="$dir/$tree.json"
logdir="$dir/$tree.logs"

print_jobs() { # print_jobs <result json>
  jq -r '.jobs[] | "  \(.name) [\(.lane)] exit \(.exit_code) \(.seconds) s\(if .hit then " (reused)" else "" end) \(.log)"' "$1"
}

prior=
if [ -f "$res" ] && [ "$(jq -r '.fingerprint // empty' "$res" 2> /dev/null)" = "$fingerprint" ]; then
  prior=$res
fi
if [ "$force" -eq 0 ] && [ -n "$prior" ] && [ "$(jq -r .exit_code "$prior")" = 0 ]; then
  echo "integrate-long: HIT code tree $tree, proven at $(jq -r .finished_at "$prior") (milestone $(jq -r .milestone "$prior"))"
  print_jobs "$prior"
  echo "integrate-long: result $res"
  exit 0
fi

mkdir -p "$logdir" "$tmp/frag"
lanes=$(awk -F '\t' '!seen[$1]++ { print $1 }' "$tmp/jobs.tsv")
echo "integrate-long: $m code tree $tree: $total jobs; lanes run in parallel, the jobs of a lane in order"

fragment() { # fragment <seq> <lane> <name> <cmd> <exit> <seconds> <log> <hit>
  jq -n --arg name "$3" --arg lane "$2" --arg command "$4" --argjson exit_code "$5" --argjson seconds "$6" \
    --arg log "$7" --argjson hit "$8" \
    '{name: $name, lane: $lane, command: $command, exit_code: $exit_code, seconds: $seconds, log: $log, hit: $hit}' \
    > "$tmp/frag/$1.json"
}

run_lane() { # run_lane <lane>
  seq=0
  while IFS="$tab" read -r lane name cmd; do
    seq=$((seq + 1))
    [ "$lane" = "$1" ] || continue
    id=$(printf '%03d' "$seq")
    log="$logdir/$name.log"
    if [ "$force" -eq 0 ] && [ -n "$prior" ] \
      && [ "$(jq -r --arg n "$name" '[.jobs[] | select(.name == $n)][0].exit_code // "x"' "$prior")" = 0 ]; then
      jq --arg n "$name" '[.jobs[] | select(.name == $n)][0] | .hit = true' "$prior" > "$tmp/frag/$id.json"
      continue
    fi
    s=$(now)
    sh -c "$cmd" > "$log" 2>&1 < /dev/null &
    wait $!
    code=$?
    e=$(now)
    fragment "$id" "$lane" "$name" "$cmd" "$code" "$(awk -v a="$s" -v b="$e" 'BEGIN { printf "%.3f", b - a }')" "$log" false
  done < "$tmp/jobs.tsv"
}

for l in $lanes; do
  run_lane "$l" &
done
wait

t1=$(now)
finished_at=$(stamp)
seconds=$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f", b - a }')
mkdir -p "$dir"
# shellcheck disable=SC2046 # the fragment names are zero-padded and have no spaces
jq -s --arg milestone "$m" --arg code_tree "$tree" --arg fingerprint "$fingerprint" --arg tool "$tool" \
  --arg started_at "$started_at" --arg finished_at "$finished_at" --argjson seconds "$seconds" \
  '{milestone: $milestone, code_tree: $code_tree, fingerprint: $fingerprint, tool: $tool, started_at: $started_at,
    finished_at: $finished_at, seconds: $seconds, exit_code: ([.[].exit_code] | max), jobs: .}' \
  $(ls "$tmp"/frag/*.json) > "$res.tmp" && mv "$res.tmp" "$res"

worst=$(jq -r .exit_code "$res")
echo "integrate-long: $m code tree $tree: worst exit code $worst in $seconds s"
print_jobs "$res"
jq -r '.jobs[] | select(.exit_code != 0) | "\(.name)\t\(.exit_code)\t\(.log)"' "$res" | while IFS="$tab" read -r name code log; do
  echo "---- $name (exit $code), last $TAIL_LINES lines of $log"
  tail -n "$TAIL_LINES" "$log"
done
echo "integrate-long: result $res"
exit "$worst"
