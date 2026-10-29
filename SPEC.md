# droid-observatory — Spec for the mission

Owner: David Carbajosa (architect, reviewer, the human who merges). Implementers: one Factory
Mission (orchestrator, workers, validators) under ai/RULES.md. Status: v0.1 approved by the owner
(2026-10-22); v0.2: §5.8 concurrency limits added (owner decision 2026-10-22); v0.3: amended
as-built at the S integration (2026-10-23, rule 8): §3 M0, §3 M4, §5.2, §8, §9, §10; v0.4: F-028..F-031
at the M0 to M1 boundary (2026-10-23): §1, §2, §3 M1/M3/M4/M5, §5.2, §5.3, §5.6, §6, §7, §9, §10; v0.5: F-032..F-034 during M1 (2026-10-23): §5.2, §5.6, §9. Living document: §9 is the human feedback channel, §10 the as-built history.

Inputs: `ai/prd/droid-observatory-prd.md` (v0.5), `ai/prd/review-2026-10-22.md`, `ai/RULES.md`,
`ai/SPEC-TEMPLATE.md`, `docs/decisions/D-001..D-010`, `BACKLOG.md`, `config/observatory.example.toml`,
`config/prices/2026-10-22.toml`, `ai/session/2026-10-22-session.md` entries 17–21.

Approval boundary: this document is the contract the mission plans from. Until the owner answers
`approved`, nothing else happens: no milestone plan, no worktree, no application code, no GitHub
object. After approval the mission plans from this file only (rule 9: deviations are proposed with
reason and cost, and wait).

## 1. Purpose

**What it is.** A Rust desktop application (Tauri 2 shell, Leptos UI, `observatory-core` library)
that watches a Factory Mission. It measures the waste and effectiveness (cycle time, cost) of the
engineering cycle inside a mission, and across a run, from the spec to the pull request's checks
going green. Release and observe belong to the delivery platform; the two meet at the merge.
Lanes are the mission's own phases, one axis: 1 Plan, 2 Contract, 3 Build, 4 Scrutiny, 5 User
testing, 6 Integrate, 7 Gate. Roles are the colour and the attribution on each event; the human's
interventions and a commander's actions are overlays (`overlay: human|commander`), not lanes; time,
waste, cost and model drift are the Panel, an area above the lanes (F-028, D-024). Two clocks are
named on the board and never confused: the **engineering cycle time** (Roci's clock: `feature.defined`
to `handed_to_cd`, the one final event that carries the merge sha, the PR number and the attestation
ids) and the **lead time to production** (the delivery platform's clock, which starts at the same
sha; a label and a link, never a number in Roci). Cards are milestones with their features inside
(PRD §8); every lane entry is a view over the raw JSONL line it came from, never a copy (PRD §11.2).

**For whom.** The mission owner who runs missions from a terminal and wants to see flow and cost;
the reviewer who opens Details and merges; the engineering lead who compares models over time
(PRD §6). Single user, local data, the user's own GitHub access.

**What it must prove.** The mission is done when all of these hold on the mission branch:

1. **Attach.** A mission started from any terminal on this machine appears on the board and every
   event of it shows within 1 s (FR-L1, FR-B6), with no control channel.
2. **Replay with a frozen mapping.** `observatory-cli replay` over the frozen capture ends with
   `unknown events: 0`, and the board renders the same capture with correct phase positions and
   the human overlay markers (FR-L2, FR-B7, rule 18).
3. **Host.** A mission launched from the app shows events within 1 s and the pause, message and
   answer controls reach the orchestrator (FR-L3, FR-H1..H4).
4. **Gate and fix loop.** Against the demo branch pair in this repository (§3 M4): a red check
   opens a `fix` cycle on the affected card (counted as rework), a green check with a human policy
   shows Merge, and the owner's merge is observed as the one final event `handed_to_cd` (merge sha,
   PR number, attestation ids) that closes the engineering cycle; nothing after the merge is
   observed (FR-G3..G8, F-028).
5. **Time, waste, cost.** Metrics recomputed from the frozen capture match the owner's reference
   spreadsheet within rounding (FR-M1..M7, PRD §10); every cost names its price-table date.
6. **The build is the first dataset.** The sessions of this mission are the capture that M0
   freezes and M5 is measured against (rule 17).

**Out of scope (explicit).** PRD §5 non-goals: planning inside the app, multi-user or server
deployment, writing to Factory's hosted API beyond what `droid` does, editing or reviewing code in
the app, CI systems other than GitHub Actions. Also out: CI telemetry to Grafana (B-002), a
dedicated error tracker (B-003), Linux and Windows bundles (macOS is the target; the devcontainer
is core-only, B-001), the app performing any merge other than the human-confirmed Merge button
(FR-G5): v1 never merges on its own. Observing a merge performed by a ruleset or a bot stays in
scope: `handed_to_cd` is emitted on detection with the merger attributed by role, human or
external (FR-G6, M4). Also out (F-028): anything after the merge, namely deploys, flags and fix-forward
Issues opened by the delivery platform (future work toward autonomy; Roci's last event is
`handed_to_cd`). Also out: a second mapping
version for Droid releases newer than the one captured. Guard: the mapper is versioned from day
one, every `Event` carries `mapping_version`, and the unknown-event count is always visible (tray,
CLI summary, exported series), so the first Droid upgrade shows as a rising count, never as a
silently broken board. And out: migrating the backlog before the mission starts (entry 19).

**What this spec answers.** The review note `ai/prd/review-2026-10-22.md` item 5 (attach is the
primary live mode, D-008). The owner decisions logged after PRD v0.5: the telemetry contract and
series budget (D-010, B-017, §5.7), M4 on a demo branch pair in this repository instead of a
private demo repository (entry 19, B-007, §3 M4), the hybrid backlog driven by the mission
(entry 19, §8; withdrawn as-built, D-015), the orchestrator model id `claude-fable-5.1` (entry 21, D-006 correction, §2),
and the backlog items the spec session owns: B-005 (§6), B-010 (§3), B-014 (§7), B-017 (§5.7).

## 2. Mission setup

**Command** (run by the owner from the repository root on a checkout of `main` that contains this
approved spec; the mission creates `mission/observatory-v1` from it):

```sh
droid exec --mission --auto high \
  --model claude-fable-5.1          --reasoning-effort high \
  --worker-model claude-sonnet-5-5  --worker-reasoning-effort high \
  --validator-model claude-opus-5-5 --validator-reasoning-effort high \
  --output-format text \
  -f SPEC.md
```

| Flag | Value | Reason |
|---|---|---|
| `--mission --auto high` | required | Missions auto-approve proposals and need `--auto high` (Droid 0.247.0 `droid exec --help`). The hooks in `.factory/hooks.json` are the guard, not the prompt. |
| `--model` | `claude-fable-5.1` | Orchestrator (D-006, corrected 2026-10-22 from the misspelt `claude-fable-5-1`, which fell back to GPT-6 Sol). Factory catalog id, 4× multiplier. |
| `--reasoning-effort` | `high` | Fable's default; planning and integration are where judgment is bought. `xhigh`/`max` on a 4× model is not justified before a measured need. |
| `--worker-model` | `claude-sonnet-5-5` | Workers (D-006). 0.8× multiplier. Catalog id per docs.factory.com/models; `droid exec --help` on this machine lists `claude-sonnet-5` but not `claude-sonnet-5-5`; owner confirmed available on 2026-10-22. |
| `--worker-reasoning-effort` | `high` | Owner decision 2026-10-22: the model default; Rust compile errors and Leptos (little training data, PRD §12) need the reasoning. The cost/quality split the Panel measures is between models, not efforts. |
| `--validator-model` | `claude-opus-5-5` | Validators, scrutiny and user testing (D-006). 1.6× multiplier. |
| `--validator-reasoning-effort` | `high` | Above Opus's default `medium`: validation is where correctness is bought (D-006 alternatives). |
| Scratch missions (self-checks) | `claude-haiku-4-5-20251001` | `low`. Declared by the owner on 2026-10-24 (F-042, §9) for the scratch missions a live self-check hosts, for every role of the scratch mission and for nothing else; the workers, the validators and the orchestrator keep the models above. Not a flag of this mission's launch command. |
| Subagents | `claude-sonnet-5-5` for mechanical work and flow runners; `claude-opus-5-5` for scrutiny reviewers and verdicts | Declared by the owner on 2026-10-29 (M3 boundary items 5 and 12b, D-039). A role with a declared model, pinned by the droid files in `.factory/droids/` (`mechanical-sonnet`, `user-testing-flow-validator`); the built-in droids inherit the parent's model and the Task `complexity` tier routes nothing here (`## Tooling`). Not a flag of this mission's launch command. |
| `--output-format` | `text` | The terminal is for the human; the dataset is the session JSONL under `~/.factory/sessions/`, which attach and replay read. |
| `-f SPEC.md` | this file | The mission plans from the approved spec (template, last paragraph). |

The same values are declared in `.factory/settings.json` so that a mission started from the
Factory App uses them: `missionOrchestratorModel`, `missionOrchestratorReasoningEffort`,
`missionModelSettings.workerModel`, `.workerReasoningEffort`, `.validationWorkerModel`,
`.validationWorkerReasoningEffort`. The three reasoning-effort keys are added with the spec PR
(§8). A change to any model is a decision file (rule 19).

**Mission branch.** `mission/observatory-v1`, created from `main` at the commit that contains this
approved spec. One pull request `mission/observatory-v1` → `main` at the end of the mission, opened
by the orchestrator, merged by the human after `gate` is green. Nothing else reaches `main`
(rule 12). The branch is deleted at the merge (PRD §2).

**Worktree policy.** One worktree per milestone: `git worktree add ../observatory-wt/<m> -b wt/<m>
mission/observatory-v1` where `<m>` is `s`, `m0` … `m5`. Workers and validators of a milestone
work only inside its worktree. Two worktrees never edit the same file (rule 13); the shared files
(`SPEC.md` §9–10, `README.md`, `BACKLOG.md`, `docs/decisions/README.md`, `reports/`) are written by
the orchestrator on the mission branch at integration. Integration is `git merge --no-ff wt/<m>`
into `mission/observatory-v1` by the orchestrator, then `make check` locally, then the
`mission-checks` run on the push must be green before the next milestone starts. A worktree is
removed after integration; its branch stays (history).

