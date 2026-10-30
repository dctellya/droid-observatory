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
- **As built (amended at the M4 integration, 2026-10-29; rows F-161 to F-275).** `GitSource` (lane 6 merge and gate-command
  rows, runs attributed to the milestone integrated last, the card names its worktree, F-172) and `GitHubSource` behind the `Forge`
  trait (PR detection for the configured mission branch and target branch, Actions runs and commit statuses, never the Checks API;
  rulesets and reviews; merge policy with rule attribution; `handed_to_cd` on detection with role human or external) landed with
  the gate group and badge, the return arc and the fix cycle (migrations 0003 and 0004), the Merge button with its confirmation and
  the clean `403` refusal, and the `demo/base` pair with the `demo-gate` workflow (D-040). The PAT stays read-only (owner answer
  of the M3 boundary, F-160): the owner merges by hand, and the live items that need his merge or his keychain are owner-run
  (VAL-GATE-027 and VAL-GATE-033 are `blocked`, rows 28, 33 and 40 of the owner questions are open). The forge is off without
  `GITHUB_TOKEN` or the owner's `FORGE_LIVE=1` launch (F-218, F-219). Tooling ordered by the M3 boundary landed with it
  (`make integrate-long`, `make check-contract`, `make check-kit`, the code-path gate key and CI `paths-ignore`, the usage report
  tables). Acceptance deviations: the walk ran against the read-only token, so "the owner merges" stands for the merge step; the
  `demo/*` branches are deleted (only `demo/base` remains, VAL-GATE-030). The test list above names nine tests and
  VAL-GATE-006 grades eight; all nine are `ok` in `reports/gate/e8f7cea9…/test.tests`.

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
  computed); export of the §5.7 series to a local OTLP collector (`[telemetry].endpoint`, default `http://localhost:4318`,
  D-046) behind `[telemetry].enabled`; the sink behind the collector (Grafana Cloud or any provider) is the collector's configuration.
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
  tolerance; with `[telemetry].enabled = true` and a local collector with a file or debug exporter
  on `[telemetry].endpoint`, the series are received with only the allowed labels and the test reads them back (D-046; an unattended
  receipt replaces the attended Grafana screenshot).
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
**rework causes** `scrutiny`, `user_testing`, `integrate`, `gate` (4; the cause is the template lane id, D-035 item 4, F-276). Waste is measured by the
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
| F-001..F-266 | 2026-10-22 to 2026-10-29 | Rows F-001..F-266 (S to M4; F-138 to F-160 are the owner items and answers of the M3 boundary, received 2026-10-29; F-161 to F-266 are the M4 window, 2026-10-29) are archived verbatim in [docs/spec/section-9-archive.md](docs/spec/section-9-archive.md). F-161 there is the archive pattern (sized fix, rule 29) and F-276 the third pass | all | archived |
| F-267..F-329 | 2026-10-29 to 2026-10-30 | Row `integrator` (kind: sized fix, rule 29, scope: cycle; fourth archive pass of the F-161 pattern, M6 unit 1 boundary integration): SPEC.md was 198642 bytes against the 204800-byte limit of `scripts/check-file-size.sh` before the pass 41 and 42 rows, the unit 1 close row and the unit 1 evolution row. Rows F-267 to F-329 (the M4 close and the M5 build window, 63 rows, 51374 bytes) moved byte-identical to [docs/spec/section-9-archive-2.md](docs/spec/section-9-archive-2.md) (`cmp` of the 63 moved lines against `git show HEAD:SPEC.md` is empty; none holds a relative link); the first archive ([docs/spec/section-9-archive.md](docs/spec/section-9-archive.md), F-001..F-266) has about 19.9 KB of room and the 63 rows are 51374 bytes, so they go to a second file in the same pattern. Rows F-330 on stay. **Alternatives considered:** raising `MAX_FILE_KB` (rejected: hides the growth), archiving §10 as well (not needed). | `SPEC.md` §9, `docs/spec/section-9-archive-2.md` | archived |
| F-330 | 2026-10-30 | Row `integrator` (kind: contract wording, rule 24, scope: cycle; the drafts of `contract-work/pass-36-ruling.md`): **D-048, money for M5.** The owner's decision of 2026-10-29 23:40 (inbox `2026-10-29-2345`) moves the mission notify-and-ask line from 1350 to 1500 USD-eq and sets the M5 cap at 150; the owner's chat message of 2026-10-30 09:55Z raises the M5 cap to 190. Both are stop-and-decide lines under R-055, never walls; the orchestrator cap stays 30 per milestone. Mission-to-date is the recorded M4 base 1260.74 plus the M5 window total (the log files before 2026-10-22 rotated out of `~/.factory/logs`). **Alternatives considered:** keep 1350 and 140 (rejected: the mission was at 1260.74 before M5), remove the caps (rejected, D-038), hold M5 at 150 (not kept). | `docs/decisions/D-048-*.md`, `AGENTS.md` Money bullet, `reports/usage/m5.md` | done |
| F-331 | 2026-10-30 | Row `integrator` (kind: contract wording, rule 24, scope: cycle; the drafts of `contract-work/pass-36-ruling.md`), pass 36 item 1: VAL-METRIC-005 and the M5 area note name the rework cause as the template lane id `integrate` (D-035 item 4; `integration` became `integrate` in migration 0002); the reference CSV has no cause column, so it never arbitrated the spelling. Wording only, no behaviour change. **Alternatives considered:** rename the cause back to `integration` in code (rejected: lanes are data, D-035, and migration 0002 already stores the lane id). | validation contract (orchestrator's file); SPEC §5.6 corrected at F-276 | done |
| F-332 | 2026-10-30 | Row `integrator` (kind: contract wording, rule 24, scope: cycle; the drafts of `contract-work/pass-36-ruling.md`), pass 36 item 2: VAL-METRIC-008 stands as written; `cost_usd` must include the cache_write term (`tokens_cache_write` x the table's cache_write rate). Product fix (F-291), no amendment. | validation contract; F-291 | done |
| F-333 | 2026-10-30 | Row `integrator` (kind: contract wording, rule 24, scope: cycle; the drafts of `contract-work/pass-36-ruling.md`), pass 36 item 3: VAL-METRIC-011 defines a completed feature as `task.done` and validation green, and integrated where the capture records the integration (`integration.merged` naming the feature or its milestone); the M0 capture records none, so there the integrated condition is not applicable. **Alternatives considered:** require integration literally (rejected: completed = 0 and every figure null on the fixture); drop the condition (rejected: PRD §10.4 names it and live missions carry `integration.merged`). Product half is F-290. | validation contract; F-290 | done |
| F-334 | 2026-10-30 | Row `integrator` (kind: contract wording, rule 24, scope: cycle; the drafts of `contract-work/pass-36-ruling.md`), pass 36 item 5: VAL-METRIC-022's diff base is the latest integration commit before M5 (`3f38afe`, `Integrate m4`); the M2 merge `7385b62` pre-dates F-041's mapping source change (`6b0948d`, integrated at `0bf60b0`), which the clause never named. The fixtures and the golden hashes are unchanged since their last refreeze. The literal `7385b62..wt/m5` range therefore includes `6b0948d`; the assertion's intent, `3f38afe..HEAD`, is empty (a contract-wording finding, not a defect). | validation contract | done |
| F-335 | 2026-10-30 | Row `integrator` (kind: contract wording, rule 24, scope: cycle; the drafts of `contract-work/pass-36-ruling.md`), pass 36 item 6: VAL-METRIC-023's sum check is defined per row kind: feature rows, `waste_*_s + 60 x throttled_minutes <= cycle_s`; milestone rows are feature-second sums, so each `waste_*_s` equals the sum over the milestone's features and 60 x throttled_minutes <= cycle_s. Reason: the literal check compared feature-seconds with a wall-clock bound (m0 12278 > 9676 with throttled 0). | validation contract | done |
| F-336 | 2026-10-30 | Row `integrator` (kind: contract wording, rule 24, scope: cycle; the drafts of `contract-work/pass-36-ruling.md`), pass 37: VAL-METRIC-024 prices cost with the four-class formula of VAL-METRIC-008 (input, output, cache_read, cache_write at the plan price table rates, times the role multiplier); the earlier text said three classes and matched the engine before the fix slice `m5-fix-cost-cache-write` (F-291). Status reset to pending (criterion changed). **Alternatives considered:** keep three classes in 024 (rejected: two cost definitions in one contract). | validation contract; F-291 | done |
| F-337 | 2026-10-30 | Row `integrator` (kind: decision, rule 24, scope: cycle; VAL-PROC-028): **orchestrator overrun in the M5 window.** `reports/usage/m5.md` reads 71.27 USD-eq for the orchestrator session alone (22.71 more in its subagents) against the 30 per milestone line (F-040, F-044); the M5 window total is 163.57 against the cap 190, mission to date 1424.31 of the 1500 line. The owner was told the figure in chat (63.03 USD-eq at 09:44Z, over the line) at about 09:55Z and chose to raise the M5 cap from 150 to 190 and finish M5; that answer is row 80 in `MD/contract-work/m4-boundary-questions.md` (the mission directory, not the repository) and the cap change is D-048. The 30 line is a stop-and-decide line under R-055, not a wall (D-038): the stop-and-decide happened, the answer is recorded, and the overrun is accepted as data like M3 (68.81) and M4 (111.86). **Alternatives considered:** stop at 30 (not asked for by the owner, who chose to finish M5), hide the overrun in the subagent share (rejected, rule 7). | `reports/usage/m5.md`, D-048, `ai/session/2026-10-22-session.md` entry 40 | done |
| F-338 | 2026-10-30 | Row `integrator` (kind: backlog item with an owner decision pending, scope: cycle; VAL-PROC-013): **M3 and M4 have no `reports/readiness/<date>-<m>.json`** (B-259). The files exist for s, m0, m1, m2 and m5; M3 and M4 wrote only the markdown boundary reports (`2026-10-25-m3.md`, `2026-10-29-m4.md`). The M5 file is a hand re-evaluation (four criteria changed, named in its `method`); `/readiness-report` is a slash command a worker cannot run. **Open:** the owner decides at the M5 boundary whether to waive the two files or to run `/readiness-report`. They are not backfilled: a re-evaluation dated now is not the evidence of that boundary. **Alternatives considered:** backfill by hand from the markdown reports (rejected, a number written later is not pasted from the tool at the time, rule 7). | `reports/readiness/`, BACKLOG B-259 | open (owner) |
| F-339 | 2026-10-30 | Row `integrator` (kind: report as is, scope: cycle; VAL-PROC-011): **the only M5 CI run is cold and over the warm budget.** `mission-checks` run 41045311362 on the boundary commit `2bcca58`: success, 228 s wall (rust 161 s, coverage 212 s), against the 3 min warm budget (180 s). Every later commit of the boundary carries `[skip ci]`, so no warm run exists and the budget verdict stays open; there is no coverage-to-final-PR decision file (D-016 is the 80 % gate only). The final PR's `gate` run is the next measurement. Reported as it is in `reports/ci/2026-10-30-m5.md`. | `reports/ci/2026-10-30-m5.md` | open (verdict) |
| F-340 | 2026-10-30 | Row `integrator` (kind: correction, scope: cycle; VAL-PROC-019; owner chat and inbox `2026-10-30-1230`, "I didn't take it"): **the owner's screenshot is not available.** The untracked `board-browser-m0.png` of the repository root (1280 x 5353, 07:50Z) was the round-1 validator's full-page browser capture of the board in Replay on the M0 fixture, not the owner's unlocked-display screenshot; it now sits in the mission directory as validator evidence (`evidence/m5/resume-0738/`, `evidence/m5/cross008-board/`). BACKLOG B-261 and the m5 boundary report called it the owner's: both lines are corrected in a report-only commit (20b90c9). None of the seven expected files `reports/screenshots/<date>-<m>.png` exists. VAL-PROC-019 stays `handed_to_qa`, an owner item for the M6 boundary. | BACKLOG B-261, `reports/readiness/2026-10-30-m5.md`, entry 40 | open (owner, M6) |
| F-341 | 2026-10-30 | Row `integrator` (kind: model episode record, scope: cycle; VAL-PROC-027; row 71 in `MD/contract-work/m4-boundary-questions.md`): **model fallback to `kimi-k3` in the M5 window.** After the first usage limit of the night, validator `364e8787` (`user-testing-validator-m5`) ran 3 turns on `kimi-k3` (2026-10-30T04:14:18Z to 04:15:42Z) and the orchestrator ran 7 turns on it (0.42 USD-eq, 03:44Z to 04:21Z); the hosted-gate run of seat `7865a50a` produced on `kimi-k3` after 03:43:46Z was discarded (owner delegate inbox `0515`). The mission was paused at 04:15:43Z and resumed at 06:38:43Z on the owner's word "resume mission" (143.0 min, `Throttled` no: the reason is a model fallback, not a throttle); the resumed validator did the model check first. `.factory/settings.json` never changed (VAL-PROC-027 values re-read at this boundary: `claude-fable-5.1`, `claude-sonnet-5-5`, `claude-opus-5-5`, all `high`). The model-guard hook (rule G) is proposed for the M6 cut. | `reports/usage/m5.md` (Model fallback, Throttled), entry 40 | done |
| F-342 | 2026-10-30 | Row `integrator` (kind: process record, scope: cycle; `final-process-record`): the process audit VAL-PROC-001 to 036 on the mission branch at `20b90c9` is `reports/process/2026-10-30-process-record.md` (commands and outputs pasted). Gaps found and not fixable by report: VAL-PROC-002, 14 production commits without an earlier red commit (list in the record); VAL-PROC-006, S literal checks (`rusqlite` in `Cargo.lock` as an optional dependency, `docs/decisions/README.md` touched by D-011); VAL-PROC-008, 2 commits without the trailer (`8ab88e2`, owner commit `bb7be45`); VAL-PROC-018, entries 37 and 39 lack the run id or few session ids; VAL-PROC-024, `apps/observatory-app/e2e/package-lock.json` (252861 bytes) is exempt in `scripts/check-file-size.sh` but not in the contract's list; VAL-PROC-033, 23 commits on worktree branches touched shared files (generated decisions index, usage reports), 14 not in `exceptions.txt`; VAL-PROC-023, the build-log was not committed at the M4 and M5 passes (committed at this record, `[skip ci]`). The rows are BACKLOG B-262 to B-269. | `reports/process/2026-10-30-process-record.md`, BACKLOG B-262..B-269 | done |
| F-343 | 2026-10-30 | Pass 40, owner ruling of 2026-10-30 ~12:30Z (kind: decision, scope: cycle; contract pass 40): the contract-literal gaps of the process record are waived by contract amendment, wording only, no history rewrite (RULES rule 12): VAL-PROC-002 (B-264: production commits without an earlier red commit, 183 red against 152 production commits, red runs read from hand-offs), VAL-PROC-006 (B-265: `grep -c rusqlite Cargo.lock` prints 2 at `s`, generated decisions index in the `s` diff, 537ed0c), VAL-PROC-008 (B-266: `8ab88e2` and `bb7be45` without the co-author trailer), VAL-PROC-009 (B-270, new row added by the boundary commit: `s` runs red, m3 merge commit `0bf60b0` without a run), VAL-PROC-018 (B-269: entries 37 and 39 thin on run and session ids, entry 40 points at the usage tables), VAL-PROC-024 (B-267: `package-lock.json` 252861 B exempt as a lock file), VAL-PROC-033 (B-268: 23 shared-file commits on worktree branches, nine in `exceptions.txt`, 14 not); and VAL-PROC-013 is waived for M3 and M4 (B-259: no readiness json; nothing backfilled; pass = waiver recorded in the contract and the boundary commit). Each keeps its ID and intent; statuses stay `pending` for the validators. **Alternatives considered:** rewrite history or backfill (rejected by the owner: rule 12; the gaps are as-built facts); re-scope the checks to start after the gaps (rejected: the contract keeps its literal range with an explicit waiver). | `validation-contract.md` of the mission directory (pass 40), BACKLOG B-259, B-264..B-270 | done |
| F-344 | 2026-10-30 | Pass 40, owner ruling of 2026-10-30 ~12:30Z (kind: decision, scope: cycle): VAL-PROC-019 (the owner's seven unlocked-display screenshots) moves to M6 formally, `handed_to_qa` -> `pending`, claimed by feature `m6-owner-screenshots-process-record`; the owner supplies the files at the M6 boundary. The reference CSV `reports/reference/2026-10-22-observatory-v1-m0.csv` (VAL-METRIC-001, 003, 026) arrives with the M6 oracle-parity feature `m6-reference-csv-oracle-parity`. **Alternatives considered:** backfill screenshots now (rejected: the owner took none and no image is invented). | `validation-contract.md` of the mission directory (pass 40), F-340, BACKLOG B-242, B-253, B-261 | open (owner, M6) |
| F-345 | 2026-10-30 | Row `scribe` (kind: pull request state and replan, scope: cycle; owner chat 2026-10-30 12:30Z and 12:52Z; BACKLOG B-271): **PR #40 is open and red.** Opened 2026-10-30 with head `72d0412`; the `gate` run 41052611163 was dispatched by hand (the head commit carries `[skip ci]`, which suppresses the `pull_request` run) and is red: `sast` 8 Semgrep findings, `fs-scan` 2 Trivy findings on a `ghp_fake` placeholder in a build-log file. **Replan:** fix feature `m6-carryover-fix-pr-gate`, then `final-open-pr` re-run, booked as the first M6 slice "M5 carry-over: release gate"; VAL-PROC-036 and 037 are validated at M6 as a carry-over. Rule: the last commit before `gh pr create` and every PR head must not carry `[skip ci]`. | BACKLOG B-271, AGENTS.md "Git", D-049 | open (m6) |
| F-346 | 2026-10-30 | Row `scribe` (kind: money, scope: cycle; owner chat 2026-10-30 12:52Z; D-049): **M5 closes at 186.13 USD-eq of the cap 200** (measured 2026-10-30 12:15:25Z, `--from 2026-10-29T22:30:00Z`: orchestrator 81.83, orchestrator subagents 26.33, workers 36.88, validators 39.52, self-check 1.43, other 0.14). The orchestrator's 81.83 is measured against the committed record's 71.27, which stays as written (`reports/usage/m5.md`). Mission-to-date 1446.87; the notify-and-ask line moves from 1500 to **1700** (D-049). The M6 cap is set at dispatch, priced by role, with validation priced by rounds from M5's record. | D-049, `reports/usage/m5.md`, `reports/readiness/2026-10-30-m5.md` | done |
| F-347 | 2026-10-30 | Row `scribe` (kind: verdict; inbox 1350 item 1; row 91 in `MD/contract-work/m4-boundary-questions.md`; Roci FR-M8): **Verdict, M5 against M4.** Build: ten slices from 23:47 on 29-oct, all green first time by 03:46 (3 h 59 min), zero fix slices and zero replans in the build. Scrutiny 2 rounds (round 1 failed at 04:03 on one blocker, fix `m5-fix-usage-ratios-kind-weight` 6 min, round 2 passed 04:22); user testing 4 rounds by the observer's count (30 of 41 at 08:31; 19 of 24 at 10:42; 4 of 5 at 12:38; PASS 12:44). Money: 186.13 of 200 at 12:15:25Z (orchestrator 81.83 alone, 108.16 with its subagents; workers 36.88; validators 39.52; self-check 1.43); the 11:12Z report read 163.57 against M4 253.73. Ratios on the pinned bases (10.3b) against M4: worker cost per assertion 0.62 against 0.54, coordination 4.48 against 5.99, validation 1.68 against 2.82, tooling 0.41 against 1.81, integration 0.15 against 0.40. All-in cost per assertion 4.77 (186.13 over 39 landed) against M4 7.12; M4's verdict condition (under M3's 4.75) is met on the raw 4.67 and missed by 0.02 all-in (4.81 at 11:12Z); the 04:45 to 07:38 hold (2 h 53 min) is outside cycle time. Verdict: better on time and on cost per assertion; worse on the orchestrator share (93.99 of 163.57, 57 %; 529 turns over 23 completions). M6 boundary item: the usage report splits heartbeat turns from boundary turns. | inbox 1350 item 1 | recorded |
| F-348 | 2026-10-30 | Row `scribe` (kind: rule, readiness rule; inbox 1350 item 2; row 92; Roci FR-W23; KB candidate R-081): **The validation contract was not clear while the spec was.** Receipt: ten slices green in four hours, then three assertions failed as worded while the behaviour was correct, one needed a report only integration writes, three were blocked on an owner input (VAL-METRIC-001, -003, -023 at 10:42), and contract passes 32 to 39 ran inside the milestone (pass 38 at 11:24, pass 39 at 12:44). Rule for the M6 readiness read (R-070), before the first dispatch: every assertion claimed by one slice and testable by the tool it names; every input in the tree or the assertion moves; an assertion that reads an integration output is sequenced after integration; a clause satisfiable two ways is rewritten before the build; the contract is frozen from dispatch to boundary and amended only at a boundary. Detectors: contract.drift, contract.input.missing. Verdict condition for M6: zero "as worded" failures and zero in-unit contract passes; a pass needed in flight is a boundary item with its cost. | inbox 1350 item 2, `MD/contract-work/m6-readiness.md` | recorded |
| F-349 | 2026-10-30 | Row `scribe` (kind: record, readiness rule; inbox 1350 item 3; row 93): **The owner input B-008.** The reference CSV was named as a mission input on 22-oct (BACKLOG B-008; SPEC §3 M5); the precondition of `m5-integrate-m4` said it was present and nobody asked until a slice found it missing (B-242). The delegate's draft from the M0 fixture (00:16) never entered the tree. It held VAL-METRIC-001 and -003 through M5 and is deferred to the M6 boundary (owner, 11:24). Rule: an owner input is a readiness item with a due date (the dispatch of the unit that needs it) and a reminder to the owner at the boundary before it, not a backlog row. M6 boundary item: owner inputs as dated readiness rows (`m6-readiness.md`). | inbox 1350 item 3, BACKLOG B-008, B-242 | recorded |
| F-350 | 2026-10-30 | Row `scribe` (kind: product ask and detector rule; inbox 1350 item 4; row 94; Roci 10.3 "wake rewrites"; Factory product feedback item 21): **The wake-rewrite class.** At four consecutive runner wakes (03:56, 04:12, 04:20, 04:29) the orchestrator wrote 162 k to 210 k tokens each (2.1 to 2.7 USD-eq, about 10 in 35 min) with cacheRead fixed at 38,164. The usage report's strict rule (cache write minus read at least 157,521) counts three (00:52Z, 03:20Z, 03:29Z; 7.82 USD-eq) and undercounts: cache writes are 36.33 of the orchestrator's 71.27 (51 %), and 2,253,093 of its 2,906,765 cache-write tokens sit in 145 narration turns (38.34). Rule E cannot cover it. Detector rule: recognise the class by the same short cacheRead at successive wakes. Product ask: a stable prompt prefix across heartbeat and runner-wake turns, or a documented reason it differs; candidates are the compaction summary re-attached at 04:40 (numMessagesRemoved 1898) and midConversationToolChangesActive true, and the 01:52 occurrence predates the re-attach. Goes to `library/tooling.md`. | inbox 1350 item 4 | recorded |
| F-351 | 2026-10-30 | Row `scribe` (kind: rule, tool facts; inbox 1350 item 5; row 95; Roci FR-G21 model.drift.midrun; KB candidate R-077; Factory product item 23): **The usage limit at 04:43 and Rule G.** The Model-Router moved the orchestrator from claude-fable-5.1 to kimi-k3 at 04:43:46 (overage_proactive), a user-testing seat at 04:43:45 (overage_reactive_402), two self-check sessions at 04:48 and 04:50 and the Opus validator 364e8787 at 05:13:59. Rule 31 held the orchestrator (7 turns on kimi-k3, 0.42; FR-G21's 6 should read 7) and the validator; six seat sessions ran 54 turns on kimi-k3 between 04:43 and 05:11 before one noticed (inbox 0515), about 2.5 USD-eq, reports discarded and groups re-run. Hard stop at 05:22; the router did not restore the declared model after the reset (confirmed 05:51); the owner restored it at 06:27:48Z and resumed at 07:38; mission held 2 h 53 min. The product's Throttled table reads 04:15:43Z to 06:38:43Z as not throttled with no reason and Idle time by cause books it as owner, 134 min. Rule G at the M6 cut: a PreToolUse model-guard hook on every session (sidecar model against the declared model per seat; deny with "undeclared model (rule G): <model>"); measure: turns on an undeclared model per unit (M5: 54 seat plus 7 orchestrator); inverse: remove the hook. Check the report's "1077 turns on non-declared models" before quoting it (it appears to read Rule D's Sonnet seats against their parent role). Product asks: pause, do not route, by default; tell the session when its model changes; restore the declared model on reset; a model_changed event. Owner, 07:32: "our bill is being impacted because of the limits setting, that is bad UX." M6 boundary item: the Rule G hook at the M6 cut. | inbox 1350 item 5 | recorded |
| F-352 | 2026-10-30 | Row `scribe` (kind: verdict, Rule D; held, carried into M6; inbox 1350 item 6; row 96): **Rule D on a fresh round.** M5 scrutiny round 1 (02:51Z to 03:03Z, 11.7 min): 153 Sonnet seat turns 5.8 plus 19 Opus synthesis turns 2.41, about 8.2 for ten features, against M4 round 1 about 27 (32 reviews on Opus) and M3 round 1 18.4; Feature size reads 8.68 for both rounds (34 validator turns). About three times cheaper than M4 per fresh scrutiny, judgement kept on Opus, and the round still found the real blocker (the kind-weighted cost per assertion missing from `m5-usage-ratios`). Stays for M6; user testing already runs the same shape. | inbox 1350 item 6 | recorded |
| F-353 | 2026-10-30 | Row `scribe` (kind: verdict, Rule E heartbeat; held for M4, near break-even under M5's batches; inbox 1350 item 7; row 97; FR-P9h): **Rule E.** M4's evening: three completion wakes, no TTL miss on a completion wake; one heartbeat-cycle miss at 22:58 (240 s of sleep plus the turn crossed the five-minute TTL by about 12 s), so the sleep went to 200 s (inbox 0030). M5: no heartbeat-cycle miss after the change; the three rewrites the report counts are the wake class (F-350). Under batches of about an hour the heartbeat is near break-even: 18 wakes an hour at about 0.13 is about 2.3 an hour, against one rewrite of 1.5 to 2.7 per batch end; it still buys the four-minute inbox read and inflates the turns-per-boundary line (F-347). Lever candidate for M6: heartbeat only while a return is expected within the TTL, or the inbox read moved to the runner; measure: orchestrator cost per batch and inbox latency; inverse: the 200 s heartbeat as today. M6 boundary item. | inbox 1350 item 7 | owner's call |
| F-354 | 2026-10-30 | Row `scribe` (kind: verdict, lead-plus-seats experiment; inbox 1350 item 8; row 98; inbox 2320; Roci FR-P9j): **Two slices ran as a lead worker with three mechanical-sonnet seats each:** `m5-details-tabs-ui` (18 min, 5 assertions) and `m5-panel-charts-and-library-ui` (17 min, 5 assertions); control: the six single-worker slices landed before them, 9 to 27 min each. Cost by the observer's read 2.97 and 2.63 all-in against 1.5 to 3.5; by the usage report's Feature size 2.77 and 2.42 against 0.92 to 2.80 (the product's figures are of record). Verdict at 02:24: at par on time, cost and assertions per hour; the seats carried 20 to 25 % of the turns (lead 67 and 56 turns, seats 19 and 23) because the lead read five to seven minutes before cutting. Not worse, nothing to roll back. Refine for two M6 slices (the lead cuts first from the slice text; the seats carry their parts end to end including tests; the lead integrates and runs the gate) or drop. M6 boundary item. | inbox 1350 item 8 | owner's call |
| F-355 | 2026-10-30 | Row `scribe` (kind: rule, cycle rule; inbox 1350 item 9; row 99; KB amendment to R-067 with D-045): **The batch hand-off format.** D-045 batches worked: slices chained without waking the orchestrator (00:11, 00:53 and 01:17 landed with returnToOrchestrator false; batch 1 was four slices in 75 min). One avoidable wake at 00:30: slice 2 listed its deferrals as whatWasLeftUndone, which returned it to the orchestrator; dismissed at 00:33, format fixed in flight (row 59). Rule: inside a batch returnToOrchestrator is false between independent slices and true only at the batch end; a hand-off carries an empty whatWasLeftUndone and its deferrals in its staging file as claims on the named later slices (M4 item 18). | inbox 1350 item 9, D-045 | recorded |
| F-356 | 2026-10-30 | Row `scribe` (kind: observer post-mortem, blameless, and rule; inbox 1350 item 10; row 100; row 51): **The hook path scope that named wt/m4.** The Rule C hook's path scope named the M4 worktree literally and broke at the boundary when the work moved to wt/m5; re-pointed to a worktree pattern (f13ce91, 00:12); the delegate's gap from the 2103 hook change. Rule: a path scope in a hook or a rule is relative to the current worktree or written as a pattern, never a milestone's literal name; the readiness read checks every hook scope against the next unit's worktree. M6 boundary item: `m6-readiness.md`, Hook scopes. | inbox 1350 item 10 | recorded |
| F-357 | 2026-10-30 | Row `scribe` (kind: observer post-mortem, blameless; inbox 1350 item 11; row 101; row 57): **The header that had 20, 21 or 22 names.** Inbox 0016 said VAL-METRIC-001 labels a 20-column header that lists 21 names; wrong source: the delegate read `contract-work/metrics.md`, a derived copy still carrying the pre-F-033 21-name header. The contract of record, `validation-contract.md` line 1082, has 22 names (throttled_minutes at position 16). Withdrawn at 00:45 (inbox 0045); the oracle draft's 22-name file became the main one, byte-identical to line 1082. Lesson: read the contract of record; a derived copy is a lead, never a source. | inbox 1350 item 11 | recorded |
| F-358 | 2026-10-30 | Row `scribe` (kind: rule, report rule; inbox 1350 item 12; row 102; Roci 10.3b pinned bases): **Three readings of M4's ratios on three bases.** At 01:45 (row 62) the usage report's ratios did not reproduce the hand figures: hand 1.06 / 2.86 / 1.30 / 0.85, regenerated pages 0.54 / 5.99 / 2.82 / 1.81, the product's script 0.87 / 4.64 / 1.80 / 0.86; none was wrong, they answered three questions. The definition was pinned (inbox 0145; 10.3b) and `m5-fix-usage-ratios-kind-weight` (04:07 to 04:13, commit b136279) delivered it; the Ratios section prints its bases line. One defect remains: the Ratios table compares M5 on the pinned bases with an "m4 (recorded)" row that is the hand reading on other bases (6.37 and 6.70). Ask for the next usage-report slice: recompute the previous milestone on the same bases, or mark the recorded row not comparable. Kinds: the product's 12 / 14 / 6 for M4 stands (blocked is not landed); `resume/roci/kinds-after-the-fact.json` still owes its correction. M6 boundary item. | inbox 1350 item 12 | recorded |
| F-359 | 2026-10-30 | Row `scribe` (kind: rule, practice rule candidate; inbox 1350 item 13; row 103; KB, the edition's gate section): **A gate test read host state.** `final-integrate-m5` round 1 (11:24 to 11:39) landed partial: the merge was committed (44a4b47) but the final gate was red, because `scripts/tests/usage_test.py` read the host's boot time and the host had rebooted at 07:12:35 local. The integrator's note writes that time as 07:12:35Z; the report's Host health table gives 06:12:35Z, so the note's label is a record correction. Fix `m5-fix-usage-test-boottime` (10:44:57Z to 10:49:22Z, 4.5 min, 22 turns, 0.42) pins --boottime and adds a test that fails if a runner stops passing it; the gate on code tree f0f20497 is green (11:01:40Z). Rule: a gate reads the tree and the record, never host state a restart changes (boot time, uptime, pids, ports, the clock); host facts go in the report's Host health section, printed, never asserted by a test. | inbox 1350 item 13 | recorded |
| F-360 | 2026-10-30 | Row `scribe` (kind: rule, standing rule and tool fact; inbox 1350 item 14; row 104; Factory product item 22): **The desktop client cancels a running mission.** On 29-oct the Factory desktop app, attached to the running session, cancelled the mission's calls at 21:04 ("Tool execution cancelled by user"; worker 53b27368 cut mid-edit with six files partial) and paused it again at 21:47 (a second client loaded the session). Cost: one 31-minute worker lost and 51 minutes of wall. Factory.app was quit at 21:50 and stays closed for any running mission; the observatory attaches read-only to the record and is the safe way to watch. Product asks: one driver per session, second clients read-only by default; the cancel names its client; a cancelled worker's partial edits are discarded or handed off with the file list. | inbox 1350 item 14 | recorded |
| F-361 | 2026-10-30 | Row `scribe` (kind: rule, report rule; inbox 1350 item 15; row 105; Roci FR-M10): **Three clocks.** At 23:09 on 29-oct the TUI timer read 34 h 57 m for the mission, against about 172 h of wall since its start and about 54 h with the mission running. Every milestone row carries system time (seats running and the officer awake), cycle time (dispatch to landing, pauses listed by cause) and wall time (calendar), with the like-for-like row on cycle time and system time as the base for cost per hour. M5: wall from 23:47 on 29-oct to 14:15 on 30-oct (14 h 28 min); held, no owner, 2 h 53 min; cycle 11 h 35 min; system about 10 h 53 min (cycle less the 42 min stop-and-decide wait from 10:42 to 11:24). Defect for the next usage-report slice: Idle time by cause books the hold as owner (134 min) and the gap 09:49Z to 10:20Z (31.52 min) as unexplained; the causes need the router transitions and the owner's questions as evidence. M6 boundary item. | inbox 1350 item 15 | recorded |
| F-362 | 2026-10-30 | Row `scribe` (kind: record, time decomposition of M5; inbox 1350 item 16; row 106): **Time decomposition of M5** (from the record and the boundary report of 12:15Z). Build 23:47 to 03:46 (3 h 59 min, four batches); scrutiny round 1 03:51 to 04:03, fix 04:07 to 04:13, round 2 to 04:22; user testing round 1 04:24 to 05:15 and 07:38 to 08:31; held, no owner, 04:45 to 07:38; fix batch F1 to F4 08:45 to 09:56 (71 min); user testing round 2 10:00 to 10:42; owner stop at the M5 cap 10:42 to 11:24; integration round 1 11:24 to 11:39 (partial), fix 11:45 to 11:49, round 2 11:49 to 12:20; user testing rounds 3 and 4 to 12:38 and 12:44; PR #40 opened at 14:13, boundary report at 14:15. Totals by segment (min): building 320 (build 239, scrutiny fix 6, fix batch 71, integration fix 4); validating 190 (scrutiny 21, user testing 103 + 42 + 24); integrating 46 (15 + 31); closing 78; owner wait 42; held, no owner, 173; hand-offs 19; total 868, the wall. Validation is 59 % of build time by the clock (190 against 320) and 22 % of the wall; by money at 11:12Z it was 119 % of the contract and tooling workers (35.33 against 29.62). | inbox 1350 item 16 | recorded |
| F-363 | 2026-10-30 | Row `scribe` (kind: record, money; stop-and-decide lines, R-055; inbox 1350 item 17; row 107): **Money.** The mission line moved from 1350 to 1500 at the M4 boundary (23:40 on 29-oct); the M5 cap from 150 to 190 (D-048; the usage report dates it 09:55Z; the seven answers logged at 11:24, row 80). Readings: the orchestrator's own at 09:57, M5 window 135.49 and mission to date 1,396.23; at 11:49, M5 158.5 of 190; the usage report at 11:12Z, M5 163.57 (26.43 under the cap) and mission to date 1,260.74 + 163.57 = 1,424.31 (the tool's all-lines table reads 1,033.56 because the logs before 22-oct rotated out, B-254). The orchestrator alone is over its 30 line (71.27, plus 22.71 in its subagents). M5 landing 186.13 of 200 (13.87 under the cap); mission to date 1446.87 of 1500 (53.13 under the line); M6 fits only if its window stays under 53.13 less the fix feature and the PR re-run (about 10 to 15), so about 40, or the line moves (it moved to 1700, D-049, F-346). The report's weekly share forecast for M6 (2 claimed assertions at 4.81, 9.62) prices only the claims. M6 boundary item: the cap is set at dispatch, priced by role, with validation by rounds from M5's record. | inbox 1350 item 17, D-048, D-049 | recorded |
| F-364 | 2026-10-30 | Row `scribe` (kind: record, owner inputs still open, for the M6 readiness read; inbox 1350 item 18; row 108; item 3): **Owner inputs still open.** (a) VAL-PROC-019: the owner's screenshot at `reports/screenshots/2026-10-30-m5.png`; the untracked `board-browser-m0.png` is the validator's browser capture, not the owner's (owner, 12:30: "I didn't take it"; inbox 1230). (b) The reference CSV: the draft at `resume/roci/oracle-draft/` (19 rows, the 22-name header byte-identical to `validation-contract.md` line 1082, METHOD.md with 15 review points); deferred to the M6 boundary with VAL-METRIC-001, -003 and the new VAL-METRIC-026 owned by `m6-reference-csv-oracle-parity` (12:42). (c) Seven contract-literal gaps rowed by `final-process-record` at 12:57: VAL-PROC-002, -006, -008, -009, -018, -024 and -033 (ruled at row 90). (d) The native save dialog: `m5-export-and-parity-ui` kept to a path field under rule 3; adding tauri-plugin-dialog needs `make deny` and a version check on the day, the owner's decision. (e) The haiku scratch-mission directories named in the usage report (B-255), the owner's to delete (ruled at row 90). M6 boundary item: (a) and (b) are dated readiness rows with a nag at the M6 boundary. | inbox 1350 item 18, B-255 | recorded; (d) owner's call |
| F-365 | 2026-10-30 | Row `scribe` (kind: observer post-mortem, blameless; the delegate scores itself at the boundary; inbox 1350 item 19; row 109): **Mistakes of the night and their corrections.** (a) The 0016 header claim from a stale derived copy, withdrawn at 00:45 (F-357). (b) A seat-activity read that took the maximum of 'HH:MM:SS' strings across midnight and reported 'last seat turn 23:57' while seats were active at 00:39; totals unaffected, corrected on re-read at 00:28 and 00:40. (c) The 12:44 watch-log line first written as 12:58, a guessed clock; standing rule: read the clock in the same turn. (d) Hand reads the product computes differently (the experiment's slice costs, F-354; M4 first dispatch 10:25 not 09:27 and completions 36 not 31 plus 8; FR-G21's 6 orchestrator turns on kimi-k3 against the report's 7). (e) The night's forecast of 95 to 120 at the boundary against 163.57 at 11:12Z: it priced one scrutiny fix and one user-testing round where the record had four user-testing rounds, six fix slices and two integration runs; lesson: forecast validation by rounds from the previous milestone. (f) The validation phase was not flagged before M5: attention followed the bill (the orchestrator, 45 to 54 % of M1 to M4); validation cost per assertion (2.50, 1.48, about 1.5 and about 0.9 from M2 to M5; Rule D's fresh round from about 27 to 8.2) fell every milestone while the phase share and time stayed flat (validating 89, 105, 102, 102 and about 92 against building 128, 104, 63, 103 and about 61). Lesson: read the same ratio across phases at every boundary, with a band; a phase that stays flat while the others fall is the first card (Roci FR-M9). Rules that held: behaviour levers only in flight (FR-P9i); the bounds on the oracle fallback (inbox 0230); the one-process answer (2103); the 150 stop (the 05:22 hard stop, M5 at 99.2 by the observer's read; the raise to 190 was the owner's); the choice not to pause at 05:15. | inbox 1350 item 19 | recorded |
| F-366 | 2026-10-30 | Row `scribe` (kind: rule; inbox 1319, owner 2026-10-30 13:19 "let's carry this work in M6"; row 110; relayed by pre-roci): **M6 runs under the validation-loop rules** (a boundary item for the M6 plan, not an in-flight change). Record M1 to M5: validation has cost 89 to 105 USD-eq per milestone while the build fell to about 61; in M5 validation cost one and a half times the build (rowed in the M5 feedback, items 2, 3, 6, 8, 9; Cody delta 6; KB candidates R-081 to R-086). Six rules in force from the first M6 dispatch: (1) contract readiness before the first dispatch and the contract frozen for the unit, no contract pass between dispatch and boundary (F-348); (2) a claim is a receipt: the exact command run and its output, validators replay mechanical claims and spend judgement only on behavioural and mixed ones; (3) from round 2 re-grade only the affected assertions, carrying the passed set as evidence; (4) no dismissal of a hand-off item without the assertion grep, an item that names behaviour an assertion covers is a fix; (5) validators as leads with Sonnet seats (Rule D), the owner will consider a cheaper model for the user-testing validator kind, scrutiny keeps judgement on Opus; (6) owner inputs named by the contract are readiness rows with a date and an inbox nag (the reference CSV, the owner screenshots). Verdict condition: "validation under the build's cost" (with item 2 of inbox 1350: zero "as worded" failures and zero in-unit contract passes). Record per unit: validation share of cost and time, rounds and failed rounds, assertions re-tested per round, dismissed items later failed. | inbox 1319, `MD/AGENTS.md` "M6 rules" | in force from the first M6 dispatch |
| F-367 | 2026-10-30 | Row `scribe` (kind: money, scope: cycle; owner: "M6 gets its own cap at dispatch, priced by role with validation by rounds from M5's record"; D-049; R-055): **M6 unit 1 "M5 carry-over: release gate" cap 49 USD-eq**, declared 2026-10-30 13:06Z; window from 2026-10-30T12:15:25Z (`--milestone m6`), so spend after the M5 close books to M6. Boundary-closure line 7 (measured 13:04Z, cross-check identical on the second run: orchestrator 3.96, orchestrator subagents 2.51, total 6.46 USD-eq). Slice priced by role from M5's unit costs (per completed feature 1.94; user testing per round 7.71, M5 rounds 16.45, 10.20, 3.34, 0.85; scrutiny per run 4.34; orchestrator per runner wake 4.31; scribe per wake 1.39): workers 6 (two features), orchestrator 9, orchestrator subagents 6, scrutiny 5 (one run over the three `final` features), user testing 15 (two rounds over the 44 pending `final` assertions, round 2 affected-only), self-check 1 = 42. Cap 7 + 42 = 49; stop-and-decide under R-055; mission line 1700 (D-049). The rest of M6 is priced at its cut. Contract frozen from 13:06Z (dispatch of `m6-carryover-fix-pr-gate`); last pass 40. | D-049, `MD/AGENTS.md` "M6 rules", `MD/contract-work/m6-readiness.md` | open (m6) |
| F-368 | 2026-10-30 | Pass 41, owner rulings of 2026-10-30 ~14:55Z (kind: decision, scope: cycle; contract pass 41 is a boundary amendment, wording only, `MD/contract-work/pass-41-ruling.md`): nine wording gaps of the final user-testing round 1 are waived by wording-only amendment (VAL-PROC-005, 011, 023, 027, 028, 034, 035, 036, 046; 036 reads "opened by the integration worker the orchestrator dispatched (session 56c00d03, 2026-10-30T12:02:53Z)"; 011 accepts PR `gate` run 41068058423 and the cold merge run 41045311362, 228 s, as the m5 measurement with no decision file; 027 accepts the fallback models of recorded episodes as a record), status failed -> pending for round 2; two forward-fixable rows (VAL-PROC-002, 015) get a forward docs addendum through feature `m6-carryover-docs-addenda` (B-264 row adds `b9cdd45`; README and m0/m1 usage-report addenda with wall times and left-out reasons), failed -> pending; four historical rows (VAL-PROC-003, 016, 029, 032) stay failed on the record with the validator's fact beside each, no history rewrite (RULES rule 12). Round 2 (affected-only, head `273eb9e`) passed 002, 011, 015, 023, 027, 034, 035, 036 and 046 and failed 005 and 028 as worded (F-370). **Alternatives considered:** rewrite history or backfill (rejected: rule 12); waive the four historical rows (rejected by the owner: they are facts that cannot be made true); one more full round (rejected: one affected-only round inside unit 1's 49). | `validation-contract.md`, `MD/contract-work/pass-41-ruling.md` | done (pass 41; round 2 graded) |
| F-369 | 2026-10-30 | Pass 41, owner ruling of 2026-10-30 ~14:55Z (kind: decision, scope: cycle): VAL-CROSS-002 and VAL-CROSS-007 are narrowed to one m6 measurement on the m6 head (002: one live pair, line timestamp against board-visible time under 1000 ms, plus the attach_latency p50/p95/max line; 007: CLI `unknown events: N` against the tray for the M0 and S captures at the m6 head, both 0), status stays pending, claimed by `m6-boundary-cross-002-007-ruling`, which the orchestrator renames and rewrites at the M6 cut. **Alternatives considered:** measure at every milestone retroactively (rejected: the milestone heads no longer run). | `validation-contract.md`, `MD/contract-work/pass-41-ruling.md` | open (m6) |
| F-370 | 2026-10-30 | Pass 42, owner ruling of 2026-10-30 ~15:45Z (kind: decision, scope: cycle; row 120 in `MD/contract-work/m4-boundary-questions.md`, verbatim: "Option (a): pass 42 wording-only correction with the validator's text for VAL-PROC-005 and 028, regraded from the round-2 evidence by one Sonnet seat, no new run."): VAL-PROC-005 and 028 failed round 2 as worded, because the pass 41 clauses misstated a fact. For 005 the clause listed a `com.apple.WebKit` ESTABLISHED line on 3141 for m3 that no m3 evidence holds (it now reads "at m3 the Running line, the e2e harness run and LISTEN exit 1, without the WebKit and System Events lines"); for 028 the clause said the m0 to m2 reports carry the 20 % orchestrator line, but `reports/usage/m2.md:12` retires it under F-040 and measures 122.64 against the 30 stop (the Pass sentence now reads "m0 and m1 carry the 20 % line; m2 the delta against the 30 stop (F-040); m3 to m5 the log-derived row, m4 with the comparison in D-043"). Wording only, one clause per assertion, the pass 41 waiver lines stay as history; regraded from the round-2 evidence (`evidence/final/r2-git-gh/`, `evidence/final/r2-reports/`) by one Sonnet seat, no new run, verdict file `validation/final/user-testing/r3-regrade.verdict.json`. Timing: a boundary amendment made after unit 1 closed (15:33:01Z) and before the M6 cut, not an in-unit pass. Count: 272 assertion headings before and after, two changed. | `validation-contract.md`, `MD/contract-work/pass-42-ruling.md` | done (pass 42; regrade by the seat) |
| F-371 | 2026-10-30 | Row `scribe` (kind: money and verdict facts, scope: cycle; owner chat 2026-10-30 ~15:45Z, row 120; D-049; R-055): **M6 unit 1 "M5 carry-over: release gate" closed 2026-10-30T15:33:01Z (the end of `user-testing-validator-final` round 2) at 48.34 of 49 USD-eq** (`python3 scripts/usage-report.py --milestone m6 --from 2026-10-30T12:15:25Z --to 2026-10-30T15:34:49Z --cross-check`, identical, 31 sessions, 894 turns; `reports/usage/m6-unit1.md`): orchestrator 24.82 against its slice 9, orchestrator subagents 5.45 against 6, workers 3.59 against 6, scrutiny 4.77 against 5, user testing 9.71 against 15 in two rounds (6.15 and 3.56), self-check 0.00 against 1. Mission to date 1446.87 + 48.34 = **1495.21 of 1700** (204.79 left). Validation share 14.48 of 48.34 = 30.0 %, against the build line 3.59. PR #40 head `273eb9e`, `gate` run 41077321670 green, `mergeStateStatus` CLEAN, open, not merged. Verdict condition facts (inbox 1319, facts only): two as-worded failures after round 2 (VAL-PROC-005, 028, corrected by pass 42, F-370) and one in-unit contract pass (pass 41, F-368, F-369). Owner dispatch rules of the same message, verbatim: "Proceed with the M6 cut now under a boundary allowance of 14, then dispatch M6 and run it to its close: caps by role from M5's unit costs, validation priced by rounds, the 1319 validation-loop rules, mission line 1700, stop-and-decide only at a cap or a contract gap." The boundary allowance is 14 USD-eq; the orchestrator line is over its slice by 15.82 and the window total is 0.66 under the cap, which the ruling accepts as recorded (row 119 held the question). | `reports/usage/m6-unit1.md`, D-049, `MD/AGENTS.md` "M6 rules" | done (unit 1 closed); M6 cut open |
| F-372 | 2026-10-30 | M5 carry-over, release gate. PR #40 head `72d0412` (gate run 41052611163) failed `sast` and `fs-scan`. Fix on the PR head: the six `rust.lang.security.temp-dir.temp-dir` findings are gone because the test helpers use `tempfile::tempdir()` (a `TempDir` that deletes itself) instead of `std::env::temp_dir()`; the two `bash.lang.security.ifs-tampering` findings are gone because `named-tests.sh` splits the body patterns with `tr` and reads them with `IFS= read`; the two Trivy `github-pat` criticals on the logged placeholder are suppressed by a path-scoped `.trivyignore.yaml` loaded through `trivy.yaml` (the build-log is append-only, history is not rewritten). The first push of the fix (run 41063340658) showed a third finding the secrets had hidden: the glib advisory (id in `.trivyignore.yaml`) (glib 0.18.5, MEDIUM, `Cargo.lock:1545`, through tauri's Linux GTK3 stack); it was already in the old run's SARIF as a warning. It is ignored for `Cargo.lock` with an expiry of 2027-01-28 (90 days, the cargo-deny re-check habit). Alternatives: a plain `.trivyignore` (rule id for the whole repository, rejected: it would silence the rule everywhere); a redaction commit (rejected: it leaves lines 218 and 219 in history and the scan reads the tree, not history). No dependency added: `tempfile` 3.27.0 was already a dev-dependency of the three crates, so `make deny` did not run. | m6-carryover-fix-pr-gate | done (m6 integration) |

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
| M4 | 2026-10-29 | `wt/m4` merged as `3f38afe` (`Integrate m4: wt/m4`): `GitSource` and `GitHubSource` behind the `Forge` trait (PR detection, Actions runs and commit statuses, rulesets, merge policy with attribution, `handed_to_cd`), the gate group and badge, return arc and fix cycle (migrations 0003, 0004), the Merge button with the clean `403`, the demo pair `demo/base` with the `demo-gate` workflow (D-040), forge off without `GITHUB_TOKEN` or `FORGE_LIVE=1`, the card names its worktree; tooling: `make integrate-long`, `make check-contract`, `make check-kit`, the code-path gate key and CI `paths-ignore`, usage report tables, rule C and rule F hooks (D-041, D-042); D-043 to D-046 at this boundary | `make check` exit 0 on the merged code tree `e8f7cea9` (real run, 443.829 s; a MISS against the validated `d2d4b6e9` for the three `.factory` hook paths); the nine M4 tests of §3 `ok` in `reports/gate/e8f7cea9…/test.tests`; `contract checks: 19 ok, 12 skipped, 0 FAIL`; coverage 93.71 % lines on `observatory-core` (175.776 s, second attempt; first attempt red on a latency test, B-181); `mission-checks` green on attempt 2 of run 40964294625 (cold, `workflow_dispatch`, `reports/ci/2026-10-29-m4.md`); validation: 181 assertions passed mission-wide, VAL-PROC-047 to 050 and VAL-TOOL-005 failed, VAL-GATE-027 and 033 blocked (owner rows 28, 33, 40 open), VAL-E2E-004 and VAL-PROC-019 handed to QA | `reports/usage/m4.md`: window total 253.73 USD-eq against the cap 200; orchestrator 111.86 against 30; 7.93 USD-eq per assertion (M3 4.75); mission to date 1260.74 of the 1350 line (93.4 %) | milestone cap 200 and orchestrator cap 30 exceeded and accepted as data by the owner (D-043, F-267); slice cap of item 21 rolled back (D-045, F-269); telemetry to a local collector (D-046); `reports/reference/2026-10-22-observatory-v1-m0.csv` missing at the boundary: stall flag (B-008); SPEC §9 rows F-071 to F-160 archived (`docs/spec/section-9-archive.md`) | 0 on the S and M0 fixtures (golden `cecf727d…` unchanged); 0 `unknown.event` from the git and GitHub sources (VAL-GATE-029) |
| M5 | 2026-10-30 | `wt/m5` merged as `44a4b47` (`Integrate m5: wt/m5`, second parent `d62c256`): time states and rework windows, cost and tokens on the four-class formula, the metrics report and its oracle (`observatory-cli metrics`, `--csv`, `--json`, `throttled` apart from the eight waste causes), the telemetry series plan (D-047, 182 worst case under `max_series` 200) and the exporter to a local collector (D-046), the five Details tabs, Panel charts and library, export and parity UI, usage ratios by kind weight, and the gate and flake fixes; D-047 and D-048 at this boundary | `make check` exit 0 on the merged code tree `f0f20497` (a MISS against the red tree `afe89608`, real run 656.131 s: lint 197.286, test 333.934); 634 named tests `ok` and no `FAILED` in `reports/gate/f0f20497…/test.tests`; `contract checks: 19 ok, 12 skipped, 0 FAIL`; coverage 94.08 % lines on `observatory-core` (197.772 s); `mission-checks` green, run 41045311362 on `2bcca58`, cold, 228 s, `machete` and `coverage` success (`reports/ci/2026-10-30-m5.md`); 220 assertions `passed` in the state file, 5 failed, 2 blocked, 2 `handed_to_qa` at this record | `reports/usage/m5.md`: window total 186.13 USD-eq against the cap 200 (D-048, D-049; measured 2026-10-30 12:15:25Z, F-346; the committed record reads 163.57); orchestrator 81.83 against 30 (over, F-337; the committed record reads 71.27 and stays as written) plus 26.33 in its subagents; 4.77 USD-eq per assertion (186.13 / 39 fulfilled; M4 7.93); mission to date 1446.87 of the 1700 line (D-049) | M5 cap 105, then 140, 150, 190 (D-048, owner chat 09:55Z); orchestrator over 30 and accepted (F-337); `kimi-k3` fallback and 143.0 min pause (F-341); VAL-METRIC-001 and 003 deferred to M6 with the missing reference CSV (B-242, B-253); VAL-PROC-013 open for M3 and M4 (F-338); cold CI over the 3 min budget (F-339); VAL-PROC-019 open, owner screenshot missing at all seven boundaries (F-340); rotated logs lost, mission to date from the recorded M4 base (B-254) | 0 on the S and M0 fixtures (golden `cecf727d…` unchanged, `g5-frozen-fixtures ok`); 0 `unknown.event` from the git and GitHub sources |
| M6 unit 1, M5 carry-over: release gate | 2026-10-30 | No worktree merge (unit 1 ran in the main checkout on `mission/observatory-v1`): the PR #40 gate fix (`tempfile::tempdir()` in the three crates' test helpers, `named-tests.sh` with `tr` and `IFS= read`, `.trivyignore.yaml` loaded through `trivy.yaml`, the glib advisory ignored until 2027-01-28; F-372, B-272 to B-274), the `final` scrutiny run (1 run, 38 claims, no blocking issue) and two user-testing rounds (38 assertions, then 11 affected-only; VAL-PROC-002, 011, 015, 023, 027, 034, 035, 036, 046 passed in round 2), the docs addenda for VAL-PROC-002 and 015 (`273eb9e`), pass 41 and pass 42 (F-368 to F-370) | PR #40 head `273eb9e`: `gate` run 41077321670 green (1096 s from 15:04:30Z to 15:22:46Z, `light / mission-checks` 219 s), `mergeStateStatus` CLEAN, open; round 2 passed 9 of 11 assertions, 005 and 028 failed as worded and were corrected by pass 42; `validation-state.json` at the hand-off: 252 passed, 11 failed, 6 pending, 2 blocked, 1 `handed_to_qa` of 272 | window 2026-10-30T12:15:25Z to 15:33:01Z: **48.34 of 49 USD-eq** (orchestrator 24.82 against 9, orchestrator subagents 5.45, workers 3.59, scrutiny 4.77, user testing 9.71 in two rounds 6.15 + 3.56, self-check 0.00); mission to date 1495.21 of the 1700 line (D-049); validation 14.48 = 30.0 % of the window (`reports/usage/m6-unit1.md`) | orchestrator over its slice by 15.82 (F-371); `m6-carryover-fix-pr-gate` 64 turns on a budget of 60; the docs worker's hand-off carries no claims array and no `model =` line (rule 30); four historical rows stay failed on the record (VAL-PROC-003, 016, 029, 032; F-368); two as-worded failures and one in-unit pass against the verdict condition (F-370, F-371) | not measured in unit 1 (no mapping change; VAL-CROSS-007 is measured once at the m6 head, F-369) |

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
