#!/bin/sh
# Test for scripts/integrate.sh (owner item 19, VAL-TOOL-006) on throwaway repositories: a no-ff merge with a
# templated message and a parsed trailer, the gate as a cache hit or a real run, red gate abort, staging files merged
# into SPEC section 9, BACKLOG and decision addenda and removed, a refusal on a dirty tree.
set -u
unset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE FORCE FACTORY_SESSION_ID DROID_SESSION_ID INTEGRATE_LONG INTEGRATE_LONG_JOBS
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

echo "== code paths: docs commits on the mission branch keep the hit, the miss lists code files only =="
mkrepo docshit
mkbranch wt/m9 m9-feat
validate
git checkout -q main
mkdir -p docs/notes
echo "# a note" > docs/notes/note.md
echo "# readme" > README.md
git add -A
git commit -q -m "docs: a note and a readme after validation"
runs_before=$(runs)
sh scripts/integrate.sh m9 wt/m9 > "$tmp/docshit.out" 2>&1
code=$?
check "docs hit: exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/docshit.out"
check "docs hit: summary says gate HIT" grep -q 'gate: HIT' "$tmp/docshit.out"
check "docs hit: the gate printed HIT with the code tree" grep -q 'gate check: HIT code tree' "$tmp/docshit.out"
check "docs hit: the gate command did not run" [ "$(runs)" = "$((runs_before))" ]

mkrepo codemiss
mkbranch wt/m9 m9-feat
validate
git checkout -q main
mkdir -p docs/notes
echo "# a note" > docs/notes/note.md
echo moved > moved.txt
git add -A
git commit -q -m "chore: code and docs moved after validation"
sh scripts/integrate.sh m9 wt/m9 > "$tmp/codemiss.out" 2>&1
code=$?
check "code miss: exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/codemiss.out"
check "code miss: the gate printed MISS" grep -q 'gate check: MISS' "$tmp/codemiss.out"
summary_part=$(sed -n '/Boundary summary/,$p' "$tmp/codemiss.out")
check "code miss: the summary reason names the code file" sh -c "printf '%s' '$summary_part' | grep -q moved.txt"
check "code miss: the summary reason does not name the docs file" sh -c "! printf '%s' '$summary_part' | grep -q note.md"

echo "== inner hit, outer miss: only the lint gate was proven =="
mkrepo inner
printf 'GATE := sh scripts/gate.sh\n\ncheck:\n\t@$(GATE) check -- sh -c '"'"'make --no-print-directory lint; echo run >> "$$COUNTER"; echo "test result: ok. 2 passed; 0 failed"'"'"'\n\nlint:\n\t@$(GATE) lint -- sh -c '"'"'echo "lint ran"; echo "test result: ok. 1 passed; 0 failed"'"'"'\n\nusage:\n\t@mkdir -p reports/usage && echo "usage table for $(M)" > reports/usage/$(M).md\n\ndecisions-index:\n\t@sh scripts/gen-decisions-index.sh > docs/decisions/README.md\n' > Makefile
git add -A
git commit -q -m "chore: check runs an inner lint gate"
mkbranch wt/m9 m9-feat
make -s lint > /dev/null 2>&1
git add -A reports/gate
git commit -q -m "chore: lint record only"
git checkout -q main
runs_before=$(runs)
sh scripts/integrate.sh m9 wt/m9 > "$tmp/inner.out" 2>&1
code=$?
check "inner: exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/inner.out"
check "inner: the inner lint gate was served from cache" grep -q '^cached, proven at ' "$tmp/inner.out"
check "inner: the check gate itself ran" [ "$(runs)" = "$((runs_before + 1))" ]
check "inner: summary says gate MISS" grep -q 'gate: MISS' "$tmp/inner.out"
check "inner: summary does not say gate HIT" sh -c "! grep -q 'gate: HIT' $tmp/inner.out"
check "inner: the miss carries a reason" grep -q 'reason: ' "$tmp/inner.out"
check "inner: report says the gate ran" grep -q 'real run' reports/readiness/2026-10-25-m9.md

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

