# M6 live measurements, 2026-10-30

Worker `m6-live-measurements`, model `claude-sonnet-5-5`. Measured on the main checkout, branch `mission/observatory-v1`,
head `50125c855ff9606e0cfd5459d9c4515c6c1cfb8c` (`git log -1 --format=%H`; machine load `4,06 3,86 3,71` from `uptime` at 21:15 local).
Wording under test: VAL-CROSS-002 and VAL-CROSS-007 under pass 41 narrowing and pass 43 wording; VAL-GATE-027 and VAL-GATE-033
under pass 43 ruling (C) and its resolution. Every number below is pasted from the command above it.

## VAL-CROSS-002: one live pair and the attach_latency line

### attach_latency (m6 head)

```text
$ cargo test -p observatory-core --test attach_latency -- --nocapture
attach latency over 200 lines at 20 ms gap: p50=22.911667ms p95=33.503959ms max=47.783208ms
test attach_latency_under_one_second_on_fixture_replayed_live ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.99s
real    0m11.552s
```

### One live pair (Tauri app attached to a scratch session)

Setup: `make app-dev` (`cargo tauri dev`) with `OBSERVATORY_CONFIG=observatory-scratch/m6-live/config.toml`
(`[ingest].factory_sessions_dir` = the scratch session directory only, own store `observatory-scratch/m6-live/app.sqlite`),
`GITHUB_TOKEN="$(gh auth token)"`, no keychain. Smoke at the time of the pair (log `/tmp/observatory-tauri.log`):
`Running /Users/dct/.cargo/shared_rust_target/debug/observatory-app` (after `Finished dev profile ... in 50.86s`),
process `31328 144512 .../debug/observatory-app` (pid, rss KB), `lsof -nP -iTCP:3141` shows `com.apple ... TCP [::1]:56307->[::1]:3141 (ESTABLISHED)`
(the WKWebView), `osascript ... process "observatory-app"` prints `observatory-app, 31328, true, true` (name, pid, frontmost, visible).

Scratch session (one `droid exec`, not a hosted mission; R-050): command
`droid exec --cwd <observatory-scratch/m6-live/work> -m claude-haiku-4-5-20251001 -r low --auto medium --tag scratch "Run the shell command 'date' three times ... then reply with the single word done."`,
exit 0, output `done`. Session `ff845ce5-e0bf-416b-bf2f-df0df6f04893`; its `.settings.json`: `"model": "claude-haiku-4-5-20251001"`, tags `exec`, `scratch`;
tokens `inputTokens 75, outputTokens 704, cacheCreationTokens 10822, cacheReadTokens 51729, thinkingTokens 223, factoryCredits 8921`.

Line timestamp (from the watched file) against the time the app emitted the batch to the window (`events_appended` in the app log):

```text
$ jq -r '[.type, .timestamp] | @tsv' <session>.jsonl | sed -n 2p
message    2026-10-30T20:20:35.401Z
$ sqlite3 -readonly app.sqlite "select id,kind,at from events where at like '2026-10-30T20:20:35%'"
ff845ce5-e0bf-416b-bf2f-df0df6f04893:0:0|intent.created|2026-10-30T20:20:35.401Z
$ grep events_appended /tmp/observatory-tauri.log
2026-10-30T20:20:35.515014Z  INFO observatory_app_lib::ingest: events_appended mission_id=ff845ce5-... count=1 unknown_total=0
```

Pair 1 (first line, board event `intent.created`): 20:20:35.401Z to 20:20:35.515014Z = **113.9 ms** (below 1000 ms).

```text
$ jq -r '[.type, .timestamp] | @tsv' <session>.jsonl | tail -n2     (last timestamped line; the file's final line, agent_turn_outcome, has no timestamp)
message    2026-10-30T20:20:54.788Z
$ grep events_appended /tmp/observatory-tauri.log
2026-10-30T20:20:54.849672Z  INFO observatory_app_lib::ingest: events_appended mission_id=ff845ce5-... count=5 unknown_total=0
```

Pair 2 (appended line to the batch that carried it, `turn.usage` at 20:20:54.788Z in the store): **61.7 ms**.

