#!/bin/sh
# Test for scripts/gate.sh (rule 27, VAL-TOOL-001..004): miss, hit, force, key changes and key stability,
# in a scratch git repository so the real worktree and its records are never touched.
set -u
unset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE FORCE FACTORY_SESSION_ID
root=$(cd "$(dirname "$0")/../.." && pwd)
gate="$root/scripts/gate.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0
fail() {
  echo "FAIL: $1"
  failures=$((failures + 1))
}
counter="$tmp/count"
: > "$counter"
runs() { wc -l < "$counter" | tr -d ' '; }

cat > "$tmp/ok.sh" <<EOF
echo run >> "$counter"
echo "compiling noise"
echo "test result: ok. 3 passed; 0 failed; 0 ignored"
EOF
cat > "$tmp/bad.sh" <<EOF
echo run >> "$counter"
echo "test result: FAILED. 0 passed; 1 failed; 0 ignored"
exit 3
EOF
mkdir -p "$tmp/bin1" "$tmp/bin2"
for v in 1 2; do
  printf '#!/bin/sh\necho "rustc 1.99.%s"\n' "$v" > "$tmp/bin$v/rustc"
  printf '#!/bin/sh\necho "cargo 1.99.%s"\n' "$v" > "$tmp/bin$v/cargo"
  chmod +x "$tmp/bin$v/rustc" "$tmp/bin$v/cargo"
done

repo="$tmp/repo"
mkdir -p "$repo/src" "$repo/build-log/2026-10-25" "$repo/reports/gate"
cd "$repo" || exit 1
git init -q .
echo "fn main() {}" > src/main.rs
echo '{"n":1}' > build-log/2026-10-25/s.jsonl
echo '{}' > reports/gate/.keep
git add -A
git -c user.name=t -c user.email=t@t commit -q -m base

# Independent tree computation: the sequence documented in library/tooling.md, plus the `git rm --cached`
# that drops the committed copies of the excluded paths (an exclude pathspec alone keeps HEAD's version
# in the index, so committing a record would change the key).
indep_tree() {
  idx=$(mktemp "$tmp/idx.XXXXXX")
  rm -f "$idx"
  GIT_INDEX_FILE="$idx" sh -c 'git read-tree HEAD && git rm -r -q --cached --ignore-unmatch -- build-log reports/gate && git add -A -- . ":(exclude)build-log" ":(exclude)reports/gate" && git write-tree'
  rm -f "$idx"
}
gate_key() { sh "$gate" --key | sed -n 1p; }
json() { jq -r "$2" "reports/gate/$tree/$1.json"; }
hits_log=reports/gate/hits.log
hit_count() { if [ -f "$hits_log" ]; then wc -l < "$hits_log" | tr -d ' '; else echo 0; fi; }
hit_field() { LC_ALL=C awk -F '\t' -v n="$1" 'END { print $n }' "$hits_log"; }

export PATH="$tmp/bin1:$PATH"
export DROID_SESSION_ID=sess-one

# The hit log is committed with the records: the repository must not ignore it.
git -C "$root" check-ignore -q reports/gate/hits.log && fail "reports/gate/hits.log is gitignored"

# VAL-TOOL-004 (1): the key prints the independent tree.
tree=$(indep_tree)
[ -n "$tree" ] || fail "independent tree is empty"
[ "$(gate_key)" = "$tree" ] || fail "--key differs from the independent tree: $(gate_key) vs $tree"

# VAL-TOOL-001: a miss runs the command and writes every field.
out=$(sh "$gate" unit -- sh "$tmp/ok.sh" 2>&1)
rc=$?
[ "$rc" -eq 0 ] || fail "miss exit $rc"
case "$out" in *"miss $tree"*) ;; *) fail "no miss line naming the tree: $out" ;; esac
case "$out" in *"compiling noise"*) ;; *) fail "miss did not stream the command output: $out" ;; esac
[ "$(runs)" -eq 1 ] || fail "miss ran the command $(runs) times"
rec="reports/gate/$tree/unit.json"
[ -f "$rec" ] || fail "no record at $rec"
for f in name tree tool command exit_code summary seconds started_at finished_at session session_source; do
  v=$(jq -r ".$f // empty" "$rec" 2>/dev/null)
  [ -n "$v" ] || [ "$f" = exit_code ] || fail "record field $f missing or empty"
