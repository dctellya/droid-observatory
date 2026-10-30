# m5-fix-usage-ratios-kind-weight

## SPEC §9

- Row (F-NEW): **usage report ratios pinned to the delegate's definition (inbox 0145) and kind-weighted** (fix feature from scrutiny-validator-m5 round 1, blocking finding on `m5-usage-ratios`; rule 24, no scope, cap or model change). `scripts/usage_ratios.py` now buckets by `fulfills`, not by skill: contract worker w = worker cost on features that claim at least one contract assertion, divided by the assertions landed (blocked is not landed); coordination = orchestrator with its subagents / w; validation = validators with their seats / w; tooling = worker cost on features that claim none / w; integration (the integration-worker features) is its own column, outside the product. A `bases:` line under the Ratios table prints the window and the five numerators so a reader can rebuild the figure. Alternatives: keep the skill-based buckets (rejected: the delegate's note says the strict `fulfills` rule is the one to pin, and a core-worker fix round that claims nothing is tooling); put integration inside the product (rejected: the delegate shows it as its own term; the recorded rows have none).
- Row (F-NEW): **kind-weighted cost per assertion.** `Weighted cost per assertion` has a third row, `kind-weighted mechanical assertion`, next to raw and plan-weighted: units = landed assertions counted at `[kind_weights]` of `config/usage-ratios.toml` (mechanical 1.0, behavioural 1.25, mixed 1.5, each with a one-line reason; an unclassified assertion weighs 1.0). M4: 12 mechanical, 14 behavioural, 6 mixed = 38.50 units, worker 0.46 and 6.00 per unit (raw 0.56 and 7.22 over 32). The recorded M4 row's kinds are now 12/14/6 (inbox 0145 (b): the hand file's fifteenth behavioural assertion was blocked; blocked is not landed).
- Row (F-NEW): **M4 on the pinned bases** (window 2026-10-25T18:19:15Z to 2026-10-29T22:05:24Z, 32 landed): w 17.80 USD-eq = 0.56 per assertion, coordination 7.26, validation 2.82, tooling 1.90, integration 0.25, cost per assertion 7.22 (all-in 7.93). The three earlier readings answer different questions (inbox 0145 (a)): the hand figures 1.06 / 2.86 / 1.30 / 0.85, the 00:25 pages 0.54 / 5.99 / 2.82 / 1.81 (integration 0.40) and the earlier script row 0.87 / 4.64 / 1.80 / 0.86; the report names its own bases next to the table. The product 7.22 is unchanged by the re-bucketing (the same four buckets sum to the same total); the plan-weighted row now divides 7 claiming features with a turnBudget (502 planned turns) instead of 16 core and ui features.

## Decisions

None.

## BACKLOG

- Closed (do not file): the `m5-usage-ratios` B-new on the owner's derivation (script M4 row does not reproduce the hand figures): answered by inbox 0145 (a), and the report now pins the definition.
- Closed (do not file): the `m5-usage-ratios` B-new on kinds 12/14/6 against 12/15/6: inbox 0145 (b), the product's count stands.
- B-new: `reports/usage/m4.md` is 195686 bytes (limit 204800, 9114 bytes of headroom); no `reports/usage/<m>-sessions.md` split exists, so the per-session tables (Tokens per session, Sidecar) are still in the report. Reason: M5's report has more features and sessions than M4's and will cross the limit unless the two per-session tables move to a sessions file before `final-integrate-m5` writes `m5.md`.
- B-new: `reports/usage/m4.md` cannot be regenerated whole any more: the logs `droid-log-single.log.2026-10-20` to `.2026-10-22` have left `~/.factory/logs`, so a full run drops them from `Sources` and lowers `Mission to date` (total 1260.74 to 1129.92 USD-eq). This feature spliced only `Ratios`, `Cost by bucket` and `Weighted cost per assertion` (window-only, identical inputs, `cross-check: identical` line untouched). Reason: the integrator's `make usage M=m5` reads the same log directory, so M5's `Mission to date` will carry the same loss; keep the rotated logs (or archive them under the repo's `build-log/`-style exemption) before the boundary, or accept the lower to-date figure in the M5 report.
- B-new: the window end in the report is the `--to` that was passed, not a value the script derives from the integration landing; the ratio's "build window" is right only when the integrator passes `--to` = integration landed. Reason: deriving it needs a progress_log event for the integration landing, which the log does not carry (`worker_completed` of the integration feature is the nearest); the integrator's call stays as it is.

## Deferred clauses

- None. All four items of the feature description are done; `m5.md` is `final-integrate-m5`'s.

### Claims (rule 30)

None: `fulfills` is empty (infra fix, no contract assertion). Evidence for each `expectedBehavior` line:

| Clause | Evidence |
|---|---|
| `python3 scripts/usage_ratios.py self-test` exits 0, covers kind weighting and the 0145 bases | `python3 scripts/usage_ratios.py self-test` runs `scripts/tests/usage_ratios_test.py` (31 tests: `KindWeights` x5, `PinnedDefinition` x5, `RatiosSection` kind-weighted, bases line and recorded kinds, `SelfTest`); exit 0 |
| `m4.md` shows raw, plan-weighted, kind-weighted rows, the bases line, kinds 12/14/6 | `reports/usage/m4.md` `### Weighted cost per assertion` (three rows), `bases:` line under `### Ratios`, `kinds landed: 12 mechanical, 14 behavioural, 6 mixed` |
| `m4.md` under 204800 bytes | 195686 bytes; `sh scripts/check-file-size.sh` exit 0 |
| config carries the three weights with reasons | `config/usage-ratios.toml` `[kind_weights]`; `KindWeights.test_every_weight_has_a_one_line_reason_above_it` |
| red before green; cargo untouched | `fcf34a9` (red, exit 1, failures=9 errors=6 of 31) then `d5bbb5f` (green, OK); no Rust file changed |

## Library

- `usage_ratios.py` has one entry point: `python3 scripts/usage_ratios.py self-test`, which runs `scripts/tests/usage_ratios_test.py` with `USAGE_RATIOS_SELF_TEST_DEPTH=1` (the test that calls the self-test skips itself under it).
- Regenerating one boundary report's ratio sections after the logs rotated: run `make usage`-equivalent with the report's own `--from`, `--to`, `--now` and caps into a temp `--out`, then splice `### Ratios` up to `### Turns per assertion by assertion kind` into the committed file.
