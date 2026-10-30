---
kind: postmortem_input
mission: e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c (observatory-v1)
milestone: m2
run: null
window: 2026-10-24T07:49:27Z .. 2026-10-24T21:48:55Z
price_table: 2026-10-22
cycle_template: none (the cycle is code until m3-cycle-template-refactor, F-041)
mapping: v1@unknown (the capture header names no Droid version)
---

# Post-mortem: milestone M2 (host), observatory-v1

Written 2026-10-25 by `m3-boundary-docs-m2` (owner item 11, M2 boundary of 2026-10-24; format of
`library/post-mortem-format.md`, Roci PRD Appendix B). Sections 1 to 5 are **Roci's input**, the format Roci will
write at a milestone end, produced here by the mission's own tools because Roci does not write it yet; every command is
pasted. Sections 6 and 7 are the mission's answers and the rules it proposes. Every number is pasted from a tool
(rule 7); where a tool does not exist yet, the section says so.

The window is the span the owner named for M2 after the 402 pause: from the resume at `mission_resumed` 07:49:27.269Z
(`MD/progress_log.jsonl` line 290) to the `worker_completed` of the integrator `4b1e0a29` at 21:48:55.544Z (line 370):
**13 h 59 min 28 s = 839.5 min**. The 2 h 1 min 16.57 s `throttled` window (05:48:10Z to 07:49:27Z,
`unrecoverable_usage_402`) ends where the window starts and is outside it; it is reported in `reports/usage/m2.md`.

Tools that exist, tools that do not (checked 2026-10-25, `cargo run -p observatory-cli -- --help`):

```text
Commands:
  replay   Print one line per event of a session file, then `unknown events: N`
  inspect  Print the raw line an event was mapped from, found by its stored pointer
```

**`observatory-cli metrics` does not exist yet** (waste metrics are M5, VAL-METRIC-009). The clocks and causes below use
`replay`, `inspect`, `scripts/usage-from-log.sh`, `MD/progress_log.jsonl`, `reports/usage/m2.md` and the build-log, and
two short analysis scripts whose text is in the appendix. `make usage` does not exist yet either (F-065, `m3-usage-target`).

## Roci's input

### 1. Clocks

**Wall clock:** 839.5 min (13 h 59 min 28 s), `phases.py` on `MD/progress_log.jsonl`. The worker sessions never overlapped
(`sessions overlapping: False`): one worker or validator session ran at a time, the flow validators and reviewers ran
inside their validator's span.

**Replay, per lane.** `replay` over the orchestrator session `e194b9e3-....jsonl` and its children, with the droid log
files joined, written to a scratch store (`--store /tmp/m2pm/s.sqlite`, not the dogfooding store), then `clocks.py` over
the output for the window. Commands and output:

```sh
S=~/.factory/sessions/-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory
L=~/.factory/logs
cargo run -q -p observatory-cli -- replay --store /tmp/m2pm/s.sqlite \
  --usage-log $L/droid-log-single.log.2026-10-24 --usage-log $L/droid-log-single.log.2026-10-24.1 \
  --usage-log $L/droid-log-single.log.2026-10-24.2 --usage-log $L/droid-log-single.log \
  $S/e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c.jsonl > replay.txt        # exit 0, 7605 lines
python3 clocks.py replay.txt 2026-10-24T07:49:27Z 2026-10-24T21:48:55Z
```

```text
lane            events  turn.usage  first                     last                      span_min  active_min  sessions
1 Plan          73      229         2026-10-24T08:00:30.121Z  2026-10-24T21:31:59.448Z  811.5     37          1
3 Build         351     606         2026-10-24T07:53:35.979Z  2026-10-24T18:28:18.461Z  634.7     102         14
4 Scrutiny      202     192         2026-10-24T11:57:53.720Z  2026-10-24T17:59:45.085Z  361.9     40          18
5 User testing  166     371         2026-10-24T12:39:02.376Z  2026-10-24T18:16:43.206Z  337.7     51          15
6 Integrate     33      177         2026-10-24T12:00:14.853Z  2026-10-24T21:48:55.820Z  588.7     22          5
total events (excl turn.usage) 825
```

`active_min` is the number of distinct minutes of the window with at least one event of the lane; `span_min` is first to
last event and is not a clock (lanes interleave across the day). Lane 2 Contract and lane 7 Gate have no events in the
window (the contract passes ran as orchestrator subagents whose files `replay` does not attribute to Contract after the
M0 capture; the Gate lane is M4). The tail of the same `replay` run (all turns of the four log files, not windowed):

