# Staging: m4-forge-off-without-flag

Feature `m4-forge-off-without-flag`, worktree `observatory-wt/m4`, branch `wt/m4`. Acts on the owner's rule
`docs/owner-feedback/inbox/2026-10-29-1520-no-keychain-from-any-worker-process.md` (widens
`docs/owner-feedback/inbox/2026-10-29-1343-no-keychain-in-worker-tests.md`). Red commit `ecb193d`, green commit `d4d3194`,
harness commit `319ec1a`.

## SPEC §9

- Row (F-NEW, rule-24 decision, owner rule 1520): the forge source is `off` without a token or the owner flag. The keychain is consulted
  only when `GITHUB_TOKEN` is unset or empty AND `FORGE_LIVE=1` is set; a non-empty `GITHUB_TOKEN` wins as at 3c73008 and leaves the keychain
  alone; with neither the token source is `off`, the shell starts no `github` poll thread (no store open, no request) and logs
  `github.token source=off reason=no GITHUB_TOKEN and FORGE_LIVE unset` with no value; the Gate lane keeps its pending placeholder. The
  keychain path is the owner's own `FORGE_LIVE=1` launch from his terminal. Code: `core::secrets::resolve_forge_token` (keychain closure
  behind the flag), `forge_source_enabled`, `TokenSource::Off`, `OFF_REASON`; the shell's `github::spawn` returns without a thread when
  `forge_source_enabled_in_env()` is false. `resolve_github_token` (env first, then keychain) is unchanged and still pinned by
  `the_environment_wins_and_the_keychain_is_not_consulted`; `github_token()` and `github_token_with_source()` now go through the flag, so
  `GitHubClient::from_config` cannot reach the keychain either. Launched processes: the e2e harness (`apps/observatory-app/e2e/lib/env.ts`
  `appEnv()`) sets `GITHUB_TOKEN` to a PAT-shaped value built at run time by concatenation (never a literal) unless the variable is already
  set; `scripts/tests/attach-smoke.sh`, `make app-dev` and every other launch need no change because the default is off.
  **Alternatives considered:** only the harness fake token (rejected: `make app-dev` and every other launch from a worker would still
  read the keychain); `[github]` off in `config/observatory.toml` (rejected: a config key is another thing to forget, the environment is
  the owner's launch path anyway); a CLI flag on the app (rejected: Tauri's launch path has no argv hook worth adding for this).
  Evidence: `cargo test -p observatory-core --test secrets` (the red/green test
  `the_forge_source_is_off_without_a_token_or_the_owner_flag`: fake keychain records 0 calls when both variables are unset or `GITHUB_TOKEN`
  is empty, 1 call with `FORGE_LIVE=1`, 0 calls when `GITHUB_TOKEN` is set); `make app-dev` with both variables unset logged
  `github.token source=off reason=no GITHUB_TOKEN and FORGE_LIVE unset` and no `github_started`, `github_poll` or `github.request` line; the
  e2e suite with the harness token: 32 cases, 0 failures.
- Row (tool fact, library/tooling.md; repeats the owner's 1343 and 1520 facts): the macOS keychain ACL is per binary hash. A rebuilt app or
  test binary prompts again for `droid-observatory/github-token`; "Always Allow" binds the secret to a throwaway build and does not carry to
  the next build. Hence no process a worker starts reaches the keychain (tests, the app under e2e, scripts, the browser host); only the
  owner's `FORGE_LIVE=1` launch does.
- Row (cycle rule, as widened by inbox 1520): "no worker test reads the keychain" (1343) is now "no process a worker starts reads the
  keychain", enforced by mechanism (off by default plus the harness fake token) and not by a sentence. A worker whose launched process
  prompts the owner's GUI records the row as owner-run, does not wait, and counts the live clause as not run whatever the answer. Applies to
  `m4-merge-ui-and-403`, `m4-forge-live-walk-2` and the M4 user-testing validator.
- Row (observation, e2e, 2026-10-29): the first `make e2e` on the green tree exited 2 after 285 s (`Spec Files: 4 passed, 2 failed`: five
  `gate.e2e.ts` cases failed with "the card never reached the build lane" after 1m44s, and `panel.e2e.ts` "renders the Panel within the
  configured latency" read 4161 ms against the 3000 ms limit). Two full reruns of the same binary on a clean run directory passed, 6 spec files, 32 cases, 82 s each: one with
  `GITHUB_TOKEN=` empty (forge off), one with the harness fake token (junit.xml 32 `<testcase>`, 0 `<failure>`); the gate spec alone on a clean
  directory passed both ways (6 cases, 5 to 6 s). Two gate-only runs on the directory the failed run left behind failed again (the board
  showed the PR already "handed to CD" from the earlier run: the store is reused, so `make e2e` removes it first). The cause of the first
  failure is not proven (a loaded machine is the suspect: the Panel case is a timing case). No B-row: not reproduced.

## Decisions

None.

## BACKLOG

None. No B-row.

## Deferred clauses

None.

## Library

- `library/tooling.md` "Forge and keychain facts": add that, from `d4d3194`, the keychain is read only under `FORGE_LIVE=1` with no
  `GITHUB_TOKEN`; the owner's launch is `FORGE_LIVE=1 make app-dev` (or `GITHUB_TOKEN="$(gh auth token)" make app-dev` for the token without
  the keychain). Workers launch the app with neither variable and read `github.token source=off` in the log.
- `.env.example` and the README "Environment and secrets" paragraph should name `FORGE_LIVE` at the integration (shared files, rule 13).
- Rule 30 claims: none to claim (`fulfills` is empty).
