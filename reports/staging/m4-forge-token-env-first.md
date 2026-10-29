# Staging: m4-forge-token-env-first

Feature `m4-forge-token-env-first`, worktree `observatory-wt/m4`, branch `wt/m4`. Acts on the delegate's inbox file
`docs/owner-feedback/inbox/2026-10-29-1343-no-keychain-in-worker-tests.md` (no worker test reads the keychain; live
forge checks are owner-run). Claims (rule 30): the feature fulfils no validation assertion, so there is nothing to claim.

## SPEC §9

- Row (rule 24, F-NEW): the GitHub token resolves environment first. A non-empty `GITHUB_TOKEN` is the token and the
  keychain `droid-observatory/github-token` is not read at all (zero calls on the injected keychain); the keychain is read
  only when the variable is unset or empty, and a refusing keychain then is the answer (no token, the refusal). The owner's
  Q2 is unchanged: the read-only PAT stays in the keychain. `core::secrets::resolve_github_token(env, keychain_closure)`
  returns the token and a `TokenSource` (`env`, `keychain`, `none`); the shell reads it once at start (one secrets call per
  app process, never per poll or per mission) and logs `github.token source=env|keychain|none`, never the value.
  Alternatives: keychain first with the item's access list pre-authorised (rejected: the list binds to one binary hash, see
  the tool fact); a config switch for the order (rejected: a setting for a problem the environment variable already solves).
- Row (tool fact): the macOS keychain access list is per binary code hash. Every rebuilt test or app binary prompts anew for
  the item, and "Always Allow" binds the secret to a throwaway executable (the `forge_live-1607b840b0124955` prompt of
  2026-10-29 13:41). Reading the keychain from a test or a dev build is therefore never a check a worker can complete.
- Row (cycle rule): no worker test reads the keychain. A test that needs the real token is an owner-run row (attended class,
  M3 item 18): `#[ignore]`, returns early printing `skipped: FORGE_LIVE unset (owner-run)` unless `FORGE_LIVE=1`, takes the
  token from `GITHUB_TOKEN` only and never calls the keychain. A worker whose check would prompt the owner's GUI marks the
  row owner-run and moves on; a prompt that already appeared makes the check "not run", whatever the answer. Every
  `cargo test --workspace` test goes through an injected fake: resolution takes the keychain as a closure, and
  `a_service_without_an_item_is_none_not_an_error` runs on keyring's mock store. The renamed
  `the_live_demo_pair_is_polled_with_the_owner_run_token` was not run.

## Decisions

None.

## BACKLOG

None. (The earlier BACKLOG candidate in `m4-forge-core-and-pr-detection.md`, "read the token once at start and not per
mission", is landed here; the signed-build alternative stays as written there.)

## Deferred clauses

None.

## Library

- `GitHubClient::with_token(config, token)` builds a client for an already resolved token; `from_config` is
  `with_token(config, github_token()?)`. `Secret` has no `Clone`, so the shell copies the value into each new client.
- Red/green order: `22cdb27` (behaviour-preserving refactor, the keychain behind a closure, the order unchanged), `ca7f87d`
  (red: `the_environment_wins_and_the_keychain_is_not_consulted`, exit 101), `3c73008` (green).
- `g2-tests.tsv` has no forge or secrets row (checked: the only keychain row is `VAL-TELEM-006`, not this feature); none flipped.
