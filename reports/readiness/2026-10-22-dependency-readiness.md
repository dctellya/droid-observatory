# Dependency readiness: toolchain, crates, Grafana OTLP, GitHub, session schema

Date: 2026-10-22. Worker session `82ad1581-fe75-4436-ac9c-9bc1e20a789b` (dependency readiness).
Machine: macOS (darwin 25). Scratch trees under `/tmp/obs-readiness/` (`leptos-tauri`, `core-deps`,
`core-deps-b`); install logs in `/tmp/obs-readiness/logs/`. Numbers are pasted from the tools that
produced them (ai/RULES.md rule 7). The repository was not touched by the worker.

Verdict: the toolchain, the Tauri 2 + Leptos CSR build path, the core crate set (variant B for the
keychain), the Grafana OTLP endpoint and the GitHub API are ready for the mission. Four owner
decisions were taken from these findings (see "Owner decisions" below); three of them become
decision files at S integration.

## Tools installed globally

| Tool | Version | Install time | Log |
|---|---|---|---|
| trunk | 0.22.0 | 579 s (incl. lock wait) | `logs/install-trunk.log` |
| tauri-cli | 2.12.4 | 305 s | `logs/install-tauri-cli.log` |
| cargo-llvm-cov | 0.9.1 | 98 s | `logs/install-cargo-llvm-cov.log` |
| cargo-machete | 0.9.2 | 58 s | `logs/install-cargo-machete.log` |
| cargo-deny | 0.20.2 | 174 s | `logs/install-cargo-deny.log` |
| rustup component `llvm-tools-preview` | (toolchain 1.99.1) | - | `logs/install-llvm-tools-preview.log` |

`make doctor` reports all ok afterwards. `agent-browser 0.37.1` is available (used by the
validation readiness run, see `2026-10-22-validation-readiness.md`).

## Tauri 2 + Leptos CSR path

Scratch tree `/tmp/obs-readiness/leptos-tauri` (Leptos CSR `ui` crate built by Trunk, Tauri 2
`src-tauri` shell).

- `trunk build --release`: 66 s cold. Log tail (`logs/trunk-build.log`):

  ```text
      Finished `release` profile [optimized] target(s) in 52.10s
  2026-10-22T14:45:25.604033Z  INFO downloading wasm-bindgen version="0.2.129"
  2026-10-22T14:45:27.332718Z  INFO installing wasm-bindgen
  2026-10-22T14:45:27.827815Z  INFO ✅ success
  ```

- `cargo tauri build --debug` works with `bundle.targets: ["app"]`: 6 s cached
  (`logs/tauri-build2.log`):

  ```text
      Finished `dev` profile [optimized + debuginfo] target(s) in 2.83s
       Built application at: /Users/dct/.cargo/shared_rust_target/debug/src-tauri
      Bundling obs-readiness.app (/Users/dct/.cargo/shared_rust_target/debug/bundle/macos/obs-readiness.app)
      Finished 1 bundle at:
          /Users/dct/.cargo/shared_rust_target/debug/bundle/macos/obs-readiness.app (10.93 MiB)
  ```

- DMG bundling fails non-interactively (`logs/tauri-build.log` line 227):

  ```text
  execution error: Finder got an error: AppleEvent timed out. (-1712)
  Failed running AppleScript
  ```

- Icons are required (`cargo tauri icon`); `beforeBuildCommand` runs from the app root.

Owner decision: `.app` bundle locally, DMG only in the CI `bundle` job.

## Crate versions resolved 2026-10-22

| Crate | Version | Note |
|---|---|---|
| rusqlite | 0.40.2 | `features = ["bundled"]` |
| serde_json | 1.0.154 | |
| tokio | 1.54.1 | features `rt-multi-thread, process, macros`; app/host only, core stays sync |
| keyring | 4.2.0 | default features fail `cargo deny`, see below |
| keyring-core | 1.0.1 | variant B |
| apple-native-keyring-store | 1.0.2 | variant B, feature `keychain`, call `set_default_store` |
| tracing | 0.1.44 | |
| tracing-subscriber | 0.3.23 | |

`keyring 4.2.0` with default features fails `cargo deny` (scratch `core-deps`, `deny.log`):
RUSTSEC-2026-0370, RUSTSEC-2026-0371, RUSTSEC-2026-0372 (zbus/zvariant unmaintained, pulled in on
the Linux path through `zbus-secret-service-keyring-store v1.0.1` → `zbus v5.19.0`).

Variant B (scratch `core-deps-b`) passes `cargo deny` and is accepted by the owner (decision file
due at S integration):