```text
totals  orchestrator    349 188583  249696  28549089
totals  worker  1677    215477  1822080 206119878
totals  scrutiny_validator  396 494399  529960  28324689
totals  user_testing_validator  804 446573  889642  67622164
join mismatches: 0
turns of other sessions: 211
unknown turns: 0
unknown events: 36
```

(`turns, input, output, cache_read` per role.) `unknown events: 36` is the count over the whole mission tree; 10
`unknown.event` rows fall inside the window (`sqlite3 s.sqlite`, `kind = 'unknown.event'`). `inspect` on a stored pointer
works: `observatory-cli inspect --store /tmp/m2pm/s.sqlite e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c:15457704:0` prints the
raw line (`{"type":"agent_turn_outcome", ... "reason":"cancelled" ...}`), the first `mission.paused` of the tree.

**Phase clocks** (`phases.py`: `worker_started` to `worker_completed` of each session in `progress_log.jsonl`, classed;
the six classes add up to the window):

| Class | Minutes | Share of window | Sessions |
|---|---|---|---|
| Build, counted | 227.5 | 27.1 % | `f2c85ffc` 9.1 (from the resume), `6690e7b0` 138.6, `a34f0166` 73.2, `eb8cd3fd` 1.1, `afed69c8` 2.3, `2c1df39c` 3.1 |
| Scrutiny, counted | 33.8 | 4.0 % | `47f0427d` 21.4 (round 4), `e2527dd1` 12.4 (round 5) |
| User testing, counted | 173.7 | 20.7 % | `40eeeb2e` 64.1 (round 2), `f01ebe06` 94.1 (round 3), `dd590b99` 15.5 (round 4) |
| Void (model fallback or provenance) | 84.4 | 10.1 % | `9b08f77e` 24.1 (round 1), `f4033628` 2.5, `b34ee5bc` 6.1 (round 2), `2dc80c60` 46.4 (user testing round 1), `ddd209b6` 5.4 (round 3) |
| Integrate | 200.6 | 23.9 % | `4b1e0a29` (3 h 20 min 38 s) |
| No worker running (orchestrator triage, owner, platform) | 119.5 | 14.2 % | 11 gaps; the long ones: 31.0 min 13:25 to 13:56 (owner restores settings after the stop), 28.7 min 15:36 to 16:05 (triage and contract passes 19 to 21), 17.0 min 10:19 to 10:36 (owner "resume mission"), 11.6 min 18:16 to 18:28 |
| **Sum** | **839.5** | 100.0 % | |

**Spend, tokens in the window** (`scripts/usage-from-log.sh --embedded` over the three 2026-10-24 log files, summed per role by
`roles.py`; `--from 2026-10-24T07:49:27Z --to 2026-10-24T21:48:55Z`; session ids per role as in `reports/usage/m2.md`):

| role | first ts | last ts | lines | input | output | cache_read |
|---|---|---|---|---|---|---|
| orchestrator | 2026-10-24T07:54:31.667Z | 2026-10-24T21:48:06.591Z | 229 | 188257 | 112336 | 15476385 |
| workers | 2026-10-24T07:49:34.996Z | 2026-10-24T21:48:21.219Z | 650 | 1352 | 555755 | 81775380 |
| validators | 2026-10-24T13:56:28.279Z | 2026-10-24T18:16:42.735Z | 412 | 908 | 503976 | 31990959 |

The `validators` row is the 22 sessions of `reports/usage/m2.md` (round 3 onward, on `claude-opus-5-5`); the void
validator rounds are in the `model fallback (void)` group of that report. USD-eq of this window is not recomputed: the
price rows need the cache writes, which the script does not print; the milestone figures are in `reports/usage/m2.md`
(M2 total 120.61 of 50, orchestrator 118.84 of 30, mission-to-date record 694.66).

**Cycle time per feature.** Engineering cycle time is `feature.defined` to `handed_to_cd` (F-028); M2 has no
`handed_to_cd` (the pull request is at M5), so it is not computable. Worker minutes per feature in the window, from the
table above: `m2-host-shell-and-launch` 211.8 (two rounds), `m2-rpc-source-core` 9.1 (window part), `m2-fix-d032-alternatives`
3.6 (void run and verify-and-complete), `m2-fix-first-line-pointer` 2.3, `m2-fix-process-exit-outcome` 3.1,
`m3-integrate-m2` 200.6.

**Value share.** Not computed: no tool for it exists before M5.

