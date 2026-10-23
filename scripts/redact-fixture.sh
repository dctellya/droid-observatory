#!/bin/sh
# Redacts one session-stream fixture in place (owner PII rule 2026-10-23, RULES rule 22).
#
#   scripts/redact-fixture.sh fixtures/streams/<date>-<slug>.jsonl
#
# Rewrites the root .jsonl, <base>.settings.json, <base>.capture.json and every file in
# <base>.children/ so that
#   - every email address becomes user@example.invalid; an address of 20 or more bytes is
#     padded with "_" in the local part to keep its length, a shorter one takes the token
#     and the line gets longer;
#   - every /Users/dct becomes /home/user (same length);
#   - every -Users-dct, the session-directory form of the same path, becomes -home-user
#     (same length).
# Lines are never added or removed and nothing else changes. Running it on redacted
# files rewrites nothing (the output is compared first), so it is idempotent. Byte offsets
# (SourcePtr, goldens) are computed against the redacted bytes, the ones the store indexes.
set -eu

[ "$#" -eq 1 ] || { echo "usage: $0 <fixture-root.jsonl>" >&2; exit 2; }
root=$1
case "$root" in
  *.jsonl) ;;
  *) echo "redact-fixture: expected a .jsonl fixture root, got: $root" >&2; exit 2 ;;
esac
[ -f "$root" ] || { echo "redact-fixture: no such file: $root" >&2; exit 2; }
base=${root%.jsonl}

tmp=$(mktemp "${TMPDIR:-/tmp}/redact-fixture.XXXXXX")
trap 'rm -f "$tmp"' EXIT INT TERM

# Byte-wise on purpose: the fixtures are JSON text and must keep their exact framing.
# An address cannot start right after a backslash: in `\n128x128@2x.png` the `n` belongs to the JSON
# escape, and swallowing it turned the line into invalid JSON.
redact_stream() {
  LC_ALL=C perl -pe '
    s{(?<!\\)([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})}{
      my $m = $1;
      $m =~ /\@example\.invalid\z/ ? $m
        : length($m) >= 20 ? "user" . ("_" x (length($m) - 20)) . "\@example.invalid"
        : "user\@example.invalid"
    }ge;
    s{/Users/dct}{/home/user}g;
    s{-Users-dct}{-home-user}g;
  '
}

redact_file() {
  redact_stream < "$1" > "$tmp"
  if ! cmp -s "$1" "$tmp"; then
    cat "$tmp" > "$1"
    echo "redacted $1"
  fi
}

for f in "$root" "$base.settings.json" "$base.capture.json"; do
  [ -f "$f" ] && redact_file "$f"
done
if [ -d "$base.children" ]; then
  for f in "$base.children"/*; do
    [ -f "$f" ] && redact_file "$f"
  done
fi
exit 0
