#!/bin/sh
# The aggregate job `mission-checks` is the single name other tooling depends on, so a Rust
# job that is not in its `needs` list can fail without failing the gate. Assert the Rust jobs
# added with the walking skeleton are listed.
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
exit $status