echo "== long work: the result file of integrate-long is part of the boundary =="
tab=$(printf '\t')
long_jobs="$tmp/long-jobs.tsv"
printf 'cargo%slongjob%ssh -c '"'"'echo long >> "$COUNTER"; echo "long output"'"'"'\n' "$tab" "$tab" > "$long_jobs"
mkrepo longnone
mkbranch wt/m9 m9-feat
validate
git checkout -q main
sh scripts/integrate.sh m9 wt/m9 > "$tmp/longnone.out" 2>&1
code=$?
check "long none: exit 0" [ "$code" -eq 0 ]
check "long none: the summary says the long work was not run and names the target" sh -c "grep -q 'long work: not run' $tmp/longnone.out && grep -q 'make integrate-long M=m9' $tmp/longnone.out"
check "long none: the report has a Long work section saying pending" sh -c "grep -q '^## Long work' reports/readiness/2026-10-25-m9.md && grep -q 'pending' reports/readiness/2026-10-25-m9.md"

mkrepo longrun
mkbranch wt/m9 m9-feat
validate
git checkout -q main
: > "$COUNTER"
INTEGRATE_LONG=1 INTEGRATE_LONG_JOBS="$long_jobs" sh scripts/integrate.sh m9 wt/m9 > "$tmp/longrun.out" 2>&1
code=$?
check "long run: exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/longrun.out"
check "long run: the job ran once" [ "$(grep -c '^long$' "$COUNTER" | tr -d ' ')" = 1 ]
long_tree=$(sh scripts/gate.sh --key | sed -n 1p)
check "long run: the result file is committed with the boundary" git cat-file -e "HEAD:reports/integrate-long/$long_tree.json"
check "long run: the summary reports the worst exit code" grep -q 'long work: OK (worst exit code 0)' "$tmp/longrun.out"
check "long run: the report lists the job" sh -c "grep -q '^## Long work' reports/readiness/2026-10-25-m9.md && grep -q 'longjob' reports/readiness/2026-10-25-m9.md"

echo "== long work: a result written by a separate run is read on the next integrate run =="
mkrepo longlater
mkbranch wt/m9 m9-feat
validate
git checkout -q main
sh scripts/integrate.sh m9 wt/m9 > /dev/null 2>&1
: > "$COUNTER"
INTEGRATE_LONG_JOBS="$long_jobs" sh scripts/integrate-long.sh m9 > "$tmp/longlater-a.out" 2>&1
check "long later: integrate-long exit 0" [ $? -eq 0 ]
check "long later: the untracked result does not make the tree unclean for integrate" sh -c "sh scripts/integrate.sh m9 wt/m9 > $tmp/longlater-b.out 2>&1"
check "long later: the gate was a hit on the rerun" grep -q 'gate: HIT' "$tmp/longlater-b.out"
check "long later: the summary reads the record" grep -q 'long work: OK' "$tmp/longlater-b.out"
check "long later: the report lists the job after the refresh" grep -q 'longjob' reports/readiness/2026-10-25-m9.md
check "long later: the refresh commits the result file" sh -c "git ls-tree -r --name-only HEAD reports/integrate-long | grep -q '.json'"
check "long later: the rerun left a clean tree" [ -z "$(git status --porcelain -- . ':(exclude)build-log')" ]

echo "== long work: a red job is reported and fails the target after the boundary is written =="
mkrepo longred
mkbranch wt/m9 m9-feat
validate
git checkout -q main
printf 'cargo%sbadjob%ssh -c '"'"'echo "cov too low"; exit 4'"'"'\n' "$tab" "$tab" > "$tmp/long-red.tsv"
INTEGRATE_LONG=1 INTEGRATE_LONG_JOBS="$tmp/long-red.tsv" sh scripts/integrate.sh m9 wt/m9 > "$tmp/longred.out" 2>&1
code=$?
check "long red: the target exits non-zero" [ "$code" -ne 0 ]
check "long red: the summary says RED with the worst exit code" grep -q 'long work: RED (worst exit code 4)' "$tmp/longred.out"
check "long red: the boundary commit was still written" [ "$(git log -1 --format=%s)" = "chore(integration): m9 boundary reports and shared files" ]
check "long red: the report names the red job" grep -q 'badjob' reports/readiness/2026-10-25-m9.md