**Gate command.** `make check` is the only local gate: it is exactly what `mission-branch.yml`
runs (rule 5). The build is paid in series (F-029, rule 23, D-025): a worker's inner loop is `cargo
check -p <crate>` and `cargo test -p <crate>` for the crates it touches, plus one `cargo fmt --all --
--check` and `cargo clippy -p <crate> --all-targets -- -D warnings` before handoff; `make check` with
the full `cargo test --workspace` and `cargo llvm-cov` run once per milestone, by the integrator,
alone (an infra feature may run the one Makefile or CI target it changed). The orchestrator
integrates only with `make check` green on the mission branch.

**Hooks that must be active.** `.factory/hooks.json`: `guard-git.sh` (PreToolUse on `Execute`,
blocks amend/reset/rebase/stash/force-push/push-to-main) and `log-tool-call.sh` (PostToolUse,
appends one JSON line per tool call to `build-log/<UTC-day>/<session>.jsonl`). Git hooks via
`make hooks` (`core.hooksPath=.githooks`): `pre-commit` (gitleaks on staged files, file size, debt
markers, decisions index, rustfmt, markdownlint), `commit-msg` (subject ≤ 72 chars, no WIP/fixup),
`pre-push` (no `main`, no non-fast-forward). A worker that finds a hook missing stops and reports;
it never disables or bypasses one (`--no-verify` is a rule 12 violation).

**Usage budget per milestone.** Unit: USD-equivalent at `config/prices/2026-10-22.toml`, computed
from the token counts Factory reports for the mission's sessions (the usage page or the stream's
`usage.reported`), pasted into `reports/usage/<milestone>.md` at integration (rule 7). Factory
Standard Credits are recorded next to it when visible. The caps below are the owner's planning
numbers, not measurements; this is the first mission and there is no prior run to calibrate on.
At the M0 boundary the orchestrator proposed re-caps for M1–M5 from the measured S and M0 usage
(owner decision 2026-10-22); the owner accepted the table below on 2026-10-23 (F-031). Measured
inputs, pasted from `reports/usage/s.md` and `reports/usage/m0.md`: S 13.21 USD-eq of 40 (15.90 with
round 2); M0 workers, validators and orchestrator subagents 49.05 of 60 (33.28 without the three
subagents); orchestrator own delta 18.94 at the S to M0 boundary and 66.87 in M0 against its cap of 32.

| Milestone | Cap (USD-eq) | Orchestrator (20 %) | Was at approval | Reason for the size |
|---|---|---|---|---|
| S  walking skeleton | 40 | n/a | 40 | Thin slice, few files, mostly scaffolding and one fixture. |
| M0 capture and mapping | 60 | 32 | 60 | Mapper plus golden tests; the capture already exists (§3 M0). |
| M1 board, replay, attach | 110 | 22 | 150 | Largest UI milestone and the Leptos go/no-go; gains the lane table, the golden regeneration and D-023/D-024 (F-028); loses the derived human timeline and the lane-8 UI (the overlay is a field, Gate is one placeholder card). |
| M2 host | 50 | 10 | 80 | RPC adapter plus launch screen; scope unchanged, lower priority. |
| M3 intervention | 110 (was 35; owner, M2 boundary, F-055) | 30 | 60 | Human-overlay semantics over M2 controls; wording only. Re-capped from the M2 record (120.61 USD-eq against 50) and the M3 plan (cycle-template refactor, control core, controls UI, gate cache, usage and integrate targets, e2e harness). |
| M4 branch, gate, fix loop | 200 (was 110, F-055; was 75, F-031; owner, M3 boundary, F-148, D-038) | 30 | 120 | Two adapters (git, GitHub), the demo branch pair, a live PR loop; drops branch-deletion detection, `deploy.completed` and two merge kinds (F-028). |
| M5 time, waste, cost | 140 (was 60, F-055; was 105, F-031; owner, M3 boundary, F-148, D-038) | 30 | 120 | Priority: engineering cycle time redefined, stage times per phase, waste measured as 8 and displayed as 5, both clocks, the Panel. Lowered because M3 and M4 take the money the owner re-assigned. |
| Total | no hard stop (F-055); notify-and-ask line 1350 USD-eq (F-148, D-038) | 30 per milestone | 630 | S 40, M0 60, M1 110, M2 50, M3 110, M4 200, M5 140 sum 710 in caps; the notify-and-ask line for the mission is 1350 USD-eq of the record (below), not a cap that is spent down. History: until the M3 boundary the caps were M4 110 and M5 60 (sum 540) and the line was 950 USD-eq (F-055). |

**Orchestrator cap (owner decision 2026-10-23, F-018, re-set by F-031).** The orchestrator is capped
per milestone at 20 % of that milestone's cap, measured as the delta of its own session's
`tokenUsage` (session `e194b9e3`, not `inclusiveTokenUsage`) priced at the `claude-fable-5.1` row.
M0 was measured from the baseline in `reports/usage/s.md` (input 629 245, output 499 465, cache read
32 674 587, cache write 3 520 336; 83.44 USD-eq mission-to-date): cap 32 (raised from 12 by the
owner on 2026-10-23 after the second pass, F-024). The M0 overrun, 66.87 USD-eq against 32 (209 %),
is accepted as recorded (F-031); there is no retroactive cap change. M1 to M5: 22, 10, 7, 15, 21
USD-eq (were 30, 16, 12, 24, 24). The M1 baseline is the absolute `tokenUsage` recorded in
`reports/usage/m0.md`. Each `reports/usage/<m>.md` shows the delta and records the new absolute
`tokenUsage` as the next baseline.

**Orchestrator cap, retired share (owner decision 2026-10-24, F-040).** The per-milestone 20 % share above is
retired for the milestones after M1: the mission has one orchestrator cap of 320 USD-eq, with the spent-to-date
reported against it at every boundary (`reports/usage/m2.md`), and an M2 orchestrator spend above 30 USD-eq stops the
mission at the M2 boundary before M3 is dispatched. The "Orchestrator (20 %)" column of the table is kept as the
history of the plan. M1 is accepted as recorded (110.38 against 110).

**Caps and stops (owner, M2 boundary, 2026-10-24, F-055).** M2 is accepted as spent, from the record: 120.61 USD-eq
against 50, orchestrator 118.84 against 30 (86.49 of it in 121 re-issued waits), 388.69 orchestrator mission-to-date
against 320; the overruns stay as deviations, nothing rewritten. Caps from M3: M3 110, M4 110, M5 60, and the orchestrator 30
per milestone. **There is no hard stop: stop-and-decide at every boundary, at a cap, and when the record crosses 950
USD-eq.** (Superseded at the M3 boundary, see below: M4 200, M5 140, line 1350.) At any of the three the mission stops before the next dispatch, reports from the record
(`scripts/usage-from-log.sh`, `make usage` from M3), and the owner decides whether and under what caps it continues.
The record at the M2 boundary is 694.66 USD-eq (`reports/usage/m2.md`, reconciliation part 4). The gauge in Mission
Control is the account's plan allowance for the rolling week, not this mission's budget, and is not used (F-067). The
plan's limit preference is set so that at the limit the mission stops on its declared models and never switches
(Extra Usage configured 2026-10-24); a stop is resumed by the owner at the reset, models read back first (F-057).

**Money from M4 (owner, M3 boundary, F-148, D-038).** M3 closed at 216.88 USD-eq with its boundary against 110, and the
mission at 1007.73 against the 950 line. The line moves from 950 to **1350 USD-eq**, all roles; caps **M4 200**, **M5 140**,
the orchestrator **30 per milestone**; stop-and-decide lines under R-055, never walls. By the owner's go for M4
(2026-10-29), **1350 USD-eq is a notify-and-ask line, not a hard stop**: at 1350 mission-to-date, and at a usage-limit stop,
the orchestrator reports the state and the figures and asks whether to proceed; it never ends the mission on its own. The
M4 forecast (162 USD-eq) is a forecast, not a gate; the weekly allowance is the owner's to manage.

At a cap the orchestrator stops dispatching, writes the usage so far to `reports/usage/`, raises
the stall flag and waits for the human (rule 15). Spending beyond a cap is a deviation (rule 9).

**What the human will answer mid-mission.** Only at a milestone boundary or when the stall flag is
up (rule 15): the M1 go/no-go (PRD §12), a budget cap reached, a proposed deviation from this spec
with its reason and cost, a keychain item or GitHub permission the mission cannot create itself
(§3 M4, §5.9), the reference spreadsheet for M5 (B-008), and a red `gate` on the final PR.

**What the human will not answer.** Style, naming, crate choice within §4, test structure, how to
split a milestone into features, which worker does what, or anything already decided here or in
`docs/decisions/`. Those go to the backlog with a reason or into a decision file (rules 13, 14).

## 3. Milestones

Seven milestones on the board of the build: the walking skeleton `S` first (rule 2), then
`M0`–`M5` in PRD §13 order. PRD §13 counts six; the seventh is the skeleton rule 2 requires, so
the PRD count is superseded here (rule 8). A milestone is integrated into the mission branch, never
shipped. The spec itself is not a milestone: it is approved before the mission starts.

**B-010 settled: skeleton before M0, and the capture is this mission.** Rule 2 wants the thinnest
end-to-end slice first; PRD §13 wants M0 to freeze the mapping before M1–M5 are planned in detail.
Both hold because the capture needs no extra mission: the moment the owner runs the §2 command, the
orchestrator's session JSONL is growing under `~/.factory/sessions/`, and the skeleton's workers
and validators add child sessions for every role. `S` replays a snapshot of that file through a
mapper that knows nothing yet (every line is `unknown.event`, counted). `M0` snapshots the same
sessions again at `S` integration, now with all four roles present, freezes it as the fixture,
and drives the unknown count to zero. The optional B-009 pre-capture is not needed; a small
sample-repo mission (PRD §13 M0) is the fallback only if role identification cannot be established
from this mission's own files.

**Common to every milestone.**

- Tests first (rule 1): each feature begins with a red test committed in the worktree; the
  scrutiny validator checks that the red run exists in the worker's log.
- Acceptance commands are the ones in §6 and `make check`; counts are pasted, not estimated.
- Exit needs: `make check` green on the mission branch after integration, `mission-checks` green
  on the push, zero blocking findings open, §9/§10 rows written, README/BACKLOG/decisions index
  updated (rule 16), `reports/usage/<m>.md` and `reports/readiness/<date>-<m>.json` added
  (B-015), and no `unknown.event` on the capture if the milestone adds a source (rule 18).
- Must not touch: `main`, another milestone's worktree, `rust-toolchain.toml` without a decision,
  the shared files (§2), the ruleset on `main`.
- Fixtures: `fixtures/streams/<YYYY-MM-DD>-<slug>.jsonl` is the orchestrator session; its child
  sessions live in `fixtures/streams/<YYYY-MM-DD>-<slug>.children/<session-id>.jsonl` with the
  `.settings.json` beside each. A fixture is committed only after `gitleaks detect --no-git` on the
  directory is clean and a reviewer has read it; raw lines may carry tool output.

### S — Walking skeleton

- **Outcome.** Cargo workspace with `crates/observatory-core`, `crates/observatory-cli`,
  `apps/observatory-app`; the canonical `Event` (PRD appendix A) with `EventKind::Unknown`;
  `observatory-cli replay <file>` reads a session JSONL and prints one line per event and the
  summary `unknown events: N`; the Tauri window starts and shows nine empty lanes with their
  names; `Cargo.lock` committed; `cargo deny` configured against `deny.toml`.
- **Tests green.** `replay_counts_every_line_of_a_capture()`,
  `unknown_event_keeps_raw_type_and_byte_offset()`, `event_round_trips_through_serde()`,
  `lanes_are_nine_in_prd_order()`, `cli_exits_nonzero_on_missing_file()`,
  `every_event_carries_the_mapping_version()`.
- **Acceptance commands.** `make check`; `cargo run -p observatory-cli -- replay
  fixtures/streams/2026-10-22-observatory-v1-s.jsonl` (prints `unknown events: N`, N > 0 allowed);
  `make app-dev` opens the window (screenshot in the validator report).
- **Gate.** `make check`, `mission-checks` green, `make deny` green, coverage reported (not yet
  enforced, §6).
- **Exit criterion.** The skeleton replays its own snapshot end to end from a clean clone with the
  README commands (rule 5); B-016 (`release.yml` checks the workspace version against the tag) is
  wired because `Cargo.toml` now exists; B-005 and B-014 jobs are in `mission-branch.yml`.
- **Must not touch.** No mapping beyond `Unknown`, no store, no metrics, no board interaction.

### M0 — Stream capture and mapping

- **Outcome.** `fixtures/streams/2026-10-22-observatory-v1-m0.jsonl` (+ children) frozen from this
  mission's sessions at `S` integration; `mapping::v1` from raw lines to PRD §7 kinds, selected by
  the Droid version in the fixture header `<capture>.capture.json` (session files carry no Droid
  version; usage, model, tags and child links come from the sibling `<id>.settings.json`, D-012);
  confirmation of where `--tag`, child sessions, roles, feature and milestone ids are recorded
  (PRD §11.1 open items), written to `docs/decisions/` and `library` of the fixture; SQLite store with the pointer index (§5.3);
  `observatory-cli replay` and `inspect <event-id>` (prints the raw line by pointer).
- **Tests green.** Golden tests `mapping_v1_matches_golden_lane_events()` (fixture → expected
  `.events.json`), `every_prd_kind_present_in_capture_is_mapped()`, `unknown_count_is_zero_on_m0_capture()`,
  `child_sessions_attach_to_their_parent()`, `roles_resolve_for_orchestrator_worker_and_both_validators()`,
  `store_pointer_reads_back_the_exact_raw_line()`, `mapping_version_is_selected_by_droid_version()` (reads `droid_version` from the fixture header).
- **Acceptance commands.** `make replay-fixtures`; `cargo run -p observatory-cli -- replay
  fixtures/streams/2026-10-22-observatory-v1-m0.jsonl` ends with `unknown events: 0`;
  `cargo test -p observatory-core --test mapping`.
- **Gate.** All of the common gate plus `unknown events: 0` on the M0 capture (hard).
- **Exit criterion.** The mapping is frozen (`mapping::v1`, versioned by Droid 0.247.x); PRD §7
  kinds that do not occur in the capture are listed in `docs/decisions/D-0NN-mapping-v1.md` as
  "defined, not observed". **This is the gate for planning M1–M5 in detail** (PRD §13).
- **Must not touch.** No UI beyond the empty lanes; no git/GitHub adapters; no metrics.

### M1 — Board in replay and attach

- **Outcome.** Board with the seven phase lanes (Plan, Contract, Build, Scrutiny, User testing,
  Integrate, Gate), the Panel area and the human overlay markers; mapping v1's lane table re-opened
  for the phases (§5.2, D-024), the golden regenerated and its hash refrozen at this integration
  (§10). Milestone cards positioned by current phase (FR-B1), worker
  sub-lanes with the per-worker/per-feature toggle (FR-B2), filters (FR-B5), replay scrubber
  (FR-B6), unmapped tray (FR-B7); Details with the Timeline tab (FR-D1) opening raw lines;
  agent panel (FR-A1..A4) with `model.changed` and `context.compacted` markers. Attach: `notify`
  watcher on `[ingest].factory_sessions_dir` paired with the `jsonl_poll_ms` poll, startup scan of
  files already growing, tail on growth, render within 1 s, name by `--tag` or session id
  (FR-L1); replay is the same adapter at end of file (FR-L2). Read-only scrub pass before display
  (B-006) for the obvious token shapes, read-only rendering regardless. Mission library listing
  past missions with headline metrics (FR-L4; headline metrics arrive with M5, until then the list
  shows name, date, event count).
- **Tests green.** `card_sits_in_lane_of_most_recent_open_activity()`, `scrubber_replays_to_a_timestamp()`,
  `unmapped_tray_count_equals_unknown_events()`, `watcher_sees_a_new_session_file_within_poll_interval()`,
  `tail_renders_appended_lines_in_order()`, `attach_latency_under_one_second_on_fixture_replayed_live()`
  (a test harness appends the fixture line by line with timestamps), `name_prefers_tag_over_session_id()`,
  `scrub_masks_known_token_shapes_without_altering_the_store()`,
  `lanes_are_the_seven_phases_in_order()`, `gate_command_lands_in_integrate_for_every_role()`,
  `human_facing_kinds_carry_the_human_overlay()`.
- **Acceptance commands.** `make check`; `trunk serve --port 3141` (browser host of the UI for the
  validators, §6); `make app-dev`; the seven phase lanes render with the M0 fixture, with per-lane
  counts equal to the CLI replay and to the regenerated golden (expected at the M1 golden, from `jq`
  over the M0 events, after D-032: Plan 437, Contract 69, Build 424, Scrutiny 93, User testing 122, Integrate 134,
  Gate 0, and 70 human overlays; the committed golden is authoritative); a mission started in a
  terminal appears on the board, each event within 1 s, measured by the attach-latency test and by
  the validator's timestamps.
- **Gate.** Common gate plus the **Leptos go/no-go** (PRD §12): the board renders the M0 capture
  (seven phase lanes and the Panel, phase counts equal to the CLI, cards positioned correctly,
  Details timeline open) with at most 3 worker sessions
  of rework after the first scrutiny pass, and the Trunk/WASM build succeeds from a clean checkout
  with the README command. Fail → switch the UI to Tauri + TypeScript for M2 onward, core and the
  Tauri command boundary unchanged, decision file, owner approval.
- **Exit criterion.** Dogfooding starts: the rest of this mission is watched by attaching to its
  own session files (PRD §13).
- **Must not touch.** No hosting, no control channel, no GitHub.

### M2 — Host

- **Outcome.** Launch screen (FR-L3): repo, mission file, mission branch name, models and
  reasoning effort per role, autonomy, optional budget; spawn `droid exec --mission
  --input-format stream-jsonrpc --output-format stream-jsonrpc`; `RpcSource` adapter; pause,
  message, answer over the input channel; live output per agent (FR-A3).
- **Tests green.** `rpc_events_map_to_the_same_lane_events_as_the_session_file()`,
  `pause_request_is_written_as_jsonrpc_and_acknowledged()`, `answer_routes_to_the_open_question_id()`,
  `host_and_attach_of_the_same_mission_agree_on_event_count()`, `child_process_is_reaped_on_window_close()`.
- **Acceptance commands.** `make check`; from the app launch a short mission on a scratch
  directory inside the worktree; events within 1 s; pause/message/answer reach the orchestrator and
  its reaction appears; the same mission attached from its JSONL shows the same lane events.
- **Gate.** Common gate; zero `unknown.event` on the hosted capture (new source, rule 18).
- **Exit criterion.** PRD §13 M2 validation satisfied with the pasted event counts.
- **Must not touch.** Human-overlay semantics beyond logging the three controls (M3), GitHub.
- **As built (M2 integration, 2026-10-24).** The launch command is the D-034 recipe (a spawned `droid exec` with the
  stream-jsonrpc input and output formats; the mission proposal and the models go in over the channel: `droid.initialize_session`
  and `droid.update_session_settings`), not the literal `--mission` line of the first contract (F-049). Hosted and attached views
  agree except `usage.reported` and `plan.revised` (F-050).

### M3 — Intervention

- **Outcome.** Every control is a human-overlay event (`overlay = human`) with the human as actor
  (FR-H1..H4: pause/resume, message, answer and approve/deny, mark complete/skipped); stall flag at
  `[intervention].stall_threshold_min` with the matching runbook prompt (FR-H5, `docs/runbooks/`);
  no nudging under threshold (FR-H6).
- **Tests green.** `each_control_emits_one_human_overlay_event_with_actor_human()`,
  `stall_flag_raises_after_threshold_of_no_active_time()`, `stall_flag_suggests_the_matching_runbook()`,
  `no_alert_for_waits_under_threshold()`, `mark_skipped_relays_as_an_instruction_to_the_orchestrator()`.
- **Acceptance commands.** `make check`; each action against a hosted mission produces the human-overlay
  event and the orchestrator reacts (pasted event ids).
- **Gate.** Common gate.
- **Exit criterion.** `docs/runbooks/stalled-mission.md` app steps replace the CLI steps (B-012
  "revise at M3").
- **Must not touch.** GitHub, metrics.

### M4 — Mission branch, gate, and fix loop

- **Outcome.** `GitSource`: mission branch, milestone worktrees, integration commits, fast-check
  runs → Integrate (6; FR-G1, FR-G2). `GitHubSource` behind the `Forge` trait: PR detection for
  `<mission branch>` → `<target branch>` (target parameterised, not hard-coded `main`), check runs,
  required checks, reviews, rulesets, attestations (FR-G3); merge policy with rule attribution and
  the `[github].merge_policy` override (FR-G4); gate group and badge in the Gate lane (7; FR-B3);
  Merge button with confirmation naming PR and target (FR-G5); `handed_to_cd { merge_sha, pr_number,
  attestation_ids }` on detection, role human or external by the merger's login (FR-G6); CI red →
  `fix` cycle on the affected card(s), same PR, return arc, counted as rework (FR-B4, FR-G7);
  completion at `handed_to_cd`, the cycle's last event (FR-G8 narrowed: nothing after the merge is
  observed). Cut by F-028 (owner answer 1c): `deploy.completed` and branch-deletion detection; the
  `demo/*` cleanup stays a `gh` step of the run, not an observed event.
- **Test target (B-007 decided, entry 19).** A demo branch pair in this repository: long-lived
  `demo/base` created by the orchestrator from the mission branch; a ruleset `demo-gate` on
  `refs/heads/demo/base` (PR required, required check `demo-gate`, no force push); PRs from
  `demo/*` into `demo/base`; `.github/workflows/demo-gate.yml` on `pull_request` to `demo/base`
  (types `opened, synchronize, labeled, unlabeled`) that fails when the label `demo:red` is present
  and passes otherwise. Deterministic red and green, no `main` involvement, no second repository.
  The ruleset is created with `gh api` under the owner's token; if the token lacks admin, the
  owner creates it at the M3→M4 boundary (a human input, §2). Demo branches `demo/*` are deleted
  after each run; `demo/base` stays as test infrastructure and is listed in the README.
- **Tests green.** `pr_for_mission_branch_is_detected_within_poll_interval()`,
  `red_required_check_opens_a_fix_cycle_on_the_affected_card()`, `fix_cycle_keeps_the_same_pr()`,
  `merge_button_shows_only_when_policy_is_human_and_checks_are_green()`, `rule_attribution_names_the_ruleset()`,
  `policy_merged_detected_without_human_action()` (asserts `handed_to_cd.role == External`),
  `handed_to_cd_is_emitted_once_with_merge_sha_pr_and_attestations()`,
  `gate_command_runs_are_lane6_events_attributed_to_the_milestone()`,
  `target_branch_is_read_from_config_not_hardcoded()`.
- **Acceptance commands.** `make check`; `demo/red-1` PR with `demo:red` → check fails → card
  loops from Gate (7) to Build (3) with cycle count 1; label removed → check passes → Merge visible when policy
  allows → with the read-only token (D-014) the app surfaces the `403` cleanly (message naming the
  PR, the target and the missing permission; no crash, no retry storm) → the owner merges (UI or
  API) → badge `handed to CD` from the poll, naming the merge sha, the PR number, the attestation
  ids and the merger (human or policy), with the Gate header naming both clocks → `demo/red-1` is
  then deleted by `gh` as cleanup, not observed. All run ids and PR numbers pasted.
  The question of a write-capable token for the Merge button stays at the M3→M4 boundary.
- **Gate.** Common gate; zero `unknown.event` from the git and GitHub sources (rule 18).
- **Exit criterion.** PRD §13 M4 validation satisfied on the demo pair; the decision file for the
  demo branch pair is written (the PRD §13 M4 row, §14 risk row and BACKLOG B-007 are already
  reworded in the spec PR, entry 19).
- **Must not touch.** The `main` ruleset, `main` itself, metrics.

### M5 — Time, waste, cost

- **Outcome.** Time-state engine (PRD §10.1), rework windows (§10.2), mission overhead (§10.3),
  engineering cycle time (`feature.defined` → `handed_to_cd`, else the feature's first
  `task.started` with the fallback count reported; stage times per phase 1–7), waste measured by the
  eight §5.6 causes and displayed as the owner's five groups, bottleneck, fix loops (§10.4), tokens
  and cost with the dated price table and credits when exposed (§10.5), budget meter (FR-M6), model
  comparison (FR-M7), CSV/JSON export, Details tabs Cost and waste (FR-D6), Review notes (FR-D2),
  Validation criteria (FR-D3), Commits and integration (FR-D4), Gate (FR-D5); the Panel (above the
  seven lanes, not a lane) with SVG charts (stacked bars by the five groups, timeline, sparkline),
  the cycle time, the cost per role and the drift against the declared `[mission]` models; both
  clocks named (engineering cycle time; lead time to production, the delivery platform's, not
  computed); Grafana export of the §5.7 series behind `[telemetry].enabled`.
- **Oracle (B-008).** `reports/reference/2026-10-22-observatory-v1-m0.csv`, produced by the owner
  by hand from the M0 capture before M5 starts: per feature and per milestone, cycle seconds,
  first-pass active seconds, waste seconds by cause, rework windows, tokens by class, cost at
  `config/prices/2026-10-22.toml`. Rounding tolerance: 1 s on durations, 1 token, 0.01 USD.
  Missing at the M4→M5 boundary = stall flag, human input (§2).
- **Tests green.** `time_states_partition_every_instant_exactly_once()`,
  `rework_window_closes_on_resolution()`, `overlapping_windows_merge_on_one_feature()`,
  `unresolved_window_is_reported_at_mission_end()`, `first_pass_excludes_rework_windows()`,
  `overhead_is_reported_separately_unless_apportionment_is_equal()`,
  `metrics_match_reference_spreadsheet_within_tolerance()`, `cost_names_the_price_table_date()`,
  `exported_series_count_is_under_budget_on_worst_case_labels()`, `labels_outside_allowed_set_are_rejected()`,
  `telemetry_stays_off_without_keychain_item_and_ui_says_so()`.
- **Acceptance commands.** `make check`; `cargo run -p observatory-cli -- metrics
  fixtures/streams/2026-10-22-observatory-v1-m0.jsonl --csv` diffed against the reference within
  tolerance; with the keychain item present and `[telemetry].enabled = true`, series appear in the
  owner's Grafana stack with only the allowed labels (screenshot pasted).
- **Gate.** Common gate; oracle test green (hard).
- **Exit criterion.** All six "must prove" items of §1 hold; the orchestrator opens the single PR
  `mission/observatory-v1` → `main`, the `gate` runs, the human merges, the branch dies.
- **Must not touch.** Anything that changes the mapping frozen at the M1 integration (F-028) or the M0 fixture.

## 4. Versions

All checked on 2026-10-22 against the official source named (rule 3). "Pinned" means in the
repository today; "candidate" means the version a worker pins on the day it adds the dependency,
re-checking the source then (rule 3) and running `make deny`.

| Component | Version | Source | Status | Verified |
|---|---|---|---|---|
| Rust toolchain (rustfmt, clippy, `wasm32-unknown-unknown`) | 1.99.1 | static.rust-lang.org channel-rust-stable (channel 2026-10-15) | pinned `rust-toolchain.toml` | 2026-10-22 |
| `llvm-tools-preview` component (coverage) | with 1.99.1 | same manifest | add in S (decision file, §6) | 2026-10-22 |
| `tauri` (v2) | 2.12.5 | crates.io | candidate | 2026-10-22 |
| `tauri-cli` | 2.12.4 | crates.io | candidate; unpinned in `bundle.yml` today, pin in S | 2026-10-22 |
| `leptos` (`csr`) | 0.8.22 | crates.io | candidate | 2026-10-22 |
| `trunk` | 0.22.0 | crates.io | candidate; pin in `bundle.yml` in S | 2026-10-22 |
| `rusqlite` (`bundled`) | verify on the day | crates.io | candidate; chosen in §5.3 | — |
| `notify` | 8.2.0 | crates.io | candidate | 2026-10-22 |
| `serde` / `serde_json` | 1.0.229 / verify | crates.io | candidate | 2026-10-22 |
| `tokio` (app and CLI only; core stays sync) | verify on the day | crates.io | candidate | — |
| `keyring` (macOS keychain) | verify on the day | crates.io | candidate; §5.9 | — |
| `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` | 0.33.1 | crates.io | candidate (M5) | 2026-10-22 |
| `tracing-opentelemetry` | 0.34.0 | crates.io | candidate (M5) | 2026-10-22 |
| `cargo-llvm-cov` | 0.9.1 | crates.io | candidate (S, §6) | 2026-10-22 |
| `cargo-machete` | 0.9.2 | crates.io | candidate (S, §7) | 2026-10-22 |
| `cargo-deny` | 0.20.2 | GitHub releases | pinned `.devcontainer/post-create.sh`, CI action 2.1.1 | 2026-10-22 |
| actionlint / lychee / gitleaks / markdownlint-cli2 | 1.7.12 / 0.24.2 / 8.30.1 / 0.24.0 | GitHub releases, npm | pinned `.devcontainer/post-create.sh` | 2026-10-22 |
| Semgrep image | 1.182.0 | ghcr | pinned `gate.yml` | 2026-10-22 |
| GitHub Actions (checkout 7.0.1, rust-cache 2.9.3, CodeQL 4.39.1, Trivy 0.36.0, Checkov 12, SBOM 0.24.3, install-action 2.87.30, attest-build-provenance 4.2.2, cosign-installer 4.1.2, upload/download-artifact 7.0.2/8.0.2, markdownlint 24.2.0, lychee 2.9.0, gitleaks 3.0.0) | as listed | GitHub releases | pinned by commit SHA in `.github/workflows/` (D-003) | 2026-10-22 |
| Devcontainer image | `devcontainers/rust:1-1-bookworm@sha256:fc57fb…4759` | mcr.microsoft.com | pinned by digest | 2026-10-22 |
| Droid CLI | 0.247.0 | `droid --version` | the version the mapping is frozen against (`mapping::v1`) | 2026-10-22 |

Runner labels `ubuntu-latest` / `macos-latest` float by design (GitHub-hosted); the Tauri bundle
job records the runner image version in its log (rule 7). Renovate (D-005) proposes updates; a
version bump on the mission branch is a worker change with `make deny` green, not a decision.

## 5. Architecture

Each decision carries its reason. Numbers come from the PRD and `config/observatory.example.toml`
unless a reason says otherwise.

### 5.1 Crates and interfaces

| Crate / app | Responsibility | Public interface | Reason |
|---|---|---|---|
| `crates/observatory-core` | ingest, model, mapping, store, metrics, control, telemetry; **no UI dependency, sync, no tokio** | `Source` trait, `Event`, `Store`, `Metrics`, `Control`, `Config` | Testable from fixtures alone; every milestone's logic lives here so the UI can be swapped (PRD §12 go/no-go). |
| `crates/observatory-cli` | headless `replay`, `inspect`, `attach --watch`, `metrics --csv` over core | `observatory-cli <cmd>` | Rule 2: the first thing that works; rule 5: what CI runs; the validators' oracle for every milestone. |
| `apps/observatory-app` | Tauri 2 shell (`src-tauri`) + Leptos CSR UI (`src`) | Tauri commands and events, serde types imported from core | One language, no bindings drift (PRD §12). Commands are the only boundary the UI crosses, so a TypeScript UI could replace it without touching core. |

Tauri commands (M1 onward): `open_replay(path)`, `list_sessions()`, `attach(session_id)`,
`event_raw(ptr)`, `host_launch(cfg)`, `host_control(cmd)`, `metrics(scope)`, `gate(mission)`,
`merge(pr, target)`, `telemetry_status()`. Events from shell to UI: `events_appended(batch)`,
`session_discovered`, `stall_flag`, `gate_changed`. Batches flush every 100 ms (reason: well under
the 1 s requirement, avoids one render per line).

### 5.2 Canonical event and lane mapping

`Event`, `ReworkWindow`, `Lane`, `Role`, `EventKind` as in PRD appendix A, except `Lane` (the seven
phases below), the added `Event.overlay` and the kind set (`HandedToCd` replaces three PRD kinds;
F-028, D-024); `EventKind` has one variant per PRD §7 kind in `CamelCase` plus `Unknown { raw_type }`. `source: SourcePtr::Jsonl {
path, byte_offset, byte_len }` (appendix A has `path, byte_offset`; `byte_len` is added so a line
can be read without scanning for the newline) is the only thing the store keeps of a raw line
(PRD §11.2: the lane is a view). `mapping_version: MappingVersion` is a second addition to
appendix A: every `Event` names the mapper that produced it, so the first Droid upgrade shows as a
rising unknown-event count against a known version, never as a silently broken board (owner,
entry 23). Mapping is a module `mapping::v1` with a `MappingVersion`
selected from the Droid version in the fixture header `<capture>.capture.json` (`droid_version`,
written from `droid --version` at capture; a live attach records it in the `captures` row), because
the session files carry no Droid version (D-012). Usage (`tokenUsage`, `inclusiveTokenUsage`,
`childInclusiveTokenUsageBySessionId`), model, reasoning effort, tags (objects `{name, metadata}`)
and child links are read from the sibling `<id>.settings.json`. Unknown Droid versions use the
latest mapping and raise a one-line warning (reason: an old capture must keep replaying, rule 18
covers the rest). A mapping version is frozen at M0 and never edited, with one exception: its lane
column was re-opened once, before any consumer existed, at the M0 to M1 boundary (F-028, D-024,
below). A later Droid release gets `mapping::v2` and its own fixture. The golden file
`fixtures/streams/<capture>.events.json` is the contract of a version.

The Droid CLI usage log `~/.factory/logs/droid-log-single.log` (rotated daily as `droid-log-single.log.<date>`) is a second input source beside the session files, read by `ingest::usage_log` (tailed in attach at M1, replayed in history, shown in the usage panel at M5) because the per-turn token counts exist nowhere else (F-034, D-029).

**Lane table (F-028, D-024).** Lane assignment is part of the mapping. v1's kinds, pointers, role
resolution and sidecar are as frozen at M0 (D-023); only its lane column changes, at the M1
integration, to the seven mission phases, one axis: 1 Plan, 2 Contract, 3 Build, 4 Scrutiny,
5 User testing, 6 Integrate, 7 Gate (serde `plan`, `contract`, `build`, `scrutiny`, `user_testing`,
`integrate`, `gate`). Roles are colour and attribution. Human and commander events carry
`overlay: Option<Overlay>` (`human`, `commander`), never a lane. The old lane 9 is the Panel (time,
waste, cost, drift: an area, not a lane); the old lane 8 folds into Gate. `handed_to_cd { merge_sha,
pr_number, attestation_ids }` replaces `pr.merged_by_human`, `pr.merged_by_policy` and
`deploy.completed` (59 kinds). The rule in evaluation order and the authoritative v1 lane table
(copied from `architecture.md` §5 and D-024):

**Lane rule in evaluation order (D-024, `mapping::v1::Rec::push`):** lane is a function of `(kind, role, session
facts the mapper already parses)`; nothing new is read.

1. Fixed-phase kinds (independent of role): the rows marked `fixed` in the table below. `gate_command.*` is
   Integrate for every role; `pr.opened` is Integrate (owner: "hand-over; PR opened"); the PR's checks and
   attestations are Gate.
2. `commit.created` by the Orchestrator → Integrate (its `file.touched` stays Plan).
3. Integration session → Integrate for every role-neutral kind: a session whose `feature_id` matches
   `^(m[0-9]+|final)-integrate-` (already parsed by `mapping/v1/ids.rs`). M0: `m0-integrate-s`, 3 sessions, 49 events.
4. Contract session → Contract for every role-neutral kind: a child whose spawning `Task.description` (the same
   `tool_use` part `roles.rs` reads for `subagent_type`) matches `^Contract\b|[Vv]alidation contract`
   (`SessionMeta.contract: bool`, set by `roles.rs`; role stays Worker). Owner answer 1a (2026-10-23): accepted as
   the v1 heuristic, recorded in the mapping decision. M0: 9 Task calls, 9 sessions, 69 events; without it Contract is
   empty on M0 (Build 493).
5. Role home for everything else: `Lane::home(role)`.

Human-overlay events take a fixed lane by kind in v1 (owner answer 1d): `intent.*`, `question.*`, `approval.*`,
`mission.paused/resumed` → Plan; `permission.*` → the requesting session's home; `review.outcome` → Gate;
`approval.given` with `human_actions.kind = merge` → Gate. Placing a human event in "the feature's phase at that
timestamp" needs cross-session state the stateless `map_record` cannot see; it is a v2 note in D-024, not work.

**Authoritative v1 lane table (D-024).** Columns: PRD lane before F-028; phase; overlay; count on the
M0 golden expected at the M1 regeneration (`jq` over `fixtures/streams/2026-10-22-observatory-v1-m0.events.json`,
1279 events; the committed M1 golden is authoritative); reason. `R` = role home (rule 5, subject to rules 3–4).

| Kind | PRD lane | Phase | Overlay | M0 | Reason |
|---|---|---|---|---|---|
| intent.created | 1 Intent | 1 Plan fixed | Human | 1 | the mission prompt; role stays Orchestrator (D-023) |
| intent.amended | 1 Intent | 1 Plan fixed | Human | 44 | typed root prompt |
| question.asked | 1 Intent | 1 Plan fixed | Human | 13 | pulls the human in; starts waiting-on-human |
| intent.answered | 1 Intent | 1 Plan fixed | Human | 12 | the human's answer |
| spec.versioned | 1 Intent | 2 Contract fixed | none | 0 | a spec version is the contract's artefact |
| plan.created | 2 Orch | 1 Plan fixed | none | 1 | decomposition |
| feature.defined | 2 Orch | 1 Plan fixed | none | 6 | cycle start; all 6 on M0 come from the scrutiny validator session `6eb88099` at 17:35:44Z, none for `m0-*` (see the clocks in §1) |
| milestone.defined | 2 Orch | 1 Plan fixed | none | 7 | decomposition |
| task.assigned | 2 Orch | R | none | 40 | orchestrator 27 → Plan; validators' own spawns 6 + 7 → Scrutiny / User testing |
| task.skipped | 2 Orch | R | none | 18 | child outcome `cancelled`; child's role decides (10 Build, 1 Scrutiny, 4 UT, 3 Integrate) |
| plan.revised | 2 Orch | 1 Plan fixed | none | 44 | replans |
| escalation.raised | 2 Orch | 1 Plan fixed | none | 14 | root error / usage exhausted; a stall signal, not a human act |
| milestone.completed | 2 Orch | 1 Plan fixed | none | 0 | not observed |
| mission.paused | 2 Orch | 1 Plan fixed | Human | 0 | root turn cancelled = the human; M3 Pause too |
| mission.resumed | 2 Orch | 1 Plan fixed | Human | 0 | M3 Resume |
| mission.completed | 2 Orch | 1 Plan fixed | none | 0 | not observed |
| context.compacted | 2 Orch | R | none | 11 | orchestrator 10 → Plan, worker 1 → Contract (rule 4) |
| subagent.spawned | 3 Workers | R | none | 55 | child's role: 30 Build, 9 Contract, 7 Scrutiny, 9 UT |
| task.started | 3 Workers | R | none | 55 | same split as spawned (3 Integrate) |
| file.touched | 3 Workers | R | none | 452 | orchestrator 235 → Plan; worker 187 → 152 Build, 23 Contract, 12 Integrate; validators 12 / 18 |
| tests.run | 3 Workers | R | none | 83 | worker 60 → 54 Build + 6 Integrate; orchestrator 6 → Plan; scrutiny 4; UT 13 |
| commit.created | 3 Workers | R, rule 2 | none | 73 | worker 48 → 43 Build + 5 Integrate; orchestrator 25 → Integrate |
| task.done | 3 Workers | R | none | 47 | child's role |
| subagent.finished | 3 Workers | R | none | 55 | child's role |
| tool.error | 3 Workers | R | none | 59 | orchestrator 16 → Plan; worker 38 → 34 Build + 4 Integrate; validators 1 / 4 |
| permission.requested | 3 Workers | R | Human | 0 | asks the human (attached: terminal; hosted: M3 Approve/Deny) |
| permission.granted | 3 Workers | R | Human | 0 | the human's decision |
| permission.denied | 3 Workers | R | Human | 0 | the human's decision |
| hook.blocked | 3 Workers | R | none | 2 | orchestrator 2 → Plan; counted under retry |
| usage.reported | 3 Workers | R | none | 56 | lane = session's home; the Panel reads them from every lane |
| model.changed | 3 Workers | R | none | 4 | orchestrator 4 → Plan; drift input for the Panel |
| finding.blocking | 4 Scrutiny | 4 Scrutiny fixed | none | 0 | |
| finding.non_blocking | 4 Scrutiny | 4 Scrutiny fixed | none | 24 | |
| rework.requested | 4 Scrutiny | 4 Scrutiny fixed | none | 0 | |
| rework.resolved | 4 Scrutiny | 4 Scrutiny fixed | none | 0 | |
| scrutiny.passed | 4 Scrutiny | 4 Scrutiny fixed | none | 5 | |
| qa.scenario.passed | 5 UT | 5 User testing fixed | none | 26 | |
| qa.scenario.failed | 5 UT | 5 User testing fixed | none | 0 | |
| qa.evidence | 5 UT | 5 User testing fixed | none | 2 | |
| integration.merged | 6 Integration | 6 Integrate fixed | none | 0 | `GitSource`, M4 |
| integration.conflict | 6 Integration | 6 Integrate fixed | none | 0 | |
| integration.conflict_resolved | 6 Integration | 6 Integrate fixed | none | 0 | |
| gate_command.run | 6 Integration | 6 Integrate fixed | none | 35 | every role (owner: "gate command") |
| gate_command.passed | 6 Integration | 6 Integrate fixed | none | 34 | |
| gate_command.failed | 6 Integration | 6 Integrate fixed | none | 1 | |
| approval.requested | 7 Human | 1 Plan fixed | Human | 0 | M3 controls address the mission |
| approval.given | 7 Human | 1 Plan fixed (7 Gate when `human_actions.kind = merge`) | Human | 0 | M3 controls; the M4 Merge press is `approval.given` + `human_actions.kind = 'merge'` (replaces `pr.merged_by_human` as the I8 intent row) |
| question.answered | 7 Human | 1 Plan fixed | Human | 0 | M3 Answer |
| review.outcome | 7 Human | 7 Gate fixed | Human | 0 | review of the outcome at the PR |
| ~~pr.merged_by_human~~ | 7 Human | **removed** | | 0 | replaced by `handed_to_cd` with `role = Human` |
| pr.opened | 8 Outside | 6 Integrate fixed | none | 0 | owner: "hand-over; PR opened" |
| pr.review_requested | 8 Outside | 7 Gate fixed | none | 0 | |
| check.started | 8 Outside | 7 Gate fixed | none | 0 | PR checks (`gate` workflow); mission-branch fast checks stay `gate_command.*` in Integrate |
| check.passed | 8 Outside | 7 Gate fixed | none | 0 | closes a `gate` rework window |
| check.failed | 8 Outside | 7 Gate fixed | none | 0 | opens the fix cycle (rework) |
| check.required_missing | 8 Outside | 7 Gate fixed | none | 0 | |
| ruleset.blocked | 8 Outside | 7 Gate fixed | none | 0 | |
| attestation.recorded | 8 Outside | 7 Gate fixed | none | 0 | its id joins `handed_to_cd.attestation_ids` |
| attestation.verified | 8 Outside | 7 Gate fixed | none | 0 | |
| ~~pr.merged_by_policy~~ | 8 Outside | **removed** (owner answer 1b) | | 0 | replaced by `handed_to_cd` with `role = External`: one final event, merger attributed by role |
| ~~deploy.completed~~ | 8 Outside | **removed** (owner answer 1c) | | 0 | after the merge; the delivery platform's record |
| **handed_to_cd** (new) | — | 7 Gate fixed | none (role Human or External) | 0 | the cycle's last event: `merge_sha`, `pr_number`, `attestation_ids`; emitted once per PR when the required checks are green and `state = merged` is observed |

Expected phase counts on the M0 golden after the change (state as "at the M1 golden"; `jq` of the committed
golden is authoritative; after D-032): Plan 437 / Contract 69 / Build 424 / Scrutiny 93 / User testing 122 / Integrate 134 /
Gate 0 (sum 1279); 70 human overlays (1 created + 44 amended + 12 answered + 13 asked), all in Plan. Serde
names `scrutiny` and `user_testing` are unchanged, so 57 golden events keep a byte-identical `lane`; the other
1222 change string; 70 gain `"overlay":"human"`; ids, offsets, `at` and order are untouched. 7 Gate holds nothing on
M0 and cannot from a session capture (PRD §13); until M4 the Gate lane header card shows `gate: pending (no PR)`,
the mission branch, the last `gate_command.*` verdict from Integrate (M0: `gate_command.passed` after 35 runs,
1 failed) and the two clock labels of §1 with no value. Kind count: 59 (30 observed on M0, 29 defined-not-observed;
D-023 amended by D-024).

### 5.3 Store schema

SQLite via `rusqlite` with the `bundled` feature (reason: core is sync, a desktop index needs no
async driver, and bundling removes the system-library dependency from the clean-clone rule).
File: `~/.droid-observatory/observatory.sqlite` (schema version table, migrations forward-only).

| Table | Columns (keys) | Note |
|---|---|---|
| `sessions` | `id`, `path`, `droid_version`, `tag`, `role`, `parent_id`, `mission_id`, `started_at`, `ended_at` | One row per JSONL file; `tag` confirmed in M0 |
| `events` | `id`, `session_id`, `mission_id`, `milestone_id`, `feature_id`, `lane` (1..7), `overlay` (nullable: `human`, `commander`), `kind`, `raw_type`, `mapping_version`, `role`, `model`, `at`, `duration_ms`, `path`, `byte_offset`, `byte_len` | Pointer, never payload |
| `usage` | `event_id`, `input`, `output`, `cache_read`, `cache_write`, `credits` | From `usage.reported` |
| `missions`, `milestones`, `features` | ids, names, `defined_at`, `completed_at`, `state` | Board model |
| `rework_windows` | `feature_id`, `opened_by`, `closed_by`, `cause` | PRD §10.2 |
| `prs`, `checks`, `attestations`, `reviews` | GitHub ids, state, `observed_at`, `url` | Gate (7); `handed_to_cd` rows store `sha` = merge sha and `url` = the PR URL |
| `human_actions` | `id`, `kind`, `target`, `at`, `payload_ptr` | Human overlay |
| `prices` | `effective`, `model`, `class`, `per_m`, `source` | Loaded from the dated TOML |
| `mapping_versions` | `version`, `droid_version_range`, `frozen_at` | §5.2 |

`0001_init.sql` is amended in place for F-028 (lane `CHECK (lane BETWEEN 1 AND 7)`, nullable `overlay`
after `lane`; 23 columns), because no persistent store exists yet; forward-only migrations start with
the first shipped store (the M1 integration).

Raw lines are read on demand from `path`+`byte_offset`; if the file moved, the UI says so and
shows the pointer (reason: never copy raw lines, PRD §11.2).

### 5.4 Ingest adapters behind traits

```rust
pub trait Source { fn poll(&mut self) -> Result<Vec<RawRecord>>; fn kind(&self) -> SourceKind; }
pub trait Forge  { fn pr(&self, branch: &str, target: &str) -> Result<Option<Pr>>;
                   fn checks(&self, pr: &Pr) -> Result<Vec<Check>>; fn policy(&self, target: &str) -> Result<MergePolicy>;
                   fn merge(&self, pr: &Pr, method: MergeMethod) -> Result<Merged>; /* attestations, reviews */ }
