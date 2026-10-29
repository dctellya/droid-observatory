#!/bin/sh
# VAL-PROC-008: every commit the branch adds has a subject of at most 72 characters, no WIP/fixup marker and the
# factory-droid co-author trailer. The contract's `grep -vc` over `%H %(trailers)` also counts the blank lines of
# multi-line output, so the trailer is read per commit instead.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-008"
base=$(range_base) || skip "no mission branch ref" "$ids"
bad=$(mktemp "${TMPDIR:-/tmp}/contract-commit-format.XXXXXX")
bot='factory-droid[bot] <138933559+factory-droid[bot]@users.noreply.github.com>'
git log --format='%x01%h %s%n%(trailers:key=Co-authored-by,valueonly)' "$base..HEAD" 2> /dev/null |
  awk -v bot="$bot" 'BEGIN { RS = "\001" } NR > 1 {
    split($0, lines, "\n"); head = lines[1]; sha = substr(head, 1, 7); subj = substr(head, 9)
    if (length(subj) > 72) print sha " subject is " length(subj) " characters"
    if (subj ~ /^(WIP|wip|fixup!|squash!)/) print sha " subject starts with a WIP marker"
    if (index($0, bot) == 0) print sha " has no factory-droid co-author trailer"
  }' > "$bad" 2> /dev/null
while IFS= read -r line; do
  excepted "${line%% *}" || fail "$line"
done < "$bad"
rm -f "$bad"
finish "$ids"