Limits, stated: the Tauri window has no CDP endpoint, so "board-visible" is the `events_appended` emit to the window
(the UI renders from that event within `[ui].event_batch_ms` = 100, from `config/observatory.example.toml`); no DOM read of the window was possible
(the window-only reading is the e2e harness's, not run here). The session file's final line carries no `timestamp`, so the contract's
`tail -n1 ... | jq -r .timestamp` was read on the last timestamped line (`tail -n2`). The line `timestamp` is set by Droid before the append, so the pairs
overstate the latency, never understate it.

## VAL-CROSS-007: CLI `unknown events: N` against the tray, M0 and S captures

```text
$ cargo run -q -p observatory-cli -- replay fixtures/streams/2026-10-22-observatory-v1-m0.jsonl | tail -n1     (real 0m7.526s, build included)
unknown events: 0
$ cargo run -q -p observatory-cli -- replay fixtures/streams/2026-10-22-observatory-v1-s.jsonl | tail -n1      (real 0m0.934s)
unknown events: 0
```

Tray, `trunk serve` on 3141 (service `trunk-ui`), `agent-browser --session m6-cross-007`, file picker
(`upload "input[type=file]" <capture>.jsonl .settings.json .capture.json .children/*`):

```text
M0 capture:  eval tray-count text -> "0"   heading "Unmapped tray 0"   unknown-count "0"
S capture:   eval tray-count text -> "0"   heading "Unmapped tray 0"   unknown-count "0"   event-count "661" (S: 661 events, AGENTS.md)
console / errors: empty. screenshot /tmp/m6-cross-007-s.png (scratch, not committed)
```

Pairs: M0 CLI 0 / tray 0; S CLI 0 / tray 0.

## VAL-GATE-027 and VAL-GATE-033: demo merge by the single login, app attached

Procedure (D-040, no `demo:red`, no Merge press): branch `demo/green-1` created from `demo/base` (`0a853d90eb147c7d866ed1cd5020b467d7a0c2e4`) with
`gh api repos/dctellya/droid-observatory/git/refs`, one file committed by `gh api -X PUT .../contents/demo/green-1.txt` (head `63a7ee227c1d4d77f753f1f0d673f1c71873268d`),
`gh pr create --base demo/base --head demo/green-1` -> `https://github.com/dctellya/droid-observatory/pull/41`. The app (same run as above, config
`[mission].branch_pattern = "demo/green-1"`, `[github].target_branch = "demo/base"`) polled PR 41 for the attached scratch session's mission.

| What | Value | Command |
|---|---|---|
| PR number | 41 | `gh pr create ...` output; `gh pr view 41 --json number,...` |
| demo-gate run id | 41122154885 (`pull_request`, head 63a7ee2, success, created 20:18:51Z, updated 20:21:46Z) | `gh run list --repo dctellya/droid-observatory --branch demo/green-1 --json databaseId,...`; `gh run view 41122154885 --json status,conclusion,headSha,createdAt,updatedAt` |
| merge | `gh pr merge 41 --repo dctellya/droid-observatory --squash --delete-branch`, exit 0, between 20:22:30Z and 20:22:35Z | shell |
| merged | `41 2026-10-30T20:22:33Z dctellya 9a5521f8360c9d763f6a23ea203179a7e2a87572 https://github.com/dctellya/droid-observatory/pull/41` | `gh pr view 41 --repo dctellya/droid-observatory --json number,mergedAt,mergedBy,mergeCommit,url --jq '[...]\|@tsv'` |
| token owner | `dctellya` | `gh api user --jq .login` |
| ruleset | `demo-gate`, id 26112925 (`decided_by` in the log lines) | app log `gate_changed` |

App log (`/tmp/observatory-tauri.log`): before the merge `gate_changed ... badge="ci_green" ... pr=41 ... merge_offered=true required_checks=["demo-gate=passed"]` at 20:22:01.586883Z;
after it `gate_changed ... badge="human_merged" ... pr=41 ... merge_offered=false required_checks=["demo-gate=passed"]` at 20:23:08.770528Z (35.8 s after `mergedAt`, one 30 s poll interval plus the request).
`GateBadge::HumanMerged` renders `handed to CD` (`crates/observatory-core/src/view/gate.rs:56`).

Store (`sqlite3 -readonly observatory-scratch/m6-live/app.sqlite`):

```text
select id, kind, lane, role, sha, url, at from events where kind='handed_to_cd';
github:pr:41:handed_to_cd|handed_to_cd|gate|human|9a5521f8360c9d763f6a23ea203179a7e2a87572|https://api.github.com/repos/dctellya/droid-observatory/pulls/41|2026-10-30T20:22:33.000Z
select number,state,merged_by,head_sha from prs where number=41;
41|merged|dctellya|63a7ee227c1d4d77f753f1f0d673f1c71873268d
select count(*) from events where kind='handed_to_cd' and id like '%:41:%';          -> 1
select count(*) from human_actions where kind='merge' and target like '%41%';        -> 0
select count(*) from human_actions;                                                  -> 0   (also 0 before the merge)
select id,state,completed_at from missions;
ff845ce5-e0bf-416b-bf2f-df0df6f04893|complete|2026-10-30T20:22:33.000Z
```

The event's `sha` equals the merge commit above (`9a5521f...`), `lane` is `gate`, exactly one event, `role` `human`: the merger `dctellya` is the token owner,
the role GATE-026 derives for that login. With one login the merger is always the token owner, so `role='external'` cannot be observed live (pass 43
resolution); the live half is: no `human_actions` merge row, one `handed_to_cd` with lane `gate`, badge `handed to CD`.

### The `external` branch (unit test, test-first)

The role derivation was inline in `GitHubSource::poll`; `crates/observatory-core/src/ingest/github.rs` had no unit test module. Red commit `2e604ad`
(`test(core): red: merger role external branch (does not compile)`, `error[E0432]: unresolved import super::merger_role`, exit 101), green commit `50125c8`
(`feat(core): merger_role names the external merger`).

```text
$ cargo test -p observatory-core --lib --all-features ingest::github::tests        (real 0m0.352s)
test ingest::github::tests::a_merger_other_than_the_token_owner_is_attributed_as_external ... ok
test ingest::github::tests::a_merger_the_forge_cannot_name_is_attributed_as_external ... ok
test ingest::github::tests::the_token_owner_merging_is_attributed_as_human_ignoring_case ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
$ cargo test -p observatory-core --all-features --test forge_merge                 (real 0m8.424s)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```

`forge_merge.rs::policy_merged_detected_without_human_action` (merger `some-colleague`, `SELECT role FROM events WHERE kind = 'handed_to_cd'` is `external`)
still passes, so the poll behaves as before.

## Cleanup and gate

- Scratch session: `droid exec` exited 0 (`done`, `exit=0`); no `droid exec` process remains (`ps -axo pid=,command= | grep -E 'observatory-app|trunk serve|cargo-tauri|droid exec'` prints nothing, grep exit 1).
- Ports: `lsof -nP -iTCP:3141 -sTCP:LISTEN` exit 1 after killing cargo-tauri (31021), trunk (31155), app (31328) by pid; `agent-browser session list`: `No active sessions`.
- `gh api repos/dctellya/droid-observatory/branches --paginate --jq '.[].name' | grep '^demo/'` prints `demo/base` only (`--delete-branch` removed `demo/green-1`).
- Inner loop on the touched crate: `cargo clippy -p observatory-core --all-targets --all-features -- -D warnings -W clippy::cognitive_complexity` exit 0
  (`Finished dev profile ... in 33.63s`, real 33.807s); `cargo fmt --all -- --check` exit 0.
- PR gate before this feature: run 41119672975 (head `12365dd`), `gh run view 41119672975 --log-failed` shows one failing test,
  `test metrics_match_reference_spreadsheet_within_tolerance ... FAILED` (`crates/observatory-core/tests/oracle.rs:216:5`, `test result: FAILED. 5 passed; 1 failed`), in `light / coverage`; `light / mission-checks` and `gate` are aggregators that fail with `a required job failed` and no other test in the log;
  the reference CSV is copied by `m6-close-integrate` (PR gate note, 2026-10-30 ~17:30Z).
