#!/bin/sh
# Test for scripts/integrate.sh (owner item 19, VAL-TOOL-006) on throwaway repositories: a no-ff merge with a
# templated message and a parsed trailer, the gate as a cache hit or a real run, red gate abort, staging files merged
# into SPEC section 9, BACKLOG and decision addenda and removed, a refusal on a dirty tree.
set -u
unset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE FORCE FACTORY_SESSION_ID DROID_SESSION_ID
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

mkdir -p "$tmp/bin"
printf '#!/bin/sh\necho "rustc 1.99.0"\n' > "$tmp/bin/rustc"
printf '#!/bin/sh\necho "cargo 1.99.0"\n' > "$tmp/bin/cargo"
chmod +x "$tmp/bin/rustc" "$tmp/bin/cargo"
PATH="$tmp/bin:$PATH"
export PATH
COUNTER="$tmp/count"
export COUNTER
: > "$COUNTER"
runs() { wc -l < "$COUNTER" | tr -d ' '; }
INTEGRATE_DATE=2026-10-25
export INTEGRATE_DATE

# A repository with a stub Makefile whose `check` is a gate that counts its runs and fails when red.flag exists.
mkrepo() { # mkrepo <name>; leaves the current directory in the repository, on main
  repo="$tmp/$1"
  mkdir -p "$repo/scripts" "$repo/reports/gate" "$repo/docs/decisions" "$repo/build-log"
  cd "$repo" || exit 1
  git init -q -b main .
  git config user.name tester
  git config user.email tester@example.invalid
  git config commit.gpgsign false
  cp "$root/scripts/gate.sh" "$root/scripts/gen-decisions-index.sh" scripts/
  cp "$root"/scripts/integrate* scripts/ 2> /dev/null
  [ -d "$root/scripts/templates" ] && cp -R "$root/scripts/templates" scripts/templates
  cp "$root/.git-trailers" .git-trailers
  : > reports/gate/.keep
  echo '{"n":1}' > build-log/log.jsonl
  printf 'GATE := sh scripts/gate.sh\n\ncheck:\n\t@$(GATE) check -- sh -c '"'"'echo run >> "$$COUNTER"; if [ -f red.flag ]; then echo "test result: FAILED. 0 passed; 1 failed"; exit 1; fi; echo "test result: ok. 2 passed; 0 failed"'"'"'\n\nusage:\n\t@mkdir -p reports/usage && echo "usage table for $(M)" > reports/usage/$(M).md\n\ndecisions-index:\n\t@sh scripts/gen-decisions-index.sh > docs/decisions/README.md\n' > Makefile
  cat > SPEC.md <<'EOF'
# Spec

## 9. Changelog and human feedback

| Id | Date | Feedback | Applies | Status |
|---|---|---|---|---|
| F-001 | 2026-10-22 | First row | all | done |
| F-002 | 2026-10-23 | Second row | all | done |

## 10. Evolution log

Nothing yet.
EOF
  cat > BACKLOG.md <<'EOF'
# Backlog

| Id | Item | Reason deferred | Earliest |
|---|---|---|---|
| B-001 | First item | A reason. | M3 |
| B-002 | Second item | Another reason. | M4 |
EOF
  for n in 1 2; do
    printf '# D-00%s: Decision %s\n\n**Date:** 2026-10-2%s\n**Status:** accepted\n\n## Context\n\nText.\n' "$n" "$n" "$n" > "docs/decisions/D-00$n-decision-$n.md"
  done
  sh scripts/gen-decisions-index.sh > docs/decisions/README.md
  git add -A
  git commit -q -m base
}

write_staging() { # write_staging <feature> <first row text>
  mkdir -p reports/staging
  cat > "reports/staging/$1.md" <<EOF
# Staging: $1

## SPEC §9

- Row (F-NEW): $2 with a pipe | inside.
  The row continues on a second line.
- Row (F-NEW): the second row of $1.

## Decisions

- D-002 (amend, one line in Consequences): "The thing is amended by $1."
- D-003 is new and is written by another feature.

## BACKLOG

- B-NEW: split the long function; reason: it holds a hundred lines.
- Closed (do not file): a row already done.

## Deferred clauses

- VAL-X-001: the live check of $1 was not run.

## Library

- tooling: a library line from $1.
EOF
}

