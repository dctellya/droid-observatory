# Build times and per-feature compile cost (2026-10-23)

Written by `m1-f028-spec-pass` for F-029 (`ai/RULES.md` rule 23, D-025). Every figure is pasted from the
tool that produced it (rule 7); where a figure does not exist the report says "not recoverable" and
counts what is missing. Raw log of the measurement run: `/tmp/measure-builds.log` (not committed; the
excerpts below are verbatim).

## Method

- Machine: 10 cores, `CARGO_TARGET_DIR=/Users/dct/.cargo/shared_rust_target`, toolchain 1.99.1
  (`rust-toolchain.toml`). Load before the run (`uptime`): `load averages: 3,91 5,09 4,86`.
- Tree: the main checkout, branch `mission/observatory-v1`, base commit `6e22a6f`, with this pass's
  documentation edits in the working tree (no Rust file changed). The four commands ran one after the
  other from one script, `time` around each (bash `time`, `real`/`user`/`sys`), nothing else of mine
  running. The only process matching `cargo|trunk` before the warm checks and before coverage was the
  `sccache` server (`/Users/dct/.cargo/bin/sccache`, pid 15117); no cargo, trunk or tauri build was running.
- "Warm" = the shared target directory already held the build of this tree's dependencies from
  earlier runs. A cold `make check` was not re-measured: the toolchain and the dependencies did not
  change since the M0 integration (skill rule). A cold figure from the M0 user-testing handoff is quoted below.
  `trunk build` and `trunk serve` were not re-measured: no UI code changed in M0 (nothing to compare).

## Measured, warm

| Command | Exit | `real` | `user` | `sys` | Result line |
|---|---|---|---|---|---|
| `make check` | 0 | 30.383 s | 13.204 s | 7.188 s | 128 tests passed, 0 failed (sum of the `test result:` lines); markdownlint `Summary: 0 issues in 0 files`; lychee `48 Total ... 43 OK 🚫 0 Errors`; gitleaks `no leaks found`; cargo-deny `advisories ok, bans ok, licenses ok, sources ok`; machete clean; `replay` ran for the M0 and S fixtures |
| `cargo check -p observatory-core` | 0 | 1.268 s | 0.645 s | 0.146 s | `Finished dev profile [optimized + debuginfo] target(s) in 1.21s` |
| `cargo test -p observatory-core` | 0 | 11.631 s | 28.600 s | 5.386 s | 112 passed, 0 failed (sum of the `test result:` lines); `Finished test profile ... in 5.86s` |
| `cargo llvm-cov -p observatory-core --all-features --fail-under-lines 80 --summary-only` (alone) | 0 | 54.911 s | 5 m 57.653 s | 5.679 s | `TOTAL 3951 315 92.03% 373 41 89.01% 2469 143 94.21% 0 0 -` (lines 94.21 %, floor 80); `Finished test profile ... in 5.99s` |

