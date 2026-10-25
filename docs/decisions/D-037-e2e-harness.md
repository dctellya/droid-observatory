# D-037: The window suite is WebdriverIO over the app's embedded WebDriver server

**Date:** 2026-10-25
**Status:** accepted; built in `m3-e2e-harness` (F-061, owner items 14 to 17 of the M2 boundary), written late by
`m3-fix-ut-ui-kit-logs-and-attached-e2e` because F-061 promised the file and the harness feature did not write it

## Context

The Tauri window has no CDP endpoint, and driving it with `osascript` or System Events failed on a locked or
occupied display at M1 and M2 (the contract's pass-20 rule). The owner asked for the window checks to run as a
real end-to-end suite with a report validators can grade (items 14 to 17, rules 26 to 28). The suite had to run
the production webview, hold no logic of its own, and stay out of release builds.

## Decision

- **Driver:** WebdriverIO 9.32.0 (`webdriverio`, `@wdio/cli`, `@wdio/local-runner`, `@wdio/mocha-framework`,
  `@wdio/spec-reporter`, `@wdio/junit-reporter`, all pinned to 9.32.0) with `@wdio/tauri-service` 1.5.1 and
  `driverProvider: 'embedded'`. The service launches the debug app binary and talks to a W3C WebDriver server inside
  it (`tauri-plugin-wdio-webdriver` 1.5.1, port 4445). There is no `tauri-driver` and no second binary.
  `tauri-plugin-wdio` is not installed: no spec uses `browser.tauri.*`, and the `before` hook of `wdio.conf.ts`
  calls `switchToWindow` so the service stops probing the window through it (5 s per command otherwise).
- **Cargo feature `e2e`:** the plugin is an optional dependency behind `e2e = ["dep:tauri-plugin-wdio-webdriver"]`
  and is compiled in only by `make e2e` (`cargo tauri build --debug --no-bundle --features e2e`). A release build has
  no server (`src-tauri/tests/e2e_feature.rs` checks the manifest, the cfg gates and `cargo tree`).
- **Isolation:** the app reads one scratch directory (`$TMPDIR/observatory-e2e`: its config, store, spool, session
  directory and a fake Droid). Nothing under `~/.droid-observatory` or `~/.factory` is touched.
- **Report:** `reports/e2e/<E2E_DATE>/junit.xml` (one `<testsuite>` per spec file), one PNG per case (the
  `afterTest` hook), `timing.jsonl` and the app's captured stdout (`wdio-*.log`). Specs read their limits from
  `[e2e]` in `config/observatory.example.toml`. Validators grade from this report and never drive the window.
- **Locked or covered display:** DOM reads, `execute/sync` and User Timing do not need a visible window, so the
  assertions hold with the display locked. Screenshots may come back blank or the request may fail; `afterTest`
  logs it and the case result stands. Locking the display is owner-attended: a validator cannot lock it (that
  locks the owner out) and does not cover the window with System Events (VAL-E2E-007). The one strict locked run
  is the owner's, at the milestone boundary.
- **CI trigger:** the macOS job `.github/workflows/e2e.yml` runs on the tag `e2e-<milestone>` pushed by the
  integrator at the milestone boundary (the read-only PAT of D-012 cannot dispatch it). It stays outside the
  3-minute `mission-branch.yml` budget. `m3-fix-ut-gate-timing-e2e-trigger-usage` wires the trigger.

## Alternatives

- **`tauri-driver`:** rejected in the harness research note; it needs a second binary beside the app.
- **The browser host only:** rejected. The browser host cannot reach the Tauri commands (attach, host, the
  `human_action` store write), which are what the window assertions grade.
- **System Events and `osascript`:** rejected. The AX tree of a WKWebView is empty unless the window is frontmost,
  and it needs the display.
- **`tauri-plugin-wdio` for `browser.tauri.*`:** deferred. No spec needs it.

## Consequences

- One app instance at a time (`maxInstances: 1`), so the suite is serial: `E2E_DATE=2026-10-25-fix make e2e`
  ran 26 cases in 2 m 56.65 s wall (build included, warm).
- Screenshots of a locked display are not evidence; the DOM assertions are. Dark and light PNGs are taken by cases
  that press the kit's theme switch and check `data-theme`.
- `npm audit` reports findings in the dev-only WDIO tree; they are pinned by the contract and revisited at the next
  WDIO bump.
- The embedded server is a debug-only surface: it must never reach a release build, and the feature test guards it.
