# reports

Numbers pasted from the tools that produced them (ai/RULES.md rule 7).

- `readiness/<date>-<label>.json`: Agent Readiness reports (`/readiness-report`). Baseline
  `2026-10-22-baseline.json`: Level 1, 5 of 65 evaluated criteria.
  `2026-10-22-scaffold.json`: Level 3, 31 of 65, after the readiness scaffold (PR #1).
  `2026-10-22-actions.json`: 36 of 66, after the action items (PR #3; code scanning, release,
  devcontainer, runbooks).
- `readiness/<date>-<label>.md`: pre-mission readiness findings from the worker sessions.
  `2026-10-22-dependency-readiness.md`: toolchain, crates, OTLP, GitHub token, session schema.
  `2026-10-22-validation-readiness.md`: browser and Tauri validation path, concurrency limits.
  `2026-10-23-build-times.md`: measured warm build times (`make check` 30.383 s, `cargo check -p`
  1.268 s, `cargo test -p` 11.631 s, `cargo llvm-cov` alone 54.911 s) and the M0 per-feature
  compile cost (F-029, rule 23).
- `usage/<milestone>.md`: tokens per role and model, waste by cause, wall time, per
  integrated milestone (written by the orchestrator at integration).
  `usage/s.md`: milestone S, 13.21 USD-eq of the 40 cap on workers and validators.
  `usage/m0.md`: milestone M0, 49.05 USD-eq of the 60 cap priced so far; orchestrator delta 66.87 against 32;
  from F-029 a "Compile share" section.
  `usage/m1.md`: milestone M1, 110.38 USD-eq of the 110 cap on workers, validators and orchestrator
  subagents; orchestrator delta 115.74 against 22; the per-role token table from the CLI log (rule 7, D-029),
  throttled time and the compile share.
- `ci/<date>-<milestone>.md`: the `mission-checks` runs on the integration pushes: run ids, wall
  times, per-job durations, verdict against the 3-minute budget. `ci/2026-10-23-s.md`: S, red.
  `ci/2026-10-23-m0.md`: M0, green, 1 min 45 s. `ci/2026-10-24-m1.md`: M1, green, 3 min 48 s (over budget).
- `readiness/<date>-<milestone>.json`: Agent Readiness report at an integration.
  `2026-10-23-s.json`: 42 of 66, hand re-evaluation of the criteria S changed (see its `method`).
  `2026-10-23-m0.json`: 44 of 66, hand re-evaluation of the criteria M0 changed.
  `2026-10-24-m1.json`: 45 of 66, hand re-evaluation of the criteria M1 changed; `2026-10-24-m1.md`: warm
  build times at the M1 integration (`make check` 92.700 s, `cargo llvm-cov` alone 121.731 s) and per-feature compile cost.
- `screenshots/<date>-<milestone>.png`: the owner's unlocked-display screenshot per integration.
  Expected at S: `screenshots/2026-10-23-s.png`; at M0: `screenshots/2026-10-23-m0.png`; at M1: `screenshots/2026-10-24-m1.png`.