done
[ "$(json unit .exit_code)" = "0" ] || fail "stored exit_code: $(json unit .exit_code)"
[ "$(json unit .tree)" = "$tree" ] || fail "stored tree"
[ "$(json unit .session)" = "sess-one" ] || fail "stored session: $(json unit .session)"
[ "$(json unit .session_source)" = "DROID_SESSION_ID" ] || fail "session source: $(json unit .session_source)"
case "$(json unit .summary)" in *"test result: ok. 3 passed"*) ;; *) fail "summary: $(json unit .summary)" ;; esac
case "$(json unit .summary)" in *"compiling noise"*) fail "summary kept noise" ;; esac
case "$(json unit .finished_at)" in
  [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9]Z) ;;
  *) fail "finished_at is not RFC 3339 UTC: $(json unit .finished_at)" ;;
esac
first_finished=$(json unit .finished_at)
[ "$(hit_count)" -eq 0 ] || fail "a miss wrote to hits.log"
# The records are committed: no field may look like an API key to gitleaks.
if command -v gitleaks > /dev/null 2>&1; then
  gitleaks detect --no-git --no-banner --source reports/gate > "$tmp/leaks.txt" 2>&1 || fail "gitleaks flags the record: $(tail -n 3 "$tmp/leaks.txt")"
fi

# VAL-TOOL-002: a hit replays the verdict without running, under 5 s.
t0=$(date +%s)
out=$(sh "$gate" unit -- sh "$tmp/ok.sh" 2>&1)
rc=$?
t1=$(date +%s)
[ "$rc" -eq 0 ] || fail "hit exit $rc"
[ "$(runs)" -eq 1 ] || fail "hit ran the command"
[ "$(printf '%s\n' "$out" | sed -n 1p)" = "cached, proven at $first_finished by sess-one" ] \
  || fail "hit line: $(printf '%s\n' "$out" | sed -n 1p)"
case "$out" in *"test result: ok. 3 passed"*) ;; *) fail "hit lacks the stored summary: $out" ;; esac
case "$out" in *"compiling noise"*) fail "hit printed command output" ;; esac
[ $((t1 - t0)) -lt 5 ] || fail "hit took $((t1 - t0)) s"
[ "$(json unit .finished_at)" = "$first_finished" ] || fail "hit rewrote the record"

# The hit log: one tab-separated line per hit (time, name, tree, stored seconds, session, session_source).
[ "$(hit_count)" -eq 1 ] || fail "a hit left $(hit_count) hits.log lines, expected 1"
[ "$(LC_ALL=C awk -F '\t' 'END { print NF }' "$hits_log")" = "6" ] || fail "hits.log line is not 6 tab-separated fields: $(tail -n 1 "$hits_log")"
case "$(hit_field 1)" in
  [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9]Z) ;;
  *) fail "hits.log time is not RFC 3339 UTC: $(hit_field 1)" ;;
esac
[ "$(hit_field 2)" = "unit" ] || fail "hits.log name: $(hit_field 2)"
[ "$(hit_field 3)" = "$tree" ] || fail "hits.log tree: $(hit_field 3)"
[ "$(hit_field 4)" = "$(json unit .seconds)" ] || fail "hits.log stored seconds: $(hit_field 4) vs $(json unit .seconds)"
[ "$(hit_field 5)" = "sess-one" ] || fail "hits.log session: $(hit_field 5)"
[ "$(hit_field 6)" = "DROID_SESSION_ID" ] || fail "hits.log session_source: $(hit_field 6)"

# A failing command stores its non-zero code and the hit replays it.
sh "$gate" bad -- sh "$tmp/bad.sh" > "$tmp/bad.out" 2>&1
rc=$?
[ "$rc" -eq 3 ] || fail "failing miss exit $rc"
[ "$(json bad .exit_code)" = "3" ] || fail "failing record exit_code $(json bad .exit_code)"
case "$(json bad .summary)" in *"test result: FAILED"*) ;; *) fail "failing summary: $(json bad .summary)" ;; esac
[ "$(hit_count)" -eq 1 ] || fail "a failing miss wrote to hits.log"
before=$(runs)
sh "$gate" bad -- sh "$tmp/bad.sh" > "$tmp/bad.hit" 2>&1
rc=$?
[ "$(hit_count)" -eq 2 ] || fail "a failing hit did not log: $(hit_count) lines"
[ "$(hit_field 2)" = "bad" ] || fail "failing hit name in hits.log: $(hit_field 2)"
[ "$rc" -eq 3 ] || fail "failing hit exit $rc"
[ "$(runs)" -eq "$before" ] || fail "failing hit ran the command"
grep -q "^cached, proven at " "$tmp/bad.hit" || fail "failing hit lacks the cached line"

