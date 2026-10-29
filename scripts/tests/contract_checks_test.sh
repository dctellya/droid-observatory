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
  printf 'dir %s crates/observatory-core/src/mapping\n' "$(cd "$d" && find crates/observatory-core/src/mapping -type f | LC_ALL=C sort | xargs shasum -a 256 | shasum -a 256 | cut -d' ' -f1)"
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

if [ "$failures" -ne 0 ]; then
  echo "contract_checks_test: $failures failure(s)"
  exit 1
fi
echo "contract_checks_test: ok"
