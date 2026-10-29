#!/bin/sh
# Test for scripts/integrate-long.sh (owner item 16c) on throwaway repositories with a fake job set: the result file and
# its schema, lanes in parallel and jobs in a lane in order, the worst exit code, a HIT that waits zero on an unchanged
# code tree (a docs commit keeps it, a code commit loses it), the rerun of only the failed jobs, FORCE, and the CI job
# (`--ci-wait`) against a stub `gh`.
# shellcheck disable=SC2016,SC2012 # the job commands are single-quoted on purpose; ls output is counted, not parsed
set -u
unset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE FORCE FACTORY_SESSION_ID DROID_SESSION_ID INTEGRATE_LONG_JOBS
root=$(cd "$(dirname "$0")/../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0
fail() {
  echo "FAIL: $1"
  failures=$((failures + 1))
}
ok() { echo "ok: $1"; }
check() { # check <description> <command...>
  d=$1
  shift
  if "$@"; then ok "$d"; else fail "$d"; fi
}
LC_ALL=C
export LC_ALL
now() { perl -MTime::HiRes=time -e 'printf "%.3f\n", time'; }

mkdir -p "$tmp/bin"
printf '#!/bin/sh\necho "rustc 1.99.0"\n' > "$tmp/bin/rustc"
printf '#!/bin/sh\necho "cargo 1.99.0"\n' > "$tmp/bin/cargo"
chmod +x "$tmp/bin/rustc" "$tmp/bin/cargo"
PATH="$tmp/bin:$PATH"
export PATH
COUNTER="$tmp/count"
export COUNTER
: > "$COUNTER"
count_of() { grep -c "^$1\$" "$COUNTER" | tr -d ' '; }

mkrepo() { # mkrepo <name>; leaves the current directory in the repository
  repo="$tmp/$1"
  mkdir -p "$repo/scripts" "$repo/reports/gate" "$repo/docs" "$repo/build-log" "$repo/crates"
  cd "$repo" || exit 1
  git init -q -b main .
  git config user.name tester
  git config user.email tester@example.invalid
  git config commit.gpgsign false
  cp "$root/scripts/gate.sh" scripts/
  cp "$root/scripts/integrate-long.sh" scripts/ 2> /dev/null
  echo 'fn main() {}' > crates/a.rs
  echo "# note" > docs/note.md
  git add -A
  git commit -q -m base
}

tab=$(printf '\t')
jobs_file="$tmp/jobs.tsv"
fake_jobs() { # fake_jobs: lane x runs a1 then a2 (a2 needs a1's marker), lane y runs b1, each sleeping
  {
    printf '#fake job set\n'
    printf 'x%sa1%ssh -c '"'"'echo a1 >> "$COUNTER"; touch "$TMPDIR_JOB/a1.done"; sh "$NOWSH" > "$TMPDIR_JOB/a1.s"; sleep 1; sh "$NOWSH" > "$TMPDIR_JOB/a1.e"; echo a1 out'"'"'\n' "$tab" "$tab"
    printf 'x%sa2%ssh -c '"'"'test -f "$TMPDIR_JOB/a1.done" && echo a2 >> "$COUNTER"; echo a2 out'"'"'\n' "$tab" "$tab"
    printf 'y%sb1%ssh -c '"'"'echo b1 >> "$COUNTER"; sh "$NOWSH" > "$TMPDIR_JOB/b1.s"; sleep 1; sh "$NOWSH" > "$TMPDIR_JOB/b1.e"; echo b1 out'"'"'\n' "$tab" "$tab"
  } > "$jobs_file"
}
TMPDIR_JOB="$tmp/job"
NOWSH="$tmp/now.sh"
printf '#!/bin/sh\nperl -MTime::HiRes=time -e '"'"'printf "%%.3f\\n", time'"'"'\n' > "$NOWSH"
export TMPDIR_JOB NOWSH
INTEGRATE_LONG_JOBS="$jobs_file"
export INTEGRATE_LONG_JOBS

echo "== miss: lanes in parallel, one result file =="
mkrepo miss
rm -rf "$TMPDIR_JOB"
mkdir -p "$TMPDIR_JOB"
: > "$COUNTER"
fake_jobs
tree=$(sh scripts/gate.sh --key | sed -n 1p)
t0=$(now)
sh scripts/integrate-long.sh m9 > "$tmp/miss.out" 2>&1
code=$?
t1=$(now)
wall=$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f", b - a }')
res="reports/integrate-long/$tree.json"
check "miss: exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/miss.out"
check "miss: one result file keyed by the code tree" [ -f "$res" ]
check "miss: only one json in the directory" [ "$(ls reports/integrate-long/*.json | wc -l | tr -d ' ')" = 1 ]
check "miss: the result is valid json with the schema fields" jq -e '
  (.milestone == "m9") and (.code_tree | type == "string") and (.fingerprint | type == "string") and (.exit_code == 0)
  and (.seconds | type == "number") and (.started_at | type == "string") and (.finished_at | type == "string")
  and (.jobs | length == 3)
  and all(.jobs[]; (.name | type == "string") and (.lane | type == "string") and (.command | type == "string")
    and (.exit_code | type == "number") and (.seconds | type == "number") and (.log | type == "string") and (.hit == false))' "$res"
check "miss: code_tree in the file is the gate key" [ "$(jq -r .code_tree "$res")" = "$tree" ]
check "miss: every log path exists and holds the job output" sh -c "
  for l in \$(jq -r '.jobs[].log' $res); do [ -s \"\$l\" ] || exit 1; done
  grep -q 'a1 out' \$(jq -r '.jobs[] | select(.name==\"a1\") | .log' $res)"
overlap=$(awk -v as="$(cat "$TMPDIR_JOB/a1.s")" -v ae="$(cat "$TMPDIR_JOB/a1.e")" -v bs="$(cat "$TMPDIR_JOB/b1.s")" -v be="$(cat "$TMPDIR_JOB/b1.e")" \
  'BEGIN { print (as < be && bs < ae) ? "yes" : "no" }')
check "miss: the two lanes overlapped (a1 and b1 ran at the same time, wall $wall s)" [ "$overlap" = yes ]
check "miss: the record seconds is the wall time, not the sum" jq -e '.seconds < ([.jobs[].seconds] | add)' "$res"
check "miss: jobs of a lane ran in order (a2 saw a1's marker)" [ "$(count_of a2)" = 1 ]
check "miss: every job ran once" sh -c "[ \"$(count_of a1)\" = 1 ] && [ \"$(count_of b1)\" = 1 ]"
check "miss: the output lists each job with its log" sh -c "grep -c '^  .* exit 0 ' $tmp/miss.out | grep -q 3"
check "miss: the output names the result file" grep -qF "$res" "$tmp/miss.out"

echo "== hit: the same code tree waits zero =="
: > "$COUNTER"
t0=$(now)
sh scripts/integrate-long.sh m9 > "$tmp/hit.out" 2>&1
code=$?
t1=$(now)
wall=$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f", b - a }')
check "hit: exit 0" [ "$code" -eq 0 ]
check "hit: says HIT with the code tree" grep -q "integrate-long: HIT code tree $tree" "$tmp/hit.out"
check "hit: no job ran" [ ! -s "$COUNTER" ]
check "hit: under 2 s (took $wall s)" awk -v w="$wall" 'BEGIN { exit !(w < 2) }'
check "hit: the result file is unchanged" jq -e '(.jobs | length == 3)' "$res"

echo "== a docs commit keeps the hit, a code commit loses it =="
echo "# more" >> docs/note.md
echo "# readme" > README.md
git add -A
git commit -q -m "docs: a note"
sh scripts/integrate-long.sh m9 > "$tmp/docs.out" 2>&1
check "docs: still a HIT" grep -q "integrate-long: HIT code tree $tree" "$tmp/docs.out"
check "docs: no job ran" [ ! -s "$COUNTER" ]
echo 'fn other() {}' >> crates/a.rs
git add -A
git commit -q -m "feat: code"
sh scripts/integrate-long.sh m9 > "$tmp/code.out" 2>&1
check "code: a MISS, the jobs ran" sh -c "[ \"$(count_of a1)\" = 1 ] && [ \"$(count_of b1)\" = 1 ]"
check "code: the miss names the new code tree" sh -c "! grep -q 'HIT' $tmp/code.out"
tree2=$(sh scripts/gate.sh --key | sed -n 1p)
check "code: a second result file for the second tree" [ -f "reports/integrate-long/$tree2.json" ]
check "code: the first result file is still there" [ -f "$res" ]

echo "== force reruns =="
: > "$COUNTER"
FORCE=1 sh scripts/integrate-long.sh m9 > "$tmp/force.out" 2>&1
check "force: every job ran again" sh -c "[ \"$(count_of a1)\" = 1 ] && [ \"$(count_of b1)\" = 1 ]"
: > "$COUNTER"
sh scripts/integrate-long.sh --force m9 > "$tmp/force2.out" 2>&1
check "force: --force does the same" sh -c "[ \"$(count_of a1)\" = 1 ] && [ \"$(count_of b1)\" = 1 ]"

echo "== red: the worst exit code, the other jobs still run, only the failed job reruns =="
mkrepo red
rm -rf "$TMPDIR_JOB"
mkdir -p "$TMPDIR_JOB"
: > "$COUNTER"
{
  printf 'x%sok1%ssh -c '"'"'echo ok1 >> "$COUNTER"; echo fine'"'"'\n' "$tab" "$tab"
  printf 'x%sbad3%ssh -c '"'"'echo bad3 >> "$COUNTER"; echo "boom line"; if [ -f "$TMPDIR_JOB/fixed" ]; then exit 0; fi; exit 3'"'"'\n' "$tab" "$tab"
  printf 'y%sbad1%ssh -c '"'"'echo bad1 >> "$COUNTER"; if [ -f "$TMPDIR_JOB/fixed" ]; then exit 0; fi; exit 1'"'"'\n' "$tab" "$tab"
  printf 'y%sok2%ssh -c '"'"'echo ok2 >> "$COUNTER"'"'"'\n' "$tab" "$tab"
} > "$jobs_file"
tree=$(sh scripts/gate.sh --key | sed -n 1p)
sh scripts/integrate-long.sh m9 > "$tmp/red.out" 2>&1
code=$?
res="reports/integrate-long/$tree.json"
check "red: exits with the worst exit code (3)" [ "$code" -eq 3 ]
check "red: the record's exit_code is the worst" [ "$(jq -r .exit_code "$res")" = 3 ]
check "red: each job keeps its own exit code" sh -c "[ \"\$(jq -r '[.jobs[] | .exit_code] | join(\",\")' $res)\" = '0,3,1,0' ]"
check "red: a job after a failed job in the same lane still ran" [ "$(count_of ok2)" = 1 ]
check "red: the message carries the tail of a failed job's log" grep -q 'boom line' "$tmp/red.out"
check "red: the message names the worst exit" grep -q 'worst exit code 3' "$tmp/red.out"
: > "$COUNTER"
touch "$TMPDIR_JOB/fixed"
sh scripts/integrate-long.sh m9 > "$tmp/red2.out" 2>&1
code=$?
check "red rerun: exit 0 once the failed jobs pass" [ "$code" -eq 0 ]
check "red rerun: only the two failed jobs ran" sh -c "[ \"$(count_of bad3)\" = 1 ] && [ \"$(count_of bad1)\" = 1 ] && [ \"$(count_of ok1)\" = 0 ] && [ \"$(count_of ok2)\" = 0 ]"
check "red rerun: the passed jobs are marked as reused" sh -c "[ \"\$(jq -r '[.jobs[] | .hit] | join(\",\")' $res)\" = 'true,false,false,true' ]"
check "red rerun: the record is green now" [ "$(jq -r .exit_code "$res")" = 0 ]
: > "$COUNTER"
sh scripts/integrate-long.sh m9 > "$tmp/red3.out" 2>&1
check "red then green: the next run is a HIT" grep -q 'integrate-long: HIT' "$tmp/red3.out"

echo "== a changed job set is a different fingerprint =="
: > "$COUNTER"
printf 'x%sok1%ssh -c '"'"'echo ok1 >> "$COUNTER"; echo changed'"'"'\n' "$tab" "$tab" > "$jobs_file"
sh scripts/integrate-long.sh m9 > "$tmp/jobset.out" 2>&1
check "job set: the new command ran" [ "$(count_of ok1)" = 1 ]

echo "== interrupted: no result file =="
mkrepo intr
rm -rf "$TMPDIR_JOB"
mkdir -p "$TMPDIR_JOB"
printf 'x%sslow%ssh -c '"'"'sleep 30'"'"'\n' "$tab" "$tab" > "$jobs_file"
sh scripts/integrate-long.sh m9 > "$tmp/intr.out" 2>&1 &
pid=$!
i=0
while [ ! -d reports/integrate-long ] && [ "$i" -lt 20000 ]; do i=$((i + 1)); done
kill -TERM "$pid" 2> /dev/null
wait "$pid"
code=$?
check "interrupted: exit 143" [ "$code" -eq 143 ]
check "interrupted: no result file written" sh -c "! ls reports/integrate-long/*.json > /dev/null 2>&1"
check "interrupted: the job process is gone" sh -c "! pgrep -f 'sleep 30' > /dev/null"

echo "== arguments =="
sh scripts/integrate-long.sh > "$tmp/noargs.out" 2>&1
check "no milestone: exit 2" [ $? -eq 2 ]
check "no milestone: prints the usage" grep -q '^usage:' "$tmp/noargs.out"
printf 'x%sbroken\n' "$tab" > "$jobs_file"
sh scripts/integrate-long.sh m9 > "$tmp/broken.out" 2>&1
check "broken job file: exit 2" [ $? -eq 2 ]

echo "== ci-wait with a stub gh =="
mkrepo ci
unset INTEGRATE_LONG_JOBS
cat > "$tmp/bin/gh" <<'EOF'
#!/bin/sh
echo "gh $*" >> "$GH_LOG"
case "$1 $2" in
  "run list") cat "$GH_RUNS" ;;
  "run watch") exit "${GH_WATCH_EXIT:-0}" ;;
  *) exit 1 ;;
