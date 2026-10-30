#!/bin/sh
# G2 (the telemetry tests): named cargo tests of group telemetry in g2-tests.tsv, read from the gate test log.
. "$(dirname "$0")/named-tests.sh"
run_group telemetry
