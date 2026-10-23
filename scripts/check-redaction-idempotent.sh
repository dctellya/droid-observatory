#!/bin/sh
# Runs the redaction script on a copy of every committed fixture and fails if anything
# changes: a committed fixture is already redacted and a second pass must be a no-op.
set -eu
copy=$(mktemp -d "${TMPDIR:-/tmp}/redact-check.XXXXXX")
trap 'rm -rf "$copy"' EXIT INT TERM
cp -R fixtures/streams "$copy/streams"
status=0
for root in "$copy"/streams/*.jsonl; do
  [ -f "$root" ] || continue
  sh scripts/redact-fixture.sh "$root" >/dev/null
done
if ! diff -rq fixtures/streams "$copy/streams"; then
  echo "redact-check: a second redaction pass changed a fixture" >&2
  status=1
fi
[ "$status" -eq 0 ] && echo "redact-check: second pass is a no-op"
exit "$status"
