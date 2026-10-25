# Staging: m3-fix-agents-md-rules-text

Feature `m3-fix-agents-md-rules-text` (fix after scrutiny-validator-m3 round 1, `blockingIssues`), worktree
`observatory-wt/m3`, branch `wt/m3`. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): VAL-PROC-052 correction. Repo AGENTS.md and `reports/README.md` labelled rule 27 as the staging-file rule;
  rule 27 (`ai/RULES.md`) is the gate cache by tree hash, so AGENTS.md had no gate-cache rule. The (27) bullet now names
  the gate cache by tree hash (`reports/gate/<tree>/<command>.json`, a hit is evidence, `--force` reruns), (28) stays the
  gate flow, and the staging-file line stays under F-066; `reports/README.md` says "F-066; the gate cache is rule 27".
  The same edit replaces the two wait sentences (the Waits line and the rule 25 paragraph) that allowed one blocking
  call and re-issued early-returned waits with the F-056 wording: the orchestrator starts the run in a background Script,
  ends its turn, acts when the output is delivered, and makes no `WaitForScript` call. `.gitignore` gains
  `vscode_target/` (user data in the main checkout, never committed or touched), `__pycache__/` and `*.pyc` (a
  reviewer's `scripts/__pycache__` moved the gate key during scrutiny). Test `scripts/tests/agents_md_test.sh`, part of
  `make check-scripts`.

## Decisions

none

## BACKLOG

none

## Deferred clauses

none

## Library

none
