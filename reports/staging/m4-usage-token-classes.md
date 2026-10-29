# Staging: m4-usage-token-classes

Feature `m4-usage-token-classes` (owner items 5, 6 first half and 16 pricing of the M3 boundary), worktree `observatory-wt/m4`,
branch `wt/m4`.

## SPEC §9

- Row (F-NEW), tooling change (rule 26, verified on 2026-10-29 by the scripts' own tests and a regeneration of `reports/usage/m3.md`):
  `scripts/usage-report.py` and `scripts/usage-from-log.sh` carry owner items 5, 6 (first half) and 16 (pricing). (1) `Cost by token
  class`: per role and for the milestone, tokens and USD-eq by class (input, cache write, cache read, output) with the share of tokens and
  of cost. (2) `Cost per assertion and per feature`: the window total over the assertions with status `passed` and `validatedAtMilestone`
  equal to the milestone (`validation-state.json` has no timestamps, so the milestone is its window; `--validation-state <path>` or the
  mission directory's file) beside the same total over the milestone's `completed` features, and the ids of the assertions. (3) `Feature
  size` lists every feature of the milestone in `features.json` (also those that never ran, 0 turns) with `Turn budget`, `Over budget`
  and USD-eq (subagents included). (4) `Subagent classes`: a row per class (orchestrator, validator, worker subagents) with sessions,
  turns, USD-eq, the declared model and purpose read from `handoffs/*.json` and `validation/<milestone>/**/synthesis*.json`, `No declared
  model` and `Off declared model` counts, and the bullet `no declared model: N of M subagent sessions`. (5) `Models that ran`: every model
  with a turn in the window priced at the plan price (a price row with the four `plan_*_per_m` keys) or the list price of
  `config/prices/<date>.toml`; a model that ran in the window with no price row, or a zero input or output rate, stops the run with exit 2
  naming the model; `scripts/usage-from-log.sh --prices <toml>` does the same check and its rows gain `cache_write`, which the report's
  cross-check compares. Real M3 window (`make usage M=m3 CAPS='milestone=110 orchestrator=30' USAGE_ARGS='--from 2026-10-24T21:48:56Z
  --to 2026-10-25T18:19:14Z'`, 23.18 s, cross-check identical on 80 sessions): window total 210.98 USD-eq and mission to date 990.09
  unchanged; cost by class for the milestone cache write 55.1 % of cost on 6.3 % of tokens; 38 assertions, 5.55 USD-eq each, 22
  completed features, 9.59 each; subagent classes 5 orchestrator subagent sessions (27.47) and 33 validator subagent sessions (58.59), all
  38 with no declared model. Evidence: `scripts/tests/usage_test.py` and `scripts/tests/usage-from-log.sh`, red then green per addition
  (commits in the handoff). **Alternatives considered:** fail on any unpriced model in the whole log (rejected: 373 turns of
  `kimi-k3`, `gpt-6-sol`, `gpt-6.1-sol` and `claude-opus-5-5-fast` before the window have no price and the published M3 numbers must stay
  reproducible; they stay in `Unpriced turns` of Mission to date and a line names them); price from the sidecar credits (rejected: credits
  are a different unit, D-029).

## Decisions

None.

## BACKLOG

- Closed: owner item 5 (the usage report has a row per subagent class) and the first half of item 6 (cost by token class per milestone
  and per role, cost per assertion beside cost per feature) for the report; the second half (owner interventions, idle time by cause)
  stays with `m4-usage-idle-host-and-rewrites`.
- B-NEW: prices for `kimi-k3`, `gpt-6-sol`, `gpt-6.1-sol` and `claude-opus-5-5-fast` in `config/prices/<date>.toml` (the plan's price) | The
  usage report now stops when one of them runs in a window; before the window they sit in `Unpriced turns` of Mission to date (373 turns
  at the M3 window end, 261 of them `kimi-k3`). The owner supplies the plan prices; a new dated price file is a rule 3 change. | unscheduled
- B-NEW: no subagent of M3 declared its model, so `Subagent classes` reads 38 of 38 undeclared for M3 | The declaration line is
  `subagent <session id or 8-hex prefix>: model = <id>; purpose = <text>` in a handoff or synthesis string, or a `subagents` list or
  `subagentSessions` object in a synthesis file; the orchestrator's and the validators' briefs name it from M4 (rule 29 to 31 text, item 5). | M4

## Deferred clauses

- `validation-state.json` records no time of the move to `passed`, so "moved to passed in the window" is read as `validatedAtMilestone`
  equal to the milestone; a re-test round that reopens an assertion inside the same milestone is not separated. If per-round cost is
  wanted, the state file needs a `passedAt` field first.
- The gate on the final tree (item 17b) is not recorded here: this feature is not the last code change before the integration.

## Library

- `tooling.md`, usage report: `Cost by token class`, `Cost per assertion and per feature`, `Models that ran` and `Subagent classes` are
  sections of `reports/usage/<milestone>.md`; new code lives in `scripts/usage_classes.py` (imported by `usage-report.py`, standard library
  only); `--validation-state <path>` is the one new flag; exit 2 names a model with no price (or a zero rate); a price row's `plan_*_per_m`
  keys (all four) are the plan price and win over the list price; `usage-from-log.sh` prints `cache_write` as the eighth column and takes
  `--prices <toml>`.
- The usage fixture `scripts/tests/fixtures/usage/prices.toml` prices `kimi-k3` (list and plan) so the fixture window runs; its
  `mission/` holds `validation-state.json`, `handoffs/` and `validation/m9/scrutiny/synthesis.json`.