### 2. Top five

Causes are assigned by the rules stated in the Clocks table, not by the metrics engine (M5). Minutes are wall minutes of
the window; USD-eq and tokens are the figures `reports/usage/m2.md` gives for the same sessions (its windows are wider by
3 h 21 min, so the two do not add). Shares are of 839.5 min. "Previous to this": there is no earlier post-mortem (M1 had
none and its causes were not measured), so the column is `n/a, first measurement`.

| Cause by lane | Minutes | USD-eq | Tokens | Share | Lever | Previous to this |
|---|---|---|---|---|---|---|
| Build overhead, a feature over the size cap (`m2-host-shell-and-launch`, Build) | 211.8 | 18.54 (sidecar, two sessions) | 445 turns, 60343239 cache read | 25.2 % | R-041 and the 120-turn cap (F-058) | n/a |
| Integration done by hand (`m3-integrate-m2`, Integrate) | 200.6 | 11.08 (sidecar reading) | 154 turns, 18163331 cache read | 23.9 % | `make usage` and `make integrate` (F-065, F-066) | n/a |
| Validation overhead (Scrutiny and User testing, counted rounds) | 207.5 (21.3 of it timed gate commands) | 40.48 (22 validator sessions) | 31990959 cache read | 24.7 % | R-051, R-052, R-053 (rules 26 to 28) | n/a |
| Void model-fallback sessions (Scrutiny, User testing, Build) | 84.4 | 10.07 (Claude turns only) | 220 `kimi-k3` turns in the window (store); 162 turns, 9775141 cache read in the report window | 10.1 % | model check (F-057) and the limit preference (F-055) | n/a |
| Waiting on the owner (between runs) | 48.0 (17.0 + 31.0) | 0 | 0 | 5.7 % | R-042 (rule 24) and the resume protocol (F-057) | n/a |

Outside the minutes ranking and first by cost: **retry and polling** (R-044): 121 re-issued `WaitForScript` calls,
86.49 USD-eq of the 118.84 USD-eq orchestrator split (72.8 %), 0 worker minutes lost (the waits ran beside workers).
Lever: the background-Script mechanism (F-056). `throttled` is 0 min inside the window (121.3 min before it).

### 3. Signatures

- **Spikes (time, role, model, rate).** The orchestrator's wait turns: 110 on `claude-fable-5.1` and 11 on `kimi-k3` in
  `reports/usage/m2.md`, each a cache rewrite (6740714 cache-write tokens in 110 turns), at the platform's five-minute return
  (107 of the 121 returned between 290 s and 310 s). Gate-type commands: 121 commands over 18 sessions by the pasted
  classifier (`gates_tr.py`: 90 `cargo test`, 16 `cargo check`, 5 clippy, 3 `make check`, 3 `make test`, 3 `make lint`, 1
  `llvm-cov`; transcripts and build-log give the same totals); the owner counted 180 commands over 31 sessions (104, 29, 25, 7, 7, 5, 3) against
  about eleven distinct tree states. **The 180 and the eleven trees are the owner's; the record reproduces 121 over 18 with
  a stricter classifier (a command that starts a shell segment, `make check-links` excluded) and has no tree hashes, so the
  eleven states cannot be recomputed from it.** The repetition is visible in the handoffs: the scrutiny rounds 2, 3 and 4
  all ran the gate on `8ab88e2`, round 4 with `cargo llvm-cov` (137.58 s), and the integrator ran `make check` and
  `cargo llvm-cov` (133.6 s) on the merged tree; rule 27 records the tree so M3 can count it.
- **Throttle windows, models declared against used.** Declared: orchestrator `claude-fable-5.1`, workers
  `claude-sonnet-5-5`, validators `claude-opus-5-5`. Used: `kimi-k3` in 220 turns of the window (orchestrator 58, scrutiny
  validators 46, user-testing validators 91, workers 25; `turn.usage` rows of the replay store with `model = 'kimi-k3'`).
  The router transitions (`overage_reactive_402` 12:03Z, `overage_proactive` 12:21:58Z to 13:49:14Z) are in the
  model-episode table of `reports/usage/m2.md`.
- **Concurrency shape.** One worker session at a time in the window. A validator span holds its children: scrutiny rounds
  with 4 or 5 reviewers, user-testing rounds with 2 to 5 flow validators; the scratch missions (40 sessions, 195 turns) ran
  beside the host-shell worker and cut two of the orchestrator's waits short.

### 4. Anomalies

