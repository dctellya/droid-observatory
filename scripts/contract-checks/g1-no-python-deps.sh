#!/bin/sh
# VAL-TOOL-005 (grep half): the usage report imports only the standard library.
. "$(dirname "$0")/lib.sh"
ids="VAL-TOOL-005"
hits=$(grep -lE 'import (requests|pandas|numpy)' scripts/usage-report.py 2> /dev/null)
[ -z "$hits" ] || fail "third-party import in $hits"
finish "$ids"
