# m5-infra-gate-and-flake-fixes

## SPEC §9

- Row (F-NEW): **B-179 decision, the `.factory` hook files stay in the gate key** (taken under rule 24: no scope, cap or model change). The M4 boundary final gate was a MISS (443.829 s) on three hook files added after the last validated tree. Keep: `.factory/hooks/*.sh` are executed code that `scripts/tests/log-tool-call-duration.sh` runs inside `make check-scripts`, which is inside the cached gate, so a hook edit must invalidate the proof or the test would not rerun on a changed hook; the contract checks `g4-hooks-json` and `g6-hooks-config` read the hooks outside the cache and would not notice. Alternatives: (a) exclude `.factory/hooks*` from the key (`GATE_EXCLUDE_DIRS` is a list of directories; it would need a path list and a mirror in the `paths-ignore` of `mission-branch.yml`), which saves one rebuild per hook edit and lets a hook change reach a proven tree unchecked by the hook test; (b) exclude only `officer-sessions.txt` and `hooks.json` (data the hook test does not read) and keep `*.sh`, which needs the same path-list machinery for a file pair that changes at most once per milestone. No file changed. The cost of keeping is one real gate run after a hook edit; the way to avoid it is to land hook edits before the milestone's last validated tree, as rule C already requires. Reversible by editing `GATE_EXCLUDE_DIRS` and `config/observatory.example.toml` together in a later feature.
- Row (F-NEW): **M5 infra slice, six tooling items closed** (feature `m5-infra-gate-and-flake-fixes`, batch 1): B-181 the two live latency cases of `event_batches.rs` take `common::latency_bound`, 4x the plain bound when `cargo llvm-cov` exports `CARGO_LLVM_COV` (red: two of three whole-binary coverage runs failed, 1.572579958 s against 1 s and 478.450208 ms against 450 ms; `make coverage` exit 0 twice, 3m13.185s then 3m7.070s forced, `TOTAL` line coverage 93.71% in both); F-274/B-183 `make check-wasm` is in `check-all`; B-178 `integrate.sh` strips `[skip ci]`, `[ci skip]`, `[no ci]`, `[skip actions]` and `[actions skip]` from the subjects it lists in the merge message; B-182 `gate.sh` replays only a record with `exit_code` 0 (a red record is a miss that says so and is replaced by the next run); B-184 `demo-base-cut.txt` (read by `g7-demo-base`, never moves) is split from `git-baseline.txt` (read by `range_base`, moves at each boundary), both `3ccf3aac518a327d3c73c03cbc9efdf77bbc1ce8` today and documented in `scripts/contract-checks/README.md`.

## Decisions

None.

## BACKLOG

- Closed (do not file): B-178, `integrate.sh` strips the skip markers from the listed subjects (the commits keep theirs); `integrate_test.sh` checks the merge message.
- Closed (do not file): B-181, the latency bounds widen under `cargo llvm-cov`; the cases stay in the plain test run at their plain bounds.
- Closed (do not file): B-182, `gate.sh` never replays a red record; `gate_test.sh` covers the red rerun and the green replacement.
- Closed (do not file): B-183, `make check-wasm` runs in `check-all` and `g9-check-target-list` requires it (a planted wasm32-only type error made it exit 2).
- Closed (do not file): B-184, two files, documented in `scripts/contract-checks/README.md`.
- Closed (do not file): B-179, decided in the first SPEC §9 row (keep).
- B-new: the `check-wasm` gate is `cargo check` of the plain feature set; the CI `rust` job runs clippy with `--all-features` on wasm32 (`mission-branch.yml`), so a break behind a feature that only `--all-features` turns on shows in CI and not in `make check`; reason: the UI crate has no feature that differs today, revisit when it gets one; earliest M6.

## Deferred clauses

- B-180 (a step in `integrate.sh` that moves `scripts/contract-checks/git-baseline.txt` to the boundary tip) is not part of this feature: the file split that blocked it (B-184) has landed, so the step is now unblocked. It stays a claim on the integrator's next boundary (`m5-integrate-m5`) or a later M5 infra feature; until then the integrator moves the file by hand and `demo-base-cut.txt` stays untouched.
- Root `README.md` (a shared file, not edited by workers) names the targets and the baseline file: the integrator adds `make check-wasm` to its target list and a line for `demo-base-cut.txt` next to `git-baseline.txt`.

## Library

- `cargo llvm-cov` exports `CARGO_LLVM_COV=1` to the test processes it starts (seen: the instrumented run printed `bound=4s` and `bound=1.8s`); a test that measures wall time reads it at run time, no `cfg` is involved.
- Under `cargo llvm-cov` the whole `event_batches` binary failed in two of three runs, the same two cases passing alone (13.14 s run of the two cases, exit 0): the other `live::` tests run beside them and instrumented code is several times slower, so run the whole binary when reproducing.
- `make coverage` is cached by the gate: a second run in a row is a hit (green records only, since B-182); `make coverage FORCE=1` reruns it.
