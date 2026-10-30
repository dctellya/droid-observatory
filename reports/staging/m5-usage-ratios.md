# m5-usage-ratios

## SPEC §9

- Row (F-NEW): **usage report Ratios section** (feature `m5-usage-ratios`, batch 2; owner items 10 and 27 of the M4 boundary; rule 24 decisions inside the slice, none changes scope, caps or models). `scripts/usage_ratios.py` writes `### Ratios` and five companion tables from the tables the report already holds: cost per assertion = worker cost per assertion x (1 + coordination + validation + tooling); `Cost by bucket` reconciles the buckets to the window total; `Weighted cost per assertion` puts the raw figure next to the one weighted by plan turn budget (a standard assertion is `standard_assertion_turns` = 23.3 planned turns, `config/usage-ratios.toml`); `Turns per assertion by assertion kind` (mechanical, behavioural, mixed, unclassified, from the contract's `Kind:` lines, a feature's turns and cost split equally over the fulfilled assertions it claims); `Turns per assertion by worker skill` (B-177) and `Turns per landed assertion by role` (item 27).
- Row (F-NEW): **bucket definitions fixed where the owner's note left a choice.** Contract worker = features whose skill is core-worker or ui-worker (their sessions and the subagents they started, fix rounds included); coordination = orchestrator and orchestrator subagent roles; validation = validator role (scrutiny and user testing with their subagents); tooling = infra-worker features; integration features, self-check missions and other sessions stay in the window total and in no ratio. Each ratio is the bucket's USD-eq over the contract worker's.
- Row (F-NEW): **the owner's M3 and M4 hand figures are the first data rows**, kept in `config/usage-ratios.toml` as recorded rows (M3 coordination about 0.9, validation about 1.4; M4 worker 35 USD-eq, 1.06 per assertion, coordination 2.86, validation 1.30, tooling 0.85, 1.06 x 6.01 = 6.37 against 6.70 all-in; kinds 12 mechanical, 15 behavioural, 6 mixed). The script's own row of the same milestone is printed under them.
- Row (F-NEW): **`single-call share` line (B-176).** `usage_rewrites.single_call_line`, under `orchestrator turns per feature boundary`: assistant records of the orchestrator transcript in the window with exactly one `tool_use` block over those with at least one. M4 window: 220 / 220 = 100.0 % (211 closing-text turns with none), so the lead made no parallel call in the window.

## Decisions

None.

## BACKLOG

- Closed (do not file): B-176, the `single-call share` line exists (`usage_rewrites.single_call_line`; `usage_ratios_test.py` `SingleCallShare`).
- Closed (do not file): B-177, per-kind turns per assertion in `make usage` (`### Turns per assertion by worker skill` with the D-045 contract row, 25.72 on M4, and `### Turns per assertion by assertion kind`).
- B-new: the script's M4 row does not reproduce the owner's hand figures. Script (same window): worker 27.85 USD-eq = 0.87 per assertion (32 fulfilled), coordination 4.64, validation 1.80, tooling 0.86, 7.22 against 7.93 all-in; owner: worker 35 USD-eq = 1.06 per assertion (33), coordination 2.86, validation 1.30, tooling 0.85. The owner's derivation is not in the repository, so the bucket boundaries differ in a way the script cannot see (coordination 2.86 x 35 = 100.1 USD-eq against the orchestrator's 111.86 and 129.27 with its subagents). Reason: the recorded rows are the owner's and stay as written; ask the owner for the derivation (or the bucket boundaries) before M5's ratio is read as a verdict against M4's.
- B-new: the kinds differ by one assertion: the contract's `Kind:` lines give 12 mechanical, 14 behavioural, 6 mixed over the 32 passed M4 assertions; the owner's 12/15/6 counts 33 (one behavioural assertion is `blocked`).
- B-new: `reports/usage/m4.md` is 194308 bytes after this feature; `make check-file-size` fails at 204800. The next regeneration with a longer feature list or more sessions can cross it; reason: the Tokens per session and Sidecar tables grow with the mission. Move the two per-session tables out of the committed report (or compress their Source column) before M5's report.
- B-new: `make check-scripts` is red at the wt/m5 tip before this feature: `check-contract-checks` fails `g2-metric-tests` and `g2-telemetry-tests` because the named tests exist but `scripts/contract-checks/g2-tests.tsv` still has `landed=no` for them (VAL-METRIC-003, -005, -006, -007, -022 to -024; VAL-TELEM-001 to -003, -005, -006: the features that landed them did not flip the column in the same commit), and `g6-shared-files` fails on `4b40a09` (the D-047 commit edited `docs/decisions/README.md`, which the integrator regenerates). `g3-metrics-contract` fails by design until `m5-metrics-report-and-oracle` lands the command. This feature's own `m4.md` commit is excepted (`8bd6a7d`). Reason: each belongs to the feature that landed the test; the integrator flips the `.tsv` rows or the owners of those features do.
- B-new: `usage_test.py::test_a_price_table_name_picks_the_dated_file_it_names` asserted that `2026-10-29` appears nowhere in the report; the fixture sidecars' mtime date is printed in the Sidecar section, so the test failed on any checkout written on 2026-10-29. Fixed here (`2026-10-29.toml`); other tests that print mtimes could have the same shape.