# Second worktree at the same content: the committed record is a hit with the first session.
git add -A
git -c user.name=t -c user.email=t@t commit -q -m records
git worktree add -q "$tmp/wt2" -b wt2 2>/dev/null
before=$(runs)
out=$(cd "$tmp/wt2" && DROID_SESSION_ID=sess-other sh "$gate" unit -- sh "$tmp/ok.sh" 2>&1)
rc=$?
[ "$rc" -eq 0 ] || fail "second worktree hit exit $rc"
[ "$(runs)" -eq "$before" ] || fail "second worktree ran the command"
[ "$(printf '%s\n' "$out" | sed -n 1p)" = "cached, proven at $first_finished by sess-one" ] \
  || fail "second worktree hit line: $(printf '%s\n' "$out" | sed -n 1p)"
[ "$(cd "$tmp/wt2" && LC_ALL=C awk -F '\t' 'END { print $2 "|" $5 }' reports/gate/hits.log)" = "unit|sess-other" ] \
  || fail "second worktree hits.log line lacks the hitting session"

# VAL-TOOL-003: --force and FORCE=1 rerun and replace the record.
sleep 1
before=$(runs)
hits_before=$(hit_count)
out=$(DROID_SESSION_ID=sess-two sh "$gate" --force unit -- sh "$tmp/ok.sh" 2>&1)
rc=$?
[ "$rc" -eq 0 ] || fail "forced exit $rc"
[ "$(runs)" -eq $((before + 1)) ] || fail "--force did not run the command"
case "$out" in *"forced $tree"*) ;; *) fail "no forced line: $out" ;; esac
[ "$(json unit .session)" = "sess-two" ] || fail "forced session: $(json unit .session)"
[ "$(hit_count)" -eq "$hits_before" ] || fail "--force wrote to hits.log"
second_finished=$(json unit .finished_at)
[ "$second_finished" != "$first_finished" ] || fail "forced finished_at did not advance"
before=$(runs)
out=$(sh "$gate" unit -- sh "$tmp/ok.sh" 2>&1)
[ "$(runs)" -eq "$before" ] || fail "hit after force ran the command"
[ "$(printf '%s\n' "$out" | sed -n 1p)" = "cached, proven at $second_finished by sess-two" ] \
  || fail "hit after force: $(printf '%s\n' "$out" | sed -n 1p)"
sleep 1
before=$(runs)
hits_before=$(hit_count)
FORCE=1 sh "$gate" unit -- sh "$tmp/ok.sh" > /dev/null 2>&1
[ "$(hit_count)" -eq "$hits_before" ] || fail "FORCE=1 wrote to hits.log"
[ "$(runs)" -eq $((before + 1)) ] || fail "FORCE=1 did not run the command"
[ "$(json unit .finished_at)" != "$second_finished" ] || fail "FORCE=1 kept the old record"

# VAL-TOOL-004 (2): build-log/ and reports/gate/ writes leave the key and the hit alone.
echo '{"n":2}' >> build-log/2026-10-25/s.jsonl
echo '{}' > reports/gate/zz-probe.json
[ "$(gate_key)" = "$tree" ] || fail "build-log or reports/gate write changed the key"
before=$(runs)
sh "$gate" unit -- sh "$tmp/ok.sh" > /dev/null 2>&1
[ "$(runs)" -eq "$before" ] || fail "build-log write turned the hit into a miss"

# (3) a tracked-file edit changes the key and misses; a new untracked file does too.
echo "// edit" >> src/main.rs
edited=$(gate_key)
[ "$edited" != "$tree" ] || fail "tracked edit did not change the key"
[ "$edited" = "$(indep_tree)" ] || fail "edited key differs from the independent tree"
before=$(runs)
sh "$gate" unit -- sh "$tmp/ok.sh" > "$tmp/edit.out" 2>&1
[ "$(runs)" -eq $((before + 1)) ] || fail "tracked edit was not a miss"
grep -q "miss $edited" "$tmp/edit.out" || fail "miss line lacks the edited tree"
[ -f "reports/gate/$edited/unit.json" ] || fail "no record under the edited tree"

# (4) reverting restores the key and the hit.
git checkout -q -- src/main.rs
[ "$(gate_key)" = "$tree" ] || fail "revert did not restore the key"
before=$(runs)
sh "$gate" unit -- sh "$tmp/ok.sh" > /dev/null 2>&1
[ "$(runs)" -eq "$before" ] || fail "revert was not a hit"

# The key also carries the command and the tool.
before=$(runs)
sh "$gate" unit -- sh "$tmp/ok.sh" extra > /dev/null 2>&1
[ "$(runs)" -eq $((before + 1)) ] || fail "a changed command was not a miss"
[ "$(json unit .command)" = "sh $tmp/ok.sh extra" ] || fail "stored command: $(json unit .command)"
sh "$gate" test -- sh "$tmp/ok.sh" > /dev/null 2>&1
case "$(json test .tool)" in "rustc 1.99.1 | cargo 1.99.1") ;; *) fail "stored tool: $(json test .tool)" ;; esac
before=$(runs)
PATH="$tmp/bin2:$PATH" sh "$gate" test -- sh "$tmp/ok.sh" > /dev/null 2>&1
[ "$(runs)" -eq $((before + 1)) ] || fail "a changed toolchain was not a miss"
case "$(json test .tool)" in "rustc 1.99.2 | cargo 1.99.2") ;; *) fail "stored tool after bump: $(json test .tool)" ;; esac

