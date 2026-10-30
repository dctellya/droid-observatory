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
  throttled time and the compile share; the table was backfilled at the M2 integration.
  `usage/m2.md`: milestone M2, 120.61 USD-eq of the 50 cap on workers, validators and orchestrator subagents;
  the orchestrator's spend split (waits, decisions, narration) against the 30 USD-eq stop, the mission-level 320 cap,
  the model-fallback (void) sessions, the self-check missions, the reconciliation with Mission Control (F-045) and the
  tokens per role.
  `usage/m3.md`: milestone M3, written by `make usage M=m3`: window total 210.98 USD-eq against the cap 110, the
  orchestrator 68.81 against 30, mission to date 990.09 (past 950), the gates run and served from cache, the feature size
  table and the scorecard (re-issued waits, turns per feature, validation overhead, model-check lines).
- `ci/<date>-<milestone>.md`: the `mission-checks` runs on the integration pushes: run ids, wall
  times, per-job durations, verdict against the 3-minute budget. `ci/2026-10-23-s.md`: S, red.
  `ci/2026-10-23-m0.md`: M0, green, 1 min 45 s. `ci/2026-10-24-m1.md`: M1, green, 3 min 48 s (over budget). `ci/2026-10-24-m2.md`: M2, green, cold merge run 2 min 48 s
  reported apart, warm runs against the 3 min budget (F-039), per-job seconds and the critical-path split (B-056).
  `ci/2026-10-25-m3.md`: M3, the `mission-checks` runs of the integration pushes and the macOS `e2e` run (tag `e2e-m3`).
- `readiness/<date>-<milestone>.json`: Agent Readiness report at an integration.
  `2026-10-23-s.json`: 42 of 66, hand re-evaluation of the criteria S changed (see its `method`).
  `2026-10-23-m0.json`: 44 of 66, hand re-evaluation of the criteria M0 changed.
  `2026-10-24-m1.json`: 45 of 66, hand re-evaluation of the criteria M1 changed; `2026-10-24-m1.md`: warm
  build times at the M1 integration (`make check` 92.700 s, `cargo llvm-cov` alone 121.731 s) and per-feature compile cost.
  `2026-10-25-m3.md`: the M3 integration report (merges, gate, merged staging notes, warm build times, coverage 94.21 %).
  `2026-10-24-m2.json`: hand re-evaluation of the criteria M2 changed; `2026-10-24-m2.md`: warm build times at the M2
  integration (`make check` 118.050 s, `cargo llvm-cov` alone 133.599 s) and per-feature compile cost.
- `post-mortem/observatory-v1-<milestone>.md`: the delivery post-mortem per milestone (clocks per lane and phase, signatures,
  the questions Roci raises and the answers, waste causes including validation overhead). `post-mortem/observatory-v1-m2.md`: M2.
- `staging/<feature-id>.md`: what a feature in a worktree wants written in the shared files (SPEC §9 rows, D-row
  amendments, BACKLOG rows, deferred clauses); committed with the feature and merged by the integrator (F-066; the gate cache is rule 27).
- `process/<date>-process-record.md`: the process audit of the contract's VAL-PROC-001 to 037 on the mission branch, each with the command and the pasted
  output, the verdict and the BACKLOG row of every gap. `process/2026-10-30-process-record.md`: written at the M5 boundary by `final-process-record`.
- `usage/m4.md`, `usage/m5.md`, `ci/2026-10-29-m4.md`, `ci/2026-10-30-m5.md`, `readiness/2026-10-29-m4.md`, `readiness/2026-10-30-m5.json` and `.md`: the M4 and M5
  boundary reports (M5: window 163.57 USD-eq against the cap 190, orchestrator 71.27 against 30, one cold CI run of 228 s).
- `screenshots/<date>-<milestone>.png`: the owner's unlocked-display screenshot per integration.
  Expected at S: `screenshots/2026-10-23-s.png`; at M0: `screenshots/2026-10-23-m0.png`; at M1: `screenshots/2026-10-24-m1.png`; at M2: `screenshots/2026-10-24-m2.png`; at M3: `screenshots/2026-10-25-m3.png` (neither the M2 nor the M3 file exists yet, SPEC §9 F-136 Q2).
