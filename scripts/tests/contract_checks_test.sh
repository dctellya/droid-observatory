#!/bin/sh
# Test of scripts/contract-checks/*.sh (owner item 14b): every check is green on the real tree and red on a scratch
# root with one planted violation. A check prints exactly one line `<check> <ok|FAIL|skip ...> <assertion ids>` and
# exits 0 (ok, skip) or 1 (FAIL); CONTRACT_ROOT points a check at a scratch tree.
set -u
unset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE
root=$(cd "$(dirname "$0")/../.." && pwd)
checks="$root/scripts/contract-checks"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0
fail() {
  echo "FAIL: $1"
  failures=$((failures + 1))
}

# run <check> <scratch root or ''>: sets out (stdout) and rc.
run() {
  if [ -n "$2" ]; then
    out=$(CONTRACT_ROOT="$2" sh "$checks/$1.sh" 2> "$tmp/err")
  else
    out=$(sh "$checks/$1.sh" 2> "$tmp/err")
  fi
  rc=$?
}
expect_ok() { # expect_ok <check> <scratch root or ''> <label>
  run "$1" "$2"
  case "$out" in
    "$1 ok "*) [ "$rc" -eq 0 ] || fail "$3: exit $rc, wanted 0" ;;
    *) fail "$3: wanted '$1 ok ...', got '$out' (stderr: $(cat "$tmp/err"))" ;;
  esac
  [ "$(printf '%s\n' "$out" | wc -l | tr -d ' ')" -eq 1 ] || fail "$3: more than one output line"
}
expect_red() {
  run "$1" "$2"
  case "$out" in
    "$1 FAIL "*) [ "$rc" -eq 1 ] || fail "$3: exit $rc, wanted 1" ;;
    *) fail "$3: wanted '$1 FAIL ...', got '$out'" ;;
  esac
}
expect_skip() {
  run "$1" "$2"
  case "$out" in
    "$1 skip ("*) [ "$rc" -eq 0 ] || fail "$3: exit $rc, wanted 0" ;;
    *) fail "$3: wanted '$1 skip (...', got '$out'" ;;
  esac
}
# fresh <name>: an empty scratch root.
fresh() {
  rm -rf "$tmp/$1"
  mkdir -p "$tmp/$1"
  echo "$tmp/$1"
}
# put <root> <path> <content>
put() {
  mkdir -p "$(dirname "$1/$2")"
  printf '%s\n' "$3" > "$1/$2"
}
# copy <root> <path>...: real files into the scratch root.
copy() {
  r=$1
  shift
  for p in "$@"; do
    mkdir -p "$(dirname "$r/$p")"
    cp -R "$root/$p" "$r/$p"
  done
}

# --- G1: static source greps -------------------------------------------------------------------------------------

expect_ok g1-no-literal-main '' "g1-no-literal-main on the real tree"
d=$(fresh main1)
put "$d" crates/observatory-core/src/ingest/github.rs 'let base = cfg.target_branch.clone();'
put "$d" crates/observatory-core/src/ingest/forge.rs 'fn x() {}'
expect_ok g1-no-literal-main "$d" "clean github.rs and forge.rs"
case "$out" in *pending*) fail "g1-no-literal-main: clean files must not report pending" ;; esac
put "$d" crates/observatory-core/src/ingest/github.rs 'let base = "main";'
expect_red g1-no-literal-main "$d" "literal main in github.rs"

expect_ok g1-no-check-runs '' "g1-no-check-runs on the real tree"
d=$(fresh cr1)
put "$d" crates/observatory-core/src/ingest/github.rs '// never call check-runs (no Checks permission)'
expect_ok g1-no-check-runs "$d" "check-runs only in a comment"
put "$d" crates/observatory-core/src/ingest/github.rs 'let url = format!("{base}/commits/{sha}/check-runs");'
expect_red g1-no-check-runs "$d" "check-runs in code"
put "$d" crates/observatory-core/src/ingest/github.rs 'let u = "check-suites";'
expect_red g1-no-check-runs "$d" "check-suites in code"

expect_ok g1-integrated-wording '' "g1-integrated-wording on the real tree"
d=$(fresh wd1)
put "$d" apps/observatory-app/ui/src/details.rs 'let s = "integrated in mission";'
put "$d" crates/observatory-core/src/view/details.rs 'let k = "integration_merged";'
expect_ok g1-integrated-wording "$d" "integrated in mission, no merged string"
put "$d" crates/observatory-core/src/view/details.rs 'let s = "merged into mission";'
expect_red g1-integrated-wording "$d" "merged in an integration-lane string"

expect_ok g1-no-python-deps '' "g1-no-python-deps on the real tree"
d=$(fresh py1)
put "$d" scripts/usage-report.py 'import json'
expect_ok g1-no-python-deps "$d" "stdlib import only"
put "$d" scripts/usage-report.py 'import pandas as pd'
expect_red g1-no-python-deps "$d" "pandas import"

