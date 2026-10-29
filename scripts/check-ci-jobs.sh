#!/bin/sh
# The aggregate job `mission-checks` is the single name other tooling depends on, so a Rust
# job that is not in its `needs` list can fail without failing the gate. Assert the Rust jobs
# added with the walking skeleton are listed. Two more facts are pinned because a miss is
# invisible until a run fails or the budget (3 min warm, SPEC section 6) is blown: the Linux rust
# job leaves the Tauri shell (apps/observatory-app/src-tauri, which needs the Tauri 2 Linux
# libraries: apt install and a larger cache) to the macOS bundle job, worktree
# pushes (`wt/**`) trigger the same checks, the core tests run once (the coverage job, B-056) and a push that
# changes only reports, the tool-call log or docs triggers nothing (the paths scripts/gate.sh leaves out of its key).
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
exit $status
