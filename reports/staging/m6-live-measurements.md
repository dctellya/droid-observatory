# Staging: m6-live-measurements

## SPEC §9

| F-next | 2026-10-30 | Row `m6-live-measurements` (kind: measurement, pass 41 narrowing and pass 43 ruling C): VAL-CROSS-002 live pair 113.9 ms (first line) and 61.7 ms (appended line) against `events_appended` at the m6 head, `attach_latency` p50=22.911667ms p95=33.503959ms max=47.783208ms; VAL-CROSS-007 CLI `unknown events: 0` on M0 and S, tray 0 on both; VAL-GATE-027/033 demo PR 41 (run 41122154885, squash `9a5521f` by `dctellya` through `gh pr merge`, no Merge press): one `handed_to_cd` lane `gate` role `human`, `human_actions` merge rows 0, badge `handed to CD`; `external` branch proved by `ingest::github::tests` (`merger_role` extracted test-first, 2e604ad red, 50125c8 green). Report `reports/readiness/2026-10-30-m6-measurements.md`. | validation-contract passes 41 and 43 | applied |

## Decisions

None.

## BACKLOG

- New: the board-visible time of a live pair is the `events_appended` emit, not a DOM read, because the Tauri window has no CDP endpoint. Reason: only the e2e harness (WebdriverIO) reads the window; a `live-pair` spec on a real `droid exec` session would close it.
- New: with `branch_pattern` set to a literal demo branch the git source logs `git_poll_failed ... the branch demo/green-1 does not exist` every 30 s (the branch exists on the remote only). Reason: the git source reads the local repository; harmless for the gate lane, noisy in the log.
- New: the `handed to CD` badge appeared 35.8 s after `mergedAt` (one 30 s poll interval plus the request), above the 30 s of VAL-GATE-026's wording. Reason: `[github].poll_interval_s = 30`; the owner decides whether the contract or the interval moves.

## Deferred clauses

None. Self-check missions line (cap 2): 1 used, a `droid exec` scratch session `ff845ce5-e0bf-416b-bf2f-df0df6f04893`, model `claude-haiku-4-5-20251001`, tag `scratch`, inputTokens 75, outputTokens 704, cacheCreationTokens 10822, cacheReadTokens 51729, factoryCredits 8921.

## Library

- A live pair without the owner's other sessions: set `[ingest].factory_sessions_dir` to the scratch cwd's own directory under `~/.factory/sessions/<mangled cwd>` (create it first), so the watcher attaches only the scratch session; with the default directory every session of the last 24 h (this mission's) attaches and each one polls the demo PR.
- Demo pair for the gate without a worktree: `gh api repos/dctellya/droid-observatory/git/refs` (branch from `demo/base`), `gh api -X PUT .../contents/<file>` (one commit), `gh pr create --base demo/base --head demo/<name>`; the `demo-gate` run was queued 2 min before it started.

### Claims (rule 30)

model = claude-sonnet-5-5 (read from `~/.factory/sessions/-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/3d76a652-6515-49ff-b36a-170107544d05.settings.json`; `model-settings.json` `workerModel` is `claude-sonnet-5-5`, match)

- VAL-CROSS-002: command `cargo test -p observatory-core --test attach_latency -- --nocapture`; output `attach latency over 200 lines at 20 ms gap: p50=22.911667ms p95=33.503959ms max=47.783208ms`, `test result: ok. 1 passed`; live pair: line `2026-10-30T20:20:35.401Z` (`jq -r '[.type,.timestamp]|@tsv' <session>.jsonl`) against `2026-10-30T20:20:35.515014Z ... events_appended ... count=1` in `/tmp/observatory-tauri.log` = 113.9 ms; second pair 20:20:54.788Z to 20:20:54.849672Z = 61.7 ms. File `reports/readiness/2026-10-30-m6-measurements.md` section VAL-CROSS-002.
- VAL-CROSS-007: commands `cargo run -q -p observatory-cli -- replay fixtures/streams/2026-10-22-observatory-v1-m0.jsonl` and `...-s.jsonl`, last lines `unknown events: 0` and `unknown events: 0`; tray via `agent-browser --session m6-cross-007` on 3141 after `upload "input[type=file]" ...`: `eval` of `[aria-label=tray-count]` reads `0` for M0 and `0` for S (event-count `661`).
- VAL-GATE-027: command `gh pr merge 41 --repo dctellya/droid-observatory --squash --delete-branch` (exit 0, no app Merge press); `sqlite3 -readonly app.sqlite "select id,kind,lane,role,sha ... where kind='handed_to_cd'"` prints `github:pr:41:handed_to_cd|handed_to_cd|gate|human|9a5521f8360c9d763f6a23ea203179a7e2a87572|...`; `select count(*) from human_actions where kind='merge' and target like '%41%'` prints `0`; badge `gate_changed ... badge="human_merged"` (renders `handed to CD`, `view/gate.rs:56`); external branch: `cargo test -p observatory-core --lib --all-features ingest::github::tests` prints three `ok` (`a_merger_other_than_the_token_owner_is_attributed_as_external` among them) and `test result: ok. 3 passed`. Role `human` is the forge's derivation for the token owner (pass 43 resolution), not `external`.
- VAL-GATE-033: PR number 41 from `gh pr create` and `gh pr view 41 --json number,...`; demo-gate run id 41122154885 from `gh run list --repo dctellya/droid-observatory --branch demo/green-1 --json databaseId,...`; merge sha `9a5521f8360c9d763f6a23ea203179a7e2a87572` from `gh pr view 41 --json mergeCommit`.
- Red then green: `cargo test -p observatory-core --lib --all-features merger_role > /tmp/red.txt` exit 101 `error[E0432]: unresolved import super::merger_role` (2e604ad), green at 50125c8.
