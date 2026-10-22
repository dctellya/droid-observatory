# Backlog

Deferred items with their reasons (ai/RULES.md rule 14). Reference as `B-NNN` from code
(`TODO(B-NNN)`) and decisions. Scheduled items move to the spec; dropped items stay here with
the reason struck through, not deleted.

| Id | Item | Reason deferred | Earliest |
|---|---|---|---|
| B-001 | Devcontainer for Linux CI parity | Tauri on macOS is the target; a container cannot run the window. Useful for core-only work. | after skeleton |
| B-002 | CI telemetry to Grafana (workflow durations, test timings) via OTLP | Owner: document only for now. Secret `GRAFANA_OTLP_HEADERS`, variable `GRAFANA_OTLP_ENDPOINT` reserved. | after M5 |
| B-003 | Dedicated error tracking (Sentry or Grafana Faro) | D-004 uses span events first; add a tracker only if they prove insufficient. | after M2 |
| B-004 | Release notes automation (git-cliff from conventional-ish subjects) | No releases until the mission PR; one PR means one changelog entry. | at first release |
| B-005 | Coverage thresholds (`cargo-llvm-cov`, fail under a stated %) | Needs code; the number needs a reason. Spec §6 to set it. | with skeleton |
| B-006 | Log scrubbing utility for raw JSONL rendering | PRD renders raw lines read-only; a redaction pass before display is the next step. | M1 |
| B-007 | Private demo repository with PR flow, CI and rulesets for M4 | Needed before M4 starts; owner creates it. | before M4 |
| B-008 | Human-computed reference spreadsheet from the M0 capture for M5 | Mission input; cannot be produced by validators. | before M5 |
| B-009 | Optional manual ~20-minute mission capture before the mission starts | Insurance for M0 planning, not structural. | before mission |
| B-010 | Settle walking-skeleton vs M0 boundary in SPEC.md | Rule 2 and PRD §13 overlap; the spec session decides order. | spec session |
| B-011 | Fill `config/prices/2026-10-22.toml` from the vendor price page, set `verified = true` | Prices must be read on the day, not recalled. | before M5 |
| B-012 | Runbook: stalled mission, red gate, fix cycle, hooks disabled | Needs the app to exist to describe recovery honestly. | M3 |
| B-013 | GitHub code scanning default setup (CodeQL) | Enabled if the API accepts Rust for this repo; otherwise Semgrep in the gate covers SAST. | now / M1 |
| B-014 | Dead code and unused dependency checks (`cargo-udeps`, `cargo-machete`) in the mission-branch CI | Needs code; nightly toolchain for udeps is a cost to weigh. Spec §7 decides. | with skeleton |
| B-015 | Readiness report re-run at every milestone integration, stored in `reports/readiness/` | Baseline exists; cadence belongs to the spec. | each milestone |
