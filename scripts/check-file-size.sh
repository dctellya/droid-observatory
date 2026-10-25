#!/bin/sh
# Fails if any tracked file exceeds MAX_FILE_KB. Captured stream fixtures and the
# build log are exempt: they are data, their size is the point.
set -eu
MAX_FILE_KB=${MAX_FILE_KB:-200}
limit=$((MAX_FILE_KB * 1024))
status=0
for f in $(git ls-files); do
  case "$f" in
    fixtures/streams/*|build-log/*|Cargo.lock|*/package-lock.json|*.png|*.webp|*.mp4) continue ;;
  esac
  [ -f "$f" ] || continue
  size=$(wc -c < "$f")
  if [ "$size" -gt "$limit" ]; then
    echo "too large: $f ($((size / 1024)) KB > ${MAX_FILE_KB} KB)"
    status=1
  fi
done
exit $status