echo "== arguments: unknown branch and missing milestone =="
git checkout -q -- SPEC.md
sh scripts/integrate.sh m9 wt/nope > "$tmp/nope.out" 2>&1
check "unknown branch: exit non-zero" [ $? -ne 0 ]
check "unknown branch: says so" grep -q 'wt/nope' "$tmp/nope.out"
sh scripts/integrate.sh > "$tmp/noargs.out" 2>&1
check "no arguments: exit 2" [ $? -eq 2 ]

echo "== staging shapes: every bullet and table row is filed, a dry run writes a copy =="
mkrepo shapes
mkdir -p reports/staging
cat > reports/staging/shape-bullets.md <<'EOF'
# Staging: shape-bullets

## SPEC §9

- Row (F-NEW): bullets row one.

## Decisions

- D-001 (amend, one line): "Amended by shape-bullets."

## BACKLOG

- B-NEW: bullets item; reason: bullets reason.

## Deferred clauses

- VAL-S-001: bullets deferred.

## Library

- lib: bullets library.
EOF
cat > reports/staging/shape-none.md <<'EOF'
# Staging: shape-none

## SPEC §9

None.

## Decisions

none

## BACKLOG

None added.

## Deferred clauses

None.

## Library

None.
EOF
cat > reports/staging/shape-plain.md <<'EOF'
# Staging: shape-plain

## SPEC §9 rows

Rows for the integrator to number:

- Plain row one, no F-NEW marker.
- Plain row two
  continues here.

## Decisions

- D-002 consequence 3 amended in words only: plain amendment text.
- D-001 is new and written by another feature.
- A decision note with no number.

## BACKLOG

- Plain backlog item without any marker. Reason: it is plain.
- New: second plain item; reason: second reason.
- Closed (do not file): plain closure.
- B-041: done (this feature).

## Deferred clauses

- VAL-P-001: plain deferred.

## Library

- plain library line.
EOF
cat > reports/staging/shape-table.md <<'EOF'
# Staging: shape-table

## SPEC s9

| Id | Row |
|---|---|
| F-NEW | Table row one |
| F-NEW | Table row two |

## BACKLOG