# --- G4: workflow fields -----------------------------------------------------------------------------------------

expect_ok g4-demo-gate-workflow '' "g4-demo-gate-workflow on the real tree"
d=$(fresh dg1)
copy "$d" .github/workflows/demo-gate.yml
sed 's/^      - unlabeled$//' "$d/.github/workflows/demo-gate.yml" > "$d/x" && mv "$d/x" "$d/.github/workflows/demo-gate.yml"
expect_red g4-demo-gate-workflow "$d" "unlabeled trigger type removed"
copy "$d" .github/workflows/demo-gate.yml
sed 's#demo/base#main#' "$d/.github/workflows/demo-gate.yml" > "$d/x" && mv "$d/x" "$d/.github/workflows/demo-gate.yml"
expect_red g4-demo-gate-workflow "$d" "trigger on main"
copy "$d" .github/workflows/demo-gate.yml
sed 's/contents: read/contents: write/' "$d/.github/workflows/demo-gate.yml" > "$d/x" && mv "$d/x" "$d/.github/workflows/demo-gate.yml"
expect_red g4-demo-gate-workflow "$d" "write permission"

expect_ok g4-e2e-workflow '' "g4-e2e-workflow on the real tree"
d=$(fresh e2e1)
copy "$d" .github/workflows/e2e.yml .github/workflows/mission-branch.yml SPEC.md
sed 's/trunk@0.22.0/trunk@0.21.0/' "$d/.github/workflows/e2e.yml" > "$d/x" && mv "$d/x" "$d/.github/workflows/e2e.yml"
expect_red g4-e2e-workflow "$d" "trunk version differs from the SPEC Tooling row"
copy "$d" .github/workflows/e2e.yml
sed 's/macos-latest/ubuntu-latest/' "$d/.github/workflows/e2e.yml" > "$d/x" && mv "$d/x" "$d/.github/workflows/e2e.yml"
expect_red g4-e2e-workflow "$d" "e2e job not on macos-latest"
copy "$d" .github/workflows/e2e.yml
printf '# e2e job in the 3-minute workflow\n' >> "$d/.github/workflows/mission-branch.yml"
expect_red g4-e2e-workflow "$d" "mission-branch.yml mentions e2e"

expect_ok g4-hooks-json '' "g4-hooks-json on the real tree"
d=$(fresh hk1)
copy "$d" .factory/hooks.json
jq 'del(.PostToolUse)' "$d/.factory/hooks.json" > "$d/x" && mv "$d/x" "$d/.factory/hooks.json"
expect_red g4-hooks-json "$d" "PostToolUse hooks removed"

# --- G5: frozen fixtures and secrets -----------------------------------------------------------------------------

expect_ok g5-frozen-fixtures '' "g5-frozen-fixtures on the real tree"
d=$(fresh fz1)
put "$d" fixtures/streams/a.jsonl 'line one'
mkdir -p "$d/fixtures/streams/a.children"
put "$d" fixtures/streams/a.children/c.jsonl 'child'
put "$d" crates/observatory-core/src/mapping/m.rs 'fn m() {}'
pins="$d/pins.txt"
{
  printf '%s  fixtures/streams/a.jsonl\n' "$(shasum -a 256 "$d/fixtures/streams/a.jsonl" | cut -d' ' -f1)"
  printf '%s  fixtures/streams/a.children/c.jsonl\n' "$(shasum -a 256 "$d/fixtures/streams/a.children/c.jsonl" | cut -d' ' -f1)"
  printf 'dir crates/observatory-core/src/mapping %s\n' "$(cd "$d" && find crates/observatory-core/src/mapping -type f | LC_ALL=C sort | xargs shasum -a 256 | shasum -a 256 | cut -d' ' -f1)"
} > "$pins"
out=$(CONTRACT_ROOT="$d" CONTRACT_FROZEN="$pins" sh "$checks/g5-frozen-fixtures.sh" 2> /dev/null)
rc=$?
case "$out" in "g5-frozen-fixtures ok "*) [ "$rc" -eq 0 ] || fail "frozen pins: exit $rc" ;; *) fail "frozen pins match: got '$out'" ;; esac
put "$d" fixtures/streams/a.children/c.jsonl 'child edited'
out=$(CONTRACT_ROOT="$d" CONTRACT_FROZEN="$pins" sh "$checks/g5-frozen-fixtures.sh" 2> /dev/null)
rc=$?
case "$out" in "g5-frozen-fixtures FAIL "*) [ "$rc" -eq 1 ] || fail "frozen child edit: exit $rc" ;; *) fail "frozen child edit: got '$out'" ;; esac
put "$d" fixtures/streams/a.children/c.jsonl 'child'
put "$d" crates/observatory-core/src/mapping/new.rs 'fn n() {}'
out=$(CONTRACT_ROOT="$d" CONTRACT_FROZEN="$pins" sh "$checks/g5-frozen-fixtures.sh" 2> /dev/null)
rc=$?
case "$out" in "g5-frozen-fixtures FAIL "*) [ "$rc" -eq 1 ] || fail "mapping file added: exit $rc" ;; *) fail "mapping file added: got '$out'" ;; esac