| Id | What | Evidence | Cause | Cost |
|---|---|---|---|---|
| A-01 | Model fallback, episode 4 (F-046): live sessions switched to `kimi-k3` | model-episode table of `reports/usage/m2.md`; 220 `kimi-k3` turns in the window | usage 402, router reason `overage_reactive_402` and `overage_proactive`; the platform never switches back | 84.4 void minutes, 10.07 USD-eq priced plus unpriced `kimi-k3` tokens |
| A-02 | 121 re-issued waits | `reports/usage/m2.md`, orchestrator spend by class | `WaitForScript` returns after about five minutes and takes no timeout | 86.49 USD-eq (110 fable turns), 7 of them under 60 s |
| A-03 | A 445-turn feature in two rounds | `reports/usage/m2.md`, "Feature size": 266 + 179 turns, 36073608 + 24269631 cache read | one feature held the launch form, the host shell, D-034 and the window checks | 211.8 min, 18.54 USD-eq |
| A-04 | Gate reruns | 121 commands over 18 sessions (pasted), 180 over 31 (owner); no tree hash recorded | no gate record keyed by tree | validator and integrator timed gate commands 21.3 min, lower bound |
| A-05 | Phantom pause 10:24 local (08:24:53Z) | `progress_log.jsonl` line 298; `interrupt_session` 08:24:51Z and 08:26:30Z | lifecycle of scratch mission `6e346d07` hosted by the host-shell worker's self-check on the user-default models | one orchestrator wait cut short; platform artefact, not a pause of this mission |
| A-06 | Commit `8ab88e2` has a stray heredoc tail that breaks trailer parsing | owner item 10 | a commit message built inside a chained shell command by `f4033628` (on `kimi-k3`) | forward-only fix: `.git-trailers` and `.gitmessage` (F-059) |
| A-07 | VAL-HOST-014 reworded to pass (F-050) | owner answer 2 | hosted mapper did not read `todo_state` from the session file | reversed by the owner: fix feature `m3-hosted-todo-state` (F-070) |
| A-08 | Replay of the mission tree ends with `unknown events: 36` (10 in the window) | `replay` tail above | event shapes the v1 mapper counts and shows (rule 18) | none; counted, not dropped |

### 5. Questions

The five of FR-P5, numbered. The mission answers them in section 6.

1. Which features broke R-041 and why?
2. Which boundary questions could R-042 have answered?
3. Which scrutiny findings caused rework, and were they contract gaps?
4. What did the idle worker wait for?
5. One rule to add, with the waste it removes and how Roci will see it.

## The mission's answers

