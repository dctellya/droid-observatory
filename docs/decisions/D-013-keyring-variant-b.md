# D-013: keyring variant B, `keyring-core` with `apple-native-keyring-store`

**Date:** 2026-10-23
**Status:** accepted (owner decision 2026-10-22, entry 26; recorded at the S integration)

## Context

The app reads the GitHub token and the OTLP headers from the OS keychain (rule 4). `keyring`
4.2.0 with default features fails `cargo deny`: RUSTSEC-2026-0370, RUSTSEC-2026-0371 and
RUSTSEC-2026-0372 (zbus and zvariant unmaintained), reached on the Linux path through
`zbus-secret-service-keyring-store` (`reports/readiness/2026-10-22-dependency-readiness.md`).
The app ships for macOS only. Both items exist in the keychain as generic passwords with service
`droid-observatory/<name>` and account `dct`.

## Decision

Use variant B: `keyring-core` 1.0.1 and `apple-native-keyring-store` 1.0.2 with feature
`keychain`, selected with `set_default_store`, behind `observatory_core::secrets` (feature
`secrets`). Never `keyring` 4.x. Both crates are declared in the workspace `Cargo.toml` and
listed in `[package.metadata.cargo-machete] ignored` until the secrets module uses them.

## Alternatives

`keyring` 4.2.0 with default features (fails `cargo deny`); `keyring` 4.2.0 with the Linux
backends off (still a larger tree than the store needs); calling the macOS `security` CLI
(a process per read, and the value would pass through a pipe).

## Consequences

Linux has no secret store; there the environment variable is the only source (what CI uses).
The `security find-generic-password` lookups in tests use the full service string
`droid-observatory/<name>` and never `-w`. When the secrets module lands the machete ignore
entries are removed.
