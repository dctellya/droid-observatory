#!/bin/sh
# VAL-METRIC-017 (mechanical half): integration-lane entries in Details read "integrated in mission", never "merged".
# Event identifiers such as integration_merged are not wording. The check waits (pending) until the tab's strings exist.
. "$(dirname "$0")/lib.sh"
ids="VAL-METRIC-017"
files="apps/observatory-app/ui/src/details.rs crates/observatory-core/src/view/details.rs"
# shellcheck disable=SC2086 # the file list is a word list
if ! grep -qs 'integrated in mission' $files; then
  pending "no 'integrated in mission' string yet"
else
  for f in $files; do
    [ -f "$f" ] || continue
    hits=$(sed -E 's/[A-Za-z_]*[Ii]ntegration_?[Mm]erged[A-Za-z_]*//g' "$f" | grep -n -i -E '"[^"]*\bmerged\b[^"]*"')
    [ -z "$hits" ] || fail "$f has a string saying merged: $hits"
  done
fi
finish "$ids"
