# Staging: m4-usage-boundary-turns

Feature `m4-usage-boundary-turns`, worktree `observatory-wt/m4`, branch `wt/m4`. It acts on the owner's inbox file
`docs/owner-feedback/inbox/2026-10-29-1408-orchestrator-boundary-cost.md`, rule A.3 ("the usage report's Lead context rewrites section
adds a line `orchestrator turns per feature boundary`, target 3").

## SPEC §9

- Row (F-NEW, rule A.3 of `docs/owner-feedback/inbox/2026-10-29-1408-orchestrator-boundary-cost.md`), tooling change (rule 26, verified on
  2026-10-29 by `scripts/tests/usage_rewrites_test.py` and a regeneration of the M3 window into `/tmp`): the `Lead context rewrites per
  milestone` part of `scripts/usage-report.py` (in `scripts/usage_rewrites.py`) prints
  `orchestrator turns per feature boundary: <turns> / <boundaries> = <ratio> (target 3)`. `<turns>` is the section's own lead-turn count
  (the `Lead turns` column: orchestrator log turns in the window). `<boundaries>` is the count of `worker_completed` entries of the
  mission's `progress_log.jsonl` in the window, the runner's feature returns (validator sessions included, they return a hand-off too;
  `worker_failed` is not one; `handoffs/*.json` is not read because the report already reads `progress_log.jsonl`; on 2026-10-29 the
  directory holds 118 files and the log 117 `worker_completed` entries). With no boundary the ratio reads `n/a`. When the window holds
  a compaction marker, a second line `before compaction: <turns> turns / <boundaries> boundaries = <ratio>, <USD-eq> USD-eq per turn;
  after: ... (marker: <kind> <time>)` splits at the earliest marker: a `mission_resumed` entry of the progress_log (rule B has the
  orchestrator record its compaction as one) or a lead context rewrite turn of the table above it; a `mission_resumed` wins a tie. With no
  marker the one line stands alone. Command (writes nothing under `reports/usage`; the M3 window the report was generated with):
  `python3 scripts/usage-report.py --milestone m3 --cross-check --cap orchestrator=30 --from 2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z --now 2026-10-29T10:34:11Z --out /tmp/m3-new.md`
  (what `make usage M=m3 CAPS='orchestrator=30' USAGE_ARGS='--from ... --to ...'` runs, but `--out` keeps `reports/usage/m3.md` as it is;
  wall 24.3 s). **Alternatives considered:** count `handoffs/*.json` (rejected: the file count and the log differ by one, and the report reads the progress_log everywhere else); count only features of `features.json` (rejected:
  a validator hand-off costs the orchestrator turns like any other);
  split at every marker (rejected: the M3 window holds 8 rewrites and the log 17 resumes in all, so it would give fragments; the owner asked for before and after one
  compaction); split at the last marker (rejected: a late cache expiry would leave an after-side of one turn).

## Decisions

None.

## BACKLOG

None.

## Deferred clauses

None.

## Library

- Claims (rule 30): the feature has an empty `fulfills`; none to claim.
- Inner loop (rule 23), no cargo involved: `python3 scripts/tests/usage_rewrites_test.py` 21 tests OK, 1.7 s; the other four usage python
  tests (`usage_test` 82, `usage_idle_test` 20, `usage_testing_test` 7, `usage_e2e_test` 11) OK, 8.4 s together. Red run
  `/tmp/red-boundary.txt`: exit 1, `FAILED (failures=8)` of 9 (the ninth, no second line without a marker, passes before the code).
- Gate (rule 28, item 17b; last implementation worker before m5-integrate-m4): one full `make check` on the final tree,
  `make check > /tmp/make-check-m4-usage.txt 2>&1`, exit 0, real 5m47.367s; record `reports/gate/d0c73e0b40618412ac59d97a5b1d7e37bd997792/check.json`
  (code tree `d0c73e0b...`, `exit_code` 0, 341.718 s), with `lint.json` (8.191 s) and `test.json` (209.225 s) beside it. The first run on
  the tree before d344112 failed `lint` (exit 2, 2m42.391s): `apps/observatory-app/src-tauri/src/host.rs:617` `fn store` had a cognitive
  complexity of 27 against the limit of 25, introduced by 92b18dc (m4-wire-dedupe-paused, whose inner loop did not run the
  `-W clippy::cognitive_complexity` clippy of `make lint`). d344112 extracts the echoed-pause retraction into an associated function
  (behaviour and log lines unchanged, the same split as 995fa86); `cargo clippy -p observatory-app --all-targets -- -D warnings -W
  clippy::cognitive_complexity` exit 0 and `cargo test -p observatory-app` exit 0 (19.067 s) before the rerun.

**M3 reading**

The two new lines in the M3 window (`reports/usage/m3.md` is not regenerated here; the integrator's boundary run writes them):

```text
orchestrator turns per feature boundary: 194 / 24 = 8.1 (target 3)

before compaction: 7 turns / 0 boundaries = n/a, 0.23 USD-eq per turn; after: 187 turns / 24 boundaries = 7.8, 0.36 USD-eq per turn (marker: lead context rewrite 2026-10-25T07:24:46.425Z)
```

M3 already ran at 8.1 turns per boundary (the owner's reading at 14:00 on M4 is about 8: 125 turns for 16 boundaries), so the rule is
the change, not a regression. The cost per turn is 68.81 / 194 = 0.35 USD-eq over the window. The split at 07:24:46 is a cache expiry
(8 rewrites in the window), not a compaction the orchestrator chose; the first 7 turns were before any boundary. The M4 reading will
show whether rule A.3's target of 3 held once the orchestrator records its compaction as a `mission_resumed` line.
