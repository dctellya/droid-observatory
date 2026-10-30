#!/bin/sh
# VAL-PROC-007 (fixtures clause): gitleaks over the committed stream fixtures finds nothing. The scan reads 15 MB and
# takes about 12 s, so a clean verdict is remembered per committed fixtures tree in the temp directory; a dirty or
# non-git fixtures directory is always scanned.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-007"
need_tool gitleaks
dir=${CONTRACT_FIXTURES:-fixtures/streams}
[ -d "$dir" ] || { fail "$dir is missing"; finish "$ids"; }
stamp=
if tree=$(git rev-parse "HEAD:$dir" 2> /dev/null) && [ -z "$(git status --porcelain -- "$dir" 2> /dev/null)" ]; then
  stamp="${TMPDIR:-/tmp}/contract-checks-secrets-$tree"
fi
if [ -n "$stamp" ] && [ -f "$stamp" ]; then
  finish "$ids"
fi
if gitleaks detect --no-git --no-banner --redact --source "$dir" > /dev/null 2>&1; then
  [ -z "$stamp" ] || : > "$stamp"
else
  fail "gitleaks detect reports a leak in $dir"
fi
finish "$ids"