| Item | Reason |
|---|---|
| Table item | Table reason |
EOF
git add -A
git commit -q -m "chore: staging files of four shapes"
clean_before=$(git status --porcelain)
python3 scripts/integrate-staging.py --milestone m9 --date 2026-10-25 --dry-run "$tmp/dry" > "$tmp/dry.out" 2>&1
code=$?
check "shapes dry run: exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/dry.out"
check "shapes dry run: unfiled: 0" grep -qx 'unfiled: 0' "$tmp/dry.out"
check "shapes dry run: SPEC §9 rows 5" grep -qx 'SPEC §9 rows: 5' "$tmp/dry.out"
check "shapes dry run: BACKLOG rows 4" grep -qx 'BACKLOG rows: 4' "$tmp/dry.out"
check "shapes dry run: closures 2" grep -qx 'closures: 2' "$tmp/dry.out"
check "shapes dry run: decision addenda 2" grep -qx 'decision addenda: 2' "$tmp/dry.out"
check "shapes dry run: decision notes 2" grep -qx 'decision notes: 2' "$tmp/dry.out"
check "shapes dry run: deferred clauses 2" grep -qx 'deferred clauses: 2' "$tmp/dry.out"
check "shapes dry run: library 2" grep -qx 'library: 2' "$tmp/dry.out"
check "shapes dry run: per-file line" grep -qx 'file shape-plain: spec=2 backlog=2 closures=2 addenda=1 notes=2 deferred=1 library=1' "$tmp/dry.out"
check "shapes dry run: the repository is untouched" [ "$(git status --porcelain)" = "$clean_before" ]
check "shapes dry run: staging files kept" [ -f reports/staging/shape-plain.md ]
check "shapes dry run: copy has the F-003 to F-007 rows" sh -c "grep -q '^| F-007 |' $tmp/dry/SPEC.md && ! grep -q '^| F-008 |' $tmp/dry/SPEC.md"
check "shapes dry run: plain row continuation joined" grep -qF 'Plain row two continues here.' "$tmp/dry/SPEC.md"
check "shapes dry run: table row filed" grep -qF 'Table row two' "$tmp/dry/SPEC.md"
check "shapes dry run: plain backlog bullet filed" grep -qF '| B-003 |' "$tmp/dry/BACKLOG.md"
check "shapes dry run: table backlog row filed with its reason" grep -qF '| Table item | Table reason (shape-table) |' "$tmp/dry/BACKLOG.md"
check "shapes dry run: closed bullet not filed as a row" sh -c "! grep -qF 'plain closure' $tmp/dry/BACKLOG.md"
check "shapes dry run: plain amendment in the copy of D-002" grep -qF 'plain amendment text.' "$tmp/dry/docs/decisions/D-002-decision-2.md"
check "shapes dry run: the real D-002 has no addendum" sh -c "! grep -q Addendum docs/decisions/D-002-decision-2.md"
check "shapes dry run: report keeps the closures and notes" sh -c "grep -qF 'plain closure' $tmp/dry/report.md && grep -qF 'A decision note with no number.' $tmp/dry/report.md"

cat > reports/staging/shape-unknown.md <<'EOF'
# Staging: shape-unknown

## Misc

- A bullet in no known section.
EOF
python3 scripts/integrate-staging.py --milestone m9 --date 2026-10-25 --out "$tmp/unknown.md" > "$tmp/unknown.out" 2>&1
code=$?
check "unknown section: exit 2" [ "$code" -eq 2 ]
check "unknown section: the heading is reported with file and line" grep -qF 'reports/staging/shape-unknown.md:3: ' "$tmp/unknown.out"
check "unknown section: nothing was written" sh -c "! grep -q '^| F-003 |' SPEC.md && [ -f reports/staging/shape-plain.md ]"
rm reports/staging/shape-unknown.md

python3 scripts/integrate-staging.py --milestone m9 --date 2026-10-25 --out "$tmp/real.md" > "$tmp/real.out" 2>&1
code=$?
check "shapes real run: exit 0 and unfiled: 0" sh -c "[ $code -eq 0 ] && grep -qx 'unfiled: 0' $tmp/real.out"
check "shapes real run: five SPEC rows F-003 to F-007" sh -c "grep -q '^| F-007 |' SPEC.md && ! grep -q '^| F-008 |' SPEC.md"
check "shapes real run: D-001 and D-002 have addenda" sh -c "grep -q '^## Addendum 2026-10-25 (m9 integration, shape-bullets)' docs/decisions/D-001-decision-1.md && grep -q '^## Addendum 2026-10-25 (m9 integration, shape-plain)' docs/decisions/D-002-decision-2.md"
check "shapes real run: staging files deleted" [ -z "$(ls reports/staging)" ]

echo "== comma markers: 'Row (F-NEW),' merges like the colon form =="
mkrepo comma
mkdir -p reports/staging
cat > reports/staging/comma-rows.md <<'EOF'
# Staging: comma-rows

## SPEC §9

- Row (F-NEW), comma row one.
- Row (F-NEW): colon row two.
- (F-NEW), parenthesis row three.
- F-NEW, bare row four.

## BACKLOG

