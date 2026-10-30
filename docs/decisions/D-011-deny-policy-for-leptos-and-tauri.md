# D-011: cargo-deny exceptions for the Leptos and Tauri dependency trees

**Date:** 2026-10-22
**Status:** accepted (worker decision at the S scaffold); confirmed by owner 2026-10-23

## Context

`deny.toml` was written before any dependency existed. The first `cargo deny check` over
the workspace (Leptos 0.8.22, Tauri 2.12.5, core with all features) failed with:

- licence `BSL-1.0` of `xxhash-rust` 0.8.19, pulled in by Leptos `server_fn`;
- advisory RUSTSEC-2024-0436 (`paste` 1.0.15 unmaintained), a build-time proc-macro used by
  Leptos, with no safe upgrade available;
- advisory RUSTSEC-2024-0370 (`proc-macro-error` 1.0.4 unmaintained), reached only through
  the GTK crates Tauri uses on Linux.

Rule 6 and `deny.toml` make any new licence or ignored advisory a decision file.

## Decision

- Allow `BSL-1.0` (Boost Software Licence, permissive, OSI approved).
- Ignore RUSTSEC-2024-0436 with the reason recorded in `deny.toml`.
- Set `[graph].targets` to `aarch64-apple-darwin`, `x86_64-apple-darwin` and
  `wasm32-unknown-unknown`: the app is bundled for macOS only (`bundle.targets = ["app"]`)
  and the UI compiles to wasm. The Linux GTK tree, the only path to RUSTSEC-2024-0370, is
  not part of anything shipped.

## Alternatives

Ignore RUSTSEC-2024-0370 as well and keep all targets (checks code nobody ships, and the
ignore would hide a real advisory if a shipped crate ever pulled it in); drop Leptos (the
UI choice is D-002); `unmaintained = "workspace"` (silences every transitive unmaintained
advisory, including future ones nobody reviewed).

## Consequences

Any new licence, ignored advisory or target still needs a decision file. Remove the
RUSTSEC-2024-0436 ignore when Leptos drops `paste`. Linux support would need the target
list revisited and RUSTSEC-2024-0370 decided.

## Addenda (owner, 2026-10-23, S to M0 boundary)

- BSL-1.0 is the Boost Software License (not Business Source).
- RUSTSEC-2024-0436 is ignored with a re-check date 90 days out: 2027-01-21.