# A worktree branch that was validated: its tree has a gate record committed.
mkbranch() { # mkbranch <branch> <feature> [red]
  git checkout -q -b "$1"
  mkdir -p src
  echo "fn feature() {}" > "src/$2.rs"
  [ "${3:-}" = red ] && echo red > red.flag
  write_staging "$2" "the $2 adds a thing"
  mkdir -p reports/staging
  printf '# Staging: %s-none\n\n## SPEC §9\n\nNone.\n\n## BACKLOG\n\nNone.\n' "$2" > "reports/staging/$2-none.md"
  git add -A
  git commit -q -m "feat: $2"
  git commit -q --allow-empty -m "docs: second commit on $1"
}
validate() { # run the gate on the branch and commit the record
  make -s check > /dev/null 2>&1
  git add -A reports/gate
  git commit -q -m "chore: gate record"
}
trailer_ok() { # trailer_ok <commit>: the trailer parses and ends the message
  v=$(git log -1 --format='%(trailers:key=Co-authored-by,valueonly)' "$1" | sed '/^$/d')
  last=$(git log -1 --format=%B "$1" | sed '/^$/d' | tail -n 1)
  [ "$v" = "$(cut -d' ' -f2- .git-trailers)" ] && [ "$last" = "$(cat .git-trailers)" ]
}
subject_short() { [ "$(git log -1 --format=%s "$1" | wc -c | tr -d ' ')" -le 73 ]; }

echo "== hit: the merged tree is the validated tree =="
mkrepo hit
mkbranch wt/m9 m9-feat
validate
git checkout -q main
before=$(git rev-parse HEAD)
echo 'dirty build log' >> build-log/log.jsonl
runs_before=$(runs)
sh scripts/integrate.sh m9 wt/m9 > "$tmp/hit.out" 2>&1
code=$?
cp "$tmp/hit.out" "$tmp/hit.keep"
check "hit: exit 0 (see $tmp/hit.out)" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/hit.out"
merge=$(git log --merges --format=%H "$before..HEAD" | tail -n 1)
check "hit: one merge commit with two parents" [ "$(git log --merges --format=%p "$before..HEAD" | wc -l | tr -d ' ')" = 1 ] 
check "hit: merge parents are 2" [ "$(git log -1 --format=%p "$merge" | wc -w | tr -d ' ')" = 2 ]
check "hit: merge subject names milestone and branch" [ "$(git log -1 --format=%s "$merge")" = "Integrate m9: wt/m9" ]
check "hit: merge subject is short" subject_short "$merge"
check "hit: merge trailer parses and ends the message" trailer_ok "$merge"
check "hit: merge body lists the merged subjects" sh -c "git log -1 --format=%B $merge | grep -qF 'feat: m9-feat'"
check "hit: gate printed the cache hit" grep -q '^cached, proven at ' "$tmp/hit.out"
check "hit: summary says gate HIT" grep -q 'gate: HIT' "$tmp/hit.out"
check "hit: the gate command did not run again" [ "$(runs)" = "$((runs_before))" ]
boundary=$(git rev-parse HEAD)
check "hit: boundary commit follows the merge" [ "$boundary" != "$merge" ]
check "hit: boundary subject" [ "$(git log -1 --format=%s HEAD)" = "chore(integration): m9 boundary reports and shared files" ]
check "hit: boundary trailer parses" trailer_ok HEAD
check "hit: SPEC row F-003 appended" grep -qF '| F-003 | 2026-10-25 | the m9-feat adds a thing with a pipe \| inside. The row continues on a second line. |' SPEC.md
check "hit: SPEC row F-004 appended in order" grep -qF '| F-004 | 2026-10-25 | the second row of m9-feat. |' SPEC.md
check "hit: SPEC rows stay in the section 9 table" sh -c "awk '/^## 10/{exit} /^\| F-004 /{f=1} END{exit !f}' SPEC.md"
check "hit: BACKLOG row B-003 appended" grep -qF '| B-003 | split the long function | it holds a hundred lines' BACKLOG.md
check "hit: closed BACKLOG bullet is not filed" sh -c "! grep -qF 'a row already done' BACKLOG.md"
check "hit: decision amendment appended to D-002" grep -qF 'The thing is amended by m9-feat.' docs/decisions/D-002-decision-2.md
check "hit: decision amendment has an addendum heading" grep -q '^## Addendum 2026-10-25 (m9 integration, m9-feat)' docs/decisions/D-002-decision-2.md
check "hit: new-decision bullet is not applied" sh -c "! grep -qF 'written by another feature' docs/decisions/D-001-decision-1.md docs/decisions/D-002-decision-2.md"
check "hit: staging files removed" [ -z "$(git ls-tree HEAD reports/staging/)" ]
check "hit: tree clean except build-log" [ -z "$(git status --porcelain -- . ':(exclude)build-log')" ]
check "hit: usage report committed" git cat-file -e HEAD:reports/usage/m9.md
check "hit: decisions index regenerated" git cat-file -e HEAD:docs/decisions/README.md
report=reports/readiness/2026-10-25-m9.md
check "hit: boundary report committed" git cat-file -e "HEAD:$report"
check "hit: report lists the deferred clause" grep -qF 'VAL-X-001: the live check of m9-feat was not run.' "$report"
check "hit: report copies the gate numbers from the record" grep -q 'test result: ok. 2 passed' "$report"
check "hit: report says the gate was a hit" grep -q 'cache hit' "$report"
check "hit: report keeps the library line" grep -qF 'a library line from m9-feat' "$report"
check "hit: summary names the boundary report" grep -qF "$report" "$tmp/hit.out"
check "hit: summary has the boundary heading" grep -q 'Boundary summary: m9' "$tmp/hit.out"
check "hit: gate record for the merged tree is committed" sh -c "git ls-tree -r --name-only HEAD reports/gate | grep -q 'check.json'"

