# Staging: m4-e2e-keep-run-records

Feature `m4-e2e-keep-run-records`, worktree `observatory-wt/m4`, branch `wt/m4`. Follow-up of `m4-e2e-first-failure-evidence`: `make e2e`
removed `reports/e2e/<date>/` at its start, so the second run of a day deleted the first run's `record.json`, junit files and
`first-failure/` before the usage report counted them (the induced flaked run's evidence was wiped by the green proof run).

Rule 30 claims: none to claim (`fulfills` is empty); evidence is the tests (red `2b3c492`, green `e458276`) and the one real run below.
Status: partial. The one real induced `make e2e` run ended `failed`, not `flaked` (machine load 14 to 22), so the flaked junit merge on
the real wdio reporter is still unproven; see the split proposal in the hand-off.

## SPEC §9

- Row (F-NEW), tooling and test policy (rule 26, verified on 2026-10-29 by `scripts/tests/e2e_run_test.py` (26 tests),
  `scripts/tests/usage_e2e_test.py` (11) and one real `E2E_INDUCE_FAILURE=panel make e2e` run). Each `make e2e` run writes under its own
  directory `reports/e2e/<date>/<HHMMSS>/` (`E2E_STAMP`, the start time): `record.json` (field `run` is `<date>/<HHMMSS>`), `junit.xml`,
  the attempt junit files, the PNGs and `first-failure/`. No target and no script deletes a report directory; `e2e_run.py run` refuses
  a directory that already holds a `record.json` (exit 2, the suite is not started). The latest run is reachable at
  `reports/e2e/latest`, a relative symlink set by `e2e_run.py run --latest` before the suite starts (so a run that dies is still
  reachable); a symlink and not a pointer file so that every path read today (`junit.xml`, `record.json`, the PNG convention
  `card-lane3-cycle-1.png`) only gains the `latest/` segment; it is git-ignored. `scripts/usage_testing.py` counts every
  `reports/e2e/<date>/<HHMMSS>/record.json` in the window (and the flat `<run>/record.json` of the runs before stamps), skips the
  `latest` link, so a flaked run stays an `e2e flake:` line after later green runs. **Alternatives considered:** a `latest` pointer file
  (rejected: every reader needs a lookup step, the PNG paths change shape); keeping `reports/e2e/<date>/` and renaming the old run
  aside (rejected: a copy step that can be skipped, the failure this feature closes).

## Decisions

None.

## BACKLOG

- B-NEW: the e2e CI upload (`reports/e2e/**` in `.github/workflows/e2e.yml`) may follow the `latest` symlink and upload the newest run twice;
  harmless, not measured (no CI run in this feature). Fix would be an exclude line `!reports/e2e/latest/**`; the workflow is not this
  feature's file.
- B-NEW: `junit.xml` of a failed run puts the property `outcome=failed` on every suite the rerun replaced, including those whose rerun
  passed (real run: gate and panel suites passed on the rerun and carry `outcome=failed`); read the root `outcome` and the `<failure>` elements, not
  the suite property, for `failed`. `outcome=flaked` on a replaced suite is correct.
- B-NEW: with the machine at load average 14 to 22 the first attempt failed 9 cases (gate lane never reached Build, hosted latency 339 ms
  against 300, arrival stamps 11 ms apart against at least 200, panel `panel:render` 16153 ms against 3000 once the induced failure was
  counted); the rerun of those three specs failed one again (`hosted.e2e.ts`). Thresholds are load-sensitive: `make e2e` is a
  quiet-machine target.

## Deferred clauses

- The flaked outcome on the real wdio reporter (`junit.xml` root `outcome=flaked`, 0 `<failure>`, well-formed) and the `e2e flake:` line on a real
  record: needs one more induced run on a quiet machine (check `uptime` first).

## Library

For the validator:

- Where runs live: `reports/e2e/<date>/<HHMMSS>/` (git-ignored; CI artifact `e2e-reports`). Every run keeps its own directory; nothing
  deletes it. Two runs on one day are two directories.
- How to read the latest: `reports/e2e/latest` is a symlink to the newest run (`ls -l reports/e2e/latest`); use
  `reports/e2e/latest/record.json`, `reports/e2e/latest/junit.xml`, `reports/e2e/latest/first-failure/` and the PNGs under it. To list
  all runs: `ls reports/e2e/*/*/record.json` (the link is one level up and not matched).
- `usage-report.py --e2e-dir reports/e2e` counts every run directory in the window; the `run` column reads `<date>/<HHMMSS>`.
- The predecessor's run of 2026-10-29 (flat `reports/e2e/2026-10-29/`) was moved out of the worktree to `/tmp/m4-e2e-predecessor/`;
  a flat `<run>/record.json` is still counted if present.
