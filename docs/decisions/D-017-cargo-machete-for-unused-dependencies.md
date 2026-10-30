# D-017: cargo-machete for unused dependencies, not cargo-udeps

**Date:** 2026-10-23
**Status:** accepted (SPEC §7 as settled in B-014; wired in S)

## Context

B-014 asked for a dead-dependency check in the mission-branch CI. `cargo-udeps` needs a nightly
toolchain, which is a second toolchain to pin and cache. `cargo-machete` 0.9.2 runs on stable in
seconds.

## Decision

`cargo machete` is part of `make check` and a `machete` job of `mission-checks`
(`taiki-e/install-action`, `cargo-machete@0.9.2`). A dependency declared ahead of use sits in that
package's `[package.metadata.cargo-machete] ignored` list; the first module that uses it removes the
entry. No dependency is added to the list to silence a crate nobody plans to use. At the S
integration the lists hold `rusqlite`, `keyring-core` and `apple-native-keyring-store` (core) and
`tokio` and `observatory-core` (src-tauri).

## Alternatives

`cargo-udeps` (nightly; finds a strict superset only in edge cases); no check (declared-ahead
dependencies would rot unnoticed).

## Consequences

Revisit if machete misses a real unused dependency. A feature that starts using an ignored crate
must delete its entry in the same commit, or the ignore hides a later removal.
