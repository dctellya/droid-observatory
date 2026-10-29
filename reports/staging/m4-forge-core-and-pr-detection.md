# Staging: m4-forge-core-and-pr-detection

Feature `m4-forge-core-and-pr-detection` (VAL-GATE-010, -011, -013, -029, -032), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW): `ingest::forge` holds the `Forge` trait and its types; `ingest::github` the client and the source (feature
  `github`, which now implies `secrets`). The trait carries `pr(branch, target)` and `checks(pr)` only; the later slices add
  `policy`, `reviews`, `attestations`, `merge` (architecture §4.6 lists all six). `GitHubClient<T: HttpTransport>` reads
  `GET /pulls?head={owner}:{branch}&base={target}&state=all`, `GET /actions/runs?head_sha=` and `GET /commits/{sha}/status`
  and nothing else; the check-runs and check-suites API has no function and no mention in `crates/observatory-core/src`.
  `checks` keeps the latest Actions run per workflow name (cancelled, skipped and neutral runs say nothing) plus every commit
  status context. Every request logs `github.request{method, url, status}` through `tracing` (target `observatory_core::ingest::github`);
  a transport failure logs the error text instead of a status; headers and bodies never reach a log.
- Row (F-NEW): `GitHubSource::poll(now)` / `poll_if_due(now)` return a `Poll { observed_at, pr, checks, events }`; the interval
  counts from the attempt (a failing forge is not asked again before it is due). `pr.opened` is one event per PR
  (`github:pr:<n>:opened`, role `external`, `at` = the PR's `created_at`, pointer `SourcePtr::GitHub { url }` of the PR API URL,
  lane from `template.lane_of`, Integrate in the default template). `Store::record_poll` writes `prs` (`observed_at` is the
  first observation and is never overwritten), one `checks` row per (PR, name, source), and the events (idempotent).
- Row (F-NEW), TLS: `ureq` 3.5.0 with `native-tls-no-default` and `RootCerts::PlatformVerifier` (the system trust store), not
  rustls (architecture §13 said "verify"). The default `rustls` feature pulls `webpki-roots` (CDLA-Permissive-2.0), which
  `deny.toml` does not allow; a new licence is a decision file. `make deny`: advisories ok, bans ok, licenses ok, sources ok.
  The live run caught a panic the recorded fixtures cannot (native-tls without roots panics on the first https request);
  `an_https_request_builds_its_tls_connector_instead_of_panicking` pins it.
- Row (F-NEW): `core::secrets` (keychain item `droid-observatory/<name>`, account `$USER`, `GITHUB_TOKEN` fallback, `Secret`
  prints `<redacted>`). A keychain that refuses (locked display: `Platform failure: User canceled the operation.`, observed on
  the live run) gives way to `GITHUB_TOKEN`; with none the refusal is returned. `keyring-core` and `apple-native-keyring-store`
  left core's machete ignore list.
- Row (F-NEW): `[github].request_timeout_s = 15` (half the poll interval). The shell (`apps/observatory-app/src-tauri/src/github.rs`)
  polls each live mission (the `stall::live_missions` set, now `pub(crate)`) every `[github].poll_interval_s`, records the poll
  and emits `events_appended` for new events; the branch is `[mission].branch_pattern` applied to the mission name.
- Evidence (live, 2026-10-29, demo pair): ignored test `forge_live` against `dctellya/droid-observatory`, probe PR #28
  `demo/forge-probe-4 -> demo/base`: created `12:46:14Z`, detected `12:46:22.279Z`, delay 8 s 279 ms (interval 30 s); request
  log lines `pulls?head=dctellya:demo/forge-probe-4&base=demo/base&state=all`, `actions/runs?head_sha=`, `commits/<sha>/status`,
  all `status=200`; no `check-runs`; `grep -c` of token patterns and `Authorization` in the run output: 0. Probe PRs #25 to #28
  were closed and their branches deleted (`demo/base` is the only `demo/*` branch left). The first probes found two defects the
  recorded fixtures could not (keychain prompt on a binary it has not seen, native-tls roots).

## Decisions

None. (The rustls to native-tls choice is a SPEC row; `webpki-roots` under CDLA-Permissive-2.0 would need the owner's licence decision.)

## BACKLOG

- The keychain item prompts for access when a binary it has not seen reads it (`cargo test` binaries, an unsigned dev app);
  with the display locked the prompt ends `User canceled the operation` after about 60 s. The app falls back to `GITHUB_TOKEN`, but
  the wait is on the poll thread at start. A signed build with the item's access list set, or reading the token once at start
  and not per mission, would remove it. Decision for the validator run (VAL-GATE keychain path).
- `HandedToCd` store round trip (attestation ids empty, PR number parsed from the URL) is not touched here: `handed_to_cd`
  emission and the attestation join belong to `m4-forge-rulesets-and-merge` (it needs `PrState::Merged` with the merger login,
  which the pulls list does not carry; `GET /pulls/{n}` does).
- `Pr` has no merger login and `prs.merged_by` stays NULL until that slice adds the call.

## Deferred clauses

- VAL-GATE-010 live part for the validator: PR `demo/red-1` created by the validator and read by the app's own poll thread
  (`make app-dev`, app log); this feature proved the core path with the ignored `forge_live` test and the shell thread compiles
  and lints, but the window was not run.
- VAL-GATE-029 `unknown_total` before and after in the app log: the shell emits `events_appended` with `unknown_total`; the store
  count is asserted 0 in `recording_a_poll_twice_keeps_one_pr_row_and_its_first_observation`; the live window was not run.
- Coverage of `observatory-core` (80 %) runs once at the integration.

## Library

- Tracing in tests: a scoped `with_default` subscriber can lose to the per-process callsite interest cache when another test ran
  the same callsite first without a subscriber. Install one global subscriber in the one test that reads the log and filter
  lines by a value unique to the test (the loopback base URL).
- No `timeout` binary on this machine: `perl -e 'alarm N; exec @ARGV' <command>` bounds a run.
- Recorded GitHub fixtures live in `fixtures/github/<name>/` with a `routes.json` (path, status, body file); author name and
  email in `runs.json` are rewritten to `Test Author` and `user@example.invalid`.
- A local loopback HTTP server on a `TcpListener` thread replays the routes for tests that need the real `ureq` path.