esac
EOF
chmod +x "$tmp/bin/gh"
GH_LOG="$tmp/gh.log"
GH_RUNS="$tmp/runs.json"
export GH_LOG GH_RUNS
head_sha=$(git rev-parse HEAD)
runs_json() { # runs_json <sha> <status> <conclusion>
  printf '[{"databaseId":11,"headSha":"%s","status":"%s","conclusion":"%s"}]\n' "$1" "$2" "$3" > "$GH_RUNS"
}
CI_FIND_SECONDS=0
export CI_FIND_SECONDS

: > "$GH_LOG"
runs_json "$head_sha" in_progress ""
sh scripts/integrate-long.sh --ci-wait > "$tmp/ci1.out" 2>&1
check "ci: a run of the head in progress is watched and its exit code returned" sh -c "grep -q 'run watch 11 --exit-status' $GH_LOG"
check "ci: the watched run is named in the output" grep -q 'run 11' "$tmp/ci1.out"
check "ci: the listing asks for the branch and the mission workflow" grep -q 'run list --branch main --workflow mission-branch.yml' "$GH_LOG"
GH_WATCH_EXIT=1 sh scripts/integrate-long.sh --ci-wait > "$tmp/ci1b.out" 2>&1
check "ci: a failing watch exits 1" [ $? -eq 1 ]