expect_ok g5-fixture-secrets '' "g5-fixture-secrets on the real tree"
d=$(fresh sec1)
put "$d" fixtures/streams/clean.jsonl '{"type":"message","text":"hello"}'
expect_ok g5-fixture-secrets "$d" "clean fixture directory"
secret=$(LC_ALL=C tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 36)
put "$d" fixtures/streams/leak.jsonl "{\"env\":\"GITHUB_TOKEN=ghp_$secret\"}"
expect_red g5-fixture-secrets "$d" "a GitHub token in a fixture"

# --- G6: git history rules, on throwaway repositories -----------------------------------------------------------

TRAILER='Co-authored-by: factory-droid[bot] <138933559+factory-droid[bot]@users.noreply.github.com>'
# repo <name>: a repository with a `mission/observatory-v1` branch holding one commit, HEAD on it.
repo() {
  r="$tmp/$1"
  rm -rf "$r"
  mkdir -p "$r"
  (
    cd "$r" || exit 1
    git init -q -b mission/observatory-v1 .
    git config user.name t
    git config user.email t@example.invalid
    git config commit.gpgsign false
    git config advice.ignoredHook false
    printf 'base\n' > base.txt
    git add base.txt
    git commit -q -m 'base commit'
  )
  echo "$r"
}
# commit <repo> <subject> [path]: one commit with the trailer (a path is written first).
commit() {
  (
    cd "$1" || exit 1
    f=${3:-f-$(printf '%s' "$2" | cksum | cut -d' ' -f1).txt}
    mkdir -p "$(dirname "$f")"
    printf '%s\n' "$2" > "$f"
    git add -A
    printf '%s\n\n%s\n' "$2" "$TRAILER" > "$tmp/msg"
    git commit -q -F "$tmp/msg"
  )
}

r=$(repo gc1)
git -C "$r" checkout -q -b wt/m9
commit "$r" 'feat: a short subject'
expect_ok g6-commit-format "$r" "short subject with the trailer"
commit "$r" 'feat: a subject that is far longer than the seventy-two characters the commit-msg hook allows'
expect_red g6-commit-format "$r" "subject over 72 characters"
r=$(repo gc2)
git -C "$r" checkout -q -b wt/m9
commit "$r" 'wip: half done'
expect_red g6-commit-format "$r" "WIP marker"
r=$(repo gc3)
git -C "$r" checkout -q -b wt/m9
printf 'x\n' > "$r/x.txt"
git -C "$r" add x.txt
git -C "$r" commit -q -m 'feat: no trailer here'
expect_red g6-commit-format "$r" "no co-author trailer"

r=$(repo gr1)
git -C "$r" checkout -q -b wt/m9
commit "$r" 'feat: one'
expect_ok g6-no-history-rewrite "$r" "reflog without rewrites"
git -C "$r" commit -q --amend -m 'feat: one, amended'
expect_red g6-no-history-rewrite "$r" "reflog shows an amend"

r=$(repo gh1)
git -C "$r" config core.hooksPath .githooks
expect_ok g6-hooks-config "$r" "hooksPath .githooks"
git -C "$r" config core.hooksPath .git/hooks
expect_red g6-hooks-config "$r" "hooksPath elsewhere"
git -C "$r" config core.hooksPath .githooks
commit "$r" 'feat: edit a hook' .githooks/commit-msg
expect_red g6-hooks-config "$r" "hook edited after the baseline without a decision"
sha=$(git -C "$r" log -1 --format=%h)
put "$r" docs/decisions/D-900-hooks.md "The hook change $sha is recorded here."
git -C "$r" add -A
git -C "$r" commit -q -m 'docs: decision naming the hook change'
expect_ok g6-hooks-config "$r" "hook edit named by a decision"

# the worktree layout needs the observatory-wt/<m> directory shape
lay="$tmp/lay"
rm -rf "$lay"
mkdir -p "$lay"
(
  cd "$lay" || exit 1
  git init -q -b mission/observatory-v1 main-checkout
  cd main-checkout || exit 1
  git config user.name t
  git config user.email t@example.invalid
  printf 'a\n' > a.txt
  git add a.txt
  git commit -q -m base
  mkdir -p ../observatory-wt
  git worktree add -q ../observatory-wt/m9 -b wt/m9
  ln -s m9 ../observatory-wt/current
)
expect_ok g6-worktree-layout "$lay/observatory-wt/m9" "worktree on wt/m9 with current pointing at it"
rm "$lay/observatory-wt/current"
ln -s m8 "$lay/observatory-wt/current"
expect_red g6-worktree-layout "$lay/observatory-wt/m9" "current points elsewhere"
expect_skip g6-worktree-layout "$(repo notwt)" "a checkout that is not an observatory-wt worktree"

