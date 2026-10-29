#!/bin/sh
# VAL-PROC-021 and VAL-PROC-006 (m4 row): main has no commit since the mission started, still points at c8524c2, and the
# main-gate ruleset equals the committed copy byte for byte.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-021 VAL-PROC-006"
need_network "$ids"
n=$(gh api "repos/$REPO/commits?sha=main&since=2026-10-22T14:45:41Z" --jq length 2> /dev/null)
[ "$n" = 0 ] || fail "main has '$n' commit(s) since 2026-10-22T14:45:41Z, wanted 0"
sha=$(gh api "repos/$REPO/git/ref/heads/main" --jq .object.sha 2> /dev/null)
[ "$sha" = c8524c2641c00376340640859bb5381763c59ba6 ] || fail "main is at '$sha', wanted c8524c2641c00376340640859bb5381763c59ba6"
live=$(mktemp "${TMPDIR:-/tmp}/contract-main-gate.XXXXXX")
gh api "repos/$REPO/rulesets/25636994" > "$live" 2> /dev/null
cmp -s "$live" scripts/contract-checks/main-gate.json || fail "main-gate ruleset 25636994 differs from scripts/contract-checks/main-gate.json"
rm -f "$live"
finish "$ids"
