#!/bin/sh
# VAL-PROC-003 (target list): `make check` runs the named targets and none is dropped.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-003"
line=$(sed -n 's/^check-all:[[:space:]]*//p' Makefile 2> /dev/null)
[ -n "$line" ] || { fail "Makefile has no check-all target"; finish "$ids"; }
for t in check-docs check-links check-workflows check-file-size check-todos check-decisions-index check-secrets redact-check check-wasm fmt-check lint test deny machete replay-fixtures; do
  printf ' %s ' "$line" | grep -q " $t " || fail "check-all does not run $t"
done
finish "$ids"
