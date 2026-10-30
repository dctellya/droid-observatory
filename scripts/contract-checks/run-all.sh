#!/bin/sh
# make check-contract: run every contract check in parallel and print their lines in name order, then one summary line.
# Exit 1 when any check prints FAIL or no verdict. CONTRACT_SKIP="name name" skips checks (the mission-branch CI skips
# the ones that need a tool its runner lacks). The G8 and G10 clauses are gate-record stages, see README.md.
set -u
dir=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
now() { LC_ALL=C perl -MTime::HiRes=time -e 'printf "%.2f\n", time' 2> /dev/null || date +%s; }
t0=$(now)
for f in "$dir"/g[0-9]*.sh; do
  name=$(basename "$f" .sh)
  case " ${CONTRACT_SKIP:-} " in
    *" $name "*)
      printf '%s skip (CONTRACT_SKIP) -\n' "$name" > "$tmp/$name.out"
      echo 0 > "$tmp/$name.rc"
      continue
      ;;
  esac
  (
    sh "$f" > "$tmp/$name.out" 2> "$tmp/$name.err"
    echo $? > "$tmp/$name.rc"
  ) &
done
wait
ok=0
skipped=0
failed=0
for f in "$dir"/g[0-9]*.sh; do
  name=$(basename "$f" .sh)
  line=$(cat "$tmp/$name.out" 2> /dev/null)
  rc=$(cat "$tmp/$name.rc" 2> /dev/null || echo 1)
  case "$line" in
    "$name ok "*) [ "$rc" -eq 0 ] && ok=$((ok + 1)) || failed=$((failed + 1)) ;;
    "$name skip "*) skipped=$((skipped + 1)) ;;
    "$name FAIL "*) failed=$((failed + 1)) ;;
    *)
      line="$name FAIL (no verdict) -"
      failed=$((failed + 1))
      ;;
  esac
  printf '%s\n' "$line"
  [ "$rc" -eq 0 ] || cat "$tmp/$name.err" >&2 2> /dev/null
done
t1=$(now)
printf 'contract checks: %s ok, %s skipped, %s FAIL in %ss\n' "$ok" "$skipped" "$failed" "$(LC_ALL=C awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.1f", b - a }')"
[ "$failed" -eq 0 ]
