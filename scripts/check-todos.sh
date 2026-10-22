#!/bin/sh
# Every TODO or FIXME in tracked files must point at an issue (#123) or a
# backlog item (B-123) so debt is tracked, not scattered (ai/RULES.md rule 14).
set -eu
pattern='(TODO|FIXME)'
ok='(TODO|FIXME)\((#[0-9]+|B-[0-9]+)\)'
status=0
files=$(git ls-files | grep -Ev '^(build-log/|fixtures/streams/|scripts/check-todos\.sh$|ai/session/)')
for f in $files; do
  [ -f "$f" ] || continue
  grep -nE "$pattern" "$f" 2>/dev/null | grep -vE "$ok" | while IFS= read -r line; do
    echo "$f:$line"
    echo "  -> write TODO(#issue) or TODO(B-nnn)"
  done | grep . && status=1 || true
done
exit $status