# merge shape: integrated worktree branches are second parents of exactly one first-parent merge
r=$(repo gm1)
git -C "$r" checkout -q -b wt/m1
commit "$r" 'feat: m1 work'
git -C "$r" checkout -q mission/observatory-v1
printf 'Integrate m1\n\n%s\n' "$TRAILER" > "$tmp/msg"
git -C "$r" merge -q --no-ff -F "$tmp/msg" wt/m1
git -C "$r" checkout -q wt/m1
expect_ok g6-merge-shape "$r" "m1 integrated by a no-ff merge"
r=$(repo gm2)
git -C "$r" checkout -q -b wt/m1
commit "$r" 'feat: m1 work'
git -C "$r" checkout -q mission/observatory-v1
git -C "$r" merge -q --ff-only wt/m1
expect_red g6-merge-shape "$r" "m1 fast-forwarded into the mission branch"

# shared files: a worktree branch must not touch README.md, BACKLOG.md and the other integrator files
r=$(repo gs1)
git -C "$r" checkout -q -b wt/m9
commit "$r" 'feat: code' src.rs
expect_ok g6-shared-files "$r" "worktree branch touching only code"
commit "$r" 'docs: backlog edit' BACKLOG.md
expect_red g6-shared-files "$r" "worktree branch edited BACKLOG.md"
r=$(repo gs2)
git -C "$r" checkout -q -b wt/m9
printf '| F-999 | row |\n' > "$r/SPEC.md"
git -C "$r" add SPEC.md
git -C "$r" commit -q -m 'docs: spec row' -m "$TRAILER"
git -C "$r" checkout -q mission/observatory-v1
printf '| F-001 | row |\n' > "$r/SPEC.md"
git -C "$r" add SPEC.md
git -C "$r" commit -q -m 'docs: spec base'
git -C "$r" checkout -q wt/m9
git -C "$r" merge -q --no-edit mission/observatory-v1 > /dev/null 2>&1
printf '| F-999 | row |\n| F-001 | row |\n' > "$r/SPEC.md"
git -C "$r" add SPEC.md
git -C "$r" commit -q -m 'docs: spec rows again' -m "$TRAILER"
expect_red g6-shared-files "$r" "worktree branch added a SPEC section 9 row"

# metrics sources stay untouched on wt/m4
r=$(repo gx1)
git -C "$r" checkout -q -b wt/m4
commit "$r" 'feat: elsewhere' crates/observatory-core/src/other.rs
expect_ok g5-metrics-untouched "$r" "wt/m4 leaves metrics alone"
commit "$r" 'feat: metrics' crates/observatory-core/src/metrics/mod.rs
expect_red g5-metrics-untouched "$r" "wt/m4 touched the metrics directory"
git -C "$r" checkout -q mission/observatory-v1
expect_skip g5-metrics-untouched "$r" "not on wt/m4"

# --- G9: make targets --------------------------------------------------------------------------------------------

expect_ok g9-check-target-list '' "g9-check-target-list on the real tree"
d=$(fresh tl1)
put "$d" Makefile 'check-all: check-docs check-links check-workflows check-file-size check-todos check-decisions-index check-secrets redact-check fmt-check lint test deny machete replay-fixtures'
expect_ok g9-check-target-list "$d" "all named targets listed"
put "$d" Makefile 'check-all: check-docs check-links check-workflows check-file-size check-todos check-decisions-index fmt-check lint test deny machete replay-fixtures'
expect_red g9-check-target-list "$d" "check-secrets dropped from check-all"

expect_ok g9-make-targets '' "g9-make-targets on the real tree"
d=$(fresh mk1)
printf 'check-decisions-index:\n\t@true\ncheck-file-size:\n\t@true\ncheck-todos:\n\t@true\n' > "$d/Makefile"
expect_ok g9-make-targets "$d" "three targets exit 0"
printf 'check-decisions-index:\n\t@true\ncheck-file-size:\n\t@false\ncheck-todos:\n\t@true\n' > "$d/Makefile"
expect_red g9-make-targets "$d" "check-file-size exits non-zero"

# --- G2: named cargo tests --------------------------------------------------------------------------------------

# g2 <check> <root> <manifest> <log>: runs the check with a scratch manifest and test log.
g2() {
  out=$(CONTRACT_ROOT="$2" CONTRACT_TESTS_FILE="$3" CONTRACT_TESTLOG="$4" sh "$checks/$1.sh" 2> "$tmp/err")
  rc=$?
}
g2_expect() { # g2_expect <ok|FAIL> <label>
  case "$out" in
    "g2-gate-tests $1 "*) if [ "$1" = ok ]; then [ "$rc" -eq 0 ] || fail "$2: exit $rc"; else [ "$rc" -eq 1 ] || fail "$2: exit $rc"; fi ;;
    *) fail "$2: wanted '$1', got '$out' ($(cat "$tmp/err"))" ;;
  esac
}
d=$(fresh g2a)
put "$d" crates/x/src/lib.rs '#[cfg(test)]
mod tests {
    #[test]
    fn first_test() {
        let lane = "build";
        assert_eq!(count, 1);
    }