: > "$GH_LOG"
runs_json "$head_sha" completed success
sh scripts/integrate-long.sh --ci-wait > "$tmp/ci2.out" 2>&1
check "ci: a completed green run exits 0 without a watch" sh -c "[ $? -eq 0 ] || true; ! grep -q 'run watch' $GH_LOG"
sh scripts/integrate-long.sh --ci-wait > /dev/null 2>&1
check "ci: a completed green run exit code" [ $? -eq 0 ]
runs_json "$head_sha" completed failure
sh scripts/integrate-long.sh --ci-wait > "$tmp/ci3.out" 2>&1
check "ci: a completed red run exits 1" [ $? -eq 1 ]
check "ci: the red conclusion is printed" grep -q 'failure' "$tmp/ci3.out"

old_sha=$head_sha
echo "# note two" >> docs/note.md
git add -A
git commit -q -m "docs: after CI ran"
runs_json "$old_sha" completed success
sh scripts/integrate-long.sh --ci-wait > "$tmp/ci4.out" 2>&1
check "ci: a docs-only head uses the run of the same code tree" [ $? -eq 0 ]
check "ci: it says which commit the run proves" grep -q "$old_sha" "$tmp/ci4.out"

echo 'fn changed() {}' >> crates/a.rs
git add -A
git commit -q -m "feat: code after CI ran"
sh scripts/integrate-long.sh --ci-wait > "$tmp/ci5.out" 2>&1
check "ci: a code change with no run of its own exits 3" [ $? -eq 3 ]
check "ci: the message says to push" grep -q 'push' "$tmp/ci5.out"
printf '[]\n' > "$GH_RUNS"
sh scripts/integrate-long.sh --ci-wait > "$tmp/ci6.out" 2>&1
check "ci: no run at all exits 3" [ $? -eq 3 ]

echo "== ci as a job of the default set =="
check "default jobs: coverage, timings and ci are listed by --list" sh -c "sh scripts/integrate-long.sh --list | awk -F'\t' '{print \$2}' | tr '\n' ' ' | grep -q 'coverage .*timing-check .*timing-test .*ci'"
check "default jobs: coverage and timings share the cargo lane, ci is on its own" sh -c "
  [ \"\$(sh scripts/integrate-long.sh --list | awk -F'\t' '\$2==\"coverage\" || \$2==\"timing-check\" {print \$1}' | sort -u)\" = cargo ] &&
  [ \"\$(sh scripts/integrate-long.sh --list | awk -F'\t' '\$2==\"ci\" {print \$1}')\" = network ]"
check "default jobs: coverage comes before the timings on the lane" sh -c "sh scripts/integrate-long.sh --list | awk -F'\t' '\$1==\"cargo\" {print \$2}' | head -n 1 | grep -qx coverage"

if [ "$failures" -ne 0 ]; then
  echo "$failures failure(s)"
  exit 1
fi
echo "integrate-long: all checks passed"
