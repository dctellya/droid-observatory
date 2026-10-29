#!/bin/sh
# VAL-PROC-017, VAL-GATE-031 (index clause), VAL-PROC-024, VAL-PROC-025: the three quick targets exit 0.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-017 VAL-GATE-031 VAL-PROC-024 VAL-PROC-025"
need_tool make
for t in check-decisions-index check-file-size check-todos; do
  make --no-print-directory -s "$t" > /dev/null 2>&1 || fail "make $t exits non-zero"
done
finish "$ids"
