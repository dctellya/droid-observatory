# D-025: The build is paid in series: check in the inner loop, test at integration, coverage at the milestone

**Date:** 2026-10-23
**Status:** accepted (owner rule F-029 at the M0 to M1 boundary; `ai/RULES.md` rule 23)

## Context

A mission runs features one after another, so every compile a worker triggers is wall clock nobody
shares. All cargo builds also serialize on the shared `CARGO_TARGET_DIR` lock (AGENTS.md "Runtime
limits"), and `cargo llvm-cov` loads the machine (1-min load 3.3 → 14.6 on 10 cores in the readiness
run). In M0 the workers' handoffs listed `make check`, `cargo test --workspace` and `cargo llvm-cov`
runs inside features, and the validators repeated them. The owner asked for a rule that puts each
command where it is paid once.

The owner wrote the rule as number "20". Rule 20 is the models invariant (D-018), so it landed as
rule 23 with no renumbering (SPEC §9 F-029).

## Decision

Rule 23, verbatim from the owner: "**The build is paid in series: check in the inner loop, test at
integration, coverage at the milestone.** A mission runs features one after another, so every compile a
worker triggers is wall clock nobody shares. Workers use the fastest check the language offers and the
tests of the crate or package they touch; the full test suite runs once per milestone at integration,
by the integrator; coverage runs only there, alone. The readiness report states measured build times
and the per-feature compile cost, and each milestone's usage report records the compile share of its
wall clock."

Concretely:

- **Inner loop (a worker, per feature):** `cargo check -p <crate>` and `cargo test -p <crate>` for the
  crates the feature touches, plus `cargo fmt --all -- --check` and
  `cargo clippy -p <crate> --all-targets -- -D warnings` once before handoff. Never `make check`,
  `make test`, `cargo test --workspace` or `cargo llvm-cov` inside a feature. Infra exception: the one
  Makefile or CI target the feature changed. `cargo-nextest` is not installed
  (`cargo nextest --version` → "no such command" on 2026-10-23) and is not to be installed.
- **Integration (once per milestone, by the integrator):** `make check`, which runs
  `cargo test --workspace`. The scrutiny validator's programmatic gate counts as milestone-level.
- **Coverage (once per milestone, alone):** `cargo llvm-cov -p observatory-core --all-features
  --fail-under-lines 80 --summary-only`, after `make check` has finished and with no other cargo,
  trunk or tauri process running.
- **Measurement duties.** Every cargo or trunk invocation goes into the handoff
  `verification.commandsRun` with its wall time pasted. The readiness report states the measured build
  times and the per-feature compile cost
  ([reports/readiness/2026-10-23-build-times.md](../../reports/readiness/2026-10-23-build-times.md)).
  Each milestone's usage report has a "Compile share" section: the summed wall times of the
  milestone's cargo, make-wrapped cargo and trunk invocations over the milestone wall clock, both
  operands shown, with "not recoverable" where a session left no wall time.

Measured warm on 2026-10-23 (pasted from the readiness report): `make check` real 30.383 s;
`cargo check -p observatory-core` real 1.268 s; `cargo test -p observatory-core` real 11.631 s;
`cargo llvm-cov` alone real 54.911 s.

## Alternatives considered

- **Let every worker run `make check` before handoff (the S and M0 practice).** Rejected: the full
  suite runs once per feature instead of once per milestone, and each run takes the shared target lock.
- **Install `cargo-nextest` for faster test runs.** Rejected by the owner's instruction; the crate-scoped
  `cargo test -p` already keeps the loop short.
- **Run coverage in every feature.** Rejected: it is the heaviest command (user time 5 m 57.653 s for
  54.911 s real) and it competes for CPU with every other process.

## Consequences

- `ai/RULES.md` gains rule 23; AGENTS.md "Setup, build, test" carries the summary; SPEC §2, §6 and §7
  state it with the evidence.
- Workers hand off `coverageTotal: n/a`; the integrator's coverage run is the only local one.
- The hook `.factory/hooks/log-tool-call.sh` records no duration (only the UTC completion second of each
  call), so the per-feature compile cost depends on wall times pasted into the handoffs; M0 handoffs
  mostly lack them (see the readiness report), which is why the duty is now a rule.
- Related: [D-018](D-018-mission-models-are-invariant.md) (rule 20, the number the owner meant to
  extend).
