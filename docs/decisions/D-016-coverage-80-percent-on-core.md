# D-016: Line coverage of observatory-core, 80 % from M0, cargo-llvm-cov

**Date:** 2026-10-23
**Status:** accepted (SPEC §6 as settled in B-005; wired in S, enforced from the M0 integration)

## Context

B-005 asked for a coverage threshold. `observatory-core` holds every piece of logic that can be
wrong silently (mapping, time states, cost). The CLI is thin and has its own tests, and the UI
crate compiles to wasm and holds no logic. At S the core is small: the scaffold reported
95.92 % lines (`make coverage`), but a percentage over so little code means little.

## Decision

- Component: `cargo-llvm-cov` 0.9.1, `cargo llvm-cov -p observatory-core --all-features
  --fail-under-lines <COVERAGE_MIN> --summary-only`, in its own `coverage` job in
  `mission-branch.yml` and as `make coverage`. The job installs `llvm-tools-preview` on the
  toolchain pinned in `rust-toolchain.toml`.
- Threshold: 80 % lines on `observatory-core` only. S reports it with `COVERAGE_MIN` at `"0"`;
  the M0 integration sets it to `"80"`. Raising it is a backlog item with a reason; lowering it is
  a decision file.
- `cargo llvm-cov` runs alone on the machine (load 3.3 to 14.6 on 10 cores in the readiness run).
- If the warm `mission-checks` time exceeds 3 minutes once coverage joins, coverage moves to the
  final pull request only, by decision file (SPEC §7).

## Alternatives

`tarpaulin` (ptrace on Linux only, not the macOS developer machine); a threshold on the whole
workspace (the UI crate drags it down for no information); branch coverage (nightly only).

## Consequences

The coverage job hard-codes the toolchain version for `dtolnay/rust-toolchain`, so it must change
together with `rust-toolchain.toml` (BACKLOG B-018). IO adapters that need a live `droid` or GitHub
cannot all run in CI; that is the 20 % of room.