Count: **5 questions, 5 answers.** Each question is quoted, the answer follows. Sources: `MD/owner-feedback/2026-10-24-m2-boundary.md`
(items 1 to 19 and the owner's answer of 23:52), mission `AGENTS.md` items 33 to 38, `reports/usage/m2.md`, SPEC §9 F-046
to F-070.

### Answer 1

> **Q1. Which features broke R-041 and why?**

R-041 is rule 23: the build is paid in series, so a worker checks in the inner loop and the full suite runs once, at
integration. By the pasted command classification no worker session ran `make check`, `make test`, `make lint`, a workspace
`cargo test` or `cargo llvm-cov` (0 in each of the nine worker sessions); the breach by the letter was by validators and the
integrator: scrutiny rounds 1, 2 and 3 ran 3, 2 and 3 full-gate commands (`9b08f77e`, `b34ee5bc`, `ddd209b6`) and rounds 4
and 5 each ran `cargo llvm-cov` on a tree already proven. The breach in effect was one feature: **`m2-host-shell-and-launch`**
broke the intent of R-041 because it was not one round of inner loop: 54 crate-level `cargo test` runs (24 in round 1, 30 in
round 2), 445 turns, two sessions, 211.8 min, all in a single feature that held the launch form, the Tauri host, the
D-034 recipe and the window-only checks. Compile time itself was small (110.50 s and 98.11 s per round, 3.55 % of the
window by `reports/usage/m2.md`); the cost was the length of the feature. Lever: the feature size cap (owner item 8,
F-058): one round, under 120 turns, split at the contract pass, `partial` at the next checkpoint when the cap is crossed.

### Answer 2

> **Q2. Which boundary questions could R-042 have answered?**

R-042 (rule 24) lets the orchestrator take a recommendation that changes neither scope, money nor models and report it
at the boundary. Of the owner's 19 items and 6 answers at this boundary, those that change none of the three are the
process rules and mechanics: the scratch-mission rule (item 1, already F-042), the rule 7 wording (item 3), the
background-Script wait mechanism (item 6, which only applied rule 25 as written), the model check after a 402 (item 7,
which applied rule 20), the feature size cap (item 8), the commit template (item 10), the officer vocabulary (item 12) and
the usage and integrate tooling (items 18 and 19). The orchestrator could have decided and recorded these as §9 rows without
the owner; the owner decided them first. Questions that did need the owner: the Haiku declaration (item 2, a model), the
caps and the hard stop (items 4 and 13, money), the e2e harness (item 14, scope and a new dependency), the reconciliation basis
(item 5) and the reversal of the VAL-HOST-014 reword (answer 2), where the orchestrator had taken a rule-24 decision (F-050)
that the owner reversed because a contract is not reworded to pass. So F-050 is the one rule-24 decision the owner
overturned.

### Answer 3

> **Q3. Which scrutiny findings caused rework, and were they contract gaps?**

Scrutiny had five rounds. Round 1 (`9b08f77e`, 12:21:56Z): all gates green, 5 features reviewed, 4 passed, 1 failed:
`m2-mapping-scrutiny-feature-defined`, because D-032 lacked the `Lane::home(role)` alternative that VAL-MAP-027 requires. That is a
**documentation gap in the feature**, not a contract gap (the contract named the alternative); rework was one docs-only fix
(`m2-fix-d032-alternatives`, 2.5 min on `kimi-k3`, void; verify-and-complete 1.1 min on sonnet). Rounds 2 (pass) and 3 (fail by
provenance) are the model fallback of episode 4, not findings. Round 4 passed, round 5 passed with the two fix features
reviewed. The user-testing rounds found the real rework, and three of the four round-2 failures were **contract gaps**:
VAL-HOST-005 named a literal `--mission` command that Droid 0.247.0 rejects (reworded in contract pass 21, F-049),
VAL-PROC-045 fixed column names after `1315ec2` was published (F-051), and VAL-HOST-014 compared two views that cannot agree
until the hosted mapper reads `todo_state` (reworded to pass, F-050, reversed by the owner, F-070). The fourth,
VAL-BOARD-038 (stale first-line pointer), was a code defect fixed in `m2-fix-first-line-pointer`; round 3 found one more code
defect (VAL-HOST-016, one `unknown.event` on window close), fixed in `m2-fix-process-exit-outcome`.

### Answer 4

> **Q4. What did the idle worker wait for?**

No worker waited. The workers ran one at a time and each started within seconds of the orchestrator's `StartMissionRun`
(`progress_log.jsonl`: `worker_selected_feature` to `worker_started` under 0.2 s). What waited was the **mission**: 119.5 min
of the window had no worker running. By the `mission_run_started` messages, 48.0 min of it waited for the owner (17.0 min, 10:19 to
10:36: "Owner: resume mission" after the host-shell round 1; 31.0 min, 13:25 to 13:56: "Resume after episode 4" once the
owner restored the models), 28.7 min for the orchestrator's triage and the contract passes 19 to 21 (15:36 to 16:05), and the
rest for orchestrator triage between rounds (11 gaps, 1.5 to 11.6 min). The orchestrator itself was not idle: it spent 121 turns
re-issuing waits (A-02) while workers ran.

### Answer 5

> **Q5. One rule to add, with the waste it removes and how Roci will see it.**

**Rule 28 with rule 27 (owner items 16 and 17): a gate is proven once per tree, validators read the record, fixes run
the delta, integration is the final run.** It removes the **validation overhead**: the validator and integrator gate-type
commands re-run on trees already proven (21.3 min timed at minimum in this window, 207.5 min of counted validator-session
wall clock around them, and the integrator's hour of ad hoc derivation, F-065). Roci will see it as `gate.repeated`
(anomaly) and as the `validation overhead` share of the Scrutiny and User testing lanes falling between M2 and M3, with gates
run and gates served from cache in the M3 usage report. Rule 26 (tooling looked up, in SPEC `## Tooling`) is the
precondition: it names the tool for every check so that a validator runs it and never improvises a harness. The three
other rules the milestone produced are in section 7.

## Waste cause: validation overhead (rule 26)

**Validation overhead** is a named waste cause from this post-mortem on: the minutes validators and the integrator spend
re-running or re-deriving what an earlier, already-proven gate gave. For M2 the record gives, with the sources:

| Measure | Minutes | Source |
|---|---|---|
| Timed gate-type commands in validator sessions | 15.6 (936.66 s) | `reports/usage/m2.md` "Compile share": `9b08f77e` 167.00, `ddd209b6` 103.70, `47f0427d` 234.51 (incl. `cargo llvm-cov` 137.58), `40eeeb2e` 142.29 (incl. clean-clone `trunk build` 56), `f01ebe06` 13.34, `e2527dd1` 272.80 (incl. `cargo llvm-cov` 137.17), `dd590b99` 3.02 |
| Timed gate-type commands in the integrator session | 5.7 (340.773 s) | same table: `make check` 118.050, `cargo llvm-cov` 133.599, `cargo check -p observatory-core` 3.621, `cargo test -p observatory-core` 69.495, `trunk build` 16.008 |
| **Validation overhead, measured (a lower bound)** | **21.3 (1277.433 s)**, 2.5 % of the window | sum of the two rows above |
| Wall clock of the validator sessions that held them | 207.5 counted (scrutiny 33.8, user testing 173.7), plus 84.4 void | `phases.py` |

It is a lower bound for two reasons the record states: the reviewers' and flow validators' commands are not timed in
the handoffs (the `build-log` has `duration_ms` only for top-level `Execute` calls, none for validators' calls nested in a
`Script`), and no tree hash was recorded, so the part of the 21.3 min spent on a tree already proven cannot be separated from
the part on a new tree. From M3 the gate record (rule 27) makes both exact: gates run, gates served from cache and the
minutes saved are lines of the usage report. The lever is R-051 (the Tooling section), R-052 and R-053 (one proof per tree,
validators read the record, fixes run the delta, integration is the final run).

## Rules proposed

| Rule | Waste it removes | Destination | Proof expected in M3 |
|---|---|---|---|
| Rule 26, tooling looked up (SPEC `## Tooling`, F-062) | validators improvising harnesses; window driving by an LLM | shared KB (R-051) and project | every check has a row; `make e2e` replaces the 8 blocked window assertions |
| Rule 27, gate cache by tree hash (F-063) | gate reruns on a proven tree (validation overhead) | shared KB (R-052) and project | the M3 usage report states gates run and served from cache with minutes saved |
| Rule 28, one proof, validators read, fixes run the delta (F-064) | validators' own full gates (A-04) | shared KB (R-053) and project | two full gates per milestone |
| Background-Script wait, no `WaitForScript` (F-056) | retry and polling, 86.49 USD-eq in 121 waits | project; shared KB (R-044) | zero re-issued waits in the M3 usage report |
| Feature size cap, 120 turns, one round (F-058) | build overhead, 211.8 min in one feature | project | turns per feature, none over 120 without a `partial` hand-off |
| Model check after any 402 or resume for every session (F-057) | void model-fallback rounds, 84.4 min | project (rule 20 mechanics) | a `model = <value>` line in every hand-off; zero void sessions |
| `make usage` and `make integrate` with staging files (F-065, F-066) | integration by hand, 200.6 min | project | integration under 15 min of model time |
| Commit message from a file with trailers from `.git-trailers` (F-059) | broken trailer parsing (A-06) | project | `%(trailers:key=Co-authored-by)` prints on every commit |

## Appendix: scripts and commands

All run from the repository root on 2026-10-25; the scripts are short and read only files named in the text.

- `phases.py`: reads `MD/progress_log.jsonl`, pairs `worker_started` and `worker_completed` per session, classes them (void
  sessions: `9b08f77e`, `b34ee5bc`, `2dc80c60`, `f4033628`, `ddd209b6`), prints the minutes per class and the gaps between sessions;
  its output sums to the window (839.5).
- `clocks.py`: reads the `replay` output (tab-separated: timestamp, lane, kind, session) and prints per lane the event count,
  the `turn.usage` count, first and last timestamps and the distinct active minutes in the window.
- `roles.py`: parses the session ids per role from `reports/usage/m2.md`, runs
  `scripts/usage-from-log.sh <3 log files> --embedded --from ... --to ... --session <ids>` per role and sums the rows.
- `gates_tr.py` and `gates.py`: scan the session transcripts and the `build-log/2026-10-24/*.jsonl` for `Execute` commands
  whose shell segment starts with `cargo test|check|clippy|llvm-cov` or `make check|test|lint` (a target followed by `-` or a word
  character, as in `make check-links`, is excluded), count them per kind and session, and sum `duration_ms` where the hook wrote it.
- `waits.py`: in `reports/usage/m2.md`, section "Waits that returned in under 60 s".
