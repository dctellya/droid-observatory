# The flaky e2e run is a feature, not a note

Source: owner ("shouldn't this be a feature?", 15:5x, 2026-10-29), relayed by pre-roci. Refers to the m4-forge-off-without-flag dismissal: first `make e2e` failed 2 of 6 specs, two clean reruns 32/32, cause not proven, filed as a validator note.

Rule 29 applies: an unexplained red is incomplete work, and a note does not make the next one cheaper. Replan it as a sized feature (turnBudget 30), `m4-e2e-first-failure-evidence`:
1. On the first failing spec the e2e harness captures the evidence before anything reruns: backend log, the spec's screenshot or trace, the run directory listing, the environment (E2E_RUN_DIR, ports, pids), into reports/e2e/<run>/first-failure/.
2. Rerun policy: at most one automatic rerun, and both results go to the record (first red, rerun green), so a flake is counted as a flake, never as a pass.
3. The gate reads a "flaked" outcome as distinct from "passed": the row is green, the flake is a line in the usage report's testing section (cost of the reruns, which spec).
Why: three full e2e runs were paid for one green; the cause is unknowable because nothing was kept. The waste is the reruns and the diagnosis that cannot happen. Not for a cause fix: the cause is not proven, and this feature is what proves it next time.
