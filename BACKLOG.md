# Backlog

Deferred items with their reasons (ai/RULES.md rule 14). Reference as `B-NNN` from code
(`TODO(B-NNN)`) and decisions. Scheduled items move to the spec; dropped items stay here with
the reason struck through, not deleted.

| Id | Item | Reason deferred | Earliest |
|---|---|---|---|
| B-001 | Devcontainer for Linux CI parity | Tauri on macOS is the target; a container cannot run the window. Useful for core-only work. `.devcontainer/` provisions `make check` tools and the Linux Tauri libraries. | done 2026-10-22 |
| B-002 | CI telemetry to Grafana (workflow durations, test timings) via OTLP | Owner: document only for now. Secret `GRAFANA_OTLP_HEADERS`, variable `GRAFANA_OTLP_ENDPOINT` reserved. | after M5 |
| B-003 | Dedicated error tracking (Sentry or Grafana Faro) | D-004 uses span events first; add a tracker only if they prove insufficient. | after M2 |
| B-004 | Release notes automation (git-cliff from conventional-ish subjects) | ~~No releases until the mission PR; one PR means one changelog entry.~~ Done differently: GitHub-generated notes by label plus `release.yml` on tags (D-009). | done 2026-10-22 |
| B-005 | Coverage thresholds (`cargo-llvm-cov`, fail under a stated %) | Settled: 80 % lines on observatory-core, enforced from M0 (SPEC §6). | done 2026-10-22 |
| B-006 | Log scrubbing utility for raw JSONL rendering | PRD renders raw lines read-only; a redaction pass before display is the next step. | M1 |
| B-007 | Demo branch pair in this repo (`demo/base`, `demo/*`, `demo-gate.yml`, label `demo:red`) for M4 | Set up by the mission before M4 (SPEC §3 M4). | before M4 |
| B-008 | Human-computed reference spreadsheet from the M0 capture for M5, at `reports/reference/2026-10-22-observatory-v1-m0.csv` | Mission input; cannot be produced by validators (SPEC §3 M5). | before M5 |
| B-009 | Optional manual ~20-minute mission capture before the mission starts | ~~Insurance for M0 planning, not structural.~~ Not needed: the capture is this mission's own sessions (SPEC §3). | dropped 2026-10-22 |
| B-010 | Settle walking-skeleton vs M0 boundary in SPEC.md | Settled: skeleton S before M0; capture = this mission's sessions (SPEC §3). | done 2026-10-22 |
| B-011 | Fill `config/prices/2026-10-22.toml` from the vendor price page, set `verified = true` | Prices must be read on the day, not recalled. | done 2026-10-22 |
| B-012 | Runbook: stalled mission, red gate, fix cycle, hooks disabled | `docs/runbooks/` written with CLI steps; app steps marked with the milestone that delivers them. Revise at M3. | done 2026-10-22 |
| B-013 | GitHub code scanning default setup (CodeQL) | Default setup enabled (languages detected: actions; Rust joins when code exists). Semgrep and Trivy SARIF uploaded from the gate. | done 2026-10-22 |
| B-014 | Dead code and unused dependency checks (`cargo-udeps`, `cargo-machete`) in the mission-branch CI | Settled: cargo-machete on the mission branch, no udeps (SPEC §7). | done 2026-10-22 |
| B-015 | Readiness report re-run at every milestone integration, stored in `reports/readiness/` | Baseline exists; cadence belongs to the spec. | each milestone |
| B-016 | `release.yml` checks that the `Cargo.toml` workspace version equals the tag | Needs `Cargo.toml` (D-009). | with skeleton |
| B-017 | Spec states the exported metric set, histogram buckets and the computed active-series count, under 200 (D-010) | Settled: 182 series worst case, 124 expected, buckets in SPEC §5.7. | done 2026-10-22 |
