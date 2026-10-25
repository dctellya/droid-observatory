# Staging: m3-fix-ut-gate-timing-e2e-trigger-usage

Feature `m3-fix-ut-gate-timing-e2e-trigger-usage` (fix 3 of 3 after M3 user-testing round 1), worktree
`observatory-wt/m3`, branch `wt/m3`. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): e2e CI trigger (rule 24, D-026): `.github/workflows/e2e.yml` runs on a pushed tag `e2e-*` plus
  `workflow_dispatch`, with `concurrency: {group: e2e-${{ github.ref }}, cancel-in-progress: false}`. The integrator
  pushes one tag per milestone at the integration commit (`git tag e2e-m3 <sha> && git push origin e2e-m3`; tags are
  never moved or deleted) and pastes the run id, conclusion, wall time and artifact name. The tag trigger reads the
  workflow file from the tagged commit, so it works before the file is on `main`. Alternatives: `workflow_dispatch`
  only (rejected: GitHub resolves it on the default branch, `gh run list --workflow e2e.yml` answered HTTP 404 and the
  read-only PAT cannot dispatch); `pull_request` to `main` (rejected: one run at the final pull request, not one per
  milestone); `push` on `mission/**` with a `paths` filter (rejected: fires on every code push, a hotfix after the
  merge commit would overlap or, with cancel-in-progress true, cancel the run at the integration sha). Changes neither
  scope, caps nor models. Cost: one macOS run per milestone (25 to 35 minutes cold, free on a public repository).
- Row (F-NEW): VAL-TOOL-001: `scripts/gate.sh` starts `started_at` and the `seconds` clock at entry (before the tree
  computation and the tool-version calls), so `seconds` covers the whole script run. The record's `exit_code` stays the
  command's own code (make reports it as `Error N` and exits 2; contract C-3).
- Row (F-NEW): VAL-TOOL-005 table gaps closed in `scripts/usage-report.py`: Feature size gains `Skill` (from the
  mission's `features.json`, listed in the Sources table), `First ts`, `Last ts` and `Source`; Subagents, Self-check
  missions and Mission to date gain a per-row `Source` column naming the log files; `--cap milestone=<USD>` and
  `--cap total=<USD>` compare against two new rows of the Mission to date table (`milestone window total`, the total
  row of Tokens per role; `total`, every role up to the window end) with `Over cap`.
  Proof: `make usage M=m3 CAPS='milestone=110 orchestrator=30'` run twice with `--to 2026-10-25T17:04:26Z`
  produced identical files (diff empty), 18.273 s and 18.077 s, cross-check identical (64 sessions).

## Decisions

- D-037 (amend): the CI trigger paragraph is implemented as written (tag `e2e-<milestone>` pushed by the integrator);
  `workflow_dispatch` stays for use after the final pull request lands. Concurrency is keyed on the ref with
  `cancel-in-progress: false`.

## BACKLOG

- Row: the `milestone` cap is compared with the windowed total of Tokens per role, which includes validators and
  every subagent; the M3 window total is above the 110 USD-eq cap in the generated table. reason: the cap's intended
  scope (workers only, or all roles) is an owner decision for the boundary report, not a script choice.

## Deferred clauses

- VAL-E2E-006 run itself (green macOS run with `failures="0"` JUnit) is produced at the integration commit by the
  tag push; this feature delivers the trigger.
- VAL-TOOL-005 graded at `m4-integrate-m3` (`time make usage M=m3 CAPS='milestone=110 orchestrator=30'`, twice with the
  same `--to`).

## Library

- Gate timing: `scripts/gate.sh` takes `t0` and `started_at` right after the `jq` check, before the 0.56 s tree
  computation (`read-tree`, `rm --cached`, `add -A`, `write-tree`) and the `rustc -V` / `cargo -V` calls, so the
  record's `seconds` is within tens of milliseconds of the wall time around the script. Before the fix it was 0.6 to
  0.75 s short. The red case in `scripts/tests/gate_test.sh` puts a `git` shim on `PATH` that sleeps 1 s on
  `write-tree` and asserts `seconds` >= 1.9 for a 1 s command (1.040 before, green after).
- `usage-report.py` reads `<mission dir>/features.json` for the feature to skill map; a mission directory without the
  file gives `-` in the Skill column and no Sources row.
