# shellcheck shell=sh
# Named cargo tests of one group of g2-tests.tsv: present in the sources, matching their body patterns and `ok` (neither
# ignored nor FAILED) in the gate's test log, reports/gate/<code tree>/test.tests (written by scripts/gate.sh).
# CONTRACT_TESTS_FILE and CONTRACT_TESTLOG override the manifest and the log (the self-test); CONTRACT_TESTLOG=none checks
# presence only (the CI job has no gate log); CONTRACT_REQUIRE_TESTLOG=1 makes a missing log a FAIL (`make check` sets it
# after running `test`).
. "$(dirname "$0")/lib.sh"
manifest=${CONTRACT_TESTS_FILE:-$(dirname "$0")/g2-tests.tsv}

test_body() { # test_body <name> <file>: the lines of the function, up to the closing brace at its indentation
  awk -v n="$1" '
    !on && $0 ~ ("fn " n "[ (<]") { on = 1; match($0, /^[ \t]*/); ind = substr($0, 1, RLENGTH); print; next }
    on { print; if ($0 == ind "}") exit }' "$2"
}

testlog() {
  [ "${CONTRACT_TESTLOG:-}" != none ] || return 1
  if [ -n "${CONTRACT_TESTLOG:-}" ]; then
    f=$CONTRACT_TESTLOG
  else
    key=$(sh scripts/gate.sh --key 2> /dev/null | sed -n 1p)
    f="reports/gate/$key/test.tests"
  fi
  [ -f "$f" ] && echo "$f"
}

run_group() { # run_group <group>
  group=$1
  ids_all=$(awk -F'\t' -v g="$group" '$1 == g { print $2 }' "$manifest" | tr ' ' '\n' | sort -u | tr '\n' ' ' | sed 's/ $//')
  log=$(testlog) || log=
  if [ -z "$log" ]; then
    if [ "${CONTRACT_TESTLOG:-}" = none ]; then
      pending "presence only, no test log"
    elif [ "${CONTRACT_REQUIRE_TESTLOG:-}" = 1 ]; then
      fail "no test log for this code tree: run make test first"
    else
      pending "no test log for this code tree"
    fi
  fi
  while IFS="$(printf '\t')" read -r g ids name landed body; do
    case "$g" in '#'* | '') continue ;; esac
    [ "$g" = "$group" ] || continue
    case "$name" in
      target:*)
        t=${name#target:}
        [ -n "$log" ] || continue
        res=$(awk -v t="$t" '/^R / { on = ($2 == t); next } on { n++; if ($1 != "ok") bad++ } END { print n + 0, bad + 0 }' "$log")
        [ "${res% *}" -gt 0 ] || fail "test binary $t has no tests in the log ($ids)"
        [ "${res#* }" -eq 0 ] || fail "test binary $t has ${res#* } test(s) not ok ($ids)"
        continue
        ;;
    esac
    src=$(grep -rlE "fn $name[ (<]" crates apps --include='*.rs' 2> /dev/null | head -n 1)
    if [ "$landed" = no ]; then
      [ -z "$src" ] && pending "$name" || fail "$name exists in $src: set landed=yes in g2-tests.tsv ($ids)"
      continue
    fi
    if [ -z "$src" ]; then
      fail "$name is missing from the sources ($ids)"
      continue
    fi
    if [ "$body" != - ]; then
      text=$(test_body "$name" "$src")
      pats=$(printf '%s' "$body" | sed 's/~~/~/g' | tr '~' '\n')
      while IFS= read -r p; do
        [ -n "$p" ] || continue
        printf '%s\n' "$text" | grep -Eq -- "$p" || fail "$name body lacks /$p/ ($ids)"
      done << EOF
$pats
EOF
    fi
    [ -n "$log" ] || continue
    grep -Eq "^ok (.*::)?$name\$" "$log" || fail "$name is not ok in the test log ($ids)"
    grep -Eq "^(ignored|FAILED) (.*::)?$name\$" "$log" && fail "$name is ignored or FAILED in the test log ($ids)"
  done < "$manifest"
  finish "$ids_all"
}