```

`JsonlSource` (attach; replay = the same source at end of file with a clock the scrubber drives),
`RpcSource` (host, M2), `GitSource` (M4), `GitHubSource: Forge` (M4). Reason: one adapter for
attach and replay removes a whole class of "works in replay, not live" bugs (D-008); the `Forge`
trait is the seam for other forges later (PRD §5 non-goal, §11.2).

### 5.5 Config keys with defaults

Everything in `config/observatory.example.toml` as committed (`[mission]`, `[ingest]`, `[github]`,
`[intervention]`, `[metrics]`, `[telemetry]`), with these additions the mission makes, each with
its reason in the file (rule 4):

| Key | Default | Reason |
|---|---|---|
| `[ingest].max_watched_sessions` | 64 | Bounds file handles and tail threads on a laptop; beyond it the newest are watched and the UI says how many are not. |
| `[ingest].max_session_file_mb` | 512 | Larger files are indexed but tail reads warn; protects memory on runaway sessions. |
| `[ingest].tail_chunk_kb` | 1024 | Read size per tail step; one chunk per poll keeps latency under 1 s at typical mission output rates. |
| `[ui].event_batch_ms` | 100 | Shell→UI flush interval (§5.1). |
| `[ui].dev_port` | 3141 | `trunk serve` port for the browser host used by validators (§6); chosen outside common dev ranges, confirmed free at planning. |
| `[store].path` | `~/.droid-observatory/observatory.sqlite` | One index per user; raw lines stay where Droid wrote them. |
| `[telemetry].max_milestone_labels` | 8 | 7 named milestones of this mission plus one `overflow` value; bounds the `milestone` label for any observed mission (§5.7). |
| `[telemetry].mission_switch_min_interval_min` | 30 | The exported mission changes at most twice an hour, so two missions' series never count as active together (§5.7). |
| `[telemetry].buckets.mission_cycle_seconds` | `[3600, 7200, 14400, 28800, 86400, 172800]` | Missions run hours to days. |
| `[telemetry].buckets.feature_cycle_seconds` | `[600, 1800, 3600, 7200, 14400]` | Features run minutes to hours. |

### 5.6 Metric definitions

By reference, not restated: time states PRD §10.1, rework windows §10.2, mission overhead §10.3,
cycle/waste/bottleneck/fix loops §10.4, cost §10.5. Closed cause lists used by the engine and the
exporter: **waste causes** `waiting_human_question`, `waiting_human_permission`, `waiting_merge`,
`waiting_validation`, `rework`, `retry_failure`, `idle_queued`, `idle_unscheduled` (8);
**rework causes** `scrutiny`, `user_testing`, `integration`, `gate` (4). Waste is measured by the
eight and displayed grouped as five (F-028): waiting on human (`waiting_human_question`,
`waiting_human_permission`, `waiting_merge`), waiting on validation (`waiting_validation`), rework
(`rework`), retry (`retry_failure`), idle (`idle_queued`, `idle_unscheduled`). The engine, the CSV
and the D-010 series keep the eight; the Panel's stacked bars show the five and the Details Cost and
waste tab the eight. A red PR check opens a `gate` rework window and reopens the mission as a `fix`
cycle; time inside it is `rework`, waiting for the re-run is `waiting_validation`. Mission overhead is a
separate line in the app and is not exported (reason: it belongs to no card and D-010 has no label
for it). The CSV export has one row per feature with every §10.4 measure and the price-table date.

**Throttled (F-033, D-028).** The waste cause `throttled` is the time the platform stopped the work: a usage window exhausted or a platform pause, measured from `mission_paused` to `mission_resumed` with the reason recorded. It is not idle and not waiting on a human. It is reported per milestone and per run, separately from the eight causes and from the five display groups, so the eight keep their meaning and the stack of five is not distorted by an overnight stop.

### 5.7 Telemetry series budget (D-010, B-017)

Contract: OTLP `http/protobuf` to `https://otlp-gateway-prod-eu-west-2.grafana.net/otlp`; header
value only from the keychain item `droid-observatory/otlp-headers`; resource attributes
`service.name=droid-observatory`, `service.version`; labels only from `mission`, `milestone`,
`role`, `model`, `cause`; counters and histograms only; one mission exported at a time.

| Metric | Instrument | Labels | Cardinality (worst case) | Series |
|---|---|---|---|---|
| `observatory.mission.cycle_seconds` | histogram, 6 buckets | `mission` | 1 × (6 + `+Inf` + `_sum` + `_count`) | 9 |
| `observatory.feature.cycle_seconds` | histogram, 5 buckets | `mission` | 1 × (5 + 3) | 8 |
| `observatory.waste_seconds` | counter | `mission`, `milestone`, `cause` | 8 milestones × 8 causes | 64 |
| `observatory.rework_windows` | counter | `mission`, `milestone`, `cause` | 8 × 4 | 32 |
| `observatory.fix_loops` | counter | `mission`, `milestone` | 8 | 8 |
| `observatory.tokens.{input,output,cache_read,cache_write}` | 4 counters | `mission`, `role`, `model` | 3 roles × 4 models = 12 pairs × 4 | 48 |
| `observatory.cost_usd` | counter | `mission`, `role`, `model` | 12 | 12 |
| `observatory.unknown_events` | counter | `mission` | 1 | 1 |
| **Total worst case** | | | | **182** |

Expected for this mission (7 milestones, 3 role×model pairs): 9 + 8 + 56 + 28 + 7 + 12 + 3 + 1 =
**124**. Both are strictly under 200. Why these shapes: the feature-cycle histogram carries no
`milestone` label because a histogram per milestone (7 series each) would cost 56 series for a
distribution the app already shows per card; waste and rework keep `milestone` because the budget
meter and the waste chart are read per card. Guards: the `milestone` label is bounded by
`max_milestone_labels` (the ninth and later milestones of any observed mission export as
`milestone="overflow"`); `role` is the closed `Role` enum; `model` beyond four distinct values per
mission exports as `model="other"`; the exporter refuses to start if its computed worst case for the
configured buckets and limits is not under `[telemetry].max_series`, and the test
`exported_series_count_is_under_budget_on_worst_case_labels()` pins the 182. Series of a previous
mission stop being written when the export target changes; `mission_switch_min_interval_min` keeps
the two sets from overlapping inside Grafana's active-series window. A `service.version` change
(release) re-keys the resource once; it is not a steady-state cost. CI telemetry stays deferred
(B-002). D-010 sized the shape at up to 6 milestones; this spec uses 8 label values because the
skeleton makes seven milestones and one `overflow` value bounds other missions. Still under 200.

### 5.8 Runtime limits

From §5.5: 64 watched sessions, 512 MB per session file before warnings, 1 MiB tail chunk,
100 ms UI batch, 500 ms JSONL poll, 30 s GitHub poll, 15 s metric export, 20 min stall threshold.
The UI holds at most the visible window of events; everything else is a store query (reason: a
mission can emit tens of thousands of lines; the board must stay at 60 fps on a laptop). The host
child process is reaped on window close (M2 test). No network access except GitHub (when a token
is present) and the OTLP gateway (when enabled); both are shown in a status bar.

Validation and build concurrency (measured 2026-10-22; source:
`reports/readiness/2026-10-22-validation-readiness.md`):

- At most 3 concurrent agent-browser validator sessions per `trunk serve` (1.34-1.54 GB RSS each,
  CPU negligible idle).
- `cargo llvm-cov` runs alone (1-min load 3.3 → 14.6 on 10 cores during the scratch run).
- All cargo builds serialize on the shared `CARGO_TARGET_DIR` lock
  (`Blocking waiting for file lock on build directory`).
- `trunk serve` and `cargo tauri dev` coexist only when exactly one trunk owns port 3141.
- Cold `trunk serve` 55.94 s, incremental 1.46-2.08 s, `cargo tauri dev` to running app 37.66 s.

### 5.9 Security

- Secrets only from the macOS keychain or the environment (rule 4): `droid-observatory/github-token`
  or `GITHUB_TOKEN` for lane 8; `droid-observatory/otlp-headers` for telemetry (no environment
  fallback in the app, D-010). Never in config, store, logs or the exported telemetry.
- Raw JSONL is rendered read-only after the B-006 scrub pass, never uploaded, never copied into the
  store (PRD §11.2); rendering read-only and the merge as the one outward write are PRD §11.3.
  Fixtures are committed only after gitleaks and a human read (§3).
- The one outward write is the merge (FR-G5): confirmation names PR number and target branch;
  the app uses the repository's allowed merge method; the action is a lane-7 event.
- Telemetry carries no session id, event id, feature id or free text (D-010); the label allow-list
  is enforced in code and by test.

## 6. Tests

Fixtures are captured streams, named and dated (§3). Local = `make check` in a worktree; MB = the
`mission-checks` aggregate on `mission/**`; PR = the `gate` aggregate on the final pull request.

| Suite | Covers | Where | Command |
|---|---|---|---|
| Unit (`#[cfg(test)]` next to code) | model, mapping helpers, time-state engine, windows, cost, label allow-list | local, MB, PR | `cargo test --workspace --all-features` |
| Fixture replay (`crates/observatory-core/tests/replay_*.rs`, `mapping.rs`) | each fixture → golden lane events, `unknown events: 0` on frozen captures | local, MB, PR | `make replay-fixtures`; `cargo test -p observatory-core --test mapping` |
| Attach latency (`tests/attach_latency.rs`) | harness appends a fixture line by line; event visible to the store within 1 s | local, MB, PR | `cargo test -p observatory-core --test attach_latency` |
| CLI (`crates/observatory-cli/tests/cli.rs`) | exit codes, `unknown events: N` line, `inspect` prints the exact raw line | local, MB, PR | `cargo test -p observatory-cli` |
| Metrics oracle (`tests/oracle.rs`, M5) | engine output vs `reports/reference/*.csv` within tolerance | local, MB, PR | `cargo test -p observatory-core --test oracle` |
| Telemetry budget (M5) | worst-case series count = 182 < 200; labels outside the set rejected | local, MB, PR | part of unit |
| UI logic | view-model functions (card position, filters, scrubber) live in core and are unit-tested there; the Leptos layer holds no logic worth a wasm test runner (reason: `wasm-bindgen-test` adds a toolchain step for little coverage) | local, MB, PR | part of unit |
| Interactive QA (validators) | board, Details, tray, filters, scrubber in the **browser host** `trunk serve --port 3141` with the fixture opened through the file picker; attach, host, controls and gate in the **Tauri window** (`make app-dev`) with screenshots and the CLI as oracle | per milestone, by the user-testing validator | `make app-dev`; `trunk serve --port 3141` |
| Coverage (B-005) | `observatory-core` lines | S: reported; **M0 onward: fail under 80 %**; MB and PR | `cargo llvm-cov -p observatory-core --fail-under-lines 80` |

