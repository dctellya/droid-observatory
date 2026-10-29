#!/bin/sh
# G2 (the eight SPEC-named M4 tests, the view-level gate tests): named cargo tests of group gate in g2-tests.tsv, read from the gate test log.
. "$(dirname "$0")/named-tests.sh"
run_group gate
