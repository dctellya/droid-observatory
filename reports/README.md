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
- `usage/<milestone>.md`: tokens per role and model, waste by cause, wall time, per
  integrated milestone (written by the orchestrator at integration).
