#!/bin/sh
# The aggregate job `mission-checks` is the single name other tooling depends on, so a Rust
# job that is not in its `needs` list can fail without failing the gate. Assert the Rust jobs
# added with the walking skeleton are listed. Two more facts are pinned here because a miss is
# invisible until a run fails: the rust job must install the Tauri 2 Linux libraries (clippy
# compiles apps/observatory-app/src-tauri, whose glib-sys needs them), and worktree pushes
# (`wt/**`) must trigger the same checks.
set -eu
wf="${1:-.github/workflows/mission-branch.yml}"
required="machete coverage"

needs=$(awk '
  /^  [A-Za-z0-9_-]+:[[:space:]]*$/ { in_job = ($1 == "mission-checks:") }
  in_job && /^    needs:/ { print; exit }
' "$wf")

if [ -z "$needs" ]; then
  echo "check-ci-jobs: no 'needs:' line found in the mission-checks job of $wf"
  exit 1
fi

status=0
for job in $required; do
  if ! printf '%s\n' "$needs" | grep -Eq "[[:space:]\[,]${job}[],[:space:]]|[[:space:]\[,]${job}\$"; then
    echo "check-ci-jobs: mission-checks.needs does not list '$job' ($needs)"
    status=1
  fi
done

rust_job=$(awk '
  /^  [A-Za-z0-9_-]+:[[:space:]]*$/ { in_job = ($1 == "rust:") }
  in_job { print }
' "$wf")
if ! printf '%s\n' "$rust_job" | grep -v '^[[:space:]]*#' | grep -q 'libwebkit2gtk-4.1-dev'; then
  echo "check-ci-jobs: the rust job has no step that installs libwebkit2gtk-4.1-dev"
  status=1
fi

push_branches=$(awk '
  /^on:/ { in_on = 1; next }
  /^[A-Za-z]/ { in_on = 0 }
  in_on && /^  push:/ { in_push = 1; next }
  in_on && /^  [A-Za-z_]/ { in_push = 0 }
  in_push && /^[[:space:]]+- / { print }
' "$wf")
if ! printf '%s\n' "$push_branches" | grep -Fq '"wt/**"'; then
  echo "check-ci-jobs: on.push.branches does not list \"wt/**\""
  status=1
fi
exit $status
