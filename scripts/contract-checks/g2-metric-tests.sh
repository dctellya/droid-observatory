#!/bin/sh
# G2 (the M5 metrics engine, cost and throttled tests, the frozen mapping test target): named cargo tests of group metric in g2-tests.tsv, read from the gate test log.
. "$(dirname "$0")/named-tests.sh"
run_group metric
