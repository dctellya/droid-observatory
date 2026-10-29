# Staging: m4-usage-plan-prices

Feature `m4-usage-plan-prices` (owner answer `2026-10-29-1127-plan-prices.md`, the B-NEW price row of
`reports/staging/m4-usage-token-classes.md`), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW), tooling change (rule 26, verified on 2026-10-29 by the scripts' own tests and a regeneration of `reports/usage/m3.md`): the
  plan prices of the owner's answer (pre-roci, under the owner's delegation of 2026-10-29) are in `config/prices/2026-10-29.toml`, a new
  dated price file that keeps the four rows of `config/prices/2026-10-22.toml` byte for byte (that file is untouched) and adds `kimi-k3`
  3.00 / 15.00 / 0.30 / 3.00, `gpt-6.1-sol` 2.00 / 10.00 / 0.10 / 2.00, `gpt-6-sol` 2.00 / 10.00 / 0.20 / 2.50 and `claude-opus-5-5-fast`
  4.0 / 20.0 / 0.20 / 5.0 (input / output / cache read / cache write per million, USD-eq) as `plan_*_per_m` keys; the `claude-opus-5-5-fast`
  row carries `note = "assumed = opus-5-5 until the owner confirms"` (the owner gave no price; it is priced as `claude-opus-5-5`). The
  file header names the inbox file and carries the provenance line verbatim: the observer's price table of 25-oct-2026
  (resume/roci/tools/milestones.py), secondary public sources, used for every milestone reading S to M3; the owner has seen those
  readings; not estimates, list prices at the date named. `scripts/usage-report.py` reads the newest dated file of `config/prices/`
  (`--prices-dir`), or the one `--price-table <date, name or path>` names (`--prices <file>` still wins); `Models that ran` gains a `Note`
  column and prints `Price provenance:`; `Mission to date by role` prints `Price provenance:` and `Price notes:` (the noted model, its turns
  to date and USD-eq, and the note). `Feature size` reads the budget from the feature's `turnBudget` key or, when the key is absent (the
  mission runner rewrites `features.json` and drops it), from the text `turnBudget <N>` of the description. Regenerated M3 window
  (`make usage M=m3 CAPS='milestone=110 orchestrator=30' USAGE_ARGS='--from 2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z'`, 22.80 s,
  cross-check identical on 80 sessions): `Unpriced turns` 0 (was 373), Mission to date total 990.09 -> 1007.00 USD-eq (+16.91, the 373
  turns priced; 3 `claude-opus-5-5-fast` turns are 1.06 of it), window total 210.98 unchanged. Evidence: `scripts/tests/usage_test.py`
  (`UsageReportPriceFiles`, `RepoPriceTables`, `UsageReportFeatureBudgets`) and `scripts/tests/usage-from-log.sh`, red then green.
  **Alternatives considered:** write the plan prices into `config/prices/2026-10-22.toml` (rejected: that file is the verified 22-oct table
  the M5 oracle and `config/observatory.example.toml` `price_table` name; a new rule 3 change is a new dated file); change `price_table` in
  `config/observatory.example.toml` to the new file (rejected: the app's cost model keeps the 22-oct table until a decision says otherwise,
  the usage tooling reads the newest file by itself); price `claude-opus-5-5-fast` as unpriced (rejected: the owner told to price it as
  `claude-opus-5-5` and mark it).

## Decisions

None.

## BACKLOG

- Closed: B-NEW prices for `kimi-k3`, `gpt-6-sol`, `gpt-6.1-sol` and `claude-opus-5-5-fast` (row of `m4-usage-token-classes`): priced at the
  owner's plan prices in `config/prices/2026-10-29.toml`; `reports/usage/m3.md` regenerated with `Unpriced turns` 0 and Mission to date
  990.09 -> 1007.00 USD-eq (+16.91); window total 210.98 unchanged.
- B-NEW: the owner confirms the `claude-opus-5-5-fast` price (priced as `claude-opus-5-5`, 4 / 20 / 0.20 / 5, marked `assumed = opus-5-5`) |
  the observer's table has no list price for it; 3 turns to date (1.06 USD-eq at the M3 window end), so a different price moves the Mission
  to date total by cents; when confirmed, edit the row of `config/prices/2026-10-29.toml` (or a new dated file) and drop the `note` key. | M4

## Deferred clauses

- The gate on the final tree (item 17b) is not recorded here: this feature is not the last code change before the integration.
- Rule 30 claims: this feature fulfils no validation-contract assertion (`fulfills` is empty), so there is nothing to claim.

## Library

- `tooling.md`, usage report: a price file may carry the top-level keys `provenance` (printed as `Price provenance:` under `Models that
  ran` and `Mission to date by role`) and `inbox`, and a model row may carry `note` (the `Note` column of `Models that ran`, and the
  `Price notes:` line of `Mission to date by role`); a row with only the four `plan_*_per_m` keys is valid. The default table is the
  newest file of `config/prices/` by name (`--prices-dir` changes the directory); `--price-table 2026-10-22` (a date, a file name or a
  path) reads another, `--prices <file>` an explicit file. `config/prices/2026-10-22.toml` and `price_table` of the app config stay as
  they were.
- `tooling.md`, `Feature size`: `turnBudget` is the feature's key, else the `N` of `turnBudget <N>` in its description (the mission
  runner drops the key when it rewrites `features.json`), else `-`.