echo "== miss: the mission branch moved after validation =="
mkrepo miss
mkbranch wt/m9 m9-feat
validate
git checkout -q main
echo moved > moved.txt
git add -A
git commit -q -m "chore: mission branch moved"
runs_before=$(runs)
sh scripts/integrate.sh m9 wt/m9 > "$tmp/miss.out" 2>&1
code=$?
check "miss: exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/miss.out"
check "miss: summary says gate MISS" grep -q 'gate: MISS' "$tmp/miss.out"
check "miss: reason shows the diff stat" grep -q 'moved.txt' "$tmp/miss.out"
check "miss: the gate command ran once" [ "$(runs)" = "$((runs_before + 1))" ]
check "miss: report says the gate ran" grep -q 'real run' reports/readiness/2026-10-25-m9.md

echo "== red: the final gate fails =="
mkrepo red
mkbranch wt/m9 m9-feat red
git checkout -q main
runs_before=$(runs)
sh scripts/integrate.sh m9 wt/m9 > "$tmp/red.out" 2>&1
code=$?
check "red: exit non-zero" [ "$code" -ne 0 ]
check "red: summary says gate RED with the failure line" sh -c "grep -q 'gate: RED' $tmp/red.out && grep -q 'test result: FAILED' $tmp/red.out"
check "red: the gate ran exactly once, no rerun" [ "$(runs)" = "$((runs_before + 1))" ]
check "red: the merge stays, forward only" [ "$(git log -1 --format=%s)" = "Integrate m9: wt/m9" ]
check "red: staging files are not merged yet" [ -f reports/staging/m9-feat.md ]
check "red: no boundary commit" [ -z "$(git log --format=%s | grep 'boundary reports')" ]

echo "== resume: a rerun after the fix skips the merge =="
rm -f red.flag
git add -A
git commit -q -m "fix: remove the red flag"
sh scripts/integrate.sh m9 wt/m9 > "$tmp/resume.out" 2>&1
code=$?
check "resume: exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/resume.out"
check "resume: reports the branch as already merged" grep -q 'already merged' "$tmp/resume.out"
check "resume: exactly one merge commit" [ "$(git log --merges --format=%h | wc -l | tr -d ' ')" = 1 ]
check "resume: staging files merged and removed" [ -z "$(git ls-tree HEAD reports/staging/)" ]
check "resume: SPEC row appended" grep -qF '| F-003 |' SPEC.md

echo "== dirty: a modified tracked file refuses =="
mkrepo dirty
mkbranch wt/m9 m9-feat
validate
git checkout -q main
before=$(git rev-parse HEAD)
echo edit >> SPEC.md
sh scripts/integrate.sh m9 wt/m9 > "$tmp/dirty.out" 2>&1
code=$?
check "dirty: exit non-zero" [ "$code" -ne 0 ]
check "dirty: names the unclean tree" grep -q 'not clean' "$tmp/dirty.out"
check "dirty: nothing merged" [ "$(git rev-parse HEAD)" = "$before" ]

echo "== arguments: unknown branch and missing milestone =="
git checkout -q -- SPEC.md
sh scripts/integrate.sh m9 wt/nope > "$tmp/nope.out" 2>&1
check "unknown branch: exit non-zero" [ $? -ne 0 ]
check "unknown branch: says so" grep -q 'wt/nope' "$tmp/nope.out"
sh scripts/integrate.sh > "$tmp/noargs.out" 2>&1
check "no arguments: exit 2" [ $? -eq 2 ]

echo
if [ "$failures" -eq 0 ]; then
  echo "integrate_test: all checks passed"
  exit 0
fi
echo "integrate_test: $failures check(s) failed"
exit 1