## Deferred clauses

- None. Every item of the feature description is done; the batch's last feature (`m5-telemetry-exporter-and-status`) has no claim from this one.

## Claims (rule 30)

`fulfills` is empty (infra, owner items 10 and 27). Evidence for each `expectedBehavior` line:

| Clause | Evidence |
|---|---|
| Ratios section per milestone with the product formula | `RatiosSection.test_the_cost_per_assertion_is_the_worker_cost_per_assertion_times_one_plus_the_three_ratios`, `..._the_summary_line_shows_the_product` in `scripts/tests/usage_ratios_test.py`; `reports/usage/m4.md` `### Ratios` |
| Raw next to weighted by plan turn budget, and by kind | `..._the_raw_cost_per_assertion_sits_next_to_the_one_weighted_by_the_plan_turn_budget`, `..._per_kind_figures_split_a_feature_over_the_assertions_it_claims` |
| `single-call share` under the boundary line (B-176) | `SingleCallShare` (4 cases); `reports/usage/m4.md` line 620 |
| Per-kind turns per assertion (B-177) | `..._per_skill_turns_per_assertion_are_given_for_core_ui_infra_and_integration`; contract row 25.72 = D-045's 25.7 |
| Turns per landed assertion by role (item 27) | `..._turns_per_landed_assertion_are_given_by_role` |
| M4 figures are the first data row | `..._the_owners_m4_figures_are_the_first_data_row_and_m3_comes_before_them`, `RatioFormula` (1.06 x (1 + 2.86 + 1.30 + 0.85) = 6.37, 6.4 to one decimal); the script's own M4 row differs, see the B-row above |
| Self-test covers the section | `make check-scripts` runs `usage_ratios_test.py` (18 cases) |
| `m4.md` regenerated, cross-check identical | `reports/usage/m4.md` line 42: `cross-check: identical (197 sessions, ...)`, the same line as before (line 41 of 849dae4) |

Red/green: `1db323e` (red, `usage_ratios_test.py`, exit 1, FAILED failures=5 errors=13) to `9fcd793` (green). The red test's recorded-row cells read 0.9 and 1.4; the green commit prints two decimals.

## Library

- `make usage M=m4 CAPS=...` takes about 29 s on the m4 window; to regenerate a boundary report byte-for-byte pass the original `--from`, `--to`, `--now` (they are in the report's first two lines) and the caps that its Mission to date table shows.
- A live log rotates: after a rotation the Source column of a regenerated report names the rotated file (`droid-log-single.log.2026-10-29.2` for the lines the m4 report read from `droid-log-single.log`); the numbers do not move, and `--cross-check` is identical.
- Options added: `--validation-contract` (default `<mission-dir>/validation-contract.md`, the `Kind:` lines) and `--ratios-config` (default `config/usage-ratios.toml`).