- B-NEW, comma backlog item; reason: comma reason.
- B-NEW: colon backlog item; reason: colon reason.
EOF
git add -A
git commit -q -m "chore: staging file with comma markers"
python3 scripts/integrate-staging.py --milestone m9 --date 2026-10-25 --dry-run "$tmp/comma-dry" > "$tmp/comma.out" 2>&1
code=$?
check "comma: dry run exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/comma.out"
check "comma: the marker is dropped from the first row" grep -qF '| F-003 | 2026-10-25 | comma row one. |' "$tmp/comma-dry/SPEC.md"
check "comma: the colon form still merges" grep -qF '| F-004 | 2026-10-25 | colon row two. |' "$tmp/comma-dry/SPEC.md"
check "comma: the parenthesis form merges" grep -qF '| F-005 | 2026-10-25 | parenthesis row three. |' "$tmp/comma-dry/SPEC.md"
check "comma: the bare form merges" grep -qF '| F-006 | 2026-10-25 | bare row four. |' "$tmp/comma-dry/SPEC.md"
check "comma: no marker text is left in SPEC" sh -c "! grep -q 'F-NEW' $tmp/comma-dry/SPEC.md"
check "comma: the B-NEW comma item is filed without the marker" grep -qF '| B-003 | comma backlog item | comma reason. (comma-rows) |' "$tmp/comma-dry/BACKLOG.md"
check "comma: the B-NEW colon item is filed" grep -qF '| B-004 | colon backlog item | colon reason. (comma-rows) |' "$tmp/comma-dry/BACKLOG.md"

echo "== stray headings: every staging file is validated before any merge step =="
mkrepo stray
mkbranch wt/m9 m9-feat
printf '\n## Notes\n\n- A stray heading inside the feature file.\n' >> reports/staging/m9-feat.md
printf '# Staging: m9-other\n\n## SPEC §9\n\nNone.\n\n## Extra findings\n\n- Another stray heading.\n\n## Misc\n\n- And a third.\n' > reports/staging/m9-other.md
git add -A
git commit -q -m "docs: staging files with stray headings"
notes_line=$(grep -n '^## Notes$' reports/staging/m9-feat.md | cut -d: -f1)
git checkout -q main
before=$(git rev-parse HEAD)
runs_before=$(runs)
sh scripts/integrate.sh m9 wt/m9 > "$tmp/stray.out" 2>&1
code=$?
check "stray: exit 2" [ "$code" -eq 2 ]
check "stray: the first file is named with its line" grep -qF "wt/m9:reports/staging/m9-feat.md:$notes_line: " "$tmp/stray.out"
check "stray: the second file's two headings are named in the same pass" sh -c "grep -qF 'wt/m9:reports/staging/m9-other.md:7: ' $tmp/stray.out && grep -qF 'wt/m9:reports/staging/m9-other.md:11: ' $tmp/stray.out"
check "stray: three problems are counted" grep -q '3 problem' "$tmp/stray.out"
check "stray: nothing merged" [ "$(git rev-parse HEAD)" = "$before" ]
check "stray: the gate did not run" [ "$(runs)" = "$runs_before" ]
check "stray: the tree is untouched" [ -z "$(git status --porcelain -- . ':(exclude)build-log')" ]

git checkout -q wt/m9
sed -i.bak '/^## Notes$/,$d' reports/staging/m9-feat.md
rm -f reports/staging/m9-feat.md.bak reports/staging/m9-other.md
git add -A
git commit -q -m "docs: remove the stray headings"
validate
git checkout -q main
sh scripts/integrate.sh m9 wt/m9 > "$tmp/stray2.out" 2>&1
code=$?
check "stray: the fixed branch integrates, exit 0" [ "$code" -eq 0 ]
[ "$code" -ne 0 ] && cat "$tmp/stray2.out"
check "stray: the fixed branch is merged" [ "$(git log --merges --format=%h | wc -l | tr -d ' ')" = 1 ]

echo
if [ "$failures" -eq 0 ]; then
  echo "integrate_test: all checks passed"
  exit 0
fi
echo "integrate_test: $failures check(s) failed"
exit 1
