#!/bin/sh
# Test of scripts/check-kit.py (owner item 14b, second half; VAL-UI-001): green on the real tree, red on a scratch
# tree with one planted violation each (a padding override, a sans identifier cell, a literal colour, ...), and the
# wiring of `make check-kit` into `make check` and the mission-branch CI. KIT_ROOT points the lint at a scratch tree.
set -u
unset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE
root=$(cd "$(dirname "$0")/../.." && pwd)
lint="$root/scripts/check-kit.py"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0
fail() {
  echo "FAIL: $1"
  failures=$((failures + 1))
}

# scratch <name>: a copy of the files the lint reads; sets $d.
scratch() {
  d="$tmp/$1"
  rm -rf "$d"
  mkdir -p "$d/ai/design" "$d/apps/observatory-app"
  cp -R "$root/ai/design/ui-kit-v2" "$d/ai/design/ui-kit-v2"
  cp -R "$root/apps/observatory-app/ui" "$d/apps/observatory-app/ui"
  cp "$root/apps/observatory-app/kit-departures.md" "$d/apps/observatory-app/kit-departures.md"
  cp "$root/apps/observatory-app/kit-pending.txt" "$d/apps/observatory-app/kit-pending.txt"
}
# edit <file> <perl substitution>: in place, portable between macOS and Linux.
edit() {
  perl -0pi -e "$2" "$d/$1"
}
run() {
  out=$(KIT_ROOT="$1" python3 "$lint" 2> "$tmp/err")
  rc=$?
}
expect_ok() {
  run "$1"
  case "$out" in
    "check-kit ok VAL-UI-001"*) [ "$rc" -eq 0 ] || fail "$2: exit $rc, wanted 0" ;;
    *) fail "$2: wanted 'check-kit ok VAL-UI-001', got '$out' (stderr: $(cat "$tmp/err"))" ;;
  esac
}
# expect_red <root> <label> <text the reasons must contain>
expect_red() {
  run "$1"
  case "$out" in
    "check-kit FAIL VAL-UI-001") [ "$rc" -eq 1 ] || fail "$2: exit $rc, wanted 1" ;;
    *) fail "$2: wanted 'check-kit FAIL VAL-UI-001', got '$out'" ;;
  esac
  grep -q -F -- "$3" "$tmp/err" || fail "$2: the reasons do not name '$3': $(cat "$tmp/err")"
}

# --- the real tree -----------------------------------------------------------------------------------------------

expect_ok "$root" "the real tree"

# --- rule 3 and 6: spacing overrides and departures ---------------------------------------------------------------

scratch pad1
printf '\n.lib td { padding: 6px; }\n' >> "$d/apps/observatory-app/ui/style.css"
expect_red "$d" "a padding override of the kit's .lib td" '`.lib td` padding'

scratch pad2
printf '\n.table-dense td { padding: 2px 4px; }\n' >> "$d/apps/observatory-app/ui/style.css"
expect_red "$d" "a td padding override under a new selector" '`.table-dense td` padding'

scratch pad3
printf '\n.kpi { padding: 4px; }\n' >> "$d/apps/observatory-app/ui/style.css"
expect_red "$d" "a padding override of the kit's .kpi" '`.kpi` padding'

scratch width1
printf '\n.page { max-width: 1400px; }\n' >> "$d/apps/observatory-app/ui/style.css"
expect_red "$d" "a page width override" '`.page` max-width'

scratch font1
printf '\n.note { font-size: 13px; }\n' >> "$d/apps/observatory-app/ui/style.css"
expect_red "$d" "a font size off the kit scale" '`.note` font-size'

scratch dep1
perl -ni -e 'print unless /^\| `\.observatory`/' "$d/apps/observatory-app/kit-departures.md"
expect_red "$d" "a departure with no row" '`.observatory` max-width'

scratch dep2
edit apps/observatory-app/kit-departures.md 's/\| 1480px \|/| 1500px |/'
expect_red "$d" "a row whose actual value is stale" 'row says 1500px'

scratch dep3
printf '| `.gone` | padding | 10px | 1px | a row of a rule that is not in style.css | F-105 |\n' >> "$d/apps/observatory-app/kit-departures.md"
expect_red "$d" "a row of a departure that is gone" '`.gone` padding is not in style.css any more'

scratch dep4
edit apps/observatory-app/kit-departures.md 's/\| F-105 \|/| none |/'
expect_red "$d" "a row with no SPEC row" 'names no SPEC row'

# The revert the gate UI feature makes: the override goes, its row goes, and the lint stays green.
scratch revert1
edit apps/observatory-app/ui/style.css 's/(\.library th, \.library td \{[^}]*?)padding: 6px 8px;/$1padding: 10px;/'
perl -ni -e 'print unless /^\| `\.library th, \.library td`/' "$d/apps/observatory-app/kit-departures.md"
expect_ok "$d" "an override set back to the kit's value with its row removed"

# --- rule 1: tokens only -----------------------------------------------------------------------------------------

scratch tok1
printf '\n.x { color: #ff0000; }\n' >> "$d/apps/observatory-app/ui/style.css"
expect_red "$d" "a literal colour in style.css" 'literal colour'

scratch tok2
printf '\n.x { background: rgb(1, 2, 3); }\n' >> "$d/apps/observatory-app/ui/style.css"
expect_red "$d" "an rgb() colour in style.css" 'literal colour'

