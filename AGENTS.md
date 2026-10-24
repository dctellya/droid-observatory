# AGENTS.md

How agents work in this repository. The rules are in [ai/RULES.md](ai/RULES.md) and win over
anything here; this file is the operational summary.

## What this is

droid-observatory: a Tauri 2 + Leptos (Rust) desktop app that watches a Factory Mission and
measures the waste and effectiveness (cycle time, cost) of the engineering cycle from the spec to
the PR's checks going green. The lanes are the seven mission phases (1 Plan, 2 Contract, 3 Build,
4 Scrutiny, 5 User testing, 6 Integrate, 7 Gate); roles are colour and attribution; human and
commander actions are overlays, not lanes; time, waste, cost and model drift are the Panel. Anything
after the merge (release, observe, deploys, flags) is out of scope: the last event is `handed_to_cd`
(F-028, D-024).
PRD: `ai/prd/droid-observatory-prd.md` (its §7, §8 and §13 are superseded by F-028 where noted).
Spec: `SPEC.md` (approved 2026-10-22, amended v0.4 at the M0 to M1 boundary; the mission plans from it).

Current state: the walking skeleton (S), the stream capture and mapping milestone (M0) and the board in replay and
attach (M1) are integrated on `mission/observatory-v1` (M1 on 2026-10-24): core crate, canonical `Event`, mapping v1 with
its golden file, SQLite pointer store, `observatory-cli replay` and `inspect`, view models, the attach watcher, the
usage-log source (`turn.usage` is Panel data, D-029), a Leptos board with Details and agents views (D-030: go) and the
Tauri host with live attach. Each later milestone deepens one area from it. From M2 on, a change to the store schema
is a new numbered migration (`0002_*.sql`); `0001_init.sql` is never edited in place again (a dogfooding store exists at
`~/.droid-observatory/observatory.sqlite`).

## Setup, build, test

```sh
make doctor          # tool check with install hints
make hooks           # enable .githooks (once)
make check           # all light checks = mission-branch CI
make fmt             # rustfmt (no-op until Cargo.toml exists)
make lint            # clippy -D warnings
make test            # cargo test --workspace
make deny            # cargo-deny advisories, licences, bans
make replay-fixtures # observatory-cli replay over fixtures/streams/*.jsonl
make app-dev         # Tauri dev window (after the app exists)
```

Toolchain: `rust-toolchain.toml` (1.99.1, rustfmt, clippy, wasm32 target). Do not change it
without a decision file.

The build is paid in series (rule 23, D-025). Inner loop, per feature: `cargo check -p <crate>` and
`cargo test -p <crate>` for the crates you touch, plus `cargo fmt --all -- --check` and
`cargo clippy -p <crate> --all-targets -- -D warnings` once before handoff. `make check` (the full
`cargo test --workspace`) and `cargo llvm-cov -p observatory-core --all-features --fail-under-lines 80
--summary-only` run once per milestone, by the integrator, the coverage run alone. Paste every cargo
or trunk wall time into the handoff. `cargo-nextest` is not installed; do not install it.

## Runtime limits (measured)

Measured 2026-10-22 on the owner's machine; source `reports/readiness/2026-10-22-validation-readiness.md`
(SPEC §5.8):

- At most 3 concurrent agent-browser validator sessions per `trunk serve` (1.34-1.54 GB RSS each,
  CPU negligible idle); use `--session <name>` per validator.
- `cargo llvm-cov` runs alone (1-min load 3.3 → 14.6 on 10 cores during the scratch run).
- All cargo builds serialize on the shared `CARGO_TARGET_DIR` lock
  (`Blocking waiting for file lock on build directory`); expect waits, not failures.
- `trunk serve` and `cargo tauri dev` coexist only when exactly one trunk owns port 3141
  (`cargo tauri dev` starts its own); killing `cargo-tauri` alone orphans trunk and the app binary.
- Cold `trunk serve` 55.94 s, incremental 1.46-2.08 s, `cargo tauri dev` to running app 37.66 s.
- The Tauri window has no CDP endpoint; validators use the smoke check (process alive, `Running`
  log line, WebKit connections on 3141, System Events listing) plus the browser path on 3141.

## Interactive QA path

Until the app exists: `make check` is the only runnable target.
Once the skeleton lands, the path the validators follow is:

1. `make doctor` reports no missing required tools.
2. `cargo run -p observatory-cli -- replay fixtures/streams/<capture>.jsonl` prints lane
   events and ends with `unknown events: N`. `replay <root.jsonl>` includes the child sessions
   and ends `unknown events: 0` on both fixtures: S 661 events over 4636 records, M0 1279 events
   over 8735 records. `replay --root-only <root.jsonl>` reads the root file alone: S 364 events
   over 2027 records, M0 474 events over 2820 records. `make replay-fixtures` replays with
   children.
3. `make app-dev` opens the window; open the same fixture (Replay); the seven phase lanes and the
   Panel render and a card opens its Details view.
4. With the window open, start `droid exec --mission ...` in a terminal; the new session
   appears on the board (Attach) and each event shows within 1 s.

No login gate: the app watches and reads local session files and uses the GitHub token from the keychain or
`GITHUB_TOKEN` only for the gate lane.

## Layout and ownership

```text
crates/observatory-core   ingest, model, metrics, store, control (no UI deps)
crates/observatory-cli    headless replay/inspect over core
apps/observatory-app      Tauri 2 shell + Leptos UI
fixtures/streams          captured mission streams, named <YYYY-MM-DD>-<slug>.jsonl
docs/decisions            D-NNN-slug.md, one per decision; README.md is generated
docs/runbooks             recovery procedures: red gate, fix cycle, stall, hooks, release
reports/                  usage, waste, readiness; pasted from tools
build-log/                tool-call log written by .factory hooks
ai/                       PRD, rules, template, session logs
```

