#!/bin/sh
# VAL-METRIC-022 and VAL-PROC-006 (m5 row): the M0 fixture files and the mapping sources equal the hashes pinned in
# frozen-hashes.txt. A line is `<sha256>  <path>` or `dir <path> <sha256 of the sorted per-file hashes>`; a refreeze
# is a reviewed edit of that file next to the SPEC section 10 row that names it.
. "$(dirname "$0")/lib.sh"
ids="VAL-METRIC-022 VAL-PROC-006"
pins=${CONTRACT_FROZEN:-$(dirname "$0")/frozen-hashes.txt}
[ -f "$pins" ] || { fail "pin list $pins is missing"; finish "$ids"; }
while IFS= read -r line; do
  case "$line" in '' | '#'*) continue ;; esac
  case "$line" in
    "dir "*)
      d=$(printf '%s' "$line" | awk '{print $2}')
      want=$(printf '%s' "$line" | awk '{print $3}')
      have=$(find "$d" -type f 2> /dev/null | LC_ALL=C sort | xargs shasum -a 256 2> /dev/null | shasum -a 256 | cut -d' ' -f1)
      [ "$have" = "$want" ] || fail "directory $d changed (pinned $want, now $have)"
      ;;
    *)
      want=$(printf '%s' "$line" | awk '{print $1}')
      f=$(printf '%s' "$line" | awk '{print $2}')
      have=$(shasum -a 256 "$f" 2> /dev/null | cut -d' ' -f1)
      [ "$have" = "$want" ] || fail "$f changed (pinned $want, now ${have:-missing})"
      ;;
  esac
done < "$pins"
# A new file under a frozen directory or the M0 children is a change too.
for d in fixtures/streams/2026-10-22-observatory-v1-m0.children; do
  [ -d "$d" ] || continue
  for f in "$d"/*; do
    grep -q "  $f\$" "$pins" || fail "$f is not pinned"
  done
done
finish "$ids"