scratch tok3
printf '\n.x { font-family: Arial; }\n' >> "$d/apps/observatory-app/ui/style.css"
expect_red "$d" "a font-family in style.css" 'font-family outside theme.css'

scratch tok4
printf '\n.x { color: #123; }\n' >> "$d/ai/design/ui-kit-v2/theme.css"
expect_red "$d" "theme.css differing from the kit's" 'differs from ai/design/ui-kit-v2/theme.css'

scratch tok5
edit apps/observatory-app/ui/src/details.rs 's/<td class="n">/<td class="n" style="padding: 2px">/'
expect_red "$d" "an inline padding in a view" 'inline style'

# --- rule 2: the mono face ---------------------------------------------------------------------------------------

scratch mono1
edit apps/observatory-app/ui/src/details.rs 's/<td class="mono">\{row\.kind\.clone\(\)\}/<td>{row.kind.clone()}/'
expect_red "$d" "a sans event-kind cell" 'identifier-shaped text is not in the mono face'

scratch mono2
edit apps/observatory-app/ui/src/details.rs 's/<td class="mono">\{row\.at\.to_rfc3339_millis\(\)\}/<td>{row.at.to_rfc3339_millis()}/'
expect_red "$d" "a sans timestamp cell" 'identifier-shaped text is not in the mono face'

scratch mono3
edit apps/observatory-app/ui/src/panel.rs 's/<td class="n">\{row\.sessions\}/<td>{row.sessions}/'
expect_red "$d" "a sans number cell" 'number-shaped text is not in the mono face'

scratch mono4
edit apps/observatory-app/ui/src/agents.rs 's/<td class="n">\{entry\.markers\}/<td class="mono">{entry.markers}/'
expect_red "$d" "a mono number cell that is not right-aligned (no class n)" 'lacks class="n"'

scratch mono5
edit apps/observatory-app/ui/src/library.rs 's/<div class="lib">//'
expect_red "$d" "a table outside the kit's .lib box" 'not inside the kit'"'"'s .lib box'

# The pending list holds comments only on the real tree: every rule 2 violation has been fixed.
if grep -v -e '^#' -e '^[[:space:]]*$' "$root/apps/observatory-app/kit-pending.txt" | grep -q .; then
  fail "kit-pending.txt lists a violation on the real tree"
fi

# A planted rule 2 violation is red until it is listed, tolerated once listed, and a listed line that no longer
# violates is red again.
scratch pend3
edit apps/observatory-app/ui/src/panel.rs 's/<td class="n">\{row\.sessions\}/<td>{row.sessions}/'
expect_red "$d" "an unlisted violation with a comment-only pending list" 'is not in the mono face'
grep -q -F 'panel.rs td `row.sessions`' "$tmp/err" || fail "an unlisted violation: the reasons do not carry the pending key: $(cat "$tmp/err")"

scratch pend4
edit apps/observatory-app/ui/src/panel.rs 's/<td class="n">\{row\.sessions\}/<td>{row.sessions}/'
printf 'panel.rs td `row.sessions`\n' >> "$d/apps/observatory-app/kit-pending.txt"
expect_ok "$d" "a violation listed in kit-pending.txt"

scratch pend5
printf 'panel.rs td `row.sessions`\n' >> "$d/apps/observatory-app/kit-pending.txt"
expect_red "$d" "a listed line that no longer violates" 'kit-pending.txt'

# --- the wiring --------------------------------------------------------------------------------------------------

grep -q '^check-kit:' "$root/Makefile" || fail "Makefile has no check-kit target"
grep -A2 '^check-kit:' "$root/Makefile" | grep -q 'scripts/check-kit.py' || fail "check-kit does not run scripts/check-kit.py"
grep -E '^check-all:' "$root/Makefile" | grep -q 'check-kit' || fail "check-all does not run check-kit"
grep -E '^check-scripts:' "$root/Makefile" | grep -q 'check-kit-lint' || fail "check-scripts does not run the kit lint's self-test"
grep -q 'kit_lint_test.sh' "$root/Makefile" || fail "Makefile never runs kit_lint_test.sh"
grep -E '^\.PHONY:' -A3 "$root/Makefile" | grep -q 'check-kit' || fail "check-kit is not declared .PHONY"
yq -e '.jobs.kit.steps[] | select(.run == "make check-kit")' "$root/.github/workflows/mission-branch.yml" > /dev/null 2>&1 || fail "the kit job does not run make check-kit"
yq -e '.jobs."mission-checks".needs | contains(["kit"])' "$root/.github/workflows/mission-branch.yml" > /dev/null 2>&1 || fail "mission-checks.needs does not list kit"
d="$tmp/ci1"
mkdir -p "$d"
sed 's/, kit\]/]/' "$root/.github/workflows/mission-branch.yml" > "$d/mb.yml"
sh "$root/scripts/check-ci-jobs.sh" "$d/mb.yml" "$root/.github/workflows/demo-gate.yml" > /dev/null 2>&1 && fail "check-ci-jobs accepts mission-checks without the kit job"

if [ "$failures" -ne 0 ]; then
  echo "kit_lint_test: $failures failure(s)"
  exit 1
fi
echo "kit_lint_test: ok"