**Where each command runs (F-029, rule 23, D-025).** The build is paid in series: a mission runs
features one after another, so every compile a worker triggers is wall clock nobody shares. A
worker's inner loop is `cargo check -p <crate>` and `cargo test -p <crate>` for the crates it
touches, with `cargo fmt --all -- --check` and `cargo clippy -p <crate> --all-targets -- -D
warnings` once before handoff; `cargo-nextest` is not installed (`cargo nextest --version` → "no such
command") and is not to be installed. The full suite (`cargo test --workspace` inside `make check`)
runs once per milestone at integration, by the integrator; `cargo llvm-cov` runs only there, alone
(the scrutiny validator's programmatic gate counts as milestone-level). Every cargo or trunk
invocation goes into the handoff `verification.commandsRun` with its wall time pasted. The readiness
report states the measured build times and the per-feature compile cost
(`reports/readiness/2026-10-23-build-times.md`); each milestone's usage report records the compile
share of its wall clock.

**As-built (D-022).** On the Linux runner the unit and fixture suites run for `observatory-core` and
`observatory-cli` only (`cargo test -p observatory-core -p observatory-cli --all-features`); the
Tauri shell crate is linted and tested by `make check` locally and compiled by the macOS bundle job
(B-024). Fixtures are redacted at capture and `make redact-check` (part of `make check`) proves a
second redaction pass changes nothing (D-020, rule 22).

**B-005 settled: 80 % lines on `observatory-core`.** Reason: core holds every piece of logic that
can be wrong silently (mapping, time states, cost), so it is where coverage buys correctness;
IO adapters that need a live `droid` or GitHub are in core too and cannot all be exercised in CI,
which is the 20 % of room. The CLI is thin and covered by its own tests without a threshold; the
UI crate is excluded (wasm target, no logic). Enforced from M0 integration because S has too little
code for a percentage to mean anything. Raising it is a backlog item with a reason, lowering it is
a decision file.

## 7. CI

Two workflows, one required check (D-003). Measured per run; the wall time of the aggregate is
pasted into `reports/ci/<date>-<milestone>.md` at integration (rule 11).

**Mission branch** (`mission-branch.yml`, on push and PR to `mission/**`; aggregate
`mission-checks`; budget 3 min warm): markdownlint, lychee, actionlint, file size, debt markers,
decisions index, gitleaks, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`,
`make replay-fixtures`, `cargo deny check`, plus from S: **`cargo machete`** (B-014) and
**`cargo llvm-cov` with the §6 threshold** (B-005). Independent jobs in parallel with the
rust-cache. Gates integration: the orchestrator waits for green before the next milestone. If the
warm time exceeds 3 min after the coverage job joins, coverage moves to the final PR only, by
decision file.

**As-built (D-022).** The Linux `rust` job installs no apt packages and does not build the Tauri
shell: `make fmt-check`; `cargo clippy --workspace --exclude observatory-app --all-targets
--all-features -- -D warnings -W clippy::cognitive_complexity`; the wasm32 clippy of
`observatory-ui`; `cargo test -p observatory-core -p observatory-cli --all-features`; `make
replay-fixtures`. The apt variant took 4 min 24 s and 4 min 50 s, over the 3 min budget.
`mission-branch.yml` also runs on push to `wt/**`. Measured: run 39928967884 on the integration
commit, wall time 2 min 56 s (`reports/ci/2026-10-23-s.md`).

**As-built at the M0 integration (budget evidence).** With the coverage job enforced at 80 %, `wt/m0`
run 39942226174 took 3 min 58 s (`rust` 220 s was the critical path, `coverage` 59 s ran in parallel), so
the fallback above (coverage on the final PR only) would not have shortened the wall time. The first
`mission-checks` run on the mission branch after the M0 merge (run 39953923391) took 1 min 45 s (`rust`
87 s, `coverage` 77 s). Per-job tables: `reports/ci/2026-10-23-m0.md`. **Decided at the M0 to M1
boundary (F-031):** the budget stays 3 min, qualified "warm cache": the warm mission-branch runs took
1 min 45 s and 1 min 43 s (`reports/ci/`), and the cold `wt/m0` run 39942226174 (3 min 58 s) does not
count against it. A cold run is reported as cold, never as a budget breach.

**Compile cost evidence (F-029, D-025).** Measured warm on the mission branch on 2026-10-23, pasted
from `reports/readiness/2026-10-23-build-times.md`: `make check` real 30.383 s (128 tests passed);
`cargo check -p observatory-core` real 1.268 s; `cargo test -p observatory-core` real 11.631 s (112
passed); `cargo llvm-cov -p observatory-core --all-features --fail-under-lines 80 --summary-only`
alone, real 54.911 s (lines 94.21 %). The inner loop costs a few seconds against 30 to 55 s for the
milestone commands, which is why only the integrator runs the latter.

**B-014 settled: `cargo-machete`, not `cargo-udeps`.** Reason: machete runs on the stable toolchain
in seconds; udeps needs nightly, which is a second toolchain to pin and cache for a check that
finds a strict superset only in edge cases. Revisit if machete misses a real unused dependency.

**Demo gate** (`demo-gate.yml`, M4, on `pull_request` to `demo/base`): one job that fails when the
label `demo:red` is present. Required by the `demo-gate` ruleset on `demo/base` only. Never touches
`main`.

**Final pull request** (`gate.yml`, on PR to `main`; aggregate `gate`, the only check the `main`
ruleset requires): everything above plus Semgrep (SARIF to code scanning), Trivy filesystem scan of
the repository (vuln, misconfig, secret; HIGH and CRITICAL; `Cargo.lock` is what makes the vuln
scan meaningful), Checkov on the workflows, SBOM (`syft`), the macOS Tauri bundle (`bundle.yml`)
with build provenance (`attest-build-provenance`) and cosign signatures, Trivy results and SBOM
attached as artifacts. It never deploys. `release.yml` on `v*` tags pushed by the human to a
commit on `main` reuses `bundle.yml` and, from S onward, checks the workspace version against the
tag (B-016, wired in S; D-009).

## 8. Deliverables

Grown at every integration, never written at the end (rule 16).

- **README.md**: what it is, the clean-clone commands (`make doctor`, `make check`, replay,
  `make app-dev`), the mission command of §2, the gate, the demo branch pair, honest time and
  usage per milestone (rule 10, from `reports/usage/`), what was left out with the reason.
- **docs/decisions/**: one file per decision, `make decisions-index` regenerates the README. The
  mission writes at least: mapping v1 and the `--tag`/child-session findings (M0), coverage
  component and threshold (S), machete (S), series allocation (M5 or earlier), demo branch pair
  superseding the private repo (M4), backlog is `BACKLOG.md` only (S, D-015; the hybrid backlog was
  withdrawn), any model or effort change (rule 19; D-018: models are invariant), and the M1 go/no-go
  outcome. Written at the S integration: D-012 session schema, D-013 keyring variant B, D-014
  read-only token, D-015 backlog, D-016 coverage, D-017 machete, D-018 invariant models.
- **BACKLOG.md** (the only backlog; owner comment on the mission proposal, 2026-10-22, D-015):
  deferred items are `B-NNN` rows with a reason and an earliest milestone, written by the
  integration feature from the workers' handoffs. No GitHub issues, no `backlog` label and no
  `scripts/gen-backlog.sh` exist. *As built:* the hybrid backlog of the first version of this
  section (issues labelled `backlog`, regenerated `BACKLOG.md`) is withdrawn. Items the spec closed
  (B-005, B-007, B-010, B-014, B-017) are marked done with this spec as the reason. The wording of
  rule 14 in `ai/RULES.md` (the owner's file) still names the issues; the proposed wording is
  "everything else goes to `BACKLOG.md` with its reason", waiting for the owner.
- **build-log/**: one JSON line per tool call per session, written by the PostToolUse hook,
  committed with each integration (exempt from the 200 KB limit).
- **ai/session/**: one entry per interaction with transcript links; the mission appends to
  `ai/session/2026-10-22-session.md` (or a dated successor) at every boundary.
- **reports/**: `usage/<m>.md` (tokens, credits, USD-eq per role and model, pasted),
  `waste/<m>.md` (from M5 onward, computed by the observatory on its own capture),
  `readiness/<date>-<m>.json` (B-015, every integration), `ci/<date>-<m>.md` (aggregate wall time),
  `reference/` (the owner's spreadsheet, B-008).
- **Spec PR** (before the mission): this file, the model-id corrections already in the working
  tree (entry 21), the three reasoning-effort keys in `.factory/settings.json`
  (`missionOrchestratorReasoningEffort = "high"`, `workerReasoningEffort = "high"`,
  `validationWorkerReasoningEffort = "high"`), session log entries 20–23, the PRD §13 M4 row and
  §14 risk row reworded to the demo branch pair, and BACKLOG B-007 reworded (entry 19), on a
  `docs/spec-v1` branch, PR to `main`, merged by the human. Reason: the mission command reads
  `SPEC.md` from the branch it starts on, and `main` is the only branch a mission starts from (§2).

## Tooling

Tooling is looked up, not remembered (rule 26, F-062). Written 2026-10-25 for M3, before
`m3-cycle-template-refactor` was dispatched. Before the first contract of a milestone the orchestrator
re-verifies every row below against the source and updates the version and date. The research behind each row is
in the mission library (`library/tooling.md`, `research/2026-10-25-*.md`); Context7 was not available, so the
sources are the official docs, npm, crates.io and GitHub, fetched on the date shown. Workers run the command in
the row; a check with no row is first added here. Decision column: `deterministic` = the tool's exit code is
the verdict; `script` = a script over the tool's output decides; `judgement` = an agent reads the output.

| Check | Tool | Version (verified) | How it runs | Source | Date | Decision |
|---|---|---|---|---|---|---|
| Format | rustfmt | toolchain 1.99.1 | `cargo fmt --all -- --check` (`make fmt-check`); CI `rust` job | `rust-toolchain.toml`, `Makefile` | 2026-10-25 | deterministic |
| Lint and type check | clippy | toolchain 1.99.1 | `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cognitive_complexity` (`make lint`); Linux CI excludes `observatory-app` (D-022) and lints `observatory-ui` for `wasm32-unknown-unknown` | `Makefile`, `mission-branch.yml` | 2026-10-25 | deterministic |
| Unit and integration tests | cargo test | toolchain 1.99.1 | `cargo test --workspace --all-features` (`make test`); inner loop `cargo test -p <crate>`; CI `rust` job tests `observatory-core` and `observatory-cli` | `Makefile` | 2026-10-25 | deterministic |
| Coverage | cargo-llvm-cov | 0.9.1 (latest, 2026-09-06) | `cargo llvm-cov -p observatory-core --all-features --fail-under-lines 80 --summary-only` (`make coverage`), alone, once per milestone; CI `coverage` job | <https://crates.io/crates/cargo-llvm-cov> | 2026-10-25 | deterministic |
| Fixture replay | observatory-cli | workspace | `make replay-fixtures`; `unknown events: 0` on both root fixtures | `Makefile`, AGENTS.md | 2026-10-25 | deterministic |
| Vulnerability scan | cargo-deny (advisories, RustSec) | 0.20.2 (latest, 2026-07-09); action v2.1.1 | `cargo deny check` (`make deny`); CI `deny` job; `cargo-audit` intentionally absent (same database) | <https://crates.io/crates/cargo-deny>, `deny.toml` | 2026-10-25 | deterministic |
| Licence check | cargo-deny (licenses) | 0.20.2 | same command; allow list in `deny.toml`, `confidence-threshold = 0.9` | `deny.toml`, D-011 | 2026-10-25 | deterministic |
| Dependency hygiene | cargo-deny (bans, sources), cargo-machete | 0.20.2; 0.9.2 (latest, 2026-04-15) | `cargo deny check`; `cargo machete` (`make machete`); CI `machete` job pins `cargo-machete@0.9.2` | `Makefile` | 2026-10-25 | deterministic |
| Secret scan | gitleaks | 8.30.1 (latest, 2026-03-21); action v3.0.0 | `gitleaks git --no-banner --redact .` (`make check-secrets`); CI `secrets` job over the full history; fixtures also pass `make redact-check` (D-020) | <https://github.com/gitleaks/gitleaks> | 2026-10-25 | deterministic |
| SAST | semgrep | 1.182.0 (image) | gate `sast` job, `p/default p/rust p/secrets p/github-actions`, SARIF to code scanning | `gate.yml` | 2026-10-25 | deterministic (exit) + judgement (findings) |
| Filesystem vulnerability and misconfiguration | trivy | trivy-action v0.36.0 | gate `fs-scan` job, `vuln,misconfig,secret`, HIGH and CRITICAL fail | `gate.yml` | 2026-10-25 | deterministic |
| SBOM | syft via anchore/sbom-action | v0.24.3 (latest, 2026-10-02) | gate `sbom` job, `spdx-json` artefact; CycloneDX via `cargo-cyclonedx` 0.5.9 only on owner request | `gate.yml`, <https://github.com/anchore/sbom-action> | 2026-10-25 | deterministic |
| Provenance and signature | actions/attest-build-provenance, cosign | v4.2.2; cosign-installer v4.1.2 | bundle job: attestation of the DMG and `.app.tar.gz`, keyless `cosign sign-blob` | `bundle.yml` | 2026-10-25 | deterministic |
| Workflow lint and policy | actionlint, checkov, `scripts/check-ci-jobs.sh` | 1.7.12 (latest, 2026-03-30) | `make check-workflows`; CI `workflows` job; gate `workflow-policy` | `Makefile`, `gate.yml` | 2026-10-25 | deterministic |
| Markdown and links | markdownlint-cli2, lychee | 0.24.0; 0.24.2 (latest, 2026-05-01) | `make check-docs`, `make check-links`; CI `docs`, `links` jobs | `Makefile` | 2026-10-25 | deterministic |
| Debt markers, file size, decisions index | repository scripts | - | `make check-todos`, `make check-file-size` (200 KB), `make check-decisions-index` | `Makefile` | 2026-10-25 | deterministic |
| End-to-end (M3, item 14) | WebdriverIO + `@wdio/tauri-service` + `tauri-plugin-wdio-webdriver` (embedded WebDriver server) | WDIO 9.32.0 (last 9.x, 2026-09-20; service peer `^9`, npm latest 10.1.0 incompatible); service 1.5.1 and crate 1.5.1 (2026-10-24); `@wdio/junit-reporter` 9.32.0 | `make e2e` = `cargo tauri build --debug --no-bundle --features e2e && cd e2e && npx wdio run wdio.conf.ts`; plugin behind cargo feature `e2e` only; JUnit and one PNG per test under `reports/e2e/<date>/` (`E2E_DATE`); macOS job `.github/workflows/e2e.yml`, triggered by the tag `e2e-<milestone>` pushed by the integrator (`workflow_dispatch` once on `main`), outside the 3-minute budget; locked-display behaviour measured once and pasted | <https://v2.tauri.app/develop/tests/webdriver/>, <https://webdriver.io/docs/desktop-testing/tauri>, <https://crates.io/crates/tauri-plugin-wdio-webdriver> | 2026-10-25 | deterministic (JUnit) |
| Latency assertions (M3) | User Timing API read through WebDriver `execute/sync`; host JSON lines via `tracing-subscriber` `json` | web-sys 0.3.106 features `Performance`, `PerformanceMark`, `PerformanceMeasure`, `PerformanceEntry`; tracing-subscriber 0.3.23 | marks and measures in the board, read with `browser.execute(() => performance.getEntriesByType('measure'))`; thresholds in `config/observatory.example.toml`; host intervals to `OBSERVATORY_TIMING_LOG` JSONL read by the suite | <https://docs.rs/crate/web-sys/latest/features>, <https://docs.rs/tracing-subscriber> | 2026-10-25 | deterministic |
| Gate cache (M3, item 16) | `scripts/gate.sh` (git write-tree on a temporary index, `rustc -V`/`cargo -V`, jq) | git 2.50.1, jq 1.8.1 | wraps `make test`, `make lint`, `make coverage`, `make check`; records `reports/gate/<tree>/<name>.json`; `FORCE=1` re-runs; `build-log/` and `reports/gate/` excluded from the key; CI uncached | <https://git-scm.com/docs/git-write-tree> | 2026-10-25 | script |
| Usage report (M3, item 18) | `scripts/usage-report.py` (python3 stdlib, `tomllib`) | python 3.14.7 | `make usage M=<m>` writes `reports/usage/<m>.md` from `~/.factory/logs/droid-log-single.log*`, the session sidecars, `progress_log.jsonl`, `features.json`, `config/prices/<date>.toml`; `scripts/usage-from-log.sh` stays as the cross-check | D-029, `reports/usage/m2.md` | 2026-10-25 | script |
| Integration (M3, item 19) | `scripts/integrate.sh` (git merge `--no-ff -F`, `interpret-trailers`) | git 2.50.1 | `make integrate M=<m> BRANCHES="..."`: merges, gate via cache, usage via item 18, concatenates `reports/staging/*.md` into `reports/readiness/<date>-<m>.md`, boundary commit with trailers from `.git-trailers` | <https://git-scm.com/docs/git-merge>, <https://git-scm.com/docs/git-interpret-trailers> | 2026-10-25 | script |
| Subagent droids (M4, items 5 and 12b) | custom droid files `.factory/droids/mechanical-sonnet.md` (`model: claude-sonnet-5-5`) and `.factory/droids/user-testing-flow-validator.md` (`model: claude-sonnet-5-5`, was `inherit`) | droid files verbatim copies of `~/.factory/droids/` (diff empty, 2026-10-29); Droid 0.247.0 custom droids | `Task` with `subagent_type: mechanical-sonnet` or `user-testing-flow-validator`; every report starts with `model = <value>` and the parent pastes it; `scrutiny-feature-reviewer` stays `inherit` (Opus, judgement); a change of a pinned model is a decision file (D-039) | D-039, `.factory/droids/` | 2026-10-29 | deterministic (the pin) + judgement (the work) |
| Leptos component tests | none (no change) | wasm-bindgen-test 0.3.79 considered | logic is tested in `observatory-core`; the DOM through the e2e suite | <https://book.leptos.dev/testing.html> | 2026-10-25 | - |

Not adopted, with the reason: `tauri-driver` 2.1.0 (Windows and Linux only), `@crabnebula/tauri-driver`
2.0.9 (paid key on macOS), `cargo-audit` 0.22.2 (duplicate of cargo-deny advisories), trufflehog v3.99.7
(network verification; gitleaks covers the history), `cargo-nextest` 0.9.149 (not to be installed, AGENTS.md),
`sccache` (compile cache, not a check), `wasm-pack test` (second driver stack for a thin UI).

**Tool facts, harness limits (rule 26; written 2026-10-29 at the M3 boundary, recorded as facts by the owner's go of
the same day, not as rules).**

1. **Built-in Task subagents inherit the parent's model; the Task `complexity` tier routes nothing here.** The two
   subagents of the M3 boundary (`c312afac`, `c1d0aec9`) ran on `claude-fable-5.1`; a custom droid with a `model:` line pins
   the model (`7e16efd6` ran on `claude-sonnet-5-5` through `mechanical-sonnet`). Hence the droid files above and D-039.
2. **The orchestrator's wait on a run returns at the run's end or after five minutes.** A warm wait inside the prompt
   cache's life (item 19w) is not available; the mechanism in force is the background Script with zero waits (rule 25,
   F-056). The cache lifetime is a factory setting: a product request to Factory (B-115).

## 9. Changelog and human feedback

Written by the orchestrator on the mission branch, never in a worktree.

| Id | Date | Feedback | Applies | Status |
|---|---|---|---|---|
| F-001 | 2026-10-22 | Orchestrator model id is `claude-fable-5.1`, not `claude-fable-5-1` (fell back to GPT-6 Sol) | §2, D-006, settings, README, config, price table | done (spec PR) |
| F-002 | 2026-10-22 | M4 is tested on a demo branch pair in this repository, not a private repo (B-007) | §3 M4, §7 | in spec |
| F-003 | 2026-10-22 | Hybrid backlog is fine, the mission drives it | §8 | withdrawn by F-014 (4): `BACKLOG.md` only, D-015; rule 14 wording awaiting the owner |
| F-004 | 2026-10-22 | Telemetry contract (D-010): labels, instruments, under 200 series | §5.7 | in spec (182 worst case) |
| F-005 | 2026-10-22 | Reasoning effort high/high/high | §2 | done (spec PR) |
| F-006 | 2026-10-22 | Automatic merge out of scope, observing a policy merge in scope | §1, §3 M4 | done (spec PR) |
| F-007 | 2026-10-22 | Mapper versioned from day one, unknown count always visible | §1, §3 S, §5.2, §5.3 | done (spec PR) |
| F-008 | 2026-10-22 | Spec reaches the mission via a spec PR to main (docs/spec-v1) | §2, §8 | done (spec PR) |
| F-009 | 2026-10-22 | Rule 14 wording change approved | §8, ai/RULES.md | done (spec PR) |
| F-010 | 2026-10-22 | Budget caps 630 accepted; re-cap proposed at the M0 boundary | §2 | done (spec PR) |
| F-011 | 2026-10-22 | claude-sonnet-5-5 confirmed available | §2 | done (spec PR) |
| F-012 | 2026-10-22 | SPEC.md v0.1 approved; mission plans from it | all | approved |
| F-013 | 2026-10-22 | Owner answers before the mission (entry 26): "Schema: agreed. Record droid --version in the fixture header as the mapping-version signal, read the sibling .settings.json for usage and child links, decision file at S integration, and amend the SPEC as-built at that integration (rule 8), not now."; keyring-core + apple-native-keyring-store accepted with a decision file; DMG only in the CI bundle job; Tauri evidence is the validators' smoke check plus one owner screenshot per integration; concurrency limits accepted; GitHub token read-only, check results from Actions runs and commit statuses | §3 M0, §5.2, §5.8, D-012, D-013, D-014 | done (as-built at the S integration) |
| F-014 | 2026-10-22 | Owner approval of the mission proposal (entry 27), with four comments: (1) "Budget vs plan: 630 USD-eq total against a Max month (and the 5-hour/7-day limits) will likely hit throttling mid-M1 or M4. State what happens at a throttle (pause at the boundary, no silent model downgrade) and keep the M0 re-cap step."; (2) "M4 acceptance contradicts my decision: it reads \"merge from the app → badge updates\". Make the M4 exit: Merge button visible when policy allows, the app surfaces the 403 cleanly with the read-only token, and the merge happens by me(UI or API); the badge updates from the poll. Write-token question stays at the M3→M4 boundary."; (3) "Reasoning effort: §8 says \"all high\"; the spec sets workers to medium. One or the other."; (4) "Backlog: \"hybrid backlog (gh issues labelled backlog)\" reopens the Issues question you closed yesterday; AGENTS.md says deferred work goes to BACKLOG.md, not Issues. Pick BACKLOG.md only, unless you want Issues back on purpose." | §2, §3 M4, §8, D-014, D-015 | (1) in mission files, M0 re-cap kept; (2) done, §3 M4 amended; (3) checked, all `high`, no change; (4) done, §8 amended, F-003 withdrawn |
| F-015 | 2026-10-23 | Owner decision at the S to M0 boundary: the models and reasoning efforts defined at mission approval never change for the lifetime of the mission; at a usage limit the mission stops, never continues on a fallback model | §2, D-018, `ai/RULES.md` rule 20 | done (as-built at the S integration) |
| F-016 | 2026-10-23 | Owner decision at the S to M0 boundary: communication. "Think as much as you need, say little. To the human: one line of status per milestone event, a decision request in three lines (what, options, your recommendation) when rule 15 requires one, and nothing else." | D-019, `ai/RULES.md` rule 21, `.factory/output-styles/mission-terse.md` | done (second S integration pass); the owner selects the style with `/settings` |
| F-017 | 2026-10-23 | Owner decision: PII in fixtures. Redact emails and `/Users/dct` paths deterministically, originals outside the repo with both SHA-256 in the decision file, offsets against the redacted file, gitleaks before commit, every future fixture | D-020, `ai/RULES.md` rule 22, S fixture redacted (`92d2208`) | done; the hyphenated `-Users-dct-code-...` form was settled by F-026 |
| F-018 | 2026-10-23 | Owner decision: orchestrator cap, 20 % of each milestone cap, measured as the delta of session `e194b9e3`'s own `tokenUsage` from the baseline in `reports/usage/s.md`; re-set at the M0 boundary | §2 | done (as-built); re-cap at M0 boundary |
| F-019 | 2026-10-23 | Owner confirms D-011 with two addenda: BSL-1.0 is the Boost Software License (not Business Source); RUSTSEC-2024-0436 ignored with a re-check date 2027-01-21 | D-011 | done |
| F-020 | 2026-10-23 | Owner decision: the tool-call hook logs a `script` field (first 200 characters of `tool_input.script`) | D-021, `.factory/hooks/log-tool-call.sh`, B-022 | done (verified: 300-character payload logs length 200, other tools log null) |
| F-021 | 2026-10-23 | Owner: `.capture.json` `reviewed_by` stays `factory-droid[bot]` | S fixture capture header | done (no change) |
| F-022 | 2026-10-23 | Owner approves the rule 14 wording "everything else goes to `BACKLOG.md` with its reason" | `ai/RULES.md` rule 14, D-015 | done; F-003 and F-009 fully closed |
| F-023 | 2026-10-23 | Owner: keep `ai/prd/review-2026-10-22.md` (restored with `git checkout`); §1 unchanged | §1 | done (no edit needed) |
| F-024 | 2026-10-23 | Owner decision after the second pass: the M0 orchestrator cap is raised from 12 to 32 USD-eq; the baseline in `reports/usage/s.md` is kept and the 18.94 USD-eq boundary delta counts against M0; M1 to M5 orchestrator caps unchanged (30, 16, 12, 24, 24) | §2 | done (as-built at the M0 integration); `reports/usage/m0.md` shows the delta against 32 |
| F-025 | 2026-10-23 | Owner decision: orchestrator subagents run at the highest effort (`Task` `complexity: heavy`); sessions `180b4a60` and `4162070f` ran at `medium` before the rule | `reports/usage/m0.md` (episode) | done (listed as an episode) |
| F-026 | 2026-10-23 | Owner decision: the PII rule covers the hyphenated session-directory form, `-Users-dct` becomes `-home-user` (third rule of `scripts/redact-fixture.sh`); S fixture re-redacted, M0 capture redacted with all three rules | D-020 addendum, `ai/RULES.md` rule 22 | done (S `fcfbc32`, M0 `c623365`); closes the open point of F-017 |
| F-027 | 2026-10-23 | Owner restates the communication rule and adds: "Decision requests carry no background paragraph; the owner asks for context when needed. Instructions are acknowledged in one line." | D-019 addendum, `ai/RULES.md` rule 21, `.factory/output-styles/mission-terse.md` | done (M0 integration) |
| F-028 | 2026-10-23 | Owner re-scopes Roci as a mission watcher that measures waste and effectiveness (cycle time, cost) of the engineering cycle from the spec to the PR's checks going green; post-merge (release, observe, deploys, flags) belongs to the delivery platform and is out of scope. Lanes are the seven mission phases, one axis: 1 Plan, 2 Contract, 3 Build, 4 Scrutiny, 5 User testing, 6 Integrate, 7 Gate; roles are colour and attribution; human interventions and commander actions are overlays (`overlay: human\|commander`), not lanes; the old lane 9 is the Panel (cycle time, waste measured as 8 and shown as 5 groups, cost per role, drift against the declared models), the old lane 8 folds into Gate; one `handed_to_cd { merge_sha, pr_number, attestation_ids }` replaces `pr.merged_by_human`, `pr.merged_by_policy` and `deploy.completed` (59 kinds: 30 observed, 29 defined-not-observed). Applied inside `mapping::v1` (kinds, `SourcePtr`, roles and sidecar unchanged). **Owner answers:** 1a contract sessions are children spawned by a `Task` whose description matches `^Contract\b\|[Vv]alidation contract` (v1 heuristic); 1b the merger is attributed by role (human / external) on one `handed_to_cd`; 1c M4 cuts `deploy.completed` and branch-deletion detection (`demo/*` cleanup stays a `gh` step); 1d human-overlay kinds take a fixed lane by kind in v1 (`intent.*`, `question.*`, `approval.*` → Plan; `permission.*` → the asking session's phase; `review.outcome` → Gate; `approval.given` with `human_actions.kind = merge` → Gate), placement by the feature's phase is a v2 note; 1e engineering cycle time = `feature.defined` → `handed_to_cd`, else the feature's first `task.started` with the fallback count reported, and "lead time to production" is named on the board, never computed. **Alternatives considered:** a `mapping::v2` for the phases (rejected: no consumer of lanes exists yet, a version bump is for a new Droid release, and v2 would need its own fixture and golden); Contract left empty on M0 until a capture has a contract-writer role (1a); `pr.merged_by_policy` kept as a second Gate kind (1b); branch-deletion detection kept as closing housekeeping (1c); human events placed in the phase their feature is in at that instant, which needs cross-session state in the stateless `map_record` (1d); a `feature.defined` ids rule amended before the reference CSV (1e). | §1, §2, §3 M1/M3/M4/M5, §5.2, §5.3, §5.6, §10, D-024, D-023 amendment (at `m1-view-models-core`), PRD notes, README, AGENTS.md, BACKLOG, mission files, features, validation contract | done in the shared files (`m1-f028-spec-pass`); mapping change, golden regeneration and hash refreeze at M1 |
| F-029 | 2026-10-23 | Owner rule on compile cost: "The build is paid in series: check in the inner loop, test at integration, coverage at the milestone." Workers run `cargo check -p` and `cargo test -p` for the crates they touch; the full suite runs once per milestone by the integrator; coverage runs only there, alone; the readiness report states measured build times and the per-feature compile cost; each usage report records the compile share of the wall clock. The owner wrote the rule as number "20"; 20 is the models invariant (F-015), so it landed as rule 23, no renumbering. Evidence: warm `make check` 30.383 s, `cargo check -p observatory-core` 1.268 s, `cargo test -p observatory-core` 11.631 s, `cargo llvm-cov` alone 54.911 s (`reports/readiness/2026-10-23-build-times.md`). `cargo-nextest` is not installed and is not to be installed | §2, §6, §7, `ai/RULES.md` rule 23, D-025, AGENTS.md, `reports/usage/m0.md` (Compile share), `reports/readiness/2026-10-23-build-times.md` | done (`m1-f028-spec-pass`) |
| F-030 | 2026-10-23 | Owner autonomy rule: when a decision comes with a recommendation and changes neither scope, nor usage caps, nor the models per role, the orchestrator takes the recommendation, records it as a §9 row with the alternatives, and reports it at the next boundary; the owner is asked only for scope, money or model changes and for rule 9 deviations | `ai/RULES.md` rule 24, D-026, AGENTS.md "Mission setup" | done (`m1-f028-spec-pass`) |
| F-031 | 2026-10-23 | Owner answers at the M0 to M1 boundary: (a) re-caps in USD-eq with the orchestrator inside: M1 110 / M2 50 / M3 35 / M4 75 / M5 105 (were 150 / 80 / 60 / 120 / 120), orchestrator 20 % = 22 / 10 / 7 / 15 / 21 (were 30 / 16 / 12 / 24 / 24), sum 375, 630 stays the hard stop; M5 gains priority, M4 shrinks with the post-merge cut; (b) the M0 orchestrator overrun, 66.87 against 32 (209 %), is accepted as recorded, no retroactive cap change; the M1 orchestrator baseline is the absolute `tokenUsage` in `reports/usage/m0.md`; (c) CI budget stays 3 min, qualified "warm cache" (warm runs 1 min 45 s and 1 min 43 s; the cold `wt/m0` run 39942226174, 3 min 58 s, does not count); (d) no PII extension for the owner's author name, the login `dctellya` or `dalimaray.io` (public identifiers, rule 22 unchanged); the unredacted S fixture blobs in history before `92d2208` stay (forward-only, rule 12), recorded in BACKLOG; (e) D-023's defined-not-observed kinds (31 at the freeze, 29 after D-024) and the `Task` input fields for roles and ids (`subagent_type`, `description`) are frozen; (f) owner actions requested: overage preference "stop", the screenshot `reports/screenshots/2026-10-23-m0.png`, optional removal of the `kimi-k3` session `82a61cf6` (the frozen fixture is unaffected either way). Alternatives for (c): raise the budget to 5 min, or shrink the `rust` job | §2, §7, §10, BACKLOG, `reports/usage/m0.md`, `reports/usage/m1.md`, D-023 | done (`m1-f028-spec-pass`); owner actions pending |
| F-032 | 2026-10-23 | Owner rule: the orchestrator never polls. "Waiting is one blocking call with a timeout equal to the feature's remaining cap, not a turn on a timer. If a worker's tool call is cancelled or its run does not return, the orchestrator re-dispatches the feature or fails it at once, in the same turn, and records a §9 row with the cause. A second identical tool call with identical arguments is a deviation under rule 9, not a retry. While the mission is paused the orchestrator does nothing." **Cause:** the 18:15Z desktop-app restart cancelled worker `f06338e6`'s test run; the orchestrator then spent seven identical `WaitForScript` turns until 18:45Z, the mission paused at 19:02Z and the 5-hour limit came at 19:10Z. **Alternatives considered:** timer polling as before (rejected: each wake-up is a model turn that bills the context and returns the same answer; the band is waste); failing the feature at the first cancelled call (rejected as the only rule: a cancelled call can be a platform event, so re-dispatch is allowed, but in the same turn and with a recorded cause). | `ai/RULES.md` rule 25, D-027, AGENTS.md "Mission setup", `reports/usage/m1.md` (Waste row 18:15Z to 19:10Z, cause `waiting on validation`, note "orchestrator polling") | done (`m1-f032-spec-pass`) |
| F-033 | 2026-10-23 | Owner: new waste cause `throttled`, the platform stopped the work (usage window exhausted or a platform pause), measured from `mission_paused` to `mission_resumed` with the reason recorded; not idle, not waiting on a human; reported per milestone and per run, separately from the other causes. On the M0 capture the overnight stop alone is about 755 minutes (the owner's figure; the engine pastes the measured one). **Alternatives considered:** fold it into idle (rejected: idle is a bottleneck or scheduling signal and the platform stop is neither); fold it into waiting on human (rejected: nobody was asked anything, and the human-wait causes would be inflated by every usage reset). | §5.6, PRD §10.1 time states (marker "Added by F-033"), D-028, M5 engine and Panel, `reports/usage/m1.md` (Throttled row) | done in the shared files (`m1-f032-spec-pass`); engine at M5 |
| F-034 | 2026-10-23 | Owner: the usage log is an input source. Session files carry no token usage; the per-turn counts are in `~/.factory/logs/droid-log-single.log` (rotated daily as `droid-log-single.log.<date>`), lines `[Agent] Streaming result` with `inputTokens`, `outputTokens`, `cacheReadInputTokens`; sessionId and modelId sit in the neighbouring metrics lines of the same turn. The mapping reads the log beside the session files (tailable in attach at M1, replayable in history), joins turns to sessions by sessionId, and records the line formats as v1 heuristics with the rotation rule; the M5 usage panel reports per role and per lane input, output and cache-read tokens per turn and "tokens burnt in waste". Rule 7 is extended: every `reports/usage/<milestone>.md` from M1 carries the per-role token table with the first and last line timestamps. **Alternatives considered:** the sidecar `tokenUsage` only (rejected: no per-turn data, no lane attribution); a provider billing export (rejected: not available). | `ai/RULES.md` rule 7, §5.2, D-029, AGENTS.md "Decisions, backlog, logs", `scripts/usage-from-log.sh`, `reports/usage/m1.md` ("Tokens per role"), BACKLOG B-036, B-037, `m1-usage-log-source-core` | done in the shared files (`m1-f032-spec-pass`); source code at `m1-usage-log-source-core` |
| F-035 | 2026-10-24 | Orchestrator decision (rule 24, D-026), reported at the M1 boundary: the tool-call hook logs `PreToolUse` as well and adds `call_id` (hash of tool name and input, because the payload carries no `tool_use_id`), `ts_ms` (UTC milliseconds) and, on `PostToolUse`, `duration_ms` (end minus the matching start, `null` without one), so `scripts/compile-share.sh` can sum compile time for the F-029 report. **Alternatives considered:** derive timing from the session transcripts (rejected: tool calls are not timed there and the transcripts are the session's, not the hook's log); no change (rejected: the compile share would stay "not recoverable" for every session). **Limits found in use:** parallel identical calls pair imperfectly, calls nested in a `Script` are untimed, so the share is a lower bound (BACKLOG B-038). | §8 hooks, D-021 addendum, `scripts/compile-share.sh`, `reports/usage/m1.md` "Compile share" | done (`m1-hook-tool-duration`, `545485d`); merged at the M1 integration |
| F-036 | 2026-10-24 | Orchestrator decision (rule 24, D-026) while implementing F-034, reported at the M1 boundary: `turn.usage` is Panel data. It feeds tokens per role and per lane and is not a lane row, card, tray item or Details timeline entry. **Alternatives considered:** show turns as rows (rejected: inflates the lane counts by 1115 events on the M0 capture and buries the work); a separate lane (rejected: D-024 fixes seven lanes). The lane of a turn is its session's current phase (105 turns in three integration-feature sessions sit in Integrate while their `subagent_spawned` sits in Build). | §5.2, D-029 addendum, D-023 addendum, `AGENTS.md` "Other rules" | done (`m1-usage-log-source-core`, `m1-fix-turn-usage-panel-only`); merged at the M1 integration |
| F-037 | 2026-10-24 | Leptos go/no-go evaluated at the M1 integration against the two conditions of D-002 (SPEC §3 M1): rework sessions after the first scrutiny pass 1 on the UI (at most 3 by raw count of fix workers), clean-clone `trunk build` 56.95 s and 61.04 s with the README command. Result: **go**; the TypeScript fallback is not taken. Owner approval is needed only for a fail, so this is reported, not asked. **Alternatives considered:** Tauri 2 + TypeScript (rejected, both conditions hold). | §3 M1, D-002, D-030 | done (`m2-integrate-m1`) |
| F-038 | 2026-10-24 | Owner (M1 to M2 boundary, B-041, rule 9): a session is named by its feature id when one is present, by its user tag otherwise, through the one shared naming function (`ingest::sessions::display_name` / `user_tag`); the id column and the card id prefix stay. **Alternatives considered:** the tag only, as built at M1 (rejected: the tags repeat, `exec`, `read-only`, `subagent`, `mission-session`); the `Task` description as the name (rejected: free text, not an identifier). | §3 M1 (FR-L1), BACKLOG B-041 (scheduled), M3 feature `m3-session-name-feature-id` | decided; code at `m3-session-name-feature-id` |
| F-039 | 2026-10-24 | Owner (B-052): the CI budget stays 3 min and applies to WARM runs; the first run after a merge is COLD, measured and reported on its own row in `reports/ci/`, outside the budget verdict. The critical-path split (coverage apart from the Rust job) goes to BACKLOG with its expected saving computed from per-job seconds (B-056: 0 s on every M2 run, because the `rust` job is the longer one). **Alternatives considered:** raise the budget to 5 min (not taken). | §7, `reports/ci/2026-10-24-m2.md`, BACKLOG B-052 (resolved), B-056 | done (`m3-integrate-m2`) |
| F-040 | 2026-10-24 | Owner (rule 9): M1 accepted as recorded (110.38 against 110). The per-milestone 20 % orchestrator share (F-018, F-024, F-031) is RETIRED; the mission has ONE orchestrator cap of 320 USD-eq, spent-to-date reported against it at every boundary (266.05 USD-eq mission-to-date at 2026-10-24T04:27:53Z was the first figure). At the M2 boundary the orchestrator's M2 spend is split into waits, decisions and boundary reports, and narration, from the droid log (F-044); an M2 orchestrator spend above 30 USD-eq stops the mission at the M2 boundary before M3 is dispatched. **Alternatives considered:** keep the per-milestone shares (rejected by the owner). | §2 (usage table and orchestrator cap), `reports/usage/m2.md` | done (`m3-integrate-m2`); the M2 spend is over 30, see the M2 row |
| F-041 | 2026-10-24 | Owner, effective for everything built from M2 on, **the cycle is data, not code**: the seven lanes, their order, the role that owns each, the final event (`handed_to_cd`) and the clocks are a CYCLE TEMPLATE read at start-up (today the one engineering cycle, shipped as the default), never names in code. No view, metric, report or post-mortem names a lane; they iterate the template. The mapping (`mapping::v1`) assigns events to template lanes by id. Acceptance: a test that loads a second template with different lane names and counts and renders the board, the panel and the post-mortem input without a code change. **Alternatives considered:** keep the `Lane` enum (rejected by the owner); config-only display names for the same seven lanes (rejected: it renames, it does not make the cycle data). The decision file is D-035 (`D-035-cycle-template.md`), written by the refactor feature `m3-cycle-template-refactor` at the top of M3; the analysis is `contract-work/f041-cycle-template-analysis.md` in the mission directory. | §5.2, repo `AGENTS.md` ("Layout", cycle template line), D-024, M3 feature `m3-cycle-template-refactor` | recorded here; D-035 and code at M3 |
| F-042 | 2026-10-24 | Owner, self-check scratch missions (applies to `m2-host-shell-and-launch` and every later live self-check). Seen 2026-10-24 10:24 to 10:27 local: the host-shell worker's self-check hosted a real scratch mission (`6e346d07`, `target/scratch/probe`, worker `claude-opus-5-5`, validator `gpt-6.1-sol`: the launch form's defaults), ran a feature and a full scrutiny validator, and its lifecycle (`interrupt_session` at 08:24:51Z and 08:26:30Z) flipped this mission to paused and cut the orchestrator's blocking wait short twice; a mission hosted inside a mission shares the daemon and the runner does not tell them apart. **Rules:** (1) a scratch mission declares its models explicitly and uses the cheapest the catalogue allows for every role (never the launch form's defaults, never an undeclared model) and runs with `skipScrutiny` and `skipUserTesting` true unless the feature under test is the validation path; (2) it is namespaced per R-050 (its own working directory outside the worktree's build tree, its own session tag `scratch`) and its process group is reaped by the test that started it before the test reports; (3) its usage is reported in the milestone usage report as the line `self-check missions`, with the models used; (4) Factory feedback item 18: a child mission's lifecycle events must not change the parent mission's state or interrupt the parent's waits (no numbered Factory feedback list exists in the repository or the mission directory as of 2026-10-24; item 18 is recorded here). **Scratch model declaration (owner, 13:57 ruling):** `claude-haiku-4-5-20251001`, effort low, for scratch missions only (§2 models table, row `Scratch missions (self-checks)`); the workers, validators and the orchestrator keep their declared models; no decision file. **Platform artefact (owner addendum):** the 10:24 phantom pause is logged as a platform artefact, not as a pause of this mission (session log entry 36; not a `## Throttled` row). Evidence for rule 4: a hosted child killed by the group reap is recorded by Factory as `worker_failed ... SIGTERM` in the scratch mission's `progress_log.jsonl`. **Alternatives considered:** declare no scratch model and keep the defaults (rejected: Opus and a GPT validator spent real budget on a probe); a side file for the declaration (rejected by the owner: the spec declares it). | §2, repo `AGENTS.md` (validation line), `reports/usage/m2.md` (`self-check missions`), BACKLOG B-072 | done (`m3-integrate-m2`) |
| F-043 | 2026-10-24 | Owner: rule 7 amendment, supersedes and formalises F-034. Appended verbatim to `ai/RULES.md` rule 7: "Usage reports also carry input, output and cache-read tokens per role for the milestone, taken from ~/.factory/logs/droid-log-single.log (per-turn `[Agent] Streaming result` lines joined to sessions by the embedded tags.sessionId and modelId; the same-second metrics join is the fallback), with the first and last line timestamps used, never from memory or estimate." Every usage report from M2 on carries the table; `reports/usage/m1.md` gained it at this integration (its workers row was corrected on the way, see the backfill note in that file). **Alternatives considered:** keep the F-034 wording (rejected: it joined by the neighbouring metrics lines only). | `ai/RULES.md` rule 7, `reports/usage/m1.md`, `reports/usage/m2.md` ("Tokens per role"), `scripts/usage-from-log.sh` | done (`m3-integrate-m2`) |
| F-044 | 2026-10-24 | Owner: orchestrator spend reporting. From M2 on the usage report splits the orchestrator's milestone spend into `waits` (`WaitForScript` re-issues, structural: the tool returns after about five minutes and takes no timeout), `decisions and boundary reports` and `narration`, from the droid log, and states the COUNT of re-issued waits and their token cost as its own line (M2: 121 waits, 110 on `claude-fable-5.1` and 11 on `kimi-k3`; the fable waits cost 86.49 USD-eq of 118.84). An orchestrator M2 spend above 30 USD-eq stops the mission at the M2 boundary before M3 is dispatched and is the first line of the boundary report (it is over, see the M2 row). **Alternatives considered:** a per-milestone share (retired by F-040). | `reports/usage/m2.md` ("Orchestrator spend split"), `integration-worker` skill step 4 | done (`m3-integrate-m2`) |
| F-045 | 2026-10-24 | Owner, usage reconciliation, text as given: "Mission Control shows USAGE 37 % for this mission at 11:03 on 24-oct. The usage reports on the mission branch (reports/usage/s.md, m0.md, m1.md) add up to about 440 USD-eq including the orchestrator session, which is 70 % of the 630 hard stop. In the M2 boundary report state, from the record (rule 7 and F-034, droid log usage lines): mission-to-date spend by role (orchestrator, workers, validators, orchestrator subagents, self-check missions) in credits and USD-eq; what Mission Control's percentage is measured against (the mission budget, the plan window, credits of workers only); and the gap between the two numbers with its cause. If the gap cannot be explained from the record, say so; do not reconcile by estimate. Until explained, the hard stop is judged against the record, not the gauge." `reports/usage/m2.md` carries the section `## Reconciliation with Mission Control` with the four parts (mission-to-date table, basis of the gauge, gap, hard-stop line). Result: record 694.66 USD-eq (110.3 % of 630) by the droid log, gauge basis `basis not determinable from the record`, `gap not explained from the record`. **Alternatives considered:** reconcile by a scale factor (rejected by the owner: no estimates). | `reports/usage/m2.md`, rule 7 | done (`m3-integrate-m2`); the gap stays open |
| F-046 | 2026-10-24 | Model invariant, episode 4 (owner: "we have a rule not to change the mission models"; item 10). The Droid Model-Router switched live sessions of this mission to `kimi-k3` (fireworks) by reason `overage_reactive_402` (source `detector_recovery`) and `overage_proactive`. **First occurrence:** the orchestrator at 07:41:41Z, reverted manually at 07:45:51Z (4 min). **Second:** at 12:03Z every live session (the orchestrator was back on `claude-fable-5.1` at 12:04:43Z by a manual transition, switched again at 12:21:58Z by `overage_proactive`, locked anthropic/bedrock from 13:49:14Z); sessions spawned 12:03 to 13:49Z inherited `kimi-k3`: scrutiny round 2 `b34ee5bc` (PASS voided, re-run), user-testing round 1 `2dc80c60` (self-invalidated), orchestrator subagents `87ab4c99`, `0447d54a`, `9b08f77e`, `f4033628`, `c23b9c08` (their text outputs were reviewed by the orchestrator and kept; recorded, not redone), the five scrutiny round-1 reviewers (`c9027ff9`, `9972eff5`, `4d9abbc9`, `964313c4`, `cc2cff75`) switched at 12:03Z, and mid-run `overage_proactive` switches (fix worker `f4033628` 12:30:12Z, `b34ee5bc` 12:32:43Z, `2dc80c60` 12:39:02Z). Void sessions were re-run (`m2-fix-d032-alternatives` verify-and-complete on `claude-sonnet-5-5` `eb8cd3fd`; scrutiny round 4 full re-review; user-testing round 2). **Commit `8ab88e2`** was authored on `kimi-k3` (`f4033628`) and accepted through the `claude-sonnet-5-5` verify-and-complete `eb8cd3fd` with no new commit; its body ends with a stray heredoc tail (`MSGEOF && git log -1 ...`), so git does not parse its `Co-authored-by` trailer; history is forward-only (rule 12), so it is recorded here and in the M2 integration commit message; the commit-message-in-a-file rule is in the core and ui worker skills. **Rule reaffirmed:** at a throttle the mission WAITS; a model switch is never accepted. **Prevention** (mission `AGENTS.md` "Other rules", MODEL CHECK): every worker and validator reads `model` in its own `.settings.json` as its first action and hands off `blocked` on a mismatch; the orchestrator checks the effective model of every handoff session. **Usage** of the void sessions is the `model fallback (void)` line of `reports/usage/m2.md`, by session id, counted toward the M2 cap. **Alternatives considered:** accept the output of the fallback model after review (rejected: the model per role is an owner invariant, rule 19). | §2, `reports/usage/m2.md` ("Model fallback (void)"), mission `AGENTS.md` | recorded (`m3-integrate-m2`) |
| F-047 | 2026-10-24 | Orchestrator decision (rule 24, D-026), `m2-rpc-source-core` follow-up `a4ed7c9`: mapping v1 treats `role = system` messages as known lines that map to no event; the golden is unchanged. **Alternatives considered:** keep them Unknown (rejected: rule 18 counts unknown lines, and a hosted stream carrying them would not reach zero). | §5.2, `mapping::v1`, D-023 | done (`a4ed7c9`) |
| F-048 | 2026-10-24 | Orchestrator decision (rule 24), `m2-host-shell-and-launch` round 2 (`b3aa52a`, `b9cdd45`): Droid 0.247.0 ignores `initialize_session.missionSettings` for the mission's `model-settings.json`; the host sends `droid.update_session_settings` with the mission models before the first message. D-034 is amended. Hosted roles ran on the user-default models until this fix (one worker turn of about 11 s on `claude-opus-5-5`); those runs are labelled as such in `reports/usage/m2.md`. **Alternatives considered:** `initialize_session.missionSettings` alone (rejected: measured as ignored). | D-034, `reports/usage/m2.md` (self-check missions) | done (`m2-host-shell-and-launch`) |
| F-049 | 2026-10-24 | Rule 9 deviation record, validation contract: VAL-HOST-005 named a literal `--mission` launch command, and Droid 0.247.0 rejects it; the D-034 recipe (spawned `droid exec` with `--input-format stream-jsonrpc --output-format stream-jsonrpc`, the mission proposal through `droid.initialize_session`) supersedes it. Contract pass 21 (`contract-work/pass21-summary.md`) reworded VAL-HOST-005, VAL-HOST-006, VAL-HOST-007 and VAL-BOARD-038 to the as-built facts. **Alternatives considered:** change the code to the literal command (impossible: the CLI refuses it). | D-034, validation contract | done |
| F-050 | 2026-10-24 | Orchestrator decision (rule 24), user-testing round 2: `plan.revised` comes only from the file record `todo_state` (`mapping/v1/mod.rs:243-248`) and the hosted stream has no counterpart, so VAL-HOST-014 compares hosted and attached views excluding `usage.reported` and `plan.revised`. Rebuilding `todo_state` from the stream envelope is B-066. **Alternatives considered:** map `TodoWrite` tool calls now (rejected: sub-id collision risk, needs a probe); drop the assertion (rejected: the agreement of the two sources is the M2 exit criterion). | VAL-HOST-014, BACKLOG B-066 | done (contract reworded) |
| F-051 | 2026-10-24 | VAL-PROC-045: contract pass 11 fixed exact column names for the tokens table (`first ts`, `last ts`, `lines`, `input`, `output`, `cache_read`) after `1315ec2` was published; the names bind at the integration commit, so `reports/usage/m2.md` and the `reports/usage/m1.md` backfill use them. **Alternatives considered:** regenerate the M1 table at the old names (rejected: the table did not exist at M1). | VAL-PROC-045, VAL-PROC-046, `reports/usage/` | done (`m3-integrate-m2`) |
| F-052 | 2026-10-24 | Orchestrator decision (rule 24), `m2-fix-process-exit-outcome` (`36a042e`): the hosted `process_exit` outcome maps like `cancelled`: the child task gets `task.skipped` and the root gets `mission.paused`. **Alternatives considered:** no event (rejected: it hides the end of the last turn). The canonical `Event` has no payload field, so a host kill and a user cancel are not told apart by the event itself (BACKLOG B-074). | §5.2, `mapping::v1`, BACKLOG B-074 | done (`36a042e`) |
| F-053 | 2026-10-24 | Owner decision 31 as built (D-032): a `feature.defined` from a scrutiny validator belongs to the Scrutiny lane and one from a user-testing validator to User testing (the validator's phase), not Plan. Golden regenerated, sha refrozen in §10 (`cecf727d…`); whole golden Plan 744 and Scrutiny 157; no fixture event exercises the user-testing case (a hand-made role test does). The row written at `m2-integrate-m1` for the six `feature_defined` events is resolved, not a backlog item (B-042). The VAL-MAP-024 rule-24 decision (a usage turn's lane is the lane its session is in at that moment) stands. **Alternatives considered:** see D-032 (keep Plan, `Lane::home(role)`, a Build sub-lane, no event, a `mapping::v2`). | §5.2, D-023, D-024, D-032 | done (`m2-mapping-scrutiny-feature-defined`, `8ab88e2`) |
| F-054 | 2026-10-25 | Owner item 1 (M2 boundary), scratch missions for live self-checks, restating F-042 (confirmed, with the 10:24 phantom pause): a scratch mission declares its models explicitly and uses the cheapest models the catalogue allows for every role, never the launch form's defaults; it runs with `skipScrutiny` and `skipUserTesting` unless the validation path is the feature under test; it is namespaced (its own directory outside the worktree's build tree, session tag `scratch`) and its process group is reaped by the test that started it before the test reports; its usage is the separate line `self-check missions` of the milestone usage report, with the models used. The 10:24 phantom pause (`interrupt_session` at 08:24:51Z and 08:26:30Z) is a platform artefact in the session log, not a pause of this mission. The scratch model is `claude-haiku-4-5-20251001`, effort `low`, declared in §2 (owner item 2, confirmed). Factory feedback: a child mission's lifecycle events must not change the parent mission's state or interrupt its waits. **Alternatives considered:** the launch form's defaults (rejected: they hosted opus workers and `gpt-6.1-sol` validators in M2). | §2, repo AGENTS.md (validation), `reports/usage/m2.md` | done (F-042 row, §2 row, AGENTS.md line, session log entry 36) |
| F-055 | 2026-10-25 | Owner items 4 and 13 and the owner's answer 1 (23:52, 24-oct): decision (a), **M2 is accepted as spent, from the record**: 120.61 USD-eq against 50, orchestrator 118.84 against 30 (86.49 of it in 121 re-issued waits), 388.69 mission-to-date against 320; the overruns stay as deviations with their rows, nothing rewritten. Item 13 amended: **there is no hard stop**. There are stops and decisions at every milestone boundary, whenever a milestone reaches its cap (M3 110, M4 110, M5 60; orchestrator 30 per milestone) and when the record crosses 950 USD-eq: the mission stops before the next dispatch, reports from the record and the owner decides whether and under what caps it continues. Record at the M2 boundary: 694.66 USD-eq. The plan's limit preference is set so that at the limit the mission stops on its declared models (Extra Usage configured 2026-10-24 18:4x); the owner resumes at the reset, models read back first. **Alternatives considered:** keep 630 as a hard stop (rejected by the owner: the record was already at 110 % of it). | §2 (caps table and "Caps and stops"), `reports/usage/` | done (§2 edited here) |
| F-056 | 2026-10-25 | Owner item 6 (rule 25's mechanism, still not landed after three days; between 14:44 and 16:37 the orchestrator re-issued `WaitForScript` every five minutes, at least thirteen identical calls, while a validator ran): start the mission run as a background Script (`startInBackground`) and end the turn with one line; the platform delivers the script's output as a message when the run returns (seen 23-oct 19:11, "Script Output from toolu_…") and the orchestrator acts then. Zero turns while a worker runs; **no `WaitForScript` at all**. Applied before M3's first dispatch; the M3 usage report must show zero re-issued waits (owner answer 1). Recorded in repo AGENTS.md under the mission loop. **Alternatives considered:** a longer `WaitForScript` (impossible: it takes no timeout and returns after about five minutes, so each re-issue rewrites the 5-minute prompt cache: 121 waits, 86.49 USD-eq in M2). | rule 25, repo AGENTS.md (mission loop), `reports/usage/m3.md` | done (docs); proven by the M3 usage report |
| F-057 | 2026-10-25 | Owner item 7 (R-039), the model invariant after a 402, the M2 episode 4 (F-046): after any 402 or resume, **the first action of every session** (orchestrator, worker, validator, subagent) is to read its own `model` from `~/.factory/sessions/-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/<session-id>.settings.json` and compare it with `model-settings.json`; on a mismatch it stops with one line and does nothing else; a ruling or a dispatch made on a non-declared model is void. The handoff pastes `model = <value>`. The M2 usage report lists every `kimi-k3` turn with its session, role and tokens as its own line. The validator's self-stop (`2dc80c60`, 46 min) and the scrutiny round-3 FAIL-by-provenance were the rule working. Verify on the next 402 that the plan's limit preference stops the mission on its declared models (item 13) and record the result in this row. Factory feedback: the platform default at recovery must be "wait on the declared model". Routing rule: a provider route that fails is retried on the declared model with backoff; a rotation to a route that answers 403 is not a reason to switch. | rules 19, 20, repo AGENTS.md (mission loop), `reports/usage/m2.md` | done (docs); the next-402 verification is closed at the M3 boundary (F-132) |
| F-058 | 2026-10-25 | Owner item 8, feature size cap: `m2-host-shell-and-launch` took two rounds, 445 worker turns (266 in `6690e7b0`, 179 in `a34f0166`) and about 60M cached tokens read (36.07M and 24.27M, `reports/usage/m2.md` "Feature size"). Rule: a feature is sized so one worker finishes it in one round under 120 turns; a feature estimated above that is split at the contract pass before dispatch; a worker that crosses 120 turns hands off `partial` at the next checkpoint instead of continuing. Applied when planning M3 and M4 and in the feature template. **Alternatives considered:** a turn cap enforced by the platform (not available; the worker counts its own turns and the usage report states turns per feature). | §3, feature template, `reports/usage/m2.md` | done (docs); M3 and M4 plans open |
| F-059 | 2026-10-25 | Owner item 10: commit `8ab88e2` carries a stray heredoc tail that breaks trailer parsing. Forward only (rule 12): not rewritten. The commit template is fixed so trailers are written from a file: `.git-trailers` (the `Co-authored-by` line), `.gitmessage` (the shape), the message written to a file and committed with `git commit -F`, trailers added with `git interpret-trailers --in-place --if-exists addIfDifferent --trailer "$(cat .git-trailers)" <file>`; `git config commit.template .gitmessage` once per clone (local config). Proof: `git log -1 --format='%(trailers:key=Co-authored-by)'` on the commit. | repo AGENTS.md (Git), README, `.git-trailers`, `.gitmessage` | done (`fffb36e`) |
| F-060 | 2026-10-25 | Owner item 12, vocabulary (contract change, small): the role that runs a cycle is the "officer" from now on, not the "commander". The watch-list contract field becomes `officer` (was `commander`); both are accepted on intake for one milestone, `officer` is written. Applied in `m3-cycle-template-refactor` with the other nouns; the D-row that records the rename is written by that feature (no row beyond it). | watch-list contract, PRD notes, `m3-cycle-template-refactor` | open (D-row at `m3-cycle-template-refactor`) |
| F-061 | 2026-10-25 | Owner item 14, decision: window-only assertions get a deterministic harness (applies to `m3-controls-ui-and-stall` and to the 8 assertions still blocked in M2). The validator stops driving the window with an LLM over AppleScript accessibility calls. Instead a WebdriverIO end-to-end suite with `@wdio/tauri-service` and `driverProvider: 'embedded'` (the embedded WebDriver server from `tauri-plugin-wdio-webdriver`, compiled in for test builds only, plus `tauri-plugin-wdio` for command mocking), the route the Tauri docs give for macOS (`tauri-driver` has no WKWebView driver; CrabNebula's macOS driver needs a paid key). The suite covers panel render, Approve/Deny, the AskUser sheet and arrival stamps; the latency numbers (VAL-HOST-009/013) are emitted by the app into the record and asserted by the suite, never read off a screen. It runs as `make e2e` in the attested CI run once per milestone and locally on demand; the user-testing validator reads its JUnit report and screenshots and grades, it does not click. The 8 blocked M2 assertions are re-graded from the first green `make e2e`. Versions and commands: Tooling section. The decision file is written by `m3-e2e-harness`. **Alternatives considered:** see the Tooling section "Not adopted". | Tooling, §3 M3, validation contract, `m3-e2e-harness`, `m3-controls-ui-and-stall` | open (`m3-e2e-harness`) |
| F-062 | 2026-10-25 | Owner item 15, rule 26 (universal counterpart R-051, same text): tooling is looked up, not remembered. Before the first contract of a milestone the orchestrator reads the milestone's cycle of work and, for every check it will run, looks up the current tooling and its documented use in Context7 and the official docs and chooses the most efficient way: a deterministic tool first, a script over its output second, agent judgement last. The choice goes in SPEC.md `## Tooling` (tool, version, source, date, decision); validators run what the section says and never improvise a harness; the section is revisited at every boundary and in the post-mortem, where "validation overhead" is a named waste cause. Written here for M3 from `MD/research/tooling-section-draft.md` (Context7 was not available; official docs, npm, crates.io and GitHub, 2026-10-25); item 14 is its first entry. ai/RULES.md rule 26, repo AGENTS.md (validation). | `## Tooling`, ai/RULES.md, repo AGENTS.md | done (`bb1dd40` rules; Tooling section here) |
| F-063 | 2026-10-25 | Owner item 16, rule 27 (universal R-052, edition M-018): gate results are cached by tree hash. A gate run is keyed by the content it proves (the worktree's `git write-tree` including uncommitted changes, the toolchain version, the command); its result (exit code, summary lines, duration, time, session) is written to `reports/gate/<tree>/<command>.json`; `make check`, `make test`, `make lint` and the coverage target look the key up first, print "cached, proven at <time> by <session>" on a hit and exit with the stored code; `--force` reruns. Validators and the integrator accept a hit as evidence (R-047) and rerun only on a miss; the usage report states gates run and served from cache with the minutes saved. The M2 record: 180 gate-type commands over 31 sessions as the owner counted (104 `cargo test`, 29 `make check`, 25 `cargo check`, 7 clippy, 7 `make test`, 5 `make lint`, 3 `llvm-cov`) against about eleven distinct tree states, the full gate at least eighteen times on a tree already proven; the post-mortem reproduces 144 commands over 25 sessions from the build-log and the transcripts. Built by `m3-gate-cache` as the second entry of the Tooling section; the first integration of M3 proves it. | rule 27, Tooling, `m3-gate-cache` | open (`m3-gate-cache`) |
| F-064 | 2026-10-25 | Owner item 17, rule 28 (R-053): the gate flow per milestone. One proof: the worker's final tree gets one full gate run, recorded (F-063). Validators read: scrutiny and user testing grade from that record and run no gate of their own; their work is judgement and the checks the gate cannot do. Fixes run the delta (the touched crate's tests, the affected checks), recorded against the new tree, and validators re-check that delta only. Integration is the final run: one full gate on the merged tree; green lands, red goes back to fixes, never a rerun hoping for green. Two full gates per milestone is the target; the M3 usage report states the count. | rule 28, validator skills, `reports/usage/m3.md` | done (docs); proven by the M3 usage report |
| F-065 | 2026-10-25 | Owner item 18: the usage report is a tool, not an exploration. The M2 integration spent most of an hour deriving per-role tables with ad hoc greps, one question per turn. From M3 `make usage M=<milestone>` (the usage sidecar scripts, finished) writes the tables of `reports/usage/<milestone>.md` in seconds from the log and the session files, with source line and timestamps per row (rule 7): tokens per role and per session, the orchestrator split (waits re-issued, decisions, narration), subagents and self-check missions as their own lines, gates run and served from cache, validation-overhead minutes, mission-to-date by role and the gauge reconciliation. The integrator runs it and writes the prose; it never computes a number by hand. Built by `m3-usage-target`; Tooling row "Usage report". | Tooling, `m3-usage-target`, `scripts/usage-from-log.sh` | open (`m3-usage-target`) |
| F-066 | 2026-10-25 | Owner item 19: integration becomes a target. `make integrate M=<milestone>` does the mechanical part: the `--no-ff` landing; the final gate as a cache lookup when the merged tree equals the validated tree, the real run only when the branch moved; the usage report and the evolution-log row from the record by tool (F-065); the §9 rows, D-row amendments and deferred clauses merged from the staging files every worker and validator writes at hand-off (`reports/staging/<feature>.md`; sections `## SPEC §9`, `## Decisions`, `## BACKLOG`, `## Deferred clauses`, `## Library`); readiness measurements taken from the proof run's gate record; the commit message from a template with trailers from `.git-trailers`. The integrator session runs on the worker model, reads the merged reports and writes the prose; it computes nothing by hand. Target: integration under fifteen minutes of model time, stated in the M3 usage report (M2: 200.6 min of wall clock). Built by `m3-integrate-target`, proven by `m4-integrate-m3`. | Tooling, `m3-integrate-target`, `reports/staging/` | open (`m3-integrate-target`) |
| F-067 | 2026-10-25 | Owner answer 3 (23:52, 24-oct): **gauge = account weekly allowance, not reconcilable to a mission.** Mission Control's USAGE percentage is the account's plan allowance for the rolling week, not this mission's budget (83 % of the week at 22:0x on 24-oct with other sessions counted in it). The mission is judged on the record only; the reconciliation of F-045 stops trying to explain the gap; `reports/usage/m2.md` part (2) and (3) keep their wording as written at the time. | F-045, `reports/usage/` | done |
| F-068 | 2026-10-25 | Owner answer 4: the CI budget is 3 minutes with the B-056 trim, measured on M3's integration run; if two consecutive warm runs exceed it, it is raised to 5 and the reason is recorded. Replaces the open question of F-039 for M3. **Alternatives considered:** raise to 5 now (rejected by the owner until two warm runs fail the 3). | rule 11, B-056, `reports/ci/` | open at the M3 boundary: one warm run 221 s (over 3 min), the B-056 trim not applied, the second warm measurement is the first code push of M4 (`reports/ci/2026-10-25-m3.md`) |
| F-069 | 2026-10-25 | Owner answer 6: the UI of the M3 and M4 screens (`m3-controls-ui-and-stall`, `m4-gate-ui-and-merge`, and the existing board where touched) follows `ai/design/ui-kit-v2/` (README, `theme.css`, `donut.js`; committed as `bb7be45`): tokens, type, spacing, the five components and the donut as written there, applied to the app's existing structure. No new navigation, no brand work. A screen that departs from the kit is a deviation with its reason. `wt/m3` is refreshed from the mission branch before the first M3 dispatch. | `ai/design/ui-kit-v2/`, `m3-controls-ui-and-stall`, `m4-gate-ui-and-merge` | done (kit committed; `wt/m3` fast-forwarded by this pass) |
| F-070 | 2026-10-25 | Owner item 9 and answer 2 (supersedes the VAL-HOST-014 part of F-050): VAL-BOARD-038 and VAL-HOST-014 were fixed in M2 in the function, not at the call site (the resync recomputes `byte_len` for every retained pointer, `m2-fix-first-line-pointer`); VAL-HOST-018 and VAL-PROC-045 went to `m3-integrate-m2` (D-034 records the absence of permission events with evidence; the `1315ec2` table is superseded by a note, history forward-only); the 8 window-only assertions passed in user-testing round 3 once the display was booked. **Answer 2: VAL-HOST-014's reword is not kept.** A contract is not reworded to pass; hosted and attached views must agree. M3 gets a fix feature (`m3-hosted-todo-state`) that reads `todo_state` from the session file in the hosted mapper as the attached one does; VAL-HOST-014's original wording is restored in contract pass 23 and re-graded from that feature's proof. **Alternatives considered:** keep F-050's exclusion of `plan.revised` (rejected by the owner). | F-050, VAL-HOST-014, `m3-hosted-todo-state` | open (`m3-hosted-todo-state`) |
| F-071 | 2026-10-25 | F-028 amendment applied: every control is exactly one event with `overlay = Human`, `role = Human`, `source = App { action_id }` on the template's first lane (pause, resume, message, answer, mark complete, mark skipped) or the request's lane (approve, deny). Tests renamed as the amendment says. Merge (M4) is not a `HumanAction` variant yet; it takes `template.final_lane()` when it lands. | m3-control-core | done (m3 integration) |
| F-072 | 2026-10-25 | Host acks (orchestrator note after m2-host-shell-and-launch): `control::ack_event` builds the overlay event of the host's acknowledgement (`SourcePtr::Rpc` of the ack line of the spool, arrival-time `at`, id `<session>:<offset>:2^30`). Kinds: pause `mission.paused`, resume `mission.resumed`, message and mark skipped `intent.amended`, answer `intent.answered`, approve `permission.granted`, deny `permission.denied`; mark complete has no host ack. `control::dedupe_paused` drops the ack-derived `mission.paused` when the stream carries its own before the next `mission.resumed`. The `intent.amended` choice for message and skip is mine: no new kind is invented (architecture §8) and the PRD names no kind for a message. | m3-control-core | done (m3 integration) |
| F-073 | 2026-10-25 | The contract pass can reword VAL-CTRL-003/006/007 to the ack events above (ids from `ack_event`, `at` = arrival time of the ack). | m3-control-core | done (m3 integration) |
| F-074 | 2026-10-25 | `FeatureActivity` gained `repeated` and `child_growing` (the architecture shows three fields): the runbook table needs "HookBlocked repeated" and "SubagentSpawned with no child growth", which the last kind alone cannot say. `stall::activity_of` fills `repeated` and leaves `child_growing = false` (a growing child has events of its own, so the spawn is not the last event); the UI feature may feed the child-file growth from the watcher. | m3-control-core | done (m3 integration) |
| F-075 | 2026-10-25 | `stall::activity_of` treats a feature as finished (never idle) when its last event is `task.done`, `task.skipped` or an app event naming the feature (mark complete or skipped). | m3-control-core | done (m3 integration) |
| F-076 | 2026-10-25 | the six controls are one Tauri command, `human_action(mission_id, launch_id?, action, text?)`. With a `launch_id` (hosted) `Hosts::human_action` writes the control to the process and then records the `human_actions` row and its human-overlay event; a control the process cannot take (a request already answered) leaves no row. Without it (attached) the event is recorded and the window shows `TERMINAL_INSTRUCTION` plus the text to paste (`view::ControlBar::attached`, `view::terminal_text`); no `host_control` line is written. Mark complete reaches no host (the row and the event only); mark skipped is a `droid.add_user_message` carrying the feature id and the reason. | m3-controls-ui-and-stall | done (m3 integration) |
| F-077 | 2026-10-25 | acknowledgement pointers (contract pass 24). The host feeds `control::ack_event` with the spool line that answers the control: for pause, resume, message and skip the response line whose `id` equals the request id of the `host_control` it wrote; for approve and deny the `permission_resolved` notification line (`params.notification.type = "permission_resolved"`, `requestId` = the JSON-RPC id of Droid's `droid.request_permission`); for an answer the `tool_result` line of the AskUser tool call (`call:<toolCallId>`). The ack event id is `<session>:<byte_offset>:1073741824`; kinds seen live: `intent.answered`, `intent.amended` (message and skip), `mission.paused`, `mission.resumed`, `permission.denied`, `permission.granted`. `ingest::rpc::resolution_of` classifies the two resolving shapes. | m3-controls-ui-and-stall | done (m3 integration) |
| F-078 | 2026-10-25 | the host synthesises the `permission.requested` event that approve and deny point at (`control::permission_requested_event`, id `<session>:<offset>:1073741825`, a human overlay on the lane of the latest non-overlay event of the stream, else the template's first lane). Closes B-075 for Approve, Deny and Resume (the acknowledgement events above are the `permission.granted`, `permission.denied`, `mission.resumed` rows). | m3-controls-ui-and-stall | done (m3 integration) |
| F-079 | 2026-10-25 | the stall flag has a feature id `"mission"` (`stall::MISSION_LEVEL`) when the mission itself waits on the human (an unanswered question or permission request with no feature). `stall::activity_of` flags it after the threshold like any idle feature; the prompt is the matching `RunbookPrompt` ("Waiting on human: answer it", `#waiting-on-human`). | m3-controls-ui-and-stall | done (m3 integration) |
| F-080 | 2026-10-25 | `[intervention].stall_poll_s` (default 15, `config/observatory.example.toml`): how often the app evaluates `stall::evaluate` over the live missions. The app logs `stall_flag` (mission_id, feature_id, idle_for, prompt, anchor) and emits `stall_flag` to the window only when the set of flags changes; nothing under the threshold is ever reported. | m3-controls-ui-and-stall | done (m3 integration) |
| F-081 | 2026-10-25 | VAL-UI-001 applied as the owner's answer 6 asked: `ui/theme.css` is a byte copy of `ai/design/ui-kit-v2/theme.css`; `ui/style.css` is token-only (no colour literal, no `font-family`; greps: 0 and 0); the kit classes `kpi`, `lib`, `chip`, `switch`, `card` are used by the views; no new navigation, no brand work; a theme switch (System, Dark, Light) is the kit's `switch`. The departures from the kit's "Eight rules" (rule 3 board page width, rule 4 page head) are the two proposed rows in `reports/staging/m3-fix-ut-ui-kit-logs-and-attached-e2e.md`; numeric cells (rule 2) and the `.lib` boxes (rule 6) are met there. The `donut.js` component is not used: this feature draws no share-of-total chart. | m3-controls-ui-and-stall | done (m3 integration) |
| F-082 | 2026-10-25 | the live self-check harness (`live_check.rs`) takes every control through `Hosts::human_action`, runs the stall poller and installs a tracing subscriber (closes B-067); `OBSERVATORY_LIVE_HOLD_S` keeps the question unanswered for N seconds to show the stall flag. | m3-controls-ui-and-stall | done (m3 integration) |
| F-083 | 2026-10-25 | `e2e` cargo feature: `tauri-plugin-wdio-webdriver` 1.5.1 is an optional dependency behind `e2e = ["dep:tauri-plugin-wdio-webdriver"]`, used only under `#[cfg(feature = "e2e")]`. The plugin needs no capability permission (`wdio-webdriver:default` is not in `capabilities/default.json`; `rg -c wdio` there is 0). A release build has no server: `strings release/observatory-app \| grep -c TAURI_WEBDRIVER_PORT` is 0, the e2e debug build prints 1. Alternative: the `tauri-driver` path, rejected in the research note (needs a second binary). | m3-e2e-harness | done (m3 integration) |
| F-084 | 2026-10-25 | WDIO pins: `webdriverio`, `@wdio/cli`, `@wdio/local-runner`, `@wdio/mocha-framework`, `@wdio/spec-reporter`, `@wdio/junit-reporter` 9.32.0, `@wdio/tauri-service` 1.5.1 with `driverProvider: 'embedded'`. `tauri-plugin-wdio` is not installed: no spec uses `browser.tauri.*`. | m3-e2e-harness | done (m3 integration) |
| F-085 | 2026-10-25 | `@wdio/tauri-service` focus workaround: the service checks the window state before every find and click through `browser.tauri.execute`, which needs `tauri-plugin-wdio` and cost 5 s per command. The `before` hook calls `switchToWindow(getWindowHandle())`, which turns that recovery off. If the service changes, remove the hook. | m3-e2e-harness | done (m3 integration) |
| F-086 | 2026-10-25 | `scripts/check-file-size.sh` exempts `*/package-lock.json` (the e2e lockfile is over 200 KB; `npm ci` needs it committed). Alternative: raising the global limit, rejected. | m3-e2e-harness | done (m3 integration) |
| F-087 | 2026-10-25 | `.markdownlint-cli2.yaml` ignores `**/node_modules/**` (was root-only): the pre-commit markdown hook linted 10 108 issues in the e2e dependencies once `npm ci` had run. | m3-e2e-harness | done (m3 integration) |
| F-088 | 2026-10-25 | `.github/workflows/e2e.yml`: macOS job, `workflow_dispatch` only, outside `mission-branch.yml` and its 3-minute budget. Pinned: trunk 0.22.0, tauri-cli 2.12.4, `actions/setup-node` v7.1.0 by SHA. It runs `make e2e` and uploads `reports/e2e/**`. `make check-workflows` exits 0. No run has been dispatched (see Deferred clauses). | m3-e2e-harness | done (m3 integration) |
| F-089 | 2026-10-25 | `panel_rendered` (architecture §7) takes an optional `ms: f64`: the UI sends its measured `panel:render` duration and the host writes it as the `panel_rendered` interval of `observatory::timing`. Added to `architectureNotes`. | m3-e2e-harness | done (m3 integration) |
| F-090 | 2026-10-25 | Latency limits live in `config/observatory.example.toml` `[e2e]` (`panel_render_p95_ms` 3000, `event_first_on_board_p95_ms` 500, `host_append_p95_ms` 300, `event_visible_max_ms` 1000, `min_samples` 3), each with its reason. `E2eConfig` in `observatory-core`; the specs read them through `lib/thresholds.ts`. | m3-e2e-harness | done (m3 integration) |
| F-091 | 2026-10-25 | Locked display: see Deferred clauses; the README states the consequence (DOM reads do not need a visible window, screenshots come from an attended run or the CI runner). | m3-e2e-harness | done (m3 integration) |
| F-092 | 2026-10-25 | VAL-PROC-052 correction. Repo AGENTS.md and `reports/README.md` labelled rule 27 as the staging-file rule; rule 27 (`ai/RULES.md`) is the gate cache by tree hash, so AGENTS.md had no gate-cache rule. The (27) bullet now names the gate cache by tree hash (`reports/gate/<tree>/<command>.json`, a hit is evidence, `--force` reruns), (28) stays the gate flow, and the staging-file line stays under F-066; `reports/README.md` says "F-066; the gate cache is rule 27". The same edit replaces the two wait sentences (the Waits line and the rule 25 paragraph) that allowed one blocking call and re-issued early-returned waits with the F-056 wording: the orchestrator starts the run in a background Script, ends its turn, acts when the output is delivered, and makes no `WaitForScript` call. `.gitignore` gains `vscode_target/` (user data in the main checkout, never committed or touched), `__pycache__/` and `*.pyc` (a reviewer's `scripts/__pycache__` moved the gate key during scrutiny). Test `scripts/tests/agents_md_test.sh`, part of `make check-scripts`. | m3-fix-agents-md-rules-text | done (m3 integration) |
| F-093 | 2026-10-25 | an interrupted gate run is not a verdict. `scripts/gate.sh` handles HUP, INT and TERM by killing the command's process tree, removing its temp files and exiting 129, 130 or 143 before any record is written; the trap used to remove the temp files and let the script go on, so an interrupted run stored an exit 1, "(no output)" record that later runs replayed as a hit. The command now runs in the background and the script waits on it, because a shell defers a trap until its foreground pipeline ends. `scripts/tests/gate_test.sh` covers TERM and HUP (exit code, no record, command killed, no temp file left); it passes under `sh` and `dash`. | m3-fix-gate-trap-and-integrate-hit | done (m3 integration) |
| F-094 | 2026-10-25 | `scripts/integrate.sh` decides HIT or MISS from the check gate's own first line (`cached, proven at` against `gate check: miss\|forced`), not from a grep over the whole `make check` output, which the inner lint and test gate hits also print. A real check run whose inner gates hit is now `gate: MISS` with a reason; with an empty diff the reason says that no check record exists for the tree. `scripts/tests/integrate_test.sh` has the inner-hit, outer-miss scenario. | m3-fix-gate-trap-and-integrate-hit | done (m3 integration) |
| F-095 | 2026-10-25 | `scripts/integrate-staging.py` files every bullet and table row under `## SPEC §9` (also `SPEC §9 rows`, `SPEC s9`), `## BACKLOG` and `## Decisions`. A SPEC bullet is a row (a `Row (F-NEW):` marker is dropped); a BACKLOG bullet is the next B-row (`B-NEW:` or `New:` optional) unless it is a closure (`Closed...`, `Close ...`, `B-NNN: done`), which goes to the report; a Decisions bullet naming a D-NNN is an addendum of that file (the text after the `D-NNN (...)` prefix), unless it says the decision is new or that there is no amendment, which goes to the report with the bullets that name no decision. Deferred clauses and Library are copied per feature. A bullet or paragraph under any other `##` section, or an addendum whose decision file does not exist, makes the script print `unfiled: N` with the items, exit 1 and write nothing, so the staging file is fixed and the run repeated. | m3-fix-gate-trap-and-integrate-hit | done (m3 integration) |
| F-096 | 2026-10-25 | `integrate-staging.py --dry-run DIR` merges into copies of SPEC.md, BACKLOG.md and docs/decisions under DIR, keeps the staging files and writes DIR/report.md; it refuses a DIR that would overwrite a shared file. Dry run over copies of the 11 pending M3 staging files (everything under `reports/staging/` except the merged `m3-boundary-docs-m2`): SPEC §9 rows 42 (F-071 to F-112), BACKLOG rows 27 (B-078 to B-104), closures 4, decision addenda 5 (D-029 twice, D-033, D-034, D-036), decision notes 6, prose notes 3, deferred clauses 20, library 30, `unfiled: 0`. The integrator numbers the rows again when this file joins the batch. | m3-fix-gate-trap-and-integrate-hit | done (m3 integration) |
| F-097 | 2026-10-25 | `scripts/usage-report.py` filters every log line by its own timestamp instead of stopping at the first line past `--to`. `~/.factory/logs/droid-log-single.log` is not time-ordered (lines up to 48 s behind an earlier one), so the old stop dropped in-window turns and the cross-check against `scripts/usage-from-log.sh` read DIFFERENT (scrutiny reproduced 3 against 6 turns on session 2036e3c5). A line past `--to` is now skipped; the scan stops only at a line more than `LOG_REORDER_MARGIN_S` (600 s) past it. A line without a timestamp counts with the timestamped line before it. Real M2 window (`--from 2026-10-24T04:27:54Z --to 2026-10-24T19:36:40Z`): `cross-check: identical (97 sessions ...)`, orchestrator row unchanged (251 turns, 188317 input, 151058 output, 17784767 cache read, 118.84 USD-eq). Real M3 window (`--from 2026-10-24T21:48:56Z --to 2026-10-25T14:10:37Z`): `cross-check: identical (33 sessions ...)`, `re-issued waits: 0`. Test `test_a_line_logged_out_of_order_at_the_window_end_is_still_counted` over `scripts/tests/fixtures/usage/logs-out-of-order/`. | m3-fix-usage-log-order | done (m3 integration) |
| F-098 | 2026-10-25 | e2e CI trigger (rule 24, D-026): `.github/workflows/e2e.yml` runs on a pushed tag `e2e-*` plus `workflow_dispatch`, with `concurrency: {group: e2e-${{ github.ref }}, cancel-in-progress: false}`. The integrator pushes one tag per milestone at the integration commit (`git tag e2e-m3 <sha> && git push origin e2e-m3`; tags are never moved or deleted) and pastes the run id, conclusion, wall time and artifact name. The tag trigger reads the workflow file from the tagged commit, so it works before the file is on `main`. Alternatives: `workflow_dispatch` only (rejected: GitHub resolves it on the default branch, `gh run list --workflow e2e.yml` answered HTTP 404 and the read-only PAT cannot dispatch); `pull_request` to `main` (rejected: one run at the final pull request, not one per milestone); `push` on `mission/**` with a `paths` filter (rejected: fires on every code push, a hotfix after the merge commit would overlap or, with cancel-in-progress true, cancel the run at the integration sha). Changes neither scope, caps nor models. Cost: one macOS run per milestone (25 to 35 minutes cold, free on a public repository). | m3-fix-ut-gate-timing-e2e-trigger-usage | done (m3 integration) |
| F-099 | 2026-10-25 | VAL-TOOL-001: `scripts/gate.sh` starts `started_at` and the `seconds` clock at entry (before the tree computation and the tool-version calls), so `seconds` covers the whole script run. The record's `exit_code` stays the command's own code (make reports it as `Error N` and exits 2; contract C-3). | m3-fix-ut-gate-timing-e2e-trigger-usage | done (m3 integration) |
| F-100 | 2026-10-25 | VAL-TOOL-005 table gaps closed in `scripts/usage-report.py`: Feature size gains `Skill` (from the mission's `features.json`, listed in the Sources table), `First ts`, `Last ts` and `Source`; Subagents, Self-check missions and Mission to date gain a per-row `Source` column naming the log files; `--cap milestone=<USD>` and `--cap total=<USD>` compare against two new rows of the Mission to date table (`milestone window total`, the total row of Tokens per role; `total`, every role up to the window end) with `Over cap`. Proof: `make usage M=m3 CAPS='milestone=110 orchestrator=30'` run twice with `--to 2026-10-25T17:04:26Z` produced identical files (diff empty), 18.273 s and 18.077 s, cross-check identical (64 sessions). | m3-fix-ut-gate-timing-e2e-trigger-usage | done (m3 integration) |
| F-101 | 2026-10-25 | VAL-CYCLE-003: `crates/observatory-core/tests/cycle_template.rs` asserts the `four_step` template's lane count once (`template.lanes.len() == 4`, the template content check of VAL-CYCLE-001, in `a_second_template_with_four_lanes_maps_every_event_into_one_of_them`). Every board, digest, lane_events, lane_cards and lane-count comparison now uses `template.lanes.len()`. Beyond the three lines named in the feedback, `:291` (`counts.len() == 4`) and `:405` (`counts.len(), 4`) had the same literal. | m3-fix-ut-round3-lanes-and-mono | done (m3 integration) |
| F-102 | 2026-10-25 | VAL-UI-001 rule 2 (ids, event kinds and timestamps in mono): closed. The kit's `.mono` class (`theme.css:33`) is on the Details timestamp, event kind and model id cells (`details.rs`), the Panel declared and observed model cells (`panel.rs`), the Agents feature id, model and declared-model cells (`agents.rs`) and the Library date cell (`library.rs`). No CSS changed; `theme.css` is byte-identical to the kit. | m3-fix-ut-round3-lanes-and-mono | done (m3 integration) |
| F-103 | 2026-10-25 | proposed, owner approval at the M3 boundary: ui-kit-v2 rule 6 (table density, the kit's `.lib td { padding: 10px }`). Departure 1: `ui/style.css:9`, `.library th, .library td { padding: 6px 8px }`. Screen: Library. Reason: row density, the library lists every opened capture and the seven-lane board sits one click away; 10 px rows halve how many captures show without scrolling. Alternative: remove the override and take the kit's 10 px (rejected by the owner for now; the override stays). Cost: none. | m3-fix-ut-round3-lanes-and-mono | done (m3 integration) |
| F-104 | 2026-10-25 | proposed, owner approval at the M3 boundary: ui-kit-v2 rule 6, departure 2: `ui/style.css:35`, `.panel th, .panel td, .agents th, .agents td, .details th, .details td { padding: 3px 8px }`. Screens: Panel, Agents, Details. Reason: row density on the seven-lane board page, where the Panel, the agents table and the Details timeline (hundreds of rows) share one viewport with the lanes; 10 px rows would push the lanes below the fold. Alternative: remove the override (rejected by the owner for now). Cost: none. | m3-fix-ut-round3-lanes-and-mono | done (m3 integration) |
| F-105 | 2026-10-25 | proposed, owner approval at the M3 boundary: ui-kit-v2 rule 3 (one scale, page max width 1180 px). Departure: the app page (`.observatory` in `ui/style.css`) keeps `max-width: 1480px` so the board's seven lanes get room (at 1180 px each lane would be about 150 px wide). The Library box is 960 px and Details is a fixed overlay inset 3 vw, both narrower than 1180 px. Every font size in `style.css` is on the kit scale (11, 12, 14, 15, 22): `.lane-name` went from 13 px to 12 px in this feature. Alternative: 1180 px with horizontal lane scroll (rejected: the board is read across all lanes at once). Cost: none. Screen: the board. | m3-fix-ut-ui-kit-logs-and-attached-e2e | done (m3 integration) |
| F-106 | 2026-10-25 | proposed, owner approval at the M3 boundary: ui-kit-v2 rule 4 (every page starts with `.eyebrow`, an `h2` and the facts). Departure: M3 adds no page head or eyebrow; the screens keep their existing headings. Deferred to `m4-gate-ui-and-merge`, which adds the Gate screens and is the first feature that may add navigation (the kit's "No new navigation" applies until then). Alternative: add the head to every M3 screen now (rejected: it needs the products › runs › missions structure that does not exist). Screens: Library, board, Details, Agents. | m3-fix-ut-ui-kit-logs-and-attached-e2e | done (m3 integration) |
| F-107 | 2026-10-25 | VAL-UI-001 rule 2 and rule 6 are only partly met by this feature (corrected in round 3: identifier and timestamp cells were still sans, and two padding overrides remain; see `m3-fix-ut-round3-lanes-and-mono.md`, which closes rule 2 and records rule 6 as proposed departures). Done here: the Panel roles, Agents, Library and Details tables sit in the kit's `.lib` box (`.lib > .scroll > table`) and every numeric cell is `class="n"` (right-aligned mono `tabular-nums` from `theme.css`; `ui/src/panel.rs`, `agents.rs`, `library.rs`, `details.rs`). `style.css` gained no `font-family` and no colour literal; `diff ai/design/ui-kit-v2/theme.css apps/observatory-app/ui/theme.css` prints nothing. | m3-fix-ut-ui-kit-logs-and-attached-e2e | done (m3 integration) |
| F-108 | 2026-10-25 | F-041 css leftover closed: `.lanes` is `display: grid; grid-auto-flow: column; grid-auto-columns: minmax(0, 1fr)`, so a template with N lanes gets N equal columns and `ui/style.css` has no `repeat(7`. | m3-fix-ut-ui-kit-logs-and-attached-e2e | done (m3 integration) |
| F-109 | 2026-10-25 | VAL-CTRL-011 app-log redaction. The app log never carries the text of a message, answer or reason. `actions.rs` logs `action = ?action` (`HumanAction` holds `text_len` / `reason_len` only) on the `host_control` and `human_action` lines; `host.rs` logs the raw `HostControl` of the `host_control` command and of the first mission message through `actions::Redacted` (kind and `text_len`). Found beyond the analysis: `host.rs:134` and `:601` had the same leak (the second printed the mission file text). | m3-fix-ut-ui-kit-logs-and-attached-e2e | done (m3 integration) |
| F-110 | 2026-10-25 | the window suite has 26 cases (was 19): `attached.e2e.ts` 3, `controls.e2e.ts` 8 (+2 theme), `stall.e2e.ts` 4 (+2 theme), `hosted.e2e.ts` 7 and `panel.e2e.ts` 4 unchanged. | m3-fix-ut-ui-kit-logs-and-attached-e2e | done (m3 integration) |
| F-111 | 2026-10-25 | gate cache. `scripts/gate.sh <name> -- <command...>` keys a gate run by tree hash (working tree including uncommitted and untracked files, `build-log/` and `reports/gate/` excluded), tool version line and command string; a hit replays the stored verdict and exit code from `reports/gate/<tree>/<name>.json`, `--force` or `FORCE=1` reruns. `make check`, `make test`, `make lint` and `make coverage` go through it; `make check-gate-cache` is the self-test and `make check-scripts` (part of `make check`) runs it with the other shell tests. CI calls cargo directly and does not use the cache. | m3-gate-cache | done (m3 integration) |
| F-112 | 2026-10-25 | the key sequence in `library/tooling.md` is amended by one step. `git read-tree HEAD` brings the committed copies of `build-log/` and `reports/gate/` into the temporary index, and the `:(exclude)` pathspec of `git add -A` only stops working-tree changes, so committing a record or a build-log line changed the tree. The script runs `git rm -r -q --cached --ignore-unmatch -- build-log reports/gate` after `read-tree`. Validators reproducing the key by hand use the amended sequence (or `scripts/gate.sh --key`). | m3-gate-cache | done (m3 integration) |
| F-113 | 2026-10-25 | the gate hit log. Each cache hit of `scripts/gate.sh` appends one tab-separated line to `reports/gate/hits.log`: RFC 3339 UTC time, name, tree, the stored `seconds` of the record, the session that was served, its `session_source`. A miss and a forced run append nothing. The minutes a hit saved are the stored seconds of the run it did not repeat. `.gitignore` stops ignoring the file (it ignored `*.log`) and `.gitattributes` merges it with `merge=union`, so two worktrees that each logged hits merge by keeping both sets of lines. `scripts/tests/gate_test.sh` covers the line, the miss, the failing miss, the forced run and `FORCE=1`. | m3-gate-hits-and-usage-name | done (m3 integration) |
| F-114 | 2026-10-25 | `scripts/usage.py` is renamed `scripts/usage-report.py`, the name VAL-TOOL-005 and `library/tooling.md` use. `make usage M=<m>` and `make check-scripts` call the new name. | m3-gate-hits-and-usage-name | done (m3 integration) |
| F-115 | 2026-10-25 | the Gates table of the usage report has one row per gate and one total row for gates run (records `reports/gate/*/*.json`: count, seconds, minutes, command, first and last time) and for gates served from cache (`hits.log`: count, minutes saved), each with its source file. The detailed run list is the `### Gate runs` table. The line `served from cache: not recorded` is gone; `gates served from cache: N, minutes saved: M` replaces it. Real run for the M3 window (`--from 2026-10-24T21:48:56Z`): 4 gates run (120.291 s), 1 served from cache (0.29 min saved), `re-issued waits: 0`. | m3-gate-hits-and-usage-name | done (m3 integration) |
| F-116 | 2026-10-25 | reproducibility of the usage report. `Sidecar readings` states the number of sidecars read and their oldest and newest mtime, and the per-session table has a `Modified` column. `--help` states that a byte-for-byte rerun needs a `--to` at least 5 minutes before the generation time (log flush lag measured by m3-usage-target) and that Sidecar readings can move while a session is live. The `--to` default is unchanged. | m3-gate-hits-and-usage-name | done (m3 integration) |
| F-117 | 2026-10-25 | VAL-HOST-014 reword reverted on owner instruction; fixed in the mapper. The hosted stream has no counterpart of Droid's `todo_state` record (round 2 of `m2-host-shell-and-launch`: hosted 18, attached 21 = 18 + `usage.reported` 1 + `plan.revised` 2), so `RpcMapper::with_session_files(&cfg.ingest)` also reads the hosted child's session file (`<factory_sessions_dir>/<cwd-slug>/<session-id>.jsonl`, found by the session id of the `droid.initialize_session` response) and maps its `todo_state` records through the same mapping v1 state as the stream. The events keep the session file's `Jsonl` pointers, so they have the ids the attach path gives the same lines. The contract keeps its wording: hosted equals attached minus `usage.reported`. Alternatives considered: exclude `plan.revised` in the contract (rejected by the owner); rebuild a `todo_state` record from the `TodoWrite` `tool_use` input in the envelope (rejected: id collision on sub 0 with the message's own event, and the file's `todos` shape was not probed); a separate `Mapper` for the file records (rejected: it loses the model and feature state, so the event would differ from the attached one). | m3-hosted-todo-state | done (m3 integration) |
| F-118 | 2026-10-25 | ordering and dedupe of file records. A `todo_state` record is mapped before the first stream line whose own time is at or after the record's time (a line with no time of its own, a turn outcome, waits for the next one that has), and the rest at `finish`. Mapping v1 reports a plan only when it differs from the one before, so a plan carried by both the stream and the file, or a file read twice (Droid rewrites its first line in place and the tail reads the file again), maps once. Ids are `<session>:<file offset>:0`, equal to the attach path's, so the store holds the event once however many readers send it (`ON CONFLICT (id) DO NOTHING`). | m3-hosted-todo-state | done (m3 integration) |
| F-119 | 2026-10-25 | `make integrate M=<milestone> FROM=<branch>...` is `scripts/integrate.sh`. It refuses on a dirty tree (only `build-log/` may differ; stray files would turn a gate hit into a miss), merges each branch with `git merge --no-ff --no-edit -F` of a message rendered from `scripts/templates/integrate-commit.txt` (subject `Integrate <m>: <branch>`, the merged subjects as the body) whose trailers are added by `git interpret-trailers --in-place --if-exists addIfDifferent` from `.git-trailers`, runs the final gate as `make check` (so `scripts/gate.sh` prints `cached, proven at <time> by <session>` when the merged tree is a proven tree and runs for real on a miss, with `git diff --stat <branch> HEAD` as the reason), runs `make usage M=<m>`, merges the staging files, regenerates the decisions index and commits the boundary files with `scripts/templates/integrate-boundary.txt` (`chore(integration): <m> boundary reports and shared files`). A red gate prints `gate: RED` with the stored summary and exits 1 without a rerun; the merges stay (forward only) and a second run skips branches that are already merged, so the fix is a new commit and the same command. Alternatives considered: `git commit --trailer` (no such option on `git merge`); `commit.template` (git-commit: no effect with `-F`); the research's `sed` template rendering (replaced by `awk` so a branch name with `/` or `&` needs no escaping). | m3-integrate-target | done (m3 integration) |
| F-120 | 2026-10-25 | staging merge rules, `scripts/integrate-staging.py` (python3 stdlib). Files are read in name order. A bullet of `## SPEC §9` becomes the next `F-NNN` row of the SPEC §9 table (`Applies` = the feature, `Status` = `done (<m> integration)`, `\|` escaped). A `## Decisions` bullet `D-NNN (amend...): "text"` becomes a section `## Addendum <date> (<m> integration, <feature>)` at the end of `docs/decisions/D-NNN-*.md`; other bullets are only listed in the report. A `## BACKLOG` bullet starting `B-NEW` becomes the next `B-NNN` row (item and reason split at `reason:`, otherwise at the first sentence; `Earliest` = `unscheduled`, the feature id appended to the reason); other bullets (for example `Closed (do not file)`) are only listed. `## Deferred clauses` and `## Library` are copied per feature into `reports/readiness/<date>-<m>.md`, between `<!-- integrate:begin -->` and `<!-- integrate:end -->` (prose outside the markers is kept). Run on the six staging files now in this worktree (copies, nothing written): F-071 to F-083, B-078 to B-088 and three addenda (D-029 twice, D-033) came out as expected. | m3-integrate-target | done (m3 integration) |
| F-121 | 2026-10-25 | the boundary report copies the gate numbers from `reports/gate/<tree>/*.json` (name, exit code, seconds, finished_at, session, summary); nothing is measured again. | m3-integrate-target | done (m3 integration) |
| F-122 | 2026-10-25 | VAL-BOARD-040 store clause: a live-attached session has a `sessions` row from discovery (`record_tree` already ran on every new tree; a characterization test pins it) and its `tag` (the display name, D-036) follows the name: `LiveIngest::announce_changes` writes it through `Store::set_session_name`. The earlier staging row "the live store writes no sessions rows" was wrong about discovery and right about renames; this feature closes the rename gap. | m3-live-session-rows | done (m3 integration) |
| F-123 | 2026-10-25 | Name policy (`watch::SessionNames`): the first name seen is kept, one change is accepted, a second change or a runner tag (`[ingest].runner_tags`) is refused, each refusal reported once. `LiveStep.renamed` / `LiveStep.refused` carry the entries; the host logs `session_renamed` (old, new, at) at INFO and `session_rename_refused` (kept, attempted, at) at WARN next to `session_discovered`. | m3-live-session-rows | done (m3 integration) |
| F-124 | 2026-10-25 | After `record_tree` (a later session joined the tree) `LiveIngest` writes back the names it holds, so a refused change never reaches the store. | m3-live-session-rows | done (m3 integration) |
| F-125 | 2026-10-25 | No migration: `sessions.tag` already exists. Golden `.events.json` unchanged (events carry no session name). | m3-live-session-rows | done (m3 integration) |
| F-126 | 2026-10-25 | F-038 status: decided; code landed at `m3-session-name-feature-id` (D-036). Name rule: feature id, else user tag, else session id. The feature id is read from the user messages before the first assistant message (`assigned to execute feature "<id>"`). M0 capture: 15 of 56 sessions have one. Golden `.events.json` unchanged (sha256 `cecf727d0944ff7e8ddf306a8aeca6cbc32022bd2ff2eb7e22050fa7b72106c7`; events carry no session name). | m3-session-name-feature-id | done (m3 integration) |
| F-127 | 2026-10-25 | `make usage M=<milestone>` runs `scripts/usage-report.py` (python3 stdlib only) and writes the tables of `reports/usage/<milestone>.md` between `<!-- usage:begin -->` and `<!-- usage:end -->`; prose outside the markers is kept, a file without markers gets the block appended, a new file gets a title and the block. The script is `scripts/usage-report.py`, the name the contract (VAL-TOOL-005) and `library/tooling.md` use (renamed from `usage.py` by m3-gate-hits-and-usage-name). Sources: `~/.factory/logs/droid-log-single.log*`, the session sidecars, the orchestrator transcript, `progress_log.jsonl` (`OBSERVATORY_MISSION_DIR`, default the mission directory), `config/prices/<newest>.toml`, `reports/gate/*/*.json`. Window `--from` (default: first `worker_started` of `<milestone>-*`) to `--to` (default: generation time minus 1 s), compared at second resolution as `scripts/usage-from-log.sh` does. Caps are passed, not typed in code: `make usage M=m3 CAPS="orchestrator=30 worker=110"`. | m3-usage-target | done (m3 integration) |
| F-128 | 2026-10-25 | role classification is deterministic from the sidecar tags and `progress_log.jsonl`: orchestrator = `mission-session` role `orchestrator` of this mission; worker or validator = `mission-session` role `worker` of this mission, validator when the `worker_started` feature matches `^(scrutiny\|user-testing)-validator-`; subagent = `subagent` tag, role of the caller's root (an orchestrator's subagent is `orchestrator subagent`; a validator's reviewers and flow validators count as validators, as `m2.md` did); self-check = a `mission-session` of another mission id (the scratch missions) and its subagents; everything else is `other`. | m3-usage-target | done (m3 integration) |
| F-129 | 2026-10-25 | the orchestrator split joins each log turn of the orchestrator to the nearest top-level assistant record of the transcript within 3 s (the m2 method; 251 of 251 matched for M2). Class `waits`: the turn's tools are only `WaitForScript`, or only `StartMissionRun` straight after a wait turn; `decisions and boundary reports`: any of Edit, Create, Task, TaskStop, StartMissionRun, DismissHandoffItems, AskUser, ProposeMission, ExitSpecMode, ApplyPatch, a background Script, a shell call whose text writes (redirect, tee, git add/commit/merge/push, mv, cp, rm, mkdir, touch, sed -i), or no tool call at all (text to the owner); `narration`: the rest; `unmatched`: no transcript record. Tools inside a `Script` are read from its `tools.<Name>(` calls. The stated line is `re-issued waits: N`; `WaitForScript calls in the window` and `waits returning in under 60 s` follow it. | m3-usage-target | done (m3 integration) |
| F-130 | 2026-10-25 | reproducibility. Every table is bounded at `--to` (log lines, progress records, transcript records, gate records), so two runs with the same `--to` differ only in the `generated:` line, with one exception: the last section, `Sidecar readings` (credits by role and sidecar totals per session), reads lifetime totals of sessions that may still be running (the orchestrator, the integrator), so it can move between runs. The contract's "byte for byte" diff excludes that section (or runs when no session is live). A second limit: the Droid log is flushed late (a turn timestamped 11:26:55Z was absent when the script read the live log at about 11:27:15Z and present in the `usage-from-log.sh` pass seconds later), so a window ending within some tens of seconds of generation can grow on a re-run and the cross-check can print `cross-check: DIFFERENT` for the live session; use a `--to` older than that (the M2 acceptance below is stable). | m3-usage-target | done (m3 integration) |
| F-131 | 2026-10-25 | acceptance against `reports/usage/m2.md` (not edited). Same window `2026-10-24T04:27:54Z` to `2026-10-24T19:36:40Z`: orchestrator 251 turns / 188317 / 151058 / 17784767 (input / output / cache read) equals the m2 row, and `118.84` USD-eq equals the m2 log split. Summed over the session lists printed in `m2.md`, workers (9 sessions) 720 / 1504 / 615393 / 91429803 and validators (22) 412 / 908 / 503976 / 31990959 equal the m2 rows exactly. The script's own role rows differ from the m2 rows by two sessions groups, both stated in `m2.md`: the M1 integrator `cbdcd8ef` (39 turns in the window; `m2.md` says its M2-window turns belong to `m1.md`) is a worker here, and the `model fallback (void)` sessions (the `kimi-k3` reviewers and validators, the void worker `f4033628`) are in their roles here and listed again in the `Model fallback` table, where `m2.md` kept them in a separate line. `scripts/usage-from-log.sh --embedded` over the same window gives identical rows for all 97 sessions (`cross-check: identical`). | m3-usage-target | done (m3 integration) |
| F-132 | 2026-10-25 | F-046 and F-057 verification on the next 402 (the model invariant held). At 2026-10-25T15:19:50Z the weekly standard usage limit came inside `user-testing-validator-m3` (session `1653b1f7-c46c-4f4d-81d9-83d1f232e5b6`, `claude-opus-5-5`): "You've reached your weekly standard usage limit (resets in 1 day) and your Extra Usage balance is empty." The runner paused the mission with `pauseReason` `unrecoverable_usage_402` on its declared models: no Model-Router transition (`grep -c 'Model-Router.*1653b1f7' ~/.factory/logs/droid-log-single.log` = 0; the usage report's Model-Router table is empty and states `model fallback: none`). `MD/progress_log.jsonl`, lines 471, 472, 473, 474, 480: `{"timestamp":"2026-10-25T15:19:50.795Z","type":"worker_failed","workerSessionId":"1653b1f7-...","spawnId":"unrecoverable_usage_402_1653b1f7-..."}`; `{"timestamp":"2026-10-25T15:19:50.866Z","type":"mission_paused","pauseReason":"unrecoverable_usage_402"}`; `{"timestamp":"2026-10-25T15:19:50.964Z","type":"worker_failed",...,"failureReason":"worker_exited_without_handoff"}`; `{"timestamp":"2026-10-25T15:35:29.259Z","type":"mission_resumed","resumeWorkerSessionId":"1653b1f7-..."}`; `{"timestamp":"2026-10-25T15:36:39.146Z","type":"worker_failed","workerSessionId":"1653b1f7-...","spawnId":"orphan_1653b1f7-...","reason":"orphan_cleanup"}`. The paused session was not resumed in the end: the owner lifted the limit (Extra Usage added) and asked for a fresh `user-testing-validator-m3` run on `claude-opus-5-5` from scratch (`11bf22ab`, 15:36:45Z). Throttled time: 938.393 s (15.64 min, `reports/usage/m3.md`). Result: the plan's limit preference stops the mission on its declared models; the F-057 verification is closed. | F-046, F-057, `reports/usage/m3.md` (Throttled, Model-Router transitions) | done (m3 integration) |
| F-133 | 2026-10-25 | Owner feedback during M3 user testing, a cycle rule (kind: rule, scope: cycle, KB candidate for R-053): **re-test scope.** A re-test round re-tests the assertions that failed or were blocked, and nothing else, unless a fix changed a file that a passed assertion's own evidence depends on. A file being shared or touched is not enough. Before testing, the synthesis lists each re-tested assertion with the changed file that reopens it; a passed assertion with no such file keeps its verdict. **Receipt:** user-testing round 2 (`cc6ebacd` and its six flow validators) re-tested 23 assertions that had passed in round 1, the whole live-host group among them, against 6 of the 11 open ones: 45.63 min and 16.09 USD-eq (root 5.39 plus six flow validators 1.34, 2.33, 0.96, 2.85, 1.79, 1.43; `reports/usage/m3.md`), where the delta was about 15 minutes and 5 USD-eq. First application: round 3 (`m3-fix-ut-round3-lanes-and-mono`, then one validator session `33e778c8`, no flow subagents, no live groups, only VAL-CYCLE-003 and VAL-UI-001): 6.22 min and 1.90 USD-eq. | `reports/usage/m3.md` (Roles; Sessions), mission `AGENTS.md` (validation), owner feedback 2026-10-25 item 1 | done (m3 integration); R-053 text for the knowledge base |
| F-134 | 2026-10-25 | M3 integration facts. (a) `make integrate M=m3 FROM=wt/m3` could not start from the main checkout: the target did not exist on `mission/observatory-v1` before the merge (the Makefile that has it is in `wt/m3`), so the integrator ran `scripts/integrate.sh` from `wt/m3`; this first run is the VAL-TOOL-006 proof. (b) The gate was a MISS, not a hit: `ce0b143` (the round-3 fix) landed after the full gate on `a92c525`, so `make check` ran for real on the merged tree `0c7b6f23` (195.592 s, exit 0) and again on `a3adb9b3` after the staging-docs commit `8ae8793` (85.676 s, exit 0, warm). (c) The staging merge stopped at `unfiled: 5`: the `## Browser-host evidence` heading of `m3-fix-ut-round3-lanes-and-mono.md` was read as a section; demoted to `###`. Ten `Row (F-NEW),` markers (comma, not colon) in three staging files were not recognised; normalised to `Row (F-NEW):` (`8ae8793`). BACKLOG candidate: `integrate-staging.py` should accept both forms and refuse a stray `##` heading before it merges anything. (d) `markdownlint` rejected the first boundary commit (BACKLOG.md MD038 on B-091, readiness MD012); fixed by hand in the boundary commit. | `scripts/integrate.sh`, `scripts/integrate-staging.py`, `reports/readiness/2026-10-25-m3.md` | done (m3 integration) |
| F-135 | 2026-10-25 | M3 caps and the stop-and-decide rule, from `make usage M=m3` (window `2026-10-24T21:48:56Z` to `2026-10-25T18:19:14Z`): the M3 window total is 210.98 USD-eq against the cap 110 (over), the orchestrator 68.81 against 30 (over), and the mission to date 990.09 USD-eq against the 950 stop-and-decide line (crossed; the original hard stop was 630, F-031). The mission stops at the M3 boundary; M4 is not dispatched until the owner decides. The cap breach was reported to the owner before user-testing round 2. | F-031, F-010, `reports/usage/m3.md` | open (owner decision) |
| F-136 | 2026-10-25 | Open owner questions at the M3 boundary (the boundary report carries the texts): Q1 locked-display `make e2e` run (VAL-E2E-004; also record `document.visibilityState` per worker); Q2 the M2 screenshot never landed (add `reports/screenshots/2026-10-24-m2.png` and `2026-10-25-m3.png`, or confirm that the e2e PNG stands in); Q3 the tag trigger `e2e-<milestone>` (informative, rule 24, F-098); Q4 the kit deviations F-103, F-104, F-105, F-106; Q5 the `dedupe_paused` B-row or a fix now; Q6 whether the 110 milestone cap covers all roles (as the `milestone window total` row computes) or workers only; Q7 whether the four post-scrutiny fix commits (`be1337c`, `cf9651d`, `eeba843`, `ce0b143`; scrutiny round 2 passed at `9b7aa16`) get a scrutiny pass in M4 or are accepted on the user-testing evidence and the full gate (`ce0b143` also has no red commit: a test refactor and a class attribute, rule 1 judged not applicable); and the write-token question for M4 (keep the read-only PAT so the app surfaces the clean 403 and the owner merges, or widen it to `pull_requests:write` for the demo). Answers are recorded here as rows. | M4, F-014 (2) | open (owner) |
| F-137 | 2026-10-25 | Owner instruction at the M3 boundary (rule 24, D-026): report-, log- or docs-only commits at a boundary (build-log chore, usage report, CI report, boundary prose) carry `[skip ci]`; no new CI run for reports, logs or docs. The code is proven by the integration gate (`make check`, exit 0 on `a3adb9b3`) and the CI runs of the merge push (`mission-checks` 40222216751, `e2e` 40222227665 green). Receipt: two pushes of this integration started redundant runs before the instruction (40222605337 completed on the build-log chore push and doubled as the warm measurement, 221 s; 40222996690 on the CI report push was cancelled with `gh run cancel`). Alternative: a CI run per boundary commit (rejected: it spends runner minutes on unchanged code). | `reports/ci/2026-10-25-m3.md`, mission `AGENTS.md` and `library/tooling.md` (CI at milestone boundaries) | done (m3 integration) |
| F-138 | 2026-10-25 (rcvd 10-29) | Owner item 1, rule, cycle (R-054, M-019): **rule 29**, incomplete work is replanned, never appended: a sized fix feature (`turnBudget`, under the 120 cap) or a route by the contract. Applied: re-cut slices, no appended lines. Full text of F-138 to F-160: [docs/owner-feedback](docs/owner-feedback/2026-10-29-m3-boundary-rows.md). | RULES 29 | done |
| F-139 | 2026-10-25 (rcvd 10-29) | Item 2, rule, cycle: **rule 30**, the worker lists `claims: [{assertion, evidence}]` in the hand-off; scrutiny reviews claims. Test: blocking issues per feature against M3's 6 in 4. | RULES 30 | done |
| F-140 | 2026-10-25 (rcvd 10-29) | Item 3, rule, cycle (CYCLE-PROTOCOL, R-039): **rule 31**, a resumed agent reads its predecessor's transcript and proofs, says in one line what was done, does the rest. Merged with the F-057 model check. | RULES 31 | done |
| F-141 | 2026-10-25 (rcvd 10-29) | Item 4, rule, cycle: rule 31 for validators, one verdict file per flow group as it finishes; a resumed round skips finished groups (17.30 USD-eq discarded on 25-oct). | RULES 31, `library/user-testing.md` | done |
| F-142 | 2026-10-25 (rcvd 10-29) | Item 5, rule, cycle (R-039): subagents declare a model; mechanical work on the worker model. Subagents cost 57.92 USD-eq in M3. Limit found: built-in droids inherit, so pinned droid files (D-039); tool fact under `## Tooling`. | D-039, §2 | done |
| F-143 | 2026-10-25 (rcvd 10-29) | Item 6, invariant with check (`make usage`), cycle: usage report by token class, per assertion, owner interventions, idle by cause (over 15 minutes unexplained). | `m4-usage-token-classes`, `m4-usage-idle-host-and-rewrites` | open (M4) |
| F-144 | 2026-10-25 (rcvd 10-29) | Item 7, rule, cycle (R-055): the weekly allowance as a stop-and-decide line, stated before dispatch. By the owner's go (10-29): a forecast, not a gate; the owner manages the allowance; notify-and-ask (D-038). | D-038 | done |
| F-145 | 2026-10-25 (rcvd 10-29) | Item 8, invariant with check, cycle: host health in the boundary report (uptime against the 49.7-day TCP limit, stalled calls, inflation). | `m4-usage-idle-host-and-rewrites` | open (M4) |
| F-146 | 2026-10-25 (rcvd 10-29) | Item 9, rule, cycle (R-057): every proposal classified as one of six kinds with scope, checked against the rules, collisions merged. This block is the first application. | integration-worker skill | done |
| F-147 | 2026-10-25 (rcvd 10-29) | Item 10, record: M3 216.88 USD-eq with its boundary against 110 (build 188.81, like-for-like 171.51), 4.75 per assertion (M1 6.48, M2 12.57); mission 1007.73 against 950; 10.9 h elapsed; cache writes 53 % of cost. The tool reads 1 re-issued wait, the owner's reading 2 (window ended 18:19:14Z). | F-135, D-038 | done |
| F-148 | 2026-10-25 (rcvd 10-29) | Item 11, decision, cycle: line 950 to **1350 USD-eq**, M4 200, M5 140, orchestrator 30; stop-and-decide lines (R-055), never walls; 1350 is notify-and-ask (go of 10-29). Alternatives: keep 950; hard stop; no caps. | D-038, §2 | done |
| F-149 | 2026-10-25 (rcvd 10-29) | Item 12, rule, cycle (R-052, R-053): lean user testing: per-group briefs, Sonnet runners, one Opus verdict, recorded live harness, re-test by `Evidence files:`; target under 5 USD-eq per round. | D-039, `library/user-testing.md` | open (M4) |
| F-150 | 2026-10-25 (rcvd 10-29) | Item 13, rule, cycle (R-053): re-test only failed, blocked and assertions whose evidence file changed (F-133). Round 3 kept to it: 2 assertions, one session, 6.22 min, 1.90 USD-eq against round 2's 45.63 min, 16.09. | F-133 | done |
| F-151 | 2026-10-25 (rcvd 10-29) | Item 14, rule with check, cycle (R-052, R-053): mechanical clauses become gate checks (`make check-kit`, `make check-contract`); user testing grades behaviour only. | `m4-ui-kit-lint`, `m4-contract-checks-gate` | open (M4) |
| F-152 | 2026-10-25 (rcvd 10-29) | Item 15, decision, cycle: cost plan for M4 (items 4, 5, 7, 12 to 14, 16, R-056); targets about 2 USD-eq per assertion, validation 15 to 20, subagents about 3; the M4 boundary reports each saving or "did not work". | D-038, D-039 | open (M4) |
| F-153 | 2026-10-25 (rcvd 10-29) | Item 16, rule, cycle (R-044): no polling by mechanism for every role: one blocking call, a background Script, or `make integrate-long`; every model priced. | `m4-integrate-long-target` | open (M4) |
| F-154 | 2026-10-25 (rcvd 10-29) | Item 17, rule, cycle (R-052, R-053), amends rule 27: the gate is keyed by code paths, the last code change records it, CI `paths-ignore`. Exception: `[skip ci]` on report-only commits until `m4-gate-key-and-ci-paths` lands (F-137). | `m4-gate-key-and-ci-paths` | open (M4) |
| F-155 | 2026-10-25 (rcvd 10-29) | Item 18, rule, cycle (R-023, R-057): clauses that need a person are `attended`, in the contract's QA section, never graded, never asked of the owner. VAL-E2E-004 is `handed_to_qa`. | contract QA section | done (passes 26, 27) |
| F-156 | 2026-10-25 (rcvd 10-29) | Item 19, rule, cycle, plus backlog item (R-044): the lead's context at each wake (16 rewrites, 24.16 USD-eq in M3). (w) is unavailable, a tool fact under `## Tooling`; (s), (a), (b) in force; (c) `m4-usage-idle-host-and-rewrites`; product request B-115. | `## Tooling`, B-115 | done; (c) open |
| F-157 | 2026-10-25 (rcvd 10-29) | Item 20, decision, cycle (R-054): `m4-github-forge-core` (15 assertions) split into three; VAL-GATE-013 moved to the first slice (5 and 5, not 4 and 6); `m4-forge-core-and-pr-detection`, `m4-forge-checks-and-fix-cycle`, `m4-forge-rulesets-and-merge`. | `features.json` | done |
| F-158 | 2026-10-25 (rcvd 10-29) | Item 21, rule, cycle (R-054, M-019): size every pending feature before dispatch at 23.3 turns per assertion (about 5 per feature); all six oversize units split; M5 confirmed at the M4 boundary, the final integration split planned (B-116). | `features.json`, B-116 | done |
| F-159 | 2026-10-25 | The orchestrator's 402 at 2026-10-25T18:56:51Z (weekly limit, `droid-log-single.log:9816-9822`): second F-057 occurrence (first F-132). Resume model check `model = claude-fable-5.1`, 0 Model-Router lines, no fallback. | F-057 | done |
| F-160 | 2026-10-25 (rcvd 10-29) | Owner answers Q1 to Q4. Q3: the PAT stays read-only, the owner merges. Q4: F-105 and F-106 approved; F-103 and F-104 (padding overrides) reverted in M4, a denser table only as one kit variant class. Q1: VAL-E2E-004, two attended runs, "Run 2 ... only the last spec (stall banner: 3 tests, green, PNGs written 20:18:46 while locked) ran on a locked display", "VAL-E2E-004 is not yet met"; run 1 unlocked, "Not evidence"; superseded by item 18 (QA). | F-155, F-136 | done; revert open (M4) |

## 10. Evolution log

One row per integrated milestone, written at integration from the workers' and validators'
reports.

| Milestone | Integrated | What landed | Tests green | Usage spent | Deviations | Unknown events left |
|---|---|---|---|---|---|---|
| S | 2026-10-23 | Cargo workspace (`observatory-core`, `observatory-cli`, `observatory-app`), `Cargo.lock`; canonical `Event` with 61 PRD kinds plus `Unknown`; `JsonlSource`, capture header and the Unknown-only `mapping::v1`; `observatory-cli replay` (`unknown events: 2027` on the S fixture); Leptos UI and Tauri window with nine empty lanes; S fixture (orchestrator session plus 25 children, gitleaks clean); `machete` and `coverage` jobs, tag check in `release.yml`, pinned bundle tools; D-011 | `cargo test --workspace --all-features`: 41 passed, 0 failed; `make check` exit 0 on the merge commit `38e5617`; coverage 94.48 % lines on `observatory-core` (reported, not enforced); 20 of 20 VAL-SKEL assertions passed; **`mission-checks` red on the first run** (run 39919592562, `rust` job: the Linux runner lacks `glib-2.0`, which the Tauri shell needs), see `reports/ci/2026-10-23-s.md` | 13.21 USD-eq of the 40 cap on workers and validators (`reports/usage/s.md`); orchestrator share not exposed | §3 M0 and §5.2 amended (mapping version from the fixture header, D-012); §3 M4 acceptance amended (owner merges, app surfaces 403, D-014); §8 hybrid backlog withdrawn (D-015); D-011 (deny exceptions) awaits owner confirmation; kimi-k3 episode at the boundary (D-018) | 2027 on the S fixture (expected: the S mapper knows no kind); 0 required from the M0 fixture |
| S, second pass (M0 pre-work) | 2026-10-23 | `wt/m0` merged as `cc05aa5` (`Integrate m0 CI fix and S fixture redaction`): Linux `rust` job without the Tauri shell and `wt/**` trigger (D-022); `scripts/redact-fixture.sh`, `make redact-check` and the redacted S fixture (D-020); owner decisions D-019, D-020, D-021, D-011 and D-018 addenda, rules 14, 21, 22; hook `script` field | `make check` exit 0 on `cc05aa5`; `mission-checks` green, run 39928967884, 2 min 56 s; user-testing round 2 on `claude-opus-5-5`: 20 of 20 VAL-SKEL passed | S unchanged at 13.21 USD-eq of 40; round-2 validators 2.69; orchestrator delta 18.94 USD-eq (`reports/usage/s.md`) | S row above: "`mission-checks` red" is superseded by this row; D-022 (Linux CI excludes the shell, B-024) | 2027 on the S fixture (unchanged); 0 required from the M0 fixture |
| M0 | 2026-10-23 | Redacted M0 capture (this mission's own sessions: root plus 55 children, 112 fixture files, gitleaks clean); sidecar reader and session tree with role resolution; `replay` over the root and its children, `--root-only`; mapping v1 (30 observed kinds, 31 defined-not-observed, golden `.events.json`, `unknown events: 0`); SQLite pointer store, `inspect` and `replay --store`; gate rule from the exit code; redactor fix for JSON escapes; coverage job enforced at 80 %; D-023 | `make check` exit 0 on the merge commit `f66a9f5`; 128 tests passed, 0 failed (`cargo test --workspace`); coverage 94.21 % lines on `observatory-core` (floor 80, `COVERAGE_MIN` "80" in `mission-branch.yml`); `mission-checks` green, run 39953923391, 1 min 45 s (`reports/ci/2026-10-23-m0.md`); scrutiny round 2 PASS at `88866ef`; user testing round 1: 26 of 27 passed, VAL-PROC-039 failed until the D-020 addendum written at this integration | Workers and validators 33.28 USD-eq plus orchestrator subagents 15.77 plus this pass, 49.05 priced of the 60 cap; orchestrator own delta 66.87 USD-eq against 32 (209 %); `reports/usage/m0.md` | D-023 added; D-020 addendum (third rule, hash tables, redactor fix); M0 orchestrator cap 32 (F-024); `source_dir` stored redacted; CI warm budget 3 min exceeded on `wt/m0` (run 39942226174, 3 min 58 s, cold rust job 3 min 40 s), met on the mission branch (1 min 45 s); orchestrator cap exceeded (deviation, rule 9) | 0 on the S and M0 fixtures (S 661 events, M0 1279 events over 8735 records) |
| M1 | 2026-10-24 | `wt/m1` merged as `c2fb42a` (`Integrate m1: board, replay, attach, usage-log source`): seven phase lanes in mapping v1 and the golden (D-024), `view` models (board, card, details, agents, panel, filters, scrubber, raw-line scrub), attach watcher with a SQLite-backed live path and `Attach::poll`, usage-log source with `turn.usage` events (D-029), the Leptos board with Details and agents views (D-030: go), the Tauri host with live attach and an open-by-path field, hook duration logging (D-021 addendum), README UI build command; session names by `user` tag then id; D-030 | `make check` exit 0 on `c2fb42a`, real 1 m 32.700 s: 262 passed, 0 failed, 0 ignored; `cargo llvm-cov` alone exit 0, real 2 m 1.731 s, 93.89 % lines on `observatory-core`; `mission-checks` green, run 40061513431, 3 min 48 s (over the 3 min warm budget; `reports/ci/2026-10-24-m1.md`); scrutiny and user-testing rounds passed in `user-testing-m1`; clean-clone UI build 56.95 s and 61.04 s | Workers, validators and orchestrator subagents 110.38 USD-eq of the 110 cap (workers 37.05, validators 48.53, subagents 24.80; plus 3.44 for the M0 validation re-run in the window); orchestrator own delta 115.74 USD-eq against 22 (526 %); throttled 255.4 min (two `unrecoverable_usage_402` windows); compile share 1.74 % lower bound; `reports/usage/m1.md` | D-030 added (Leptos go); D-023, D-024, D-029 addenda; golden refrozen (2394 events, `25d91c53…`); usage-log extract fixture added; M1 cap exceeded by 0.38 and orchestrator cap exceeded (deviations, rule 9); CI warm budget 3 min exceeded (B-052); the M0 `.capture.json` header gained a `usage_log` block and a refreshed `gitleaks` field (per-file redaction rows and all fixture and child hashes unchanged); `0001_init.sql` was edited in place twice during M1, which is forbidden from M2 on (AGENTS.md) | 0 on the S and M0 fixtures (S 661 events, M0 1279 session events over 8735 records, plus 1115 `turn.usage` events); a live attach of a real `droid exec` session shows one extra unknown event until `m2-fix-rewritten-line-resync` lands (B-051) |
| M2 | 2026-10-24 | `wt/m2` merged as `7385b62` (`Integrate m2: rpc source, hosted launch, scrutiny feature_defined`): `RpcSource`, rpc mapper and control writer over the stream-jsonrpc channel (D-033), launch form, host shell with hosted launch, hosted controls, live output, reap on window close (D-034 recipe, amended with `droid.update_session_settings`), hosted spool replay in the CLI, rewritten-line resync (D-031, a 3-line `retract_stale` call in `apps/observatory-app/ui/src/capture.rs` and a `retracted` field in the `events_appended` payload, covered by the M2 attach UI check), first-line pointer follows the rewrite, `process_exit` outcome (F-052), system-role messages known (F-047), scrutiny and user-testing `feature_defined` in their own lane (D-032, golden refrozen `cecf727d…`), 85 more tests than M1 (262 to 347) | `make check` exit 0 on `7385b62`, real 1 m 58.050 s: 347 passed, 0 failed, 1 ignored (the live check); `cargo llvm-cov` alone exit 0, real 2 m 13.599 s, 94.20 % lines on `observatory-core`; `mission-checks` green: cold run 40121391298 (the merge) 2 min 48 s, reported apart (F-039), warm runs 2 min 33 s, 3 min 26 s and 2 min 15 s (run ids in the report) (`reports/ci/2026-10-24-m2.md`); scrutiny round 4 PASS 6/6 and round 5 PASS; user-testing round 4 PASS at `36a042e` (rounds 1 to 3 void or failed: model fallback, VAL-BOARD-038, VAL-HOST-014, VAL-HOST-018, VAL-PROC-045, VAL-HOST-016) | Workers, validators and orchestrator subagents 120.61 USD-eq of the 50 cap, void `kimi-k3` turns and the model-fallback sessions included where priced (workers 37.55, validators 40.48, subagents 32.52, void priced Claude turns 10.07; the `kimi-k3` tokens are unpriced); the orchestrator spent more than 30 USD-eq in M2 (118.84 USD-eq on the fable turns of the droid log; 121 re-issued waits cost 86.49 of it) and is 388.7 of the 320 mission cap by the sidecar baseline; the droid-log record mission-to-date is 694.66 USD-eq, 110.3 % of the 630 hard stop (`reports/usage/m2.md`, F-045); throttled 121.3 min (`unrecoverable_usage_402`); compile share 3.55 % | F-038 to F-053 (§9); D-031 to D-034 added, D-023 and D-024 amended by D-032; golden refrozen (2394 events, `cecf727d…`); M2 cap and mission orchestrator cap exceeded (deviations, rule 9; the M2 orchestrator stop condition of F-040 is met); CI warm budget exceeded on one of three warm runs, 3 min 26 s (B-056); void sessions of episode 4 (F-046); the literal `--mission` command of VAL-HOST-005 superseded (F-049); `reports/usage/m1.md` backfilled with the tokens table | 0 on the S and M0 fixtures (S 661 events, M0 1279 session events over 8735 records, plus 1115 `turn.usage` events); hosted and attached runs agree except `usage.reported` and `plan.revised` (F-050) |
| M3 | 2026-10-25 | `wt/m3` merged as `0bf60b0` (`Integrate m3: wt/m3`, parents `d03d1b1 ce0b143`): human controls (pause, resume, message, answer, approve, deny; one `human_action` command, each control one overlay event with the host's ack, F-071 to F-079), the stall flag and its poller (`[intervention].stall_poll_s`), the cycle template as data (D-035, `config/cycles/engineering.toml`, F-041), session names from the feature id (D-036), hosted `todo_state` mapped from the session file, live session rows, the gate cache and the hit log (`scripts/gate.sh`, F-111 to F-113), `make usage` (`scripts/usage-report.py`, F-127 to F-131) and `make integrate` (`scripts/integrate.sh`, `scripts/integrate-staging.py`), the end-to-end harness (WDIO + `tauri-plugin-wdio-webdriver`, 26 window cases, D-037, `.github/workflows/e2e.yml`), UI kit v2 applied to the M3 screens (F-081, F-102 to F-110) | `make check` exit 0 on the merged tree `a3adb9b3` (real run, 85.676 s; the first merge tree `0c7b6f23` 195.592 s, exit 0; both gate MISSes, F-134); `cargo llvm-cov -p observatory-core --all-features --fail-under-lines 80` alone: 94.21 % lines (8052 lines, 466 missed; 2 min 51.329 s); scrutiny round 2 PASS at `9b7aa16`; user testing round 1 (`11bf22ab`, 32 of 43 passed) and round 2 (`cc6ebacd`) failed with fix features after each, round 3 (`33e778c8`, VAL-CYCLE-003 and VAL-UI-001 only) PASS at `ce0b143`; `mission-checks` and the macOS `e2e` run in `reports/ci/2026-10-25-m3.md` | `reports/usage/m3.md`: window total 210.98 USD-eq against the cap 110; orchestrator 68.81 against 30; mission to date 990.09 USD-eq, past the 950 stop-and-decide line (F-135) | milestone cap 110 and orchestrator cap 30 exceeded; re-issued waits 1 where VAL-PROC-048 needs 0; two features over 120 turns (136, 146); kit deviations proposed (F-103 to F-106); F-057 verified on a real 402 (F-132); the mission stops at this boundary | 0 on the S and M0 fixtures (golden `cecf727d…` unchanged, F-126) |

**Frozen from the M0 integration (D-020, D-023), refrozen at the M1 and M2 integrations.** Fixture and golden hashes
(`shasum -a 256`, bytes at the M2 merge commit `7385b62`; the golden changed at M2 by D-032, every other hash is as at the M1 pass); a
change needs a decision file:

| File | SHA-256 |
|---|---|
| `fixtures/streams/2026-10-22-observatory-v1-m0.jsonl` | `ab23da5510478bb1c519adb78e1f47f959398477bfd802a9329bc71318a9a58c` |
| `fixtures/streams/2026-10-22-observatory-v1-m0.settings.json` | `efef27eb7fe491e2cef90bd83634230e4b845a742d00141658c88dd6154076ec` |
| `fixtures/streams/2026-10-22-observatory-v1-m0.capture.json` | `4865d01dd8a1efefff751b55716cef0b8d37e916a2411228b163840c79b4b400` |
| `fixtures/streams/2026-10-22-observatory-v1-m0.events.json` (golden, 2394 events) | `cecf727d0944ff7e8ddf306a8aeca6cbc32022bd2ff2eb7e22050fa7b72106c7` |
| `fixtures/streams/2026-10-22-observatory-v1-m0.usage-log.jsonl` (new, 2230 lines) | `c9b5e2747b23aba4a8cf09f2e0e85cf1b85ef0cdd8f39c671b230deade91637d` |
| `fixtures/streams/2026-10-22-observatory-v1-s.jsonl` | `e9746b0979596003373d584abb273a921c476ff6f43d2c47d76284d81f370b25` |
| `fixtures/streams/2026-10-22-observatory-v1-s.settings.json` | `42f6b820dc6c797cf408994df08f19dd5954a16610e923ed358b1f986f0113d9` |
| `fixtures/streams/2026-10-22-observatory-v1-s.capture.json` | `3658c14010bc9b4dfb472ce8481c13af0e4bc41273b929187297bc82ba558262` |

**Refrozen at the M1 integration (2026-10-24).** The M1 golden is `25d91c53b580f8f5f3f1a62ecaa977a24dca0b61f5caf86e296182e8f23a5750`
(2394 events: 1279 session events plus 1115 `turn.usage` from the usage-log extract), and the table above carries it. The
`.jsonl` (`ab23da55…a58c`) and `.settings.json` (`efef27eb…76ec`) hashes of the M0 fixture, the three S fixture hashes
and the `.children/` hashes are unchanged (checked with `shasum -a 256` and against every `redaction.files[].redacted_sha256`:
112 files, 0 mismatches). The M0 `.capture.json` changed twice since the freeze: `m1-usage-log-source-core` added the
`usage_log` block (`d6d790fb…` to `93e0ccd0…`) and this pass refreshed the `gitleaks` field to the 68.21 MB scan
(`4865d01d…`); its per-file redaction rows did not change. The new fixture `…m0.usage-log.jsonl` is the redacted extract
of `~/.factory/logs` for the capture's sessions (D-029 addendum). gitleaks over `fixtures/streams`: `scanned ~68213177
bytes (68.21 MB) in 11.5s, no leaks found`.

**Refrozen at the M2 integration (2026-10-24).** The golden is `cecf727d0944ff7e8ddf306a8aeca6cbc32022bd2ff2eb7e22050fa7b72106c7`
(2394 events, `shasum -a 256` at `7385b62`), regenerated by `m2-mapping-scrutiny-feature-defined` for D-032 (owner decision 31: a
`feature.defined` from a scrutiny or user-testing validator is in that validator's phase). Lane totals of the replay of the root and
its children, before (the M1 golden, `25d91c53…`) and after: Plan 443 to 437, Contract 69 to 69, Build 424 to 424, Scrutiny 87
to 93, User testing 122 to 122, Integrate 134 to 134, Gate 0 to 0; sum 1279 both times (the six `feature_defined` events moved
Plan to Scrutiny). Whole golden, including the 1115 `turn.usage` events in their sessions' lanes: Plan 744, Scrutiny 157. The fixture
`.jsonl`, `.settings.json`, `.capture.json`, `.usage-log.jsonl` and the S hashes in the table above are unchanged.

**Amended by F-028 (D-024).** The golden hash `f055657a46af2713f50d1963dab684495d5f3770225f942adfcbb28e8331f3e1`
above was superseded at the M1 integration: `2026-10-22-observatory-v1-m0.events.json` was regenerated
at Step 0 of `m1-view-models-core`, for the seven phase lanes and the `overlay` field, and the
new SHA-256 is recorded in the M1 row. The fixture `.jsonl` and `.settings.json`
hashes of both fixtures and the `.children/` hashes stay as frozen (the `.capture.json` change is listed above). Expected phase counts on the
regenerated M0 golden (the committed golden is authoritative): Plan 443, Contract 69, Build 424,
Scrutiny 87, User testing 122, Integrate 134, Gate 0 (after D-032 at the M2 integration: Plan 437, Scrutiny 93, the others unchanged); 70 human overlays.

The per-file hashes of the `.children/` directories are `redaction.files[]` in each `.capture.json`
and the pairs are in the D-020 addendum. Mapping v1 golden counts: M0 1279 events (root only 474), S 661
events (root only 364); `gate_command.run/passed/failed` 35/34/1 (D-023).
