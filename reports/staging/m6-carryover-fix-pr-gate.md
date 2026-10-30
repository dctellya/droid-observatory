# m6-carryover-fix-pr-gate

## SPEC §9

- Row (F-NEW): M5 carry-over, release gate. PR #40 head `72d0412` (gate run 41052611163) failed `sast` and `fs-scan`.
  Fix on the PR head: the six `rust.lang.security.temp-dir.temp-dir` findings are gone because the test helpers use
  `tempfile::tempdir()` (a `TempDir` that deletes itself) instead of `std::env::temp_dir()`; the two
  `bash.lang.security.ifs-tampering` findings are gone because `named-tests.sh` splits the body patterns with `tr` and
  reads them with `IFS= read`; the two Trivy `github-pat` criticals on the logged placeholder are suppressed by a
  path-scoped `.trivyignore.yaml` loaded through `trivy.yaml` (the build-log is append-only, history is not rewritten).
  Alternatives: a plain `.trivyignore` (rule id for the whole repository, rejected: it would silence the rule everywhere);
  a redaction commit (rejected: it leaves lines 218 and 219 in history and the scan reads the tree, not history).
  No dependency added: `tempfile` 3.27.0 was already a dev-dependency of the three crates, so `make deny` did not run.

## Decisions

- None.

## BACKLOG

- New: the Trivy ignore is a file-and-rule pair for one build-log file; a build-log that logs another placeholder
  needs its own entry or a redaction in the hook; reason: `log-tool-call.sh` writes tool input verbatim and the
  placeholder reached the log before the shape was masked. Proposed fix: mask `ghp_`/`github_pat_` shapes in the hook.

## Deferred clauses

None.

## Library

- `trivy.yaml` at the repository root points Trivy at `.trivyignore.yaml`; Trivy picks up only a plain `.trivyignore`
  by default, a YAML ignore file needs `ignorefile:` in `trivy.yaml` (verified with Trivy 0.74.0 locally, three runs).
- Semgrep reproduces locally through Docker with the gate's image and rule sets:
  `docker run --rm -v "$PWD":/src -w /src semgrep/semgrep:1.182.0 semgrep scan --config p/default --config p/rust
  --config p/secrets --config p/github-actions --error --metrics=off`. The mount must sit under the home directory
  (Colima shares it); a `/tmp` mount is empty inside the container.

### Claims (rule 30)

`fulfills` is empty for this feature; the receipts below are for its expected behaviours. Commands run in the main
checkout on the mission branch; outputs are the recorded lines.

- sast, the eight findings reproduce on the old files:
  `docker run --rm -v "$B":/src -w /src semgrep/semgrep:1.182.0 semgrep scan --config p/default --config p/rust
  --config p/secrets --config p/github-actions --metrics=off --no-git-ignore` over the HEAD versions of the six files:
  `Findings: 8 (8 blocking)`, rules `rust.lang.security.temp-dir.temp-dir` and
  `bash.lang.security.ifs-tampering.ifs-tampering` (the six plus two of the gate run).
- sast, none after the fix: the same image and rule sets over the working tree with `--error`: exit 0, no findings
  (3 min 15 s wall).
- fs-scan: `trivy fs --scanners vuln,misconfig,secret --severity HIGH,CRITICAL --ignore-unfixed --exit-code 1
  --skip-dirs target -q .`: exit 0, `build-log/2026-10-30/50c8af83-...jsonl` Secrets 0, Cargo.lock vulnerabilities 0.
  Without `trivy.yaml` the same scan exits 1 with 2 CRITICAL (github-pat).
- `make check-scripts`: exit 0 (1 min 40 s wall), last line `OK` of the 31 python tests.
- `make check-contract`: exit 0, `contract checks: 19 ok, 12 skipped, 0 FAIL in 15.7s`.
- Rewritten body loop still fails a bad pattern: `CONTRACT_TESTS_FILE=/tmp/g2neg.tsv CONTRACT_TESTLOG=none sh
  scripts/contract-checks/g2-gate-tests.sh`: `g2-gate-tests FAIL VAL-X`, `body lacks /zzznomatch/`, exit 1.
- `cargo fmt --all -- --check`: exit 0. `cargo clippy -p observatory-core --all-targets --all-features -- -D warnings
  -W clippy::cognitive_complexity`: exit 0, 34.06 s. Same for `-p observatory-app`: exit 0, 1 m 21.72 s.
- `cargo test -p observatory-core config::`: 11 passed, 0 failed (2 m 39.64 s wall with the build).
  `cargo test -p observatory-app`: lib `20 passed; 0 failed; 2 ignored`, other binaries 5 and 4 passed (41.05 s).