Crate and app paths are the contract the Makefile and CI probe for; keep them.

## Conventions

- Rust 2024 edition, `rustfmt.toml` and `clippy.toml` as committed. Warnings are errors.
- Naming: standard Rust (`snake_case` items and modules, `CamelCase` types, `SCREAMING_SNAKE`
  consts). Event kinds mirror the PRD §7 names in `snake_case`
  (`finding.blocking` → `FindingBlocking`). Role enums use the PRD's words; the lane enum is the seven phases of D-024.
- Tests: unit tests in `#[cfg(test)] mod tests` next to the code; integration tests in
  `crates/<crate>/tests/<topic>.rs`; fixture-driven tests read `fixtures/streams/`. Test
  functions read as sentences: `rework_window_closes_on_resolution()`.
- Tests before code (rule 1): commit the red test, then the code that turns it green.
- No magic numbers: every threshold lives in `config/observatory.example.toml` with a reason.
- Debt markers: `TODO(#123)` or `TODO(B-123)`; anything else fails `make check-todos`.
- Markdown: `.markdownlint-cli2.yaml`; links must resolve (`make check-links`).
- Files over 200 KB fail CI unless under `fixtures/streams/` or `build-log/`.

## Git

- Forward only (rule 12): never amend, reset, rebase, stash, or force-push. Fix with a new
  commit. `.factory/hooks/guard-git.sh` and `.githooks/pre-push` enforce this.
  `guard-git.sh` blocks every stash form of git, including the read-only `list` subcommand;
  inspect the tree with `git status` and `git diff`.
- The Linux CI `rust` job does not lint or test `apps/observatory-app/src-tauri` (D-022). Shell
  changes are caught by local `make check` and by the macOS bundle job on the final pull request.
- Nothing reaches `main` except the mission's single pull request. `main` has a ruleset:
  PR required, `gate` check required, no force push, no bypass.
- Milestones live in worktrees and integrate into `mission/<name>`. Two worktrees never edit
  the same file (rule 13); shared files (`SPEC.md` §9–10, `README.md`, `BACKLOG.md`,
  `docs/decisions/README.md`) are written by the orchestrator at integration time.
- Commit subject under 72 characters; co-author trailer for agent commits:
  `Co-authored-by: factory-droid[bot] <138933559+factory-droid[bot]@users.noreply.github.com>`.

## Decisions, backlog, logs

- A decision is one file: `docs/decisions/D-NNN-slug.md` with `**Date:**`, `**Status:**`,
  context, decision, alternatives, consequences. Run `make decisions-index` after adding one.
- Deferred work goes to `BACKLOG.md` as `B-NNN` with a reason, not into TODOs or GitHub issues
  (rule 14; owner decision 2026-10-22 at mission approval).
- Every session is logged under `ai/session/` with transcript links; every tool call is
  logged to `build-log/` by the PostToolUse hook. Do not disable the hooks.
- Usage and waste per milestone go to `reports/` pasted from the tool that produced them; usage reports carry the
  per-role input, output and cache-read token table from `~/.factory/logs/droid-log-single.log`, extracted with
  `scripts/usage-from-log.sh` (rule 7, D-029).

## Mission setup

Models per role (rule 19; `.factory/settings.json`; D-006):

| Role | Model |
|---|---|
| Orchestrator | `claude-fable-5.1` |
| Workers | `claude-sonnet-5-5` |
| Validators (scrutiny, user testing) | `claude-opus-5-5` |

Command: see README "Running it as a mission".
Reasoning effort: orchestrator `high`, workers `high`, validators `high` (SPEC §2).
A model change is a decision file.

The human answers only at milestone boundaries or when the stall flag is up (rule 15).
Deviations from the spec are proposed with reason and cost, and wait for approval (rule 9).
When a decision comes with a recommendation and changes neither scope, nor usage caps, nor the
models per role, the orchestrator takes the recommendation, records it as a SPEC §9 row with the
alternatives and reports it at the next boundary (rule 24, D-026).

The orchestrator never polls (rule 25, D-027): it waits in one blocking call sized to the feature's remaining
cap, and a cancelled worker call or a run that does not return is re-dispatched or failed in the same turn, with
a SPEC §9 row for the cause. While the mission is paused it does nothing until the owner resumes it.

## Environment and secrets

See `.env.example`. Secrets come from the OS keychain (`droid-observatory/<name>` entries)
or the environment, never from the repository, the store, or logs. Raw session JSONL may
contain secrets: render read-only, never upload, never copy into the store.

- The GitHub token is a read-only fine-grained PAT; read check results via Actions runs
  (`/repos/{o}/{r}/actions/runs`) and commit statuses (`/repos/{o}/{r}/commits/{sha}/status`), not the
  Checks API (no Checks permission). Keychain items are generic passwords with service
  `droid-observatory/<name>` (see `reports/readiness/2026-10-22-dependency-readiness.md`).

Grafana Cloud OTLP (`OTEL_EXPORTER_OTLP_*`) is the telemetry backend; export is off unless
`[telemetry].enabled = true`.

## Do not

- Add a dependency without `make deny` passing and a version verified on the day (rule 3).
- Write to `main`, rewrite history, or touch another worktree's files.
- Estimate a count, timing or token number: paste it from the tool (rule 7).
- Drop an unknown event: count it and show it (rule 18).