    #[test]
    fn later_test() {
        assert!(true);
    }
}'
tab=$(printf '\t')
man="$d/g2.tsv"
log="$d/test.tests"
printf 'R x\nok tests::first_test\nok tests::later_test\n' > "$log"
printf 'gate%sVAL-X-1%sfirst_test%syes%sbuild~~\\b1\\b\n' "$tab" "$tab" "$tab" "$tab" > "$man"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect ok "landed test with its body patterns and an ok log line"
case "$out" in *pending*) fail "g2: a fully covered group must not report pending: $out" ;; esac
case "$out" in "g2-gate-tests ok VAL-X-1") ;; *) fail "g2: ids not listed on the line: $out" ;; esac
printf 'gate%sVAL-X-2%sunwritten_test%sno%s-\n' "$tab" "$tab" "$tab" "$tab" > "$man"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect ok "landed=no and absent is pending"
case "$out" in *"[pending: unwritten_test]"*) ;; *) fail "g2: pending note missing: $out" ;; esac
printf 'gate%sVAL-X-2%sfirst_test%sno%s-\n' "$tab" "$tab" "$tab" "$tab" > "$man"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "landed=no but the test exists (flip the row)"
printf 'gate%sVAL-X-2%sunwritten_test%syes%s-\n' "$tab" "$tab" "$tab" "$tab" > "$man"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "landed=yes but the test is missing"
printf 'gate%sVAL-X-1%sfirst_test%syes%sbuild~~\\b2\\b\n' "$tab" "$tab" "$tab" "$tab" > "$man"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "body pattern absent from the test body"
printf 'gate%sVAL-X-1%slater_test%syes%s\\b1\\b\n' "$tab" "$tab" "$tab" "$tab" > "$man"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "a body pattern that only matches the neighbouring test must not pass"
printf 'gate%sVAL-X-1%sfirst_test%syes%s-\n' "$tab" "$tab" "$tab" "$tab" > "$man"
printf 'R x\nignored tests::first_test\n' > "$log"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "test ignored in the log"
printf 'R x\nFAILED tests::first_test\n' > "$log"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "test FAILED in the log"
printf 'R x\nok tests::other_test\n' > "$log"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "test absent from the log"
printf 'gate%sVAL-X-3%starget:mapping%syes%s-\n' "$tab" "$tab" "$tab" "$tab" > "$man"
printf 'R x\nok tests::a\nR mapping\nok m::one\nok m::two\n' > "$log"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect ok "test binary all ok"
printf 'R mapping\nok m::one\nFAILED m::two\n' > "$log"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "test binary with a failure"
printf 'R x\nok tests::a\n' > "$log"
g2 g2-gate-tests "$d" "$man" "$log"
g2_expect FAIL "test binary absent from the log"
out=$(CONTRACT_ROOT="$d" CONTRACT_TESTS_FILE="$man" CONTRACT_TESTLOG=none CONTRACT_REQUIRE_TESTLOG=1 sh "$checks/g2-gate-tests.sh" 2> /dev/null)
case "$out" in "g2-gate-tests ok "*"presence only"*) ;; *) fail "CONTRACT_TESTLOG=none must check presence only: $out" ;; esac
out=$(CONTRACT_ROOT="$d" CONTRACT_TESTS_FILE="$man" CONTRACT_REQUIRE_TESTLOG=1 sh "$checks/g2-gate-tests.sh" 2> /dev/null)
case "$out" in "g2-gate-tests FAIL "*) ;; *) fail "a missing log under CONTRACT_REQUIRE_TESTLOG=1 must FAIL: $out" ;; esac
for c in g2-gate-tests g2-metric-tests g2-telemetry-tests; do
  expect_ok "$c" '' "$c on the real tree"
done

# --- G7: gh API fields, against a stub gh ------------------------------------------------------------------------