# Session sources: DROID_SESSION_ID, then FACTORY_SESSION_ID (what the Droid shell exports), then the newest
# settings file by mtime, then unknown.
(unset DROID_SESSION_ID; FACTORY_SESSION_ID=fac-id sh "$gate" factory -- sh "$tmp/ok.sh" > /dev/null 2>&1)
[ "$(json factory .session)" = "fac-id" ] || fail "FACTORY_SESSION_ID session: $(json factory .session)"
[ "$(json factory .session_source)" = "FACTORY_SESSION_ID" ] || fail "FACTORY_SESSION_ID source: $(json factory .session_source)"
mkdir -p "$tmp/sessions/proj"
: > "$tmp/sessions/proj/old-id.settings.json"
: > "$tmp/sessions/proj/new-id.settings.json"
touch -t 202601010000 "$tmp/sessions/proj/old-id.settings.json"
touch -t 202602010000 "$tmp/sessions/proj/new-id.settings.json"
(unset DROID_SESSION_ID; GATE_SESSIONS_DIR="$tmp/sessions" sh "$gate" fallback -- sh "$tmp/ok.sh" > /dev/null 2>&1)
[ "$(json fallback .session)" = "new-id" ] || fail "fallback session: $(json fallback .session)"
[ "$(json fallback .session_source)" = "newest-settings-mtime" ] || fail "fallback source: $(json fallback .session_source)"
(unset DROID_SESSION_ID; GATE_SESSIONS_DIR="$tmp/none" sh "$gate" nosession -- sh "$tmp/ok.sh" > /dev/null 2>&1)
[ "$(json nosession .session)" = "unknown" ] || fail "no-session value: $(json nosession .session)"

# An interrupted run is not a verdict: HUP and TERM kill the command, exit 129/143 and leave no record, so a later run
# is a miss and not a replayed failure. The handler must also fire while the command runs, not after it ends.
cat > "$tmp/slow.sh" <<EOS
echo "\$\$" > "$tmp/slow.pid"
echo "slow started"
sleep 20
echo "slow finished"
EOS
for sig in TERM HUP; do
  case "$sig" in TERM) want=143 ;; HUP) want=129 ;; esac
  rm -f "$tmp/slow.pid" "$tmp/slow.out"
  sig_tree=$(gate_key)
  mkdir -p "$tmp/gt"
  TMPDIR="$tmp/gt" sh "$gate" "slow-$sig" -- sh "$tmp/slow.sh" > "$tmp/slow.out" 2>&1 &
  gpid=$!
  n=0
  while [ ! -s "$tmp/slow.pid" ] && [ "$n" -lt 50 ]; do sleep 0.1; n=$((n + 1)); done
  [ -s "$tmp/slow.pid" ] || fail "$sig: the slow command never started"
  kill "-$sig" "$gpid" 2> /dev/null
  n=0
  while kill -0 "$gpid" 2> /dev/null && [ "$n" -lt 50 ]; do sleep 0.1; n=$((n + 1)); done
  if kill -0 "$gpid" 2> /dev/null; then
    fail "$sig: gate.sh still running 5 s after the signal"
    pkill -P "$gpid" 2> /dev/null
    kill -9 "$gpid" 2> /dev/null
  fi
  wait "$gpid" 2> /dev/null
  rc=$?
  [ "$rc" -eq "$want" ] || fail "$sig: interrupted run exit $rc, expected $want"
  [ ! -e "reports/gate/$sig_tree/slow-$sig.json" ] || fail "$sig: interrupted run left a record"
  if [ -s "$tmp/slow.pid" ] && kill -0 "$(cat "$tmp/slow.pid")" 2> /dev/null; then
    fail "$sig: the command outlived the interrupted gate"
    kill "$(cat "$tmp/slow.pid")" 2> /dev/null
  fi
  [ -z "$(ls -A "$tmp/gt")" ] || fail "$sig: temp files left behind: $(ls "$tmp/gt")"
done

# Argument errors.
sh "$gate" > /dev/null 2>&1 && fail "no arguments accepted"
sh "$gate" unit > /dev/null 2>&1 && fail "missing command accepted"

if [ "$failures" -ne 0 ]; then
  echo "$failures failure(s)"
  exit 1
fi
echo "gate: ok"
