#!/bin/sh
# Every TODO or FIXME marker in source files must point at an issue (#123) or a
# backlog item (B-123) so debt is tracked, not scattered (ai/RULES.md rule 14).
# Prose (markdown), data (json, jsonl) and logs are not scanned: they talk about
# markers without being one.
set -eu
pattern='(TODO|FIXME)'
ok='(TODO|FIXME)\((#[0-9]+|B-[0-9]+)\)'
status=0
files=$(git ls-files \
  | grep -E '(\.(rs|toml|ya?ml|sh|js|ts|tsx|html|css|scss|sql|py)$|^Makefile$|^\.githooks/|^\.factory/hooks/)' \
  | grep -Ev '^(build-log/|fixtures/|scripts/check-todos\.sh$)')
for f in $files; do
  [ -f "$f" ] || continue
  hits=$(grep -nE "$pattern" "$f" 2>/dev/null | grep -vE "$ok" || true)
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" | sed "s|^|$f:|"
    echo "  -> write TODO(#issue) or TODO(B-nnn)"
    status=1
  fi
done
exit $status