```toml
keyring-core = "1.0.1"

[target.'cfg(target_os = "macos")'.dependencies]
apple-native-keyring-store = { version = "1.0.2", features = ["keychain"] }
```

Suggested dependency lines for the workspace:

```toml
rusqlite = { version = "0.40.2", features = ["bundled"] }
tokio = { version = "1.54.1", features = ["rt-multi-thread", "process", "macros"] }  # app/host only
opentelemetry_sdk = { version = "<pinned on the day>", features = ["rt-tokio", "metrics"] }
opentelemetry-otlp = { version = "<pinned on the day>", default-features = false, features = ["http-proto", "metrics", "reqwest-blocking-client"] }
```

The OpenTelemetry crate versions are pinned by the worker that adds them (rule 3); only the
feature sets were verified here.

## Grafana Cloud OTLP

Ready with one caveat. The keychain item `droid-observatory/otlp-headers` holds a `name=value`
string (`Authorization=Basic …`, 212 characters). Sent verbatim as a header the gateway answers
`401`; split on the first `=` into header name and value it answers `200` with body `{}`. The
exporter must parse `name=value` from the keychain string. The value is never printed or stored.

## GitHub

- Rulesets API works; the `main-gate` ruleset has id `25636994`, as expected.
- Check-runs API answers for the `gh` token; that token can create rulesets.
- Keychain `droid-observatory/github-token`: present. The worker's first lookup used service
  `droid-observatory` with account `github-token`, which does not exist, hence a false "missing".

Keychain attributes verified on 2026-10-22 with `security find-generic-password -s <service>`
(attributes only, no `-w`; secret values never printed):

| Item | Class | `svce` | `acct` | Created |
|---|---|---|---|---|
| `droid-observatory/github-token` | `genp` (generic password) | `droid-observatory/github-token` | `dct` | `20261022151641Z` |
| `droid-observatory/otlp-headers` | `genp` (generic password) | `droid-observatory/otlp-headers` | `dct` | `20261022125310Z` |

Lookup contract for the app: generic password, service `droid-observatory/<name>`, account `dct`
(the account is the login user, not the item name).

Owner statement on the token: fine-grained PAT, owner `dctellya`, repositories `droid-observatory`
and `artstudio`, read-only on Contents, Issues, Pull requests, Actions, Commit statuses and
Metadata. Fine-grained tokens have no Checks permission, so the app reads check results through
Actions runs (`/repos/{owner}/{repo}/actions/runs`) and commit statuses
(`/repos/{owner}/{repo}/commits/{sha}/status`), not through the check-runs API.

Open point for the M3 boundary: a read-only token cannot perform the app's one outward write (the
merge); the app must surface the resulting `403` cleanly.

## Droid session file schema (M0 mapping input)

Sessions directory:
`~/.factory/sessions/-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/`. At the
worker's check: 12 `.jsonl` files, each with a sibling `<id>.settings.json` and `.bak`; no
subdirectories, child sessions are sibling files. Re-verified while writing this report
(`ls | wc -l` → `42` entries, `ls | grep -c '\.jsonl$'` → `14`). `droid --version` → `0.247.0`.

- JSONL `type` values seen: `message`, `todo_state`, `agent_turn_outcome`, `compaction_state`,
  `session_start`.
- `session_start` carries `callingSessionId` / `callingToolUseId` (parent linkage); its `version`
  is a numeric schema version (`2`). There is no Droid version in the JSONL or in `.settings.json`.
- `modelId`, `apiProvider`, `reasoningEffort` are on assistant messages.
- Token usage lives in `.settings.json` (`tokenUsage`, `childInclusiveTokenUsageBySessionId`,
  `inclusiveTokenUsage`, `lastCallTokenUsage`, `assistantActiveTimeMs`), not as `usage.reported`
  JSONL lines. `.settings.json` also has `tags`, `model`, `reasoningEffort`, `interactionMode`,
  `autonomyLevel`.

Owner decision: record `droid --version` in the fixture header as the mapping-version signal, read
the sibling `.settings.json` for usage and child links, decision file at S integration, amend the
SPEC as-built at that integration (rule 8).

## Owner decisions (2026-10-22)

1. keyring: variant B (`keyring-core` + `apple-native-keyring-store`), decision file at S integration.
2. DMG only in the CI `bundle` job; `.app` locally.
3. Session schema: `droid --version` in the fixture header, sibling `.settings.json` for usage and
   child links; decision file and SPEC amendment at S integration.
4. GitHub token is read-only; check results come from Actions runs and commit statuses, not the
   Checks API; decision file at S integration.

Related: `2026-10-22-validation-readiness.md` (validation path, concurrency limits).
