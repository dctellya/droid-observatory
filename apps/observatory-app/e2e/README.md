# Observatory window suite

WebdriverIO drives the real Tauri window through the WebDriver server embedded in the app
(`tauri-plugin-wdio-webdriver`, behind the `e2e` cargo feature). No browser, no `tauri-driver`.

## Run

```sh
make e2e
```

The target builds `observatory-cli`, builds the app with
`cargo tauri build --debug --no-bundle --features e2e`, runs `npm ci` in this directory and starts
`npx wdio run wdio.conf.ts`. Output goes to a run directory `reports/e2e/<date>/<HHMMSS>/` (the stamp is the start of the
run): `junit.xml` (one `<testsuite>` per spec file, the p50/p95/max lines in each case's `system-out`), one PNG per scenario,
`timing.jsonl` (the host's `observatory::timing` lines), `record.json` and, after a red first attempt, `first-failure/`. No run
deletes another run's directory, so two runs on one day leave two directories and the usage report counts both.
`reports/e2e/latest` is a relative symlink to the newest run (set when the run starts, so a run that dies is still
reachable): read `reports/e2e/latest/junit.xml`, `reports/e2e/latest/record.json` and the PNGs below through it. A symlink, not a
pointer file, so every path that used to be `reports/e2e/<date>/<file>` only gains the `latest` segment. The tree is git-ignored.

## What it checks

- `specs/panel.e2e.ts`: the board for a replayed fixture. Lane counts equal the CLI replay's
  per-lane totals, the unknown-events count, the Panel and `panel:render` p95.
- `specs/hosted.e2e.ts`: a hosted mission with a fake Droid (`fixtures/fake-droid.mjs`). Permission
  list, Approve and Deny (the host writes `proceed_once` and `cancel` to the request id), the AskUser
  sheet, the `intent.answered` row, arrival stamps and the event latency.
- `specs/controls.e2e.ts` and `specs/stall.e2e.ts`: the control bar, the AskUser sheet and the stall banner,
  each also in the kit's dark and light themes (cases that press `theme-dark` / `theme-light` and check
  `data-theme`, so the case PNGs show both).
- `specs/gate.e2e.ts`: the Gate lane of a hosted mission. The `e2e` feature adds the command `e2e_gate_poll`, which
  feeds one pull request's scenario (`red`, `green`, `merged`) through the same record and emit path as a GitHub
  poll. The cases read the card's lane and `cycle 1`, the return arc, the gate group and the Merge button from the DOM
  and save `card-lane3-cycle-1.png`, `return-arc-cycle-1.png` and `merge-button.png` next to the per-case PNGs.
- `specs/attached.e2e.ts`: an attached mission. `lib/attach.ts` copies the S session under the run's
  `[ingest].factory_sessions_dir`; the spec sends a message and reads the `human_actions` row with `sqlite3`
  and the app log from the newest `wdio-*.log` (no `host_control` line, a `text_len` line).
- Selectors are `data-testid` attributes. Thresholds come from the `[e2e]` table of
  `config/observatory.example.toml` (`lib/thresholds.ts`); the specs hold no literal limit.
- Latency is read from User Timing (`performance.getEntriesByType('measure')`: `panel:render`,
  `event:first_on_board`) and from the host's JSON lines (`OBSERVATORY_TIMING_LOG`), never from a screen.

## Notes

- The release build has no `e2e` feature and no WebDriver server; `src-tauri/tests/e2e_feature.rs`
  checks the manifest, the cfg gates and `cargo tree`.
- `before` in `wdio.conf.ts` calls `switchToWindow`. Without it `@wdio/tauri-service` checks the
  window before every command through `browser.tauri.execute`, which needs `tauri-plugin-wdio` and
  costs 5 s per command. The suite does not use `browser.tauri.*`, so that plugin is not installed.
- One app instance at a time (`maxInstances: 1`): the specs share a store, a timing log and the
  embedded-driver port 4445.
- Run state lives in `$TMPDIR/observatory-e2e` (own config, store, spool, fake Droid); nothing under
  `~/.droid-observatory` is touched.

## A failing run: first-failure evidence, one rerun, `flaked`

`make e2e` runs the suite through `scripts/e2e_run.py` (owner inbox file
`docs/owner-feedback/inbox/2026-10-29-1552-flaky-e2e-is-a-feature.md`; a red nobody can explain is incomplete work).

- On the first failing case the `afterTest` hook (and `afterHook`) names the spec in `failed-specs.txt` and calls
  `e2e_run.py first-failure` while the app is still up. `reports/e2e/<date>/<HHMMSS>/first-failure/` then holds
  `environment.json` (E2E_RUN_DIR, ports with the pids that listen, process lines, GITHUB_TOKEN `set` or `unset` and never its
  value, FORGE_LIVE), `run-dir-listing.txt`, `backend-logs/` (the `wdio-*.log` of `captureBackendLogs`), `screenshots/`
  (`fail-*.png`), `timing.jsonl` and `failed-specs.txt`. The runner refreshes the logs when the attempt ends and writes
  `run-dir-listing-at-end-of-attempt.txt`. All of it is written before the rerun starts, and nothing in it is written again by the rerun.
- At most one automatic rerun: the failed specs only (`--spec <path>`), on a clean run directory. A failure that names no spec
  is red without a rerun.
- `record.json` keeps both attempts (outcome, exit code, wall seconds, failed specs) and one outcome: `passed` (attempt 1
  green), `flaked` (attempt 1 red, the rerun green; exit code 0), `failed` (the rerun is red too; exit code of the rerun).
  `junit.xml` names it on its root element (`<testsuites outcome="flaked">`); a rerun-replaced suite carries the property
  `outcome=flaked`; `attempt-1-junit.xml` and `attempt-2-junit.xml` are the two attempts as the reporter wrote them.
- A flake is never counted as a pass: the gate row is green on `flaked`, and the usage report's testing section lists the
  run (`e2e flake:` line: the specs and the wall time of the rerun).
- `E2E_INDUCE_FAILURE=panel make e2e` fails the first case of `specs/panel.e2e.ts` on the first attempt only (`lib/induce.ts`):
  the proof that the evidence, the rerun and `flaked` work. Not for CI.

## CI

`.github/workflows/e2e.yml` runs `make e2e` on `macos-latest`, outside the 3-minute `mission-branch.yml` budget. It
triggers on a pushed tag `e2e-*` and on `workflow_dispatch`. At a milestone integration the integrator pushes one
tag at the integration commit (`git tag e2e-m3 <sha> && git push origin e2e-m3`; tags are never moved or deleted)
and records the run id, conclusion, wall time and the `e2e-reports` artifact name. The tag trigger reads the
workflow from the tagged commit, so it works before the file exists on `main`; `workflow_dispatch` is resolved
against the default branch and answers 404 until the final pull request lands. `concurrency` is keyed on the ref
with `cancel-in-progress: false`, so a second tag never cancels a run in flight.

## Locked or covered display

DOM reads, `execute/sync` and User Timing do not need a visible window, so the assertions hold with
the display locked. Screenshots may come back blank or the request may fail; `afterTest` logs the
failure and carries on. Screenshots for a human come from an attended run or the macOS CI job
(`.github/workflows/e2e.yml`, see CI below), never from a validator retrying on a locked display.
The locked-display result of the one run made on 2026-10-25 is recorded in
`reports/staging/m3-e2e-harness.md`.

## Live walk of the Gate (not part of `make e2e`)

`specs/walk-*.walk.ts` drive the real poll against the demo pair (`demo/base`, `demo/red-1`, workflow `demo-gate`); the default
`specs/**/*.e2e.ts` glob does not match them, so pass them with `--spec`. They need the `gh` CLI logged in and a token in the
environment (`GITHUB_TOKEN="$(gh auth token)"`; the keychain is never read), and take their own config:

```sh
E2E_RUN_DIR=/tmp/m4-walk1 E2E_REPORT_DIR=/tmp/m4-walk1-report E2E_CONFIG_FILE=/tmp/walk/config-auto.toml \
  npx wdio run wdio.conf.ts --spec specs/walk-1-red-green.walk.ts --mochaOpts.timeout 600000
```

`E2E_CONFIG_FILE` replaces the generated config (`{RUN_DIR}` stands for the run directory); the walk's config sets
`branch_pattern = "demo/red-1"`, `poll_interval_s = 30` and `api_base = "http://127.0.0.1:3147"`. `lib/forge.ts` `startForgeProxy`
listens there, forwards reads to api.github.com and answers the merge request with the 403 a read-only token gets. Run 1 opens the
PR with `demo:red`, then removes the label; run 2 (`merge_policy = "policy"`) shows the green PR without Merge; run 3 presses Merge
twice and merges as the owner with `gh pr merge --squash --delete-branch`. The commands and app-log lines go to
`walk-commands.log` in the report directory.
