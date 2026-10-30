#!/bin/sh
# VAL-GATE-011 (mechanical half): the PR lookup code reads the target branch from config; the literal "main" lives
# only in config.rs. The two files land with the gate lane (M4); until then the check reports pending.
. "$(dirname "$0")/lib.sh"
ids="VAL-GATE-011"
for f in crates/observatory-core/src/ingest/github.rs crates/observatory-core/src/ingest/forge.rs; do
  if [ ! -f "$f" ]; then
    pending "$f absent"
    continue
  fi
  hits=$(grep -n '"main"' "$f")
  [ -z "$hits" ] || fail "$f has a literal \"main\": $hits"
done
finish "$ids"
