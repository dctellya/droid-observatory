# Staging: m4-e2e-first-failure-evidence

Feature `m4-e2e-first-failure-evidence`, worktree `observatory-wt/m4`, branch `wt/m4`. It acts on the owner's inbox file
`docs/owner-feedback/inbox/2026-10-29-1552-flaky-e2e-is-a-feature.md` ("shouldn't this be a feature?", relayed by pre-roci): one
`make e2e` failed 2 of 6 specs during `m4-forge-off-without-flag` (the gate case: card never reached Build after 1m44s; the panel
case: 4161 ms against the 3000 ms limit), two clean reruns passed 32/32, nothing of the first run was kept. Rule 29: an unexplained
red is incomplete work, so it is this sized feature (turnBudget 30), not a validator note.

Rule 30 claims: none to claim (`fulfills` is empty); evidence is the tests and the two `make e2e` runs below.

## SPEC §9

- Row (F-NEW), tooling and test policy (rule 26, verified on 2026-10-29 by `scripts/tests/e2e_run_test.py`, `scripts/tests/usage_e2e_test.py` and
  two real `make e2e` runs; owner inbox file `docs/owner-feedback/inbox/2026-10-29-1552-flaky-e2e-is-a-feature.md`). `make e2e` runs the
  suite through `scripts/e2e_run.py`. (1) On the first failing case the wdio `afterTest` and `afterHook` name the spec in
  `failed-specs.txt` and call `e2e_run.py first-failure` while the app is up; `reports/e2e/<run>/first-failure/` holds
  `environment.json` (E2E_RUN_DIR, ports with the pid that listens, process lines, GITHUB_TOKEN `set` or `unset` and never its value,
  FORGE_LIVE), `run-dir-listing.txt`, `backend-logs/` (the `captureBackendLogs` `wdio-*.log`), `screenshots/` (`fail-*.png`),
  `timing.jsonl`, `failed-specs.txt`; written before any rerun and never rewritten by it. (2) At most one automatic rerun of the failed
  specs, on a clean run directory; a failure that names no spec is red without a rerun. `record.json` keeps both attempts (outcome, exit
  code, wall seconds, failed specs) and one outcome: `passed`, `flaked` (first attempt red, rerun green: exit 0, never counted as a
  pass) or `failed` (the rerun is red too, a plain red). `junit.xml` carries it as `<testsuites outcome="...">` and the rerun-replaced
  suites carry the property `outcome=flaked`; `attempt-1-junit.xml` and `attempt-2-junit.xml` are the two reports. (3) The usage
  report's testing section (`scripts/usage_testing.py`, `--e2e-dir`, default `reports/e2e`) gains `### E2E runs` (one row per record
  in the window), one `e2e flake: <run>: <specs>; rerun <s> s (first attempt <s> s)` line per flaked run and
  `e2e runs: N, passed P, flaked F, failed X; rerun seconds: S`. Proof: `E2E_INDUCE_FAILURE=panel make e2e` (the knob in
  `lib/induce.ts` fails the panel spec's first case on attempt 1 only) printed `e2e: flaked (first attempt red, rerun green):
  specs/panel.e2e.ts; rerun 6.19 s of 100.37 s`, exit 0, 2m28.899s; a plain `make e2e` printed `e2e: passed (93.73 s)`, exit 0,
  1m57.076s. **The cause of the original red is not proven and not fixed here**; this feature is what proves it next time.
  **Alternatives considered:** retry inside mocha (`mochaOpts.retries`; rejected: it hides the first failure inside one report and
  keeps no evidence); a `flaky-e2e` note in the validator report (rejected by the owner, rule 29); an unlimited or two-fold rerun
  (rejected: three full runs paid for one green is the waste the owner named).

## Decisions

None.

## BACKLOG

- B-NEW: the first-failure folder is git-ignored with the rest of `reports/e2e/*/`; a flake found by the CI job is read from the
  `e2e-reports` artifact (`.github/workflows/e2e.yml` uploads `reports/e2e/**`, unchanged). The integrator's boundary report may
  quote the `e2e flake:` lines of the usage report; no integrate script change was made (rule 13: not a worker's file).
- B-NEW (environment): Homebrew `python3` 3.14.7 on this machine cannot import `xml.etree` (`pyexpat`: `Symbol not found:
  _XML_SetAlloc`; `/usr/bin/python3` works). The runner and its test read junit as text so the harness does not depend on it; any
  later script that parses XML needs the same care or a fix of the Homebrew install.

## Deferred clauses

None.

## Library

For the validator:

- Where the evidence is: `reports/e2e/<run>/first-failure/` (`<run>` is the `E2E_DATE`, `reports/e2e/<date>/`), next to
  `record.json`, `junit.xml`, `attempt-1-junit.xml`, `attempt-2-junit.xml` (only after a failure) and the per-case PNGs. A green first
  attempt leaves no `first-failure/` folder. The directory is git-ignored; CI uploads it in the `e2e-reports` artifact.
- How to read `flaked`: `record.json` field `outcome` is `passed`, `flaked` or `failed`; `attempts[0]` is the first attempt (its
  `failed_specs`, `seconds`), `attempts[1]` the rerun (`specs`, `seconds`); `flaked_specs` and `rerun_seconds` name the flake and its
  cost. `flaked` is green (the make target exits 0) but never a pass: count it as a flake, quote the spec and the rerun seconds, and
  open `first-failure/environment.json`, `backend-logs/` and `screenshots/` before calling the cause. `failed` is a plain red.
- `make e2e` removes `reports/e2e/<date>/` at its start: a second run on the same date replaces the first run's record and evidence.
  Copy the folder before running again.
- Induced proof: `E2E_INDUCE_FAILURE=panel make e2e` gives a `flaked` run on purpose (the panel case fails on attempt 1 only). Do not
  set it in CI.
- A rerun re-uses wdio's own `junit-<cid>.xml` merge; the kept attempt files are named `attempt-N-junit.xml` because a `junit-` prefix
  is consumed and deleted by `onComplete` (found in the first induced run).
