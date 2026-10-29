#!/bin/sh
# The aggregate job `mission-checks` is the single name other tooling depends on, so a Rust
# job that is not in its `needs` list can fail without failing the gate. Assert the Rust jobs
# added with the walking skeleton are listed. Two more facts are pinned because a miss is
# invisible until a run fails or the budget (3 min warm, SPEC section 6) is blown: the Linux rust
# job leaves the Tauri shell (apps/observatory-app/src-tauri, which needs the Tauri 2 Linux
# libraries: apt install and a larger cache) to the macOS bundle job, worktree
# pushes (`wt/**`) trigger the same checks, the core tests run once (the coverage job, B-056) and a push that
# changes only reports, the tool-call log or docs triggers nothing (the paths scripts/gate.sh leaves out of its key).
# The demo target's workflow (demo-gate.yml) is pinned too: a missing trigger type means a label change on the demo pull
# request starts no run and the check keeps a stale result.
set -eu
wf="${1:-.github/workflows/mission-branch.yml}"
demo_wf="${2:-.github/workflows/demo-gate.yml}"
required="machete coverage contract kit"

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
' "$wf" | grep -v '^[[:space:]]*#')
if ! printf '%s\n' "$rust_job" | grep -q 'cargo clippy --workspace --exclude observatory-app'; then
  echo "check-ci-jobs: the rust job does not run clippy with --exclude observatory-app"
  status=1
fi
if ! printf '%s\n' "$rust_job" | grep -q 'cargo clippy -p observatory-ui --target wasm32-unknown-unknown'; then
  echo "check-ci-jobs: the rust job does not run the wasm32 clippy of observatory-ui"
  status=1
fi
if ! printf '%s\n' "$rust_job" | grep -q 'cargo test -p observatory-cli'; then
  echo "check-ci-jobs: the rust job does not test observatory-cli"
  status=1
fi
if printf '%s\n' "$rust_job" | grep -q 'cargo test .*observatory-core'; then
  echo "check-ci-jobs: the rust job tests observatory-core, which the coverage job runs (B-056)"
  status=1
fi
coverage_job=$(awk '
  /^  [A-Za-z0-9_-]+:[[:space:]]*$/ { in_job = ($1 == "coverage:") }
  in_job { print }
' "$wf" | grep -v '^[[:space:]]*#')
if ! printf '%s\n' "$coverage_job" | grep -q 'cargo llvm-cov -p observatory-core'; then
  echo "check-ci-jobs: the coverage job does not run llvm-cov over observatory-core (it is the only run of the core tests)"
  status=1
fi
if printf '%s\n' "$rust_job" | grep -q 'apt-get'; then
  echo "check-ci-jobs: the rust job installs apt packages; the Tauri shell belongs to the bundle job"
  status=1
fi

push_block=$(awk '
  /^on:/ { in_on = 1; next }
  /^[A-Za-z]/ { in_on = 0 }
  in_on && /^  push:/ { in_push = 1; next }
  in_on && /^  [A-Za-z_]/ { in_push = 0 }
  in_push { print }
' "$wf")
if ! printf '%s\n' "$push_block" | grep -Fq '"wt/**"'; then
  echo "check-ci-jobs: on.push.branches does not list \"wt/**\""
  status=1
fi
for ignored in 'reports/**' 'build-log/**' 'docs/**'; do
  if ! printf '%s\n' "$push_block" | grep -Fq "\"$ignored\""; then
    echo "check-ci-jobs: on.push.paths-ignore does not list \"$ignored\""
    status=1
  fi
done

if [ ! -f "$demo_wf" ]; then
  echo "check-ci-jobs: $demo_wf does not exist"
  exit 1
fi
demo_pr=$(awk '
  /^on:/ { in_on = 1; next }
  /^[A-Za-z]/ { in_on = 0; in_pr = 0 }
  in_on && /^  pull_request:/ { in_pr = 1; next }
  in_on && /^  [A-Za-z_]/ { in_pr = 0 }
  in_pr { print }
' "$demo_wf")
demo_on=$(awk '/^on:/ { in_on = 1; next } /^[A-Za-z]/ { in_on = 0 } in_on { print }' "$demo_wf")
if ! printf '%s\n' "$demo_pr" | grep -Eq '^      - demo/base[[:space:]]*$'; then
  echo "check-ci-jobs: $demo_wf on.pull_request.branches does not list demo/base"
  status=1
fi
if [ "$(printf '%s\n' "$demo_pr" | sed -n '/branches:/,/types:/p' | grep -c '^      - ')" != 1 ]; then
  echo "check-ci-jobs: $demo_wf on.pull_request.branches lists more than demo/base"
  status=1
fi
for type in opened synchronize labeled unlabeled; do
  if ! printf '%s\n' "$demo_pr" | sed -n '/types:/,$p' | grep -Eq "^      - ${type}[[:space:]]*\$"; then
    echo "check-ci-jobs: $demo_wf on.pull_request.types does not list $type"
    status=1
  fi
done
if [ "$(printf '%s\n' "$demo_pr" | sed -n '/types:/,$p' | grep -c '^      - ')" != 4 ]; then
  echo "check-ci-jobs: $demo_wf on.pull_request.types is not exactly opened, synchronize, labeled, unlabeled"
  status=1
fi
if printf '%s\n' "$demo_on" | grep -Eq '^  (push|pull_request_target|workflow_dispatch|schedule):'; then
  echo "check-ci-jobs: $demo_wf has a trigger besides pull_request"
  status=1
fi
if ! grep -Eq '^  demo-gate:[[:space:]]*$' "$demo_wf"; then
  echo "check-ci-jobs: $demo_wf has no job demo-gate"
  status=1
fi
if [ "$(awk '/^jobs:/ { in_jobs = 1; next } /^[A-Za-z]/ { in_jobs = 0 } in_jobs && /^  [A-Za-z0-9_-]+:[[:space:]]*$/ { n++ } END { print n + 0 }' "$demo_wf")" != 1 ]; then
  echo "check-ci-jobs: $demo_wf must hold exactly one job"
  status=1
fi
if ! awk '/^permissions:/ { p = 1; next } /^[A-Za-z]/ { p = 0 } p && /^  contents: read[[:space:]]*$/ { ok = 1 } END { exit !ok }' "$demo_wf"; then
  echo "check-ci-jobs: $demo_wf permissions is not contents: read"
  status=1
fi
exit $status
