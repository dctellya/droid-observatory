# D-022: Linux CI excludes the Tauri shell

**Date:** 2026-10-23
**Status:** accepted

## Context

Runs 39919592562, 39920664605 and 39921065456 on mission/observatory-v1 failed in the `rust` job's clippy step with `The system library glib-2.0 required by crate glib-sys was not found`: apps/observatory-app/src-tauri needs the Tauri 2 Linux libraries on ubuntu-latest. gate.yml reuses mission-branch.yml, so the final PR would fail the same way. The first fix installed libwebkit2gtk-4.1-dev, build-essential, curl, wget, file, libxdo-dev, libssl-dev, libayatana-appindicator3-dev and librsvg2-dev with apt. Run 39924948471 on wt/m0 (24e3b10) was green but took 4m24s; the workflow_dispatch rerun 39925522835 on the same sha took 4m50s (rust job 4m34s: apt 2m34s, rust-cache restore 1m33s, clippy+test 10 s once cached; the first run's apt step took 58 s). Both exceed the 3 min warm budget (SPEC section 6, rule 6).

## Decision

The Linux `rust` job in mission-branch.yml does not build the Tauri shell and installs no apt packages. It runs `make fmt-check`; `cargo clippy --workspace --exclude observatory-app --all-targets --all-features -- -D warnings -W clippy::cognitive_complexity`; `cargo clippy -p observatory-ui --target wasm32-unknown-unknown --all-targets --all-features -- -D warnings`; `cargo test -p observatory-core -p observatory-cli --all-features`; `make replay-fixtures`. The shell is covered by the macOS `bundle` job (`make app-build`, run through gate.yml on the PR to main) and locally by `make check`, which still lints and tests the whole workspace. mission-branch.yml also triggers on push to `wt/**`, so a worktree push runs the same checks. scripts/check-ci-jobs.sh pins all of this (`make check-workflows`).

## Alternatives

(1) Keep the apt step (rejected: 4m24s and 4m50s, over budget; apt time varies from 58 s to 2m34s). (2) Cache the apt packages with a third-party action (rejected: new action to pin and trust, and the rust-cache restore alone took 1m33s). (3) Move the shell clippy to the final PR only (rejected: the PR then fails late, and the bundle job already builds the shell there). (4) Drop wt/** triggers (rejected: S never saw CI on its worktree).

## Consequences

The shell crate gets no clippy -D warnings on Linux and no unit tests in CI; a shell-only lint regression is caught by `make check` locally and by the bundle job compiling it on macOS, not by mission-checks. Final run 39926453241 (79712c2): mission-checks success, wall 2m26s, rust job 2m10s, under the budget. Revisit when the shell has logic worth testing in CI (M1 onward): then add the apt step to a separate non-gating job or move shell tests into core (BACKLOG B-024).
