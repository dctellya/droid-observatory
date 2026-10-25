# Staging: m3-e2e-harness

Feature `m3-e2e-harness`, worktree `wt/m3`. Contract VAL-E2E-001 to 007.

## SPEC §9

Rows for the integrator to number (next free row at the time of writing: F-071).

- `e2e` cargo feature: `tauri-plugin-wdio-webdriver` 1.5.1 is an optional dependency behind
  `e2e = ["dep:tauri-plugin-wdio-webdriver"]`, used only under `#[cfg(feature = "e2e")]`. The plugin needs no
  capability permission (`wdio-webdriver:default` is not in `capabilities/default.json`; `rg -c wdio` there is 0). A
  release build has no server: `strings release/observatory-app | grep -c TAURI_WEBDRIVER_PORT` is 0, the e2e debug
  build prints 1. Alternative: the `tauri-driver` path, rejected in the research note (needs a second binary).
- WDIO pins: `webdriverio`, `@wdio/cli`, `@wdio/local-runner`, `@wdio/mocha-framework`, `@wdio/spec-reporter`,
  `@wdio/junit-reporter` 9.32.0, `@wdio/tauri-service` 1.5.1 with `driverProvider: 'embedded'`. `tauri-plugin-wdio` is
  not installed: no spec uses `browser.tauri.*`.
- `@wdio/tauri-service` focus workaround: the service checks the window state before every find and click through
  `browser.tauri.execute`, which needs `tauri-plugin-wdio` and cost 5 s per command. The `before` hook calls `switchToWindow(getWindowHandle())`, which turns that recovery off. If the
  service changes, remove the hook.
- `scripts/check-file-size.sh` exempts `*/package-lock.json` (the e2e lockfile is over 200 KB; `npm ci` needs it
  committed). Alternative: raising the global limit, rejected.
- `.markdownlint-cli2.yaml` ignores `**/node_modules/**` (was root-only): the pre-commit markdown hook linted 10 108
  issues in the e2e dependencies once `npm ci` had run.
- `.github/workflows/e2e.yml`: macOS job, `workflow_dispatch` only, outside `mission-branch.yml` and its 3-minute
  budget. Pinned: trunk 0.22.0, tauri-cli 2.12.4, `actions/setup-node` v7.1.0 by SHA. It runs `make e2e` and uploads
  `reports/e2e/**`. `make check-workflows` exits 0. No run has been dispatched (see Deferred clauses).
- `panel_rendered` (architecture §7) takes an optional `ms: f64`: the UI sends its measured `panel:render` duration and
  the host writes it as the `panel_rendered` interval of `observatory::timing`. Added to `architectureNotes`.
- Latency limits live in `config/observatory.example.toml` `[e2e]` (`panel_render_p95_ms` 3000,
  `event_first_on_board_p95_ms` 500, `host_append_p95_ms` 300, `event_visible_max_ms` 1000, `min_samples` 3), each with
  its reason. `E2eConfig` in `observatory-core`; the specs read them through `lib/thresholds.ts`.
- Locked display: see Deferred clauses; the README states the consequence (DOM reads do not need a visible window,
  screenshots come from an attended run or the CI runner).

## Decisions

- Selector for the AskUser sheet is the `host-question` list item in `launch.rs` (`data-testid="ask-user-sheet"`).
  When `m3-controls-ui-and-stall` builds the real sheet it keeps that test id, or the spec `hosted.e2e.ts` is
  re-pointed in the same feature.
- The merged report: the JUnit reporter writes `junit-<cid>.xml` per spec file; `onComplete` joins their
  `<testsuite>` elements into `reports/e2e/<date>/junit.xml` (the contract names that path). `addWorkerLogs: true`
  puts the p50/p95/max lines into each case's `system-out`.
- `reports/e2e/<date>/` is git-ignored (junit, PNGs, timing JSONL, wdio log); evidence is the pasted run.

## BACKLOG

- `npm audit` reports 19 vulnerabilities in the e2e dev dependencies (WDIO tree, dev only). Reason: pinned by the
  contract (9.32.x); revisit at the next WDIO bump. Not run in CI.
- The `panel.e2e.ts` spec takes 19.7 s (replay of the S fixture through the board); the hosted spec 3.3 s. Check if
  the panel scenario can open the fixture once and share the page across its four cases.
- The Panel `panel:render` p50 is 1.0 ms: it is measured from the effect that closes the mark, not from paint. A
  `requestAnimationFrame` leg would measure paint; left out until a threshold needs it.
- `thiserror` duplication would appear if `tauri-plugin-wdio` is added later; `make deny` is clean without it.
- VAL-HOST-004/005/009/010/011/012/013/015 (the eight M2 window assertions) are re-graded from `make e2e`
  (`reports/e2e/<date>/junit.xml` cases and PNGs); the validator cites case names.

## Deferred clauses

- VAL-E2E-004 (locked display): not met in the strict form. One run was made with `pmset displaysleepnow` issued
  4 s before `make e2e`: exit 0, 2 spec files, 11 passing, `document.visibilityState = visible` in both workers,
  PNGs 489 059 and 681 060 bytes (not blank). The session could not confirm that the display was asleep
  (`ioreg -n IODisplayWrangler` printed nothing, `pmset -g powerstate` failed), and a real screen lock
  (`CGSession -suspend`) would lock the owner out until a password is typed, so it was not done. Open: the owner (or the
  CI runner) runs `make e2e` once with the screen locked and pastes `visibilityState`; the README section is ready.
- VAL-E2E-006: the workflow exists and lints; no green run id is available (dispatch needs a write token; the PAT is
  read-only, D-012). The integrator dispatches it once at M3 and pastes the run id and artifact name into `reports/ci`.
- VAL-HOST-009/013 numbers come from the JUnit `system-out` of the first green `make e2e` at integration, not from this
  worktree's run.

## Library

- `RUSTC_WRAPPER=` before cargo when sccache is flaky.
- `make e2e` warm: 1 m 17 s (build, `npm ci`, 11 tests); cold, first run: 3 m 19 s. Release build of the app without
  the feature: 1 m 00 s.
- Embedded driver port 4445 (`embeddedPort`); one app at a time (`maxInstances: 1`).
- The plugin works without `wdio-webdriver:default` in capabilities.
- Python 3.14 from Homebrew has a broken `pyexpat` here; use `xmllint --noout` for JUnit checks.
- `timeout` is not installed on this macOS; the commands run bare.
- Permission requests do not map to events: the store holds only `question.asked`, `intent.created`,
  `intent.answered` and `tool.error` for the hosted run, so the permission list is asserted from the DOM and the fake
  Droid's stdin log.