gs="$tmp/ghstub"
mkdir -p "$gs/bin" "$gs/rules"
cat > "$gs/bin/gh" <<'STUB'
#!/bin/sh
# Stub gh: the first rule file (sorted) whose first line is a substring of the arguments prints its remaining lines.
args="$*"
for r in "$GH_STUB"/*.rule; do
  [ -f "$r" ] || continue
  pat=$(sed -n 1p "$r")
  case "$args" in *"$pat"*) sed 1d "$r"; exit 0 ;; esac
done
echo "stub gh: no rule for: $args" >&2
exit 1
STUB
chmod +x "$gs/bin/gh"
gh_clear() { rm -rf "$gs/rules" && mkdir -p "$gs/rules"; }
gh_rule() { # gh_rule <rank> <pattern> <output>
  printf '%s\n%s\n' "$2" "$3" > "$gs/rules/$1.rule"
}
# gh_check <check> <ok|FAIL|skip> <label> [root]: with the stub on PATH and a token set
gh_check() {
  out=$(PATH="$gs/bin:$PATH" GH_STUB="$gs/rules" GH_TOKEN=stub-token CONTRACT_REPO=o/r CONTRACT_ROOT="${4:-$root}" sh "$checks/$1.sh" 2> "$tmp/err")
  rc=$?
  case "$out" in
    "$1 $2"*) if [ "$2" = FAIL ]; then [ "$rc" -eq 1 ] || fail "$3: exit $rc"; else [ "$rc" -eq 0 ] || fail "$3: exit $rc"; fi ;;
    *) fail "$3: wanted '$1 $2', got '$out' ($(cat "$tmp/err"))" ;;
  esac
}

for c in g7-demo-base g7-demo-ruleset g7-demo-branches g7-main-untouched g7-no-force-push g7-no-backlog-issues g7-ci-run g7-mission-pr g7-demo-red-pr; do
  out=$(env -u GH_TOKEN -u GITHUB_TOKEN sh "$checks/$c.sh" 2> /dev/null)
  rc=$?
  case "$out" in "$c skip (no network) "*) [ "$rc" -eq 0 ] || fail "$c without a token: exit $rc" ;; *) fail "$c without a token must skip (no network): '$out'" ;; esac
done

gh_clear
gh_rule 1 'branches/demo/base' 'demo/base'
gh_rule 2 'compare/' 'ahead'
gh_check g7-demo-base ok "demo/base exists and descends from the baseline"
gh_rule 2 'compare/' 'diverged'
gh_check g7-demo-base FAIL "demo/base diverged from the baseline"
gh_clear
gh_rule 1 'branches/demo/base' ''
gh_check g7-demo-base FAIL "demo/base answers nothing"

gh_clear
gh_rule 1 'rulesets/42' '{"name":"demo-gate","conditions":{"ref_name":{"include":["refs/heads/demo/base"],"exclude":[]}},"rules":[{"type":"pull_request"},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"demo-gate"}]}},{"type":"non_fast_forward"}]}'
gh_rule 2 'rulesets --jq' '42'
gh_check g7-demo-ruleset ok "demo-gate ruleset fields"
gh_rule 1 'rulesets/42' '{"name":"demo-gate","conditions":{"ref_name":{"include":["~DEFAULT_BRANCH"],"exclude":[]}},"rules":[{"type":"pull_request"},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"demo-gate"}]}},{"type":"non_fast_forward"}]}'
gh_check g7-demo-ruleset FAIL "ruleset bound to the default branch"
gh_rule 1 'rulesets/42' '{"name":"demo-gate","conditions":{"ref_name":{"include":["refs/heads/demo/base"],"exclude":[]}},"rules":[{"type":"pull_request"},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"demo-gate"},{"context":"gate"}]}},{"type":"non_fast_forward"}]}'
gh_check g7-demo-ruleset FAIL "two required contexts"
gh_rule 1 'rulesets/42' '{"name":"demo-gate","conditions":{"ref_name":{"include":["refs/heads/demo/base"],"exclude":[]}},"rules":[{"type":"pull_request"},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"demo-gate"}]}}]}'
gh_check g7-demo-ruleset FAIL "non_fast_forward rule missing"
gh_clear
gh_rule 2 'rulesets --jq' ''
gh_check g7-demo-ruleset FAIL "no demo-gate ruleset"

gh_clear
gh_rule 1 'branches --paginate' 'demo/base
mission/observatory-v1
main'
gh_check g7-demo-branches ok "only demo/base among the demo branches" "$(fresh readme_ok)"
gh_rule 1 'branches --paginate' 'demo/base
demo/red-1'
gh_check g7-demo-branches FAIL "a leftover demo/red-1 branch"

gh_clear
gh_rule 1 'rulesets/25636994' '{"id":25636994,"name":"main-gate"}'
gh_rule 2 'commits?sha=main' '0'
gh_rule 3 'git/ref/heads/main' 'c8524c2641c00376340640859bb5381763c59ba6'
d=$(fresh mu1)
put "$d" scripts/contract-checks/main-gate.json '{"id":25636994,"name":"main-gate"}'
gh_check g7-main-untouched ok "main and the main-gate ruleset unchanged" "$d"
gh_rule 2 'commits?sha=main' '1'
gh_check g7-main-untouched FAIL "a commit on main since the start" "$d"
gh_rule 2 'commits?sha=main' '0'
gh_rule 1 'rulesets/25636994' '{"id":25636994,"name":"main-gate","edited":true}'
gh_check g7-main-untouched FAIL "main-gate ruleset bytes changed" "$d"

gh_clear
gh_rule 1 'branches --paginate' 'mission/observatory-v1
wt/m4
main'
gh_rule 2 'activity?ref=' 'push
branch_creation
merge'
gh_check g7-no-force-push ok "only push, branch_creation and merge"
gh_rule 2 'activity?ref=' 'push
force_push'
gh_check g7-no-force-push FAIL "a force_push activity"

gh_clear
gh_rule 1 'issue list' '0'
gh_rule 2 'label list' '0'
d=$(fresh bl1)
gh_check g7-no-backlog-issues ok "no backlog issues or label" "$d"
gh_rule 1 'issue list' '3'
gh_check g7-no-backlog-issues FAIL "three backlog issues" "$d"
gh_rule 1 'issue list' '0'
put "$d" scripts/gen-backlog.sh 'echo gen'
(cd "$d" && git init -q . && git add -A)
gh_check g7-no-backlog-issues FAIL "gen-backlog.sh tracked" "$d"

gh_clear
gh_rule 1 'run list' '{"databaseId":7,"conclusion":"success","status":"completed"}'
gh_rule 2 'run view' 'machete success
coverage success
rust success'
gh_check g7-ci-run ok "run succeeded with machete and coverage"
gh_rule 2 'run view' 'machete success
coverage failure'
gh_check g7-ci-run FAIL "coverage job failed"
gh_rule 1 'run list' '{"databaseId":7,"conclusion":"failure","status":"completed"}'
gh_check g7-ci-run FAIL "run failed"
gh_rule 1 'run list' 'null'
gh_check g7-ci-run skip "no run for the commit"

gh_clear
gh_rule 1 'pr list' '0'
gh_check g7-mission-pr skip "no PR yet"
gh_rule 1 'pr list' '1'
gh_rule 2 'pr view' '{"state":"OPEN","mergeStateStatus":"CLEAN","mergedBy":null,"isDraft":false}'
gh_rule 3 'pr checks' 'gate	pass	1m	https://example.invalid'
gh_check g7-mission-pr ok "open clean PR with gate passing"
gh_rule 2 'pr view' '{"state":"MERGED","mergeStateStatus":"CLEAN","mergedBy":{"login":"x"},"isDraft":false}'
gh_check g7-mission-pr FAIL "PR already merged"
gh_rule 2 'pr view' '{"state":"OPEN","mergeStateStatus":"CLEAN","mergedBy":null,"isDraft":false}'
gh_rule 3 'pr checks' 'gate	fail	1m	https://example.invalid'
gh_check g7-mission-pr FAIL "gate check failing"

gh_clear
gh_rule 1 'pr list' '0'
gh_check g7-demo-red-pr skip "no demo/red-1 PR"
gh_rule 1 'pr list' '1'
gh_check g7-demo-red-pr ok "exactly one PR for demo/red-1"
gh_rule 1 'pr list' '2'
gh_check g7-demo-red-pr FAIL "two PRs for demo/red-1"

# --- G3: CLI output rules over the metrics CSV and JSON ------------------------------------------------------------

header='scope,id,name,cycle_s,first_pass_active_s,rework_s,waste_waiting_human_question_s,waste_waiting_human_permission_s,waste_waiting_merge_s,waste_waiting_validation_s,waste_rework_s,waste_retry_failure_s,waste_idle_queued_s,waste_idle_unscheduled_s,rework_windows,throttled_minutes,tokens_input,tokens_output,tokens_cache_read,tokens_cache_write,cost_usd,price_table'
good_row1='feature,F-1,"Name, with comma",100,60,40,1,2,3,4,5,6,7,8,1,0,10,20,30,40,1.25,2026-10-22'
good_row2='milestone,m0,m0,900,500,400,1,2,3,4,5,6,7,8,2,755,10,20,30,40,12.50,2026-10-22'
good_json='{"overhead":{"seconds":5},"rows":[{"scope":"feature","id":"F-1","cycle_s":100,"first_pass_active_s":60,"rework_s":40,"rework_windows":1,"throttled_minutes":0,"tokens_input":10,"tokens_output":20,"tokens_cache_read":30,"tokens_cache_write":40,"cost_usd":1.25,"price_table":"2026-10-22"},{"scope":"milestone","id":"m0","cycle_s":900,"first_pass_active_s":500,"rework_s":400,"rework_windows":2,"throttled_minutes":755,"tokens_input":10,"tokens_output":20,"tokens_cache_read":30,"tokens_cache_write":40,"cost_usd":12.5,"price_table":"2026-10-22"}]}'
# m3 <csv text> <json text>: runs g3-metrics-contract on scratch output files.
m3() {
  printf '%s\n' "$1" > "$tmp/m.csv"
  printf '%s\n' "$2" > "$tmp/m.json"
  out=$(CONTRACT_METRICS_CSV="$tmp/m.csv" CONTRACT_METRICS_JSON="$tmp/m.json" sh "$checks/g3-metrics-contract.sh" 2> "$tmp/err")
  rc=$?
}
m3_expect() {
  case "$out" in
    "g3-metrics-contract $1 "*) if [ "$1" = ok ]; then [ "$rc" -eq 0 ] || fail "$2: exit $rc"; else [ "$rc" -eq 1 ] || fail "$2: exit $rc"; fi ;;
    *) fail "$2: wanted '$1', got '$out' ($(cat "$tmp/err"))" ;;
  esac
}
nl='
'
m3 "$header$nl$good_row1$nl$good_row2" "$good_json"
m3_expect ok "a conforming CSV and JSON"
case "$out" in *pending*) fail "g3: conforming output must not report pending: $out" ;; esac
m3 "$(printf '%s' "$header" | sed 's/,throttled_minutes//')$nl$good_row1" "$good_json"
m3_expect FAIL "header without throttled_minutes (21 columns)"
m3 "$(printf '%s' "$header" | sed 's/rework_windows,throttled_minutes/throttled_minutes,rework_windows/')$nl$good_row1" "$good_json"
m3_expect FAIL "reordered columns"
m3 "$header${nl}feature,F-1,n,10.5,60,40,1,2,3,4,5,6,7,8,1,0,10,20,30,40,1.25,2026-10-22" "$good_json"
m3_expect FAIL "a non-integer _s column"
m3 "$header${nl}feature,F-1,n,100,60,40,1,2,3,4,5,6,7,8,1,0,10,20,30,40,1.25,2026-10-23" "$good_json"
m3_expect FAIL "price_table other than 2026-10-22"
m3 "$header${nl}run,F-1,n,100,60,40,1,2,3,4,5,6,7,8,1,0,10,20,30,40,1.25,2026-10-22" "$good_json"
m3_expect FAIL "scope outside feature and milestone"
m3 "$header${nl}feature,F-1,n,100,60,40,1,2,3,4,5,6,7,8,1,x,10,20,30,40,1.25,2026-10-22" "$good_json"
m3_expect FAIL "a non-integer throttled_minutes"
m3 "$header${nl}feature,F-1,n,100,60,40,1,2,3,4,5,6,7,8,1,0,10,2.5,30,40,1.25,2026-10-22" "$good_json"
m3_expect FAIL "a non-integer tokens column"
m3 "$header${nl}feature,F-1,n,100,60,40,1,2,3,4,5,6,7,8,1,0,10,20,30,40,abc,2026-10-22" "$good_json"
m3_expect FAIL "a non-decimal cost_usd"
m3 "$header${nl}feature,F-1,n,100,60,40,1,2,3,4,5,6,7,8,1,0,10,20,30,40,1.25" "$good_json"
m3_expect FAIL "a row with 21 fields"
m3 "$header$nl$good_row1$nl$good_row2" "$(printf '%s' "$good_json" | sed 's/"cycle_s":100/"cycle_s":101/')"
m3_expect FAIL "JSON cycle_s disagrees with the CSV"
m3 "$header$nl$good_row1$nl$good_row2" "$(printf '%s' "$good_json" | sed 's/"id":"m0"/"id":"m9"/')"
m3_expect FAIL "a CSV row without a JSON counterpart"
m3 "$header$nl$good_row1$nl$good_row2" "$(printf '%s' "$good_json" | sed 's/"overhead":{"seconds":5}/"overhead":null/')"
m3_expect FAIL "overhead line null under the default config"
m3 "$header$nl$good_row1$nl$good_row2" 'not json'
m3_expect FAIL "JSON that does not parse"
expect_ok g3-metrics-contract '' "g3-metrics-contract on the real tree (metrics command not landed: pending)"
case "$out" in *pending*) ;; *) fail "g3: the real tree has no metrics command yet, wanted pending: $out" ;; esac

# --- the runner --------------------------------------------------------------------------------------------------

out=$(CONTRACT_SKIP="g5-fixture-secrets" sh "$checks/run-all.sh" 2> /dev/null)
rc=$?
[ "$rc" -eq 0 ] || fail "run-all.sh on the real tree: exit $rc, output: $out"
printf '%s\n' "$out" | grep -q '^g1-no-check-runs ok ' || fail "run-all.sh output lacks the g1-no-check-runs line"
printf '%s\n' "$out" | grep -q '^g5-fixture-secrets skip (CONTRACT_SKIP) ' || fail "run-all.sh did not honour CONTRACT_SKIP"
printf '%s\n' "$out" | tail -n 1 | grep -Eq '^contract checks: [0-9]+ ok, [0-9]+ skipped, 0 FAIL' || fail "run-all.sh summary line missing: $(printf '%s\n' "$out" | tail -n 1)"
d=$(fresh run1)
put "$d" crates/observatory-core/src/x.rs 'let u = "check-runs";'
out=$(CONTRACT_ROOT="$d" CONTRACT_SKIP="g5-fixture-secrets g6-commit-format" sh "$checks/run-all.sh" 2> /dev/null)
rc=$?
[ "$rc" -eq 1 ] || fail "run-all.sh with a planted violation: exit $rc, wanted 1"
printf '%s\n' "$out" | grep -q '^g1-no-check-runs FAIL ' || fail "run-all.sh lost the FAIL line"

if [ "$failures" -ne 0 ]; then
  echo "contract_checks_test: $failures failure(s)"
  exit 1
fi
echo "contract_checks_test: ok"