Pasted `time` blocks (bash `time` output; the tab after each label is shown as a space, the `===` and `start` lines are the script's own markers):

```text
=== make check            start 2026-10-23T16:52:32Z
real 0m30.383s
user 0m13.204s
sys 0m7.188s
exit=0
=== cargo check -p observatory-core            start 2026-10-23T16:53:02Z
real 0m1.268s
user 0m0.645s
sys 0m0.146s
exit=0
=== cargo test -p observatory-core            start 2026-10-23T16:53:04Z
real 0m11.631s
user 0m28.600s
sys 0m5.386s
exit=0
=== cargo llvm-cov -p observatory-core --all-features --fail-under-lines 80 --summary-only   start 2026-10-23T16:53:15Z
real 0m54.911s
user 5m57.653s
sys 0m5.679s
exit=0
```

Reading: the inner loop (`cargo check -p` 1.268 s, `cargo test -p` 11.631 s) costs 12.899 s together;
the two milestone commands cost 30.383 s and 54.911 s, and coverage burns 5 m 57.653 s of CPU against
54.911 s of wall clock. That is the case for rule 23: the milestone commands are paid once, by the
integrator; the inner loop is cheap enough to run per feature.

Existing figures this does not supersede (AGENTS.md "Runtime limits", SPEC §5.8): cold `trunk serve`
55.94 s, incremental 1.46 to 2.08 s, `cargo tauri dev` to running app 37.66 s. A cold `make check`
(handoff of `user-testing-validator-m0` session `2cfa29a1`, scratch worktree with its own
`CARGO_TARGET_DIR`): 126.8 s. The `mission-checks` CI run on the mission branch is not a local build
time; its figures are in `reports/ci/`.

## Per-feature compile cost of M0

Sources, in order (skill): the handoffs' `verification.commandsRun` in
`~/.factory/missions/<id>/handoffs/` (wall time pasted from the observation text), then
`build-log/2026-10-23/<session>.jsonl`. **The hook `.factory/hooks/log-tool-call.sh` records no
duration:** per tool call it writes only `ts` (UTC completion time, whole seconds), `session_id`,
`event`, `tool`, `command`, `file`, `summary`, `script` (first 200 characters), `exit_code`. The gap to
the previous line contains model time, so a compile time cannot be derived from it. A session without
a pasted wall time is therefore "not recoverable" and counts as 0 in the compile share.

"Compile commands" = commands in the handoff whose text starts or chains a `cargo ...`, `make check`
(or another cargo-wrapping `make` target) or `trunk`; `make check-workflows` and `make check-docs`
style targets are not counted. "Logged" = `Execute` calls in the session's build-log file matching the
same pattern (the count of invocations, never a duration). The handoffs list a representative subset:
for `m0-mapping-v1` the handoff has 6 compile commands and the build-log 28.

### Feature workers

| Feature | Session | Compile commands in handoff | With a wall time | Wall time pasted | Logged invocations |
|---|---|---|---|---|---|
| `m0-ci-rust-job-linux` | `cb3408d4` | 2 | 1 | 85 s (`real 1m25s`: clippy workspace without the shell, clippy `observatory-ui` wasm32, `cargo test -p observatory-core -p observatory-cli --all-features`) | 8 |
| `m0-fixture-redaction` | `fef523bb` | 3 | 0 | not recoverable | 5 |
| `m0-fixture-freeze` | `4dd52b9e` | 5 | 0 | not recoverable | 7 |
| `m0-capture-header-fix` | `674b5777` | 4 | 0 | not recoverable | 3 |
| `m0-sidecar-and-sessions` | `b2f17dc0` | 6 | 0 | not recoverable | 13 |
| `m0-mapping-v1` | `51822991` | 6 | 0 | not recoverable | 28 |
| `m0-store-and-inspect` | `42bd16a7` | 8 | 0 | not recoverable | 20 |
| `m0-mapping-decision-and-coverage` | `12d8f267` | 3 | 0 | not recoverable | 3 |
| `m0-mapping-v1-gate-exit-code-fix` | `49a94281` | 5 | 0 | not recoverable | 11 |
| **9 feature workers** | | **42** | **1** | **85 s** | **98** |

Eight of the nine feature workers are "not recoverable"; across the nine, 41 of the 42 handoff
commands carry no wall time, and the build-log shows 98 compile invocations, none with a duration. The
one timed command is the CI job feature's dry run of the job it wrote.

### Boundary workers and validators (M0 window)

| Session | Role | Compile commands in handoff | With a wall time | Wall time pasted | Logged invocations |
|---|---|---|---|---|---|
| `2c04bd42` | `m0-integrate-s` (first complete start) | 4 | 0 | not recoverable | 11 |
| `36d06ed0` | `m0-integrate-s` (second pass) | 2 | 0 | not recoverable | 2 |
| `0b01efdd` | `scrutiny-validator-m0`, round 1 | 5 | 0 | not recoverable | build-log has no `Execute` lines |
| `150b4068` | `scrutiny-validator-m0`, round 2 | 6 | 0 | not recoverable | build-log has no `Execute` lines |
| `45b49564` | `user-testing-validator-m0`, round 1 | 4 | 2 | 1.35 s (`cargo build -q -p observatory-cli && cargo test --workspace --all-features --no-run`, the `Finished test profile` line only) + 54.29 s (`cargo llvm-cov ... --summary-only`, alone) = 55.64 s | build-log has no `Execute` lines |
| `2cfa29a1` | `user-testing-validator-m0`, round 2 | 2 | 1 | 126.8 s (`make check`, scratch worktree, cold); the `cargo llvm-cov` run has no time | build-log has no `Execute` lines |
| `b57208e3` | `m1-integrate-m0` (M0 integration) | 2 | 2 | 39.7 s (`make check` on `f66a9f5`) + 19.1 s (`make decisions-index && make check` after the shared-file edits) = 58.8 s | 4 |

The validators' build-log files hold no `Execute` lines (they drive cargo through other tool calls), and
24 validator child sessions (scrutiny reviewers and flow validators, listed in `reports/usage/m0.md`)
have no handoff of their own; their compile time is not recoverable and counts as 0.

### Totals used by the compile share

Pasted wall times of M0, from the tables above: 85 + 55.64 + 126.8 + 58.8 = 326.24 s, from 6 compile
commands out of 67 in the 16 handoffs. The method and the denominator are in
`reports/usage/m0.md`, "Compile share". Twelve of the 16 sessions are "not recoverable"; the figure is a
lower bound, not an estimate of the real compile time.

## What changes from M1

Rule 23 makes the wall time part of the handoff (`verification.commandsRun` carries the pasted
`real` or `Finished ... in N.NNs` of every cargo and trunk invocation), so the next report derives the
per-feature compile cost from the handoffs without a gap. The integrator pastes `time make check` and
the `time cargo llvm-cov` of the milestone's single full run into the usage report's compile share.
