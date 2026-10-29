# m4-fix-g7-demo-red-pr-window

## Claims (rule 30)

```json
[
  {
    "assertion": "VAL-GATE-020",
    "evidence": "Check on the walk's PR: `GH_TOKEN=\"$(gh auth token)\" sh scripts/contract-checks/g7-demo-red-pr.sh` prints `g7-demo-red-pr ok VAL-GATE-020` (exit 0) with /tmp/m4-walk-pr.txt = 33; `gh pr list --base demo/base --head demo/red-1 --state all --json number,createdAt` lists #33 (2026-10-29T19:23:30Z, the walk's PR, in the window), #32 (16:01:25Z), #31 (15:57:12Z), #30 (15:55:32Z), the older three outside it. `make check-contract` with the token: `g7-demo-red-pr ok VAL-GATE-020`, `contract checks: 29 ok, 2 skipped, 0 FAIL in 5.1s` (/tmp/cc.txt). Self-test: scripts/tests/contract_checks_test.sh, section 'G7' after the g7-mission-pr cases: one in-window PR with two older ones is ok, two in-window PRs FAIL, only older PRs skip, a PR created at the window start counts, window start from CONTRACT_RUN_START, from the walk store's pr.opened row (store started before #32 FAILs) and from the walk's recorded PR; red on e6f5b40 and 3e1f156 (`sh scripts/tests/contract_checks_test.sh`, exit 1, 7 then 8 FAIL lines, /tmp/red.txt and /tmp/red2.txt), green on 3bd9a42 (`contract_checks_test: ok`, exit 0, /tmp/green.txt). The store-side half (`SELECT COUNT(*) FROM prs WHERE head='demo/red-1'` is 1) stays a gate-record stage (G8) and is not claimed here."
  }
]
```

## SPEC §9

| kind | row |
|---|---|
| fact | `g7-demo-red-pr` counts PRs for `demo/red-1` created at or after the run window's start (the head is reused by every walk; amended VAL-GATE-020). The start is `CONTRACT_RUN_START`, else the earliest `pr.opened` row of the store named by `CONTRACT_WALK_STORE`, else the creation time of the PR in the walk's PR file (`CONTRACT_WALK_PR_FILE`, default `/tmp/m4-walk-pr.txt`, written by `e2e/lib/walk.ts`). No window source or no PR in the window is a skip; two or more in the window is a FAIL. `PR_LIMIT=200` lifts gh's default of 30 PRs. |

## Decisions

None.

## BACKLOG

| reason | row |
|---|---|
| The walk keeps no durable run record: its store lives in a per-run scratch directory (`/tmp/m4-walk1`, `/tmp/ut-m4/walk-run`) that later runs overwrite or delete, so the window start falls back to the PR number file in `/tmp`. A walk that wrote `reports/e2e/<run>/record.json` with `started_at` (as `make e2e` does) would give the check a committed start. | candidate for M5 or the next e2e feature |

## Deferred clauses

- VAL-GATE-020 store half (`prs` count 1 for the walk's store) is the G8 gate-record stage, unchanged.
- On a CI runner or any machine without `/tmp/m4-walk-pr.txt`, a store or `CONTRACT_RUN_START`, the check skips (`no run window`); `make check-contract` there reports it as skipped, not ok.

## Library

None.
