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
| F-001..F-160 | 2026-10-22 to 2026-10-25 | Rows F-001..F-160 (S to M3; F-138 to F-160 are the owner items and answers of the M3 boundary, received 2026-10-29) are archived verbatim in [docs/spec/section-9-archive.md](docs/spec/section-9-archive.md) | all | archived |
| F-161 | 2026-10-29 | Sized fix (rule 29), pattern, cycle: SPEC.md was 202941 bytes against the 204800-byte limit of `scripts/check-file-size.sh` and every boundary adds §9 rows; the rows of the S, M0, M1 and M2 boundaries (F-001 to F-070: F-001 to F-053 dated before 2026-10-25, F-054 to F-070 the owner items of the M2 boundary, dated 2026-10-25) move byte-identical to `docs/spec/section-9-archive.md`, one pointer row stays at the top of this table, the M3 rows (F-071 on) stay in place. F-001 to F-053 alone left SPEC.md at 169649 bytes, over the 150 KB target. **Alternatives considered:** raising `MAX_FILE_KB` (rejected: hides the growth), archiving §10 too (not needed: 15225 bytes). **Second pass at the M4 boundary (integrator):** merging the 33 M4 staging files took SPEC.md to 234834 bytes, so the M3 rows F-071 to F-160 moved byte-identical to the same archive (90 rows, 51210 bytes), the pointer row now reads F-001..F-160 and SPEC.md is 183712 bytes before the boundary's own rows are added. | `scripts/check-file-size.sh`, `docs/spec/section-9-archive.md` | done |
| F-162 | 2026-10-29 | rule 28 amendment (owner item 14b): the mechanical clauses of the pending contract assertions are gate checks. `scripts/contract-checks/g<N>-<slug>.sh`, one per proposed check of `contract-work/pass26-mechanical.md`, each printing one line `<check> <ok\|FAIL\|skip (<why>)> <assertion ids> [pending: ...]` and exiting 0 or 1; `make check-contract` runs the 31 scripts in parallel (`run-all.sh`, 4.4 s without `GH_TOKEN`, 4.7 s with it), `make check` runs it after the cached gate with `CONTRACT_REQUIRE_TESTLOG=1`, and the `contract` job of `mission-branch.yml` (in `mission-checks.needs`, required by `check-ci-jobs.sh`) runs it in CI. Validators read the check's line instead of grading a `Kind: mechanical` clause. A clause whose code has not landed is `pending`, never red: `g2-tests.tsv` rows carry `landed=no` until the test is written, then flip and the test must match its body patterns and read `ok` in the gate's test log. `scripts/gate.sh` now writes `reports/gate/<code tree>/test.tests` beside the `test` record (`R <binary>`, then `ok\|FAILED\|ignored <name>`, 463 lines on this tree). G7 (`gh`) checks print `skip (no network)` without `GH_TOKEN` or `GITHUB_TOKEN`. G8 and G10 are gate-record stages, documented in `scripts/contract-checks/README.md`, not scripted. Evidence: `scripts/tests/contract_checks_test.sh` (every check green on the tree and red on a planted violation; red commits `fee904e`, `81aebaa`, `6fc0bc4`, `8b02edd`, `021ff58`, `3a4b66d`, `c6fe9f4`; green `245b0a5`, `b429a66`, `27399d7`, `3735379`, `dcb6cbe`, `9cf927f`, `4c8fa50`). | m4-contract-checks-gate | done (m4 integration) |
| F-163 | 2026-10-29 | fix to the contract text of VAL-PROC-008: the clause's `grep -vc` over `git log --format='%H %(trailers:...)'` also counts the blank lines of multi-line trailer output (366 false hits on this branch); `g6-commit-format` reads the trailer per commit instead. | m4-contract-checks-gate | done (m4 integration) |
| F-164 | 2026-10-29 | the demo branch pair exists (SPEC §3 M4 test target, entry 19): `demo/base` was pushed on 2026-10-29 from `mission/observatory-v1` at `3ccf3aac518a327d3c73c03cbc9efdf77bbc1ce8` (`git merge-base --is-ancestor` exit 0 against `origin/demo/base`); label `demo:red` created; ruleset `demo-gate` id `26112925` (enforcement active, include `refs/heads/demo/base` only, no bypass actors; rules `pull_request`, `required_status_checks` with the single context `demo-gate`, `non_fast_forward`) created with `gh api` under the owner's login by the infra worker for the orchestrator. `main` and `main-gate` (id 25636994) were not touched. Decision file D-040. | m4-demo-branch-and-ruleset | done (m4 integration) |
| F-165 | 2026-10-29 | the worktree took the mission branch's M3 boundary docs by a no-ff merge (`b82fac9`) so D-040 follows D-039. | m4-demo-branch-and-ruleset | done (m4 integration) |
| F-166 | 2026-10-29 | `.github/workflows/demo-gate.yml` (commit `1d994af`): `pull_request` to `demo/base`, types `opened`, `synchronize`, `labeled`, `unlabeled`; `permissions: contents: read`; one job `demo-gate` (`timeout-minutes: 2`) that exits 1 when `github.event.pull_request.labels` holds `demo:red`; a concurrency group per PR number with `cancel-in-progress`, so the newest label change owns the check. Red test `084ad15` (`scripts/check-ci-jobs.sh` exits 1, workflow missing), green `1d994af` (exit 0). | m4-demo-gate-workflow | done (m4 integration) |
| F-167 | 2026-10-29 | smoke proof on throwaway PR #24 (`demo/smoke-1`, head `1d994af0`): labeled `demo:red` at 2026-10-29T09:58:31Z, run `40853828900` failure; unlabeled at 09:59:02Z, run `40853894595` success. `opened` run `40853828337` was cancelled by the concurrency group because the `labeled` event arrived in the same second. PR closed, `demo/smoke-1` deleted (ref API 404 afterwards); `demo/base` stays. `main` and `main-gate` untouched. Proof file: `reports/ci/2026-10-29-demo-gate-smoke.md`. | m4-demo-gate-workflow | done (m4 integration) |
| F-168 | 2026-10-29 | The B-056 trim and the second warm measurement were already landed by `m4-gate-key-and-ci-paths` (`88ffd91`, run 40851767262 attempt 2 = 144 s); nothing to redo here. | m4-demo-gate-workflow | done (m4 integration) |
| F-169 | 2026-10-29 | tooling and test policy (rule 26, verified on 2026-10-29 by `scripts/tests/e2e_run_test.py`, `scripts/tests/usage_e2e_test.py` and two real `make e2e` runs; owner inbox file `docs/owner-feedback/inbox/2026-10-29-1552-flaky-e2e-is-a-feature.md`). `make e2e` runs the suite through `scripts/e2e_run.py`. (1) On the first failing case the wdio `afterTest` and `afterHook` name the spec in `failed-specs.txt` and call `e2e_run.py first-failure` while the app is up; `reports/e2e/<run>/first-failure/` holds `environment.json` (E2E_RUN_DIR, ports with the pid that listens, process lines, GITHUB_TOKEN `set` or `unset` and never its value, FORGE_LIVE), `run-dir-listing.txt`, `backend-logs/` (the `captureBackendLogs` `wdio-*.log`), `screenshots/` (`fail-*.png`), `timing.jsonl`, `failed-specs.txt`; written before any rerun and never rewritten by it. (2) At most one automatic rerun of the failed specs, on a clean run directory; a failure that names no spec is red without a rerun. `record.json` keeps both attempts (outcome, exit code, wall seconds, failed specs) and one outcome: `passed`, `flaked` (first attempt red, rerun green: exit 0, never counted as a pass) or `failed` (the rerun is red too, a plain red). `junit.xml` carries it as `<testsuites outcome="...">` and the rerun-replaced suites carry the property `outcome=flaked`; `attempt-1-junit.xml` and `attempt-2-junit.xml` are the two reports. (3) The usage report's testing section (`scripts/usage_testing.py`, `--e2e-dir`, default `reports/e2e`) gains `### E2E runs` (one row per record in the window), one `e2e flake: <run>: <specs>; rerun <s> s (first attempt <s> s)` line per flaked run and `e2e runs: N, passed P, flaked F, failed X; rerun seconds: S`. Proof: `E2E_INDUCE_FAILURE=panel make e2e` (the knob in `lib/induce.ts` fails the panel spec's first case on attempt 1 only) printed `e2e: flaked (first attempt red, rerun green): specs/panel.e2e.ts; rerun 6.19 s of 100.37 s`, exit 0, 2m28.899s; a plain `make e2e` printed `e2e: passed (93.73 s)`, exit 0, 1m57.076s. **The cause of the original red is not proven and not fixed here**; this feature is what proves it next time. **Alternatives considered:** retry inside mocha (`mochaOpts.retries`; rejected: it hides the first failure inside one report and keeps no evidence); a `flaky-e2e` note in the validator report (rejected by the owner, rule 29); an unlimited or two-fold rerun (rejected: three full runs paid for one green is the waste the owner named). | m4-e2e-first-failure-evidence | done (m4 integration) |
| F-170 | 2026-10-29 | tooling proof (rule 26, 2026-10-29). The flaked outcome is proven on the real wdio reporter: one `E2E_INDUCE_FAILURE=panel make e2e` on a quiet machine (1-minute load 3.28, no cargo, trunk or wdio process) took `real 2m5.402s` and ended `e2e: flaked (first attempt red, rerun green): specs/panel.e2e.ts; rerun 6.15 s of 99.96 s`; run directory `reports/e2e/2026-10-29/162525/` (`record.json` outcome `flaked`, attempts 93.81 s then 6.15 s; `junit.xml` root `outcome="flaked"`, `grep -c '<failure'` = 0, parses with `/usr/bin/python3` minidom). `usage-report.py` prints `e2e flake: 2026-10-29/162525: specs/panel.e2e.ts; rerun 6.15 s (first attempt 93.81 s)`. The earlier failed run `reports/e2e/2026-10-29/161511/` (outcome `failed`) is whole beside it: two runs one day, nothing deleted. In `junit.xml` each suite replaced by its rerun carries `rerun_outcome` (`passed` or `failed`, read from the rerun suite itself) and `first_attempt_failures`; before, the property copied the run outcome, so a failed run labelled green-on-rerun suites `failed`. The `e2e.yml` artifact upload excludes `reports/e2e/latest/**` (the symlink would upload the newest run twice). `make check-contract`: 22 ok, 9 skipped, 0 FAIL in 5.8 s. | m4-e2e-flaked-proof-run | done (m4 integration) |
| F-171 | 2026-10-29 | tooling and test policy (rule 26, verified on 2026-10-29 by `scripts/tests/e2e_run_test.py` (26 tests), `scripts/tests/usage_e2e_test.py` (11) and one real `E2E_INDUCE_FAILURE=panel make e2e` run). Each `make e2e` run writes under its own directory `reports/e2e/<date>/<HHMMSS>/` (`E2E_STAMP`, the start time): `record.json` (field `run` is `<date>/<HHMMSS>`), `junit.xml`, the attempt junit files, the PNGs and `first-failure/`. No target and no script deletes a report directory; `e2e_run.py run` refuses a directory that already holds a `record.json` (exit 2, the suite is not started). The latest run is reachable at `reports/e2e/latest`, a relative symlink set by `e2e_run.py run --latest` before the suite starts (so a run that dies is still reachable); a symlink and not a pointer file so that every path read today (`junit.xml`, `record.json`, the PNG convention `card-lane3-cycle-1.png`) only gains the `latest/` segment; it is git-ignored. `scripts/usage_testing.py` counts every `reports/e2e/<date>/<HHMMSS>/record.json` in the window (and the flat `<run>/record.json` of the runs before stamps), skips the `latest` link, so a flaked run stays an `e2e flake:` line after later green runs. **Alternatives considered:** a `latest` pointer file (rejected: every reader needs a lookup step, the PNG paths change shape); keeping `reports/e2e/<date>/` and renaming the old run aside (rejected: a copy step that can be skipped, the failure this feature closes). | m4-e2e-keep-run-records | done (m4 integration) |
| F-172 | 2026-10-29 | fact — The card names its worktree: `Capture` holds `worktrees` (`milestone_worktrees` command, asked when a mission opens and on every `events_appended` of the open mission); `card_view` draws `<p class="mono" data-testid="card-worktree" title=<path>>name</p>` for the card's milestone and nothing for a milestone git lists no worktree for. One kit class (`.mono`, kit rule 2: identifier text); no `style.css` rule, no `kit-departures.md` or `kit-pending.txt` row; `make check-kit` ok. | m4-fix-card-worktree-name | done (m4 integration) |
| F-173 | 2026-10-29 | fact — The e2e build's `e2e_gate_poll` also puts one worktree (`wt-e2e-gate`, `/e2e/worktrees/wt-e2e-gate`) in `GitCache` for the mission, as a git poll would have cached it (the e2e config polls every 3600 s, so the real thread never fills it). | m4-fix-card-worktree-name | done (m4 integration) |
| F-174 | 2026-10-29 | fact — `ingest::git::git_sync` is `#[cfg(feature = "store")]`: `observatory-ui` builds core without the store feature and `trunk build` had failed since 716f610 on `use crate::store` (E0432). The e2e build before this feature ran the stale `ui/dist`. Red: `cargo check -p observatory-ui --target wasm32-unknown-unknown` exit 101; green: exit 0 (d63f4cc). | m4-fix-card-worktree-name | done (m4 integration) |
| F-175 | 2026-10-29 | fact — `github.rs` no longer writes `missions.branch` (the git thread's `git_sync` writes the same value; `cargo test -p observatory-app` 11 passed). | m4-fix-card-worktree-name | done (m4 integration) |
| F-176 | 2026-10-29 | Rule 29 fix of the M4 scrutiny round 1 blocker on m4-contract-checks-gate: `lib.sh` `head_ref()` returns `HEAD^2` on a `pull_request` event when HEAD is the synthetic merge commit of GitHub, else `HEAD`; `range_base`, `g6-commit-format`, `g5-metrics-untouched` and `g6-shared-files` judge that ref. `g7-ci-run` lists only the `mission-branch.yml` push run. Self-test cases gc4 (merge over a clean PR head, ok), gc5 (PR head commit without the trailer, red) and gc6 (same merge without the event variable, red), plus the stubbed `gh` arguments case. | m4-fix-contract-checks-pr-range | done (m4 integration) |
| F-177 | 2026-10-29 | `exceptions.txt`: feature ids of the rows for 253e95a and aa56596 corrected to `m4-demo-gate-workflow` and `m4-demo-branch-and-ruleset` (ids only; no row added or removed). | m4-fix-contract-checks-pr-range | done (m4 integration) |
| F-178 | 2026-10-29 | fact — `g7-demo-red-pr` counts PRs for `demo/red-1` created at or after the run window's start (the head is reused by every walk; amended VAL-GATE-020). The start is `CONTRACT_RUN_START`, else the earliest `pr.opened` row of the store named by `CONTRACT_WALK_STORE`, else the creation time of the PR in the walk's PR file (`CONTRACT_WALK_PR_FILE`, default `/tmp/m4-walk-pr.txt`, written by `e2e/lib/walk.ts`). No window source or no PR in the window is a skip; two or more in the window is a FAIL. `PR_LIMIT=200` lifts gh's default of 30 PRs. | m4-fix-g7-demo-red-pr-window | done (m4 integration) |
| F-179 | 2026-10-29 | fact — The pre-gate placeholder names the mission's branch: `OpenedMission.branch` (`#[serde(default)]`, set by the shell in `open_replay`, `open_mission`, `attach`; core leaves it `None`) is `missions.branch` when git_sync recorded one, else `[mission].branch_pattern` resolved for the mission name (`commands::resolve_branch`); `Capture.branch` carries it to `Board`, which no longer builds it from `MissionConfig::default()`; the browser host (no shell, no config) keeps the default pattern. The `board` command and the `gate_changed` app-log line (`branch=<name>`, `announce_gate`) use the same value. No kit class, no `style.css` change. | m4-fix-gate-placeholder-branch | done (m4 integration) |
| F-180 | 2026-10-29 | fact — The e2e config sets `[mission].branch_pattern` to the literal `mission/observatory-v1` (`lib/env.ts` `E2E_BRANCH`), so every mission of the suite resolves to it; no other spec reads the branch (they match library rows by the launch form's branch name). | m4-fix-gate-placeholder-branch | done (m4 integration) |
| F-181 | 2026-10-29 | fact — `e2e_gate_poll` takes an optional `mission_id` and a `pending` scenario (a poll with no PR and no gate facts). Without `mission_id` it runs on the live mission that started last: the suite keeps one app for all specs, earlier hosted runs stay live, and the host's `HashMap` order picked the controls spec's mission in one run in two (`gate.e2e.ts` flaked, `reports/e2e/2026-10-29/211941/record.json`: poll hit e6b13589, the board showed 01091b34). Run 3 after the fix: outcome passed, no rerun. | m4-fix-gate-placeholder-branch | done (m4 integration) |
| F-182 | 2026-10-29 | fact — walk-2-policy waits up to 45 s for a `gate_changed` line with `merge_offered=false` (`browser.waitUntil`, `appLog()`), then keeps it in the walk's command log; the badge can read CI green from the first poll's events before the header's log line is written. The walk itself was not run (needs `gh` and the live demo pair). | m4-fix-gate-placeholder-branch | done (m4 integration) |
| F-183 | 2026-10-29 | decision (rule 24) — The git thread takes the repository of a mission from the hosted `LaunchConfig.repo` when the app launched it (`Hosts::repo_of`), else `git rev-parse --show-toplevel` of the app's working directory (`ingest::git::toplevel`, read once at thread start). A mission with neither logs `git_skipped reason=no repository` once and writes nothing. No config key, no decision file, no store column. Alternatives: a `[mission].repository` key (decision file, example config row and the `Config::default() == example` test, and one repository for every mission), a `missions.repository` column (a migration for a value only hosted launches know). | m4-fix-wire-git-source | done (m4 integration) |
| F-184 | 2026-10-29 | fact — `git_sync` (core, `ingest/git/sync.rs`) reads the merges once per poll and stores `integration.merged` (source `git`), `gate_command.*` (source `github`) and `missions.branch`; the git thread polls at `[github].poll_interval_s`; the runs part runs only when `GITHUB_TOKEN` is in the environment. The runs come from `GET /actions/workflows/mission-branch.yml/runs?branch=<b>&per_page=100` (`GitHubClient::mission_branch_runs`), not from the per-head poll. | m4-fix-wire-git-source | done (m4 integration) |
| F-185 | 2026-10-29 | a finished Actions run or commit status is one event, `check.failed` or `check.passed`, role `external`, lane from `template.lane_of` (Gate in the default template), `at` = the run's or status's own `updated_at` (the poll time when the forge gives none), pointer `SourcePtr::GitHub { url }` = the run's `html_url` (a status without `target_url`: its status API URL), so the run id is in `events.url`. The id names the run (`github:check:<pr>:<run_id>:<failed\|passed>`) or the status context on the head (`github:check:<pr>:<sha>:<context>:<verb>`): a restarted source records the same event, never a new one. Running and queued results make no event (`check.started` is not emitted here). `Poll.windows` carries the window changes. | m4-forge-checks-and-fix-cycle | done (m4 integration) |
| F-186 | 2026-10-29 | migration `0003_gate_windows.sql`: `rework_windows` gains `pr_number` and `check_name` (nullable, other causes leave them empty) and a partial index on the open ones. `Store::record_poll` applies `WindowChange::Open` (a window per failing check, none when one is open for that PR and check or this event opened one already) and `Close` (the green run of the same check sets `closed_by`) after the events they name; `missions.state` reads `fixing` while a gate window of the mission is open and `at_gate` after the last closes. `rework_windows.feature_id` of a gate window holds the card (the milestone) the fix cycle returns to: the check names no feature. The window needs a card: `GitHubSource::set_affected_milestone` (the shell sets it before each poll from `Store::affected_milestone`: the milestone of the latest `integration.merged`, else of the latest event that names one); with none the check events still land in Gate and no window opens. | m4-forge-checks-and-fix-cycle | done (m4 integration) |
| F-187 | 2026-10-29 | view: `Card.cycle` (serde default 0) counts fix cycles. A `check.failed` on a card that is not fixing sends it to `template.home(Role::Worker)` (the lane the template gives the worker role, Build in the default template; no lane name in code) and adds one; a second red while fixing is the same cycle; the next `check.passed` of a fixing card puts it back in the template's final lane. `GateGroup::of(events)` is the Gate header of a list of events. The badge states were already there (`CiRed`, `CiGreen`). | m4-forge-checks-and-fix-cycle | done (m4 integration) |
| F-188 | 2026-10-29 | shell: the GitHub poll thread logs `gate_changed{mission_id, badge, pr}` and emits the `gate_changed` window event (payload `GateGroup`) when the badge or PR number changed since the last poll, and once at the first poll (`pending` before a PR). This is the app-log line VAL-GATE-017 and -021 read (and VAL-GATE-007 for `pending`). | m4-forge-checks-and-fix-cycle | done (m4 integration) |
| F-189 | 2026-10-29 | Evidence (live, 2026-10-29, read-only, `GITHUB_TOKEN="$(gh auth token)"`, ignored test `the_live_fix_cycle_is_recorded_in_a_store_file` against PR #24 `demo/smoke-1 -> demo/base`, closed): request lines `pulls?head=dctellya:demo/smoke-1&base=demo/base&state=all`, `actions/runs?head_sha=1d994af0...`, `commits/1d994af0.../status`, all `status=200`; store `checks`: `demo-gate \| actions_run \| passed \| .../actions/runs/40853894595`; event `github:check:24:40853894595:passed \| check.passed \| gate \| github`; badge `ci_green`; `gh api .../actions/runs?head_sha=` lists run 40853894595 `success`, 40853828900 `failure`, 40853828337 `cancelled`. The red-to-green transition on a live PR was NOT run (see Deferred clauses). | m4-forge-checks-and-fix-cycle | done (m4 integration) |
| F-190 | 2026-10-29 | live walk of `m4-forge-checks-live-walk`: not run, no figures. No run id, PR number or event id exists for it: nothing was pushed or opened. The push `git push origin HEAD:refs/heads/demo/forge-checks-1` from `da5e15d` was refused by Droid-Shield before it ran (2026-10-29, `/tmp/walk-shield.txt`): `tests/forge.rs:30`, `tests/secrets.rs:7`, `tests/secrets.rs:13`. Those line numbers are not in the tip: step 1 (`0da61cb`) builds the fakes at run time, and the tip holds no token-shaped literal. The shield scans the diff of the whole push range (`origin/wt/m4..HEAD`, 103 commits), and commits `3eeacc7`, `12d2c63` and `22cdb27` of the range still carry the old literals; history is forward-only (rule 12), so fixing the tip cannot clear the range. Nothing was worked around (no API-created branch, no retry). | m4-forge-checks-live-walk | done (m4 integration) |
| F-191 | 2026-10-29 | Evidence the walk's claims rest on instead: the recorded red and green runs of PR #24 replayed through the real source, store and view (`tests/forge.rs`), and the read-only live poll of the closed PR #24 (green side, `/tmp/live1.txt`, see `m4-forge-checks-and-fix-cycle.md`). The red-to-green transition on a live PR stays unproven until a demo branch is pushed. | m4-forge-checks-live-walk | done (m4 integration) |
| F-192 | 2026-10-29 | `ingest::forge` holds the `Forge` trait and its types; `ingest::github` the client and the source (feature `github`, which now implies `secrets`). The trait carries `pr(branch, target)` and `checks(pr)` only; the later slices add `policy`, `reviews`, `attestations`, `merge` (architecture §4.6 lists all six). `GitHubClient<T: HttpTransport>` reads `GET /pulls?head={owner}:{branch}&base={target}&state=all`, `GET /actions/runs?head_sha=` and `GET /commits/{sha}/status` and nothing else; the check-runs and check-suites API has no function and no mention in `crates/observatory-core/src`. `checks` keeps the latest Actions run per workflow name (cancelled, skipped and neutral runs say nothing) plus every commit status context. Every request logs `github.request{method, url, status}` through `tracing` (target `observatory_core::ingest::github`); a transport failure logs the error text instead of a status; headers and bodies never reach a log. | m4-forge-core-and-pr-detection | done (m4 integration) |
| F-193 | 2026-10-29 | `GitHubSource::poll(now)` / `poll_if_due(now)` return a `Poll { observed_at, pr, checks, events }`; the interval counts from the attempt (a failing forge is not asked again before it is due). `pr.opened` is one event per PR (`github:pr:<n>:opened`, role `external`, `at` = the PR's `created_at`, pointer `SourcePtr::GitHub { url }` of the PR API URL, lane from `template.lane_of`, Integrate in the default template). `Store::record_poll` writes `prs` (`observed_at` is the first observation and is never overwritten), one `checks` row per (PR, name, source), and the events (idempotent). | m4-forge-core-and-pr-detection | done (m4 integration) |
| F-194 | 2026-10-29 | TLS: `ureq` 3.5.0 with `native-tls-no-default` and `RootCerts::PlatformVerifier` (the system trust store), not rustls (architecture §13 said "verify"). The default `rustls` feature pulls `webpki-roots` (CDLA-Permissive-2.0), which `deny.toml` does not allow; a new licence is a decision file. `make deny`: advisories ok, bans ok, licenses ok, sources ok. The live run caught a panic the recorded fixtures cannot (native-tls without roots panics on the first https request); `an_https_request_builds_its_tls_connector_instead_of_panicking` pins it. | m4-forge-core-and-pr-detection | done (m4 integration) |
| F-195 | 2026-10-29 | `core::secrets` (keychain item `droid-observatory/<name>`, account `$USER`, `GITHUB_TOKEN` fallback, `Secret` prints `<redacted>`). A keychain that refuses (locked display: `Platform failure: User canceled the operation.`, observed on the live run) gives way to `GITHUB_TOKEN`; with none the refusal is returned. `keyring-core` and `apple-native-keyring-store` left core's machete ignore list. | m4-forge-core-and-pr-detection | done (m4 integration) |
| F-196 | 2026-10-29 | `[github].request_timeout_s = 15` (half the poll interval). The shell (`apps/observatory-app/src-tauri/src/github.rs`) polls each live mission (the `stall::live_missions` set, now `pub(crate)`) every `[github].poll_interval_s`, records the poll and emits `events_appended` for new events; the branch is `[mission].branch_pattern` applied to the mission name. | m4-forge-core-and-pr-detection | done (m4 integration) |
| F-197 | 2026-10-29 | Evidence (live, 2026-10-29, demo pair): ignored test `forge_live` against `dctellya/droid-observatory`, probe PR #28 `demo/forge-probe-4 -> demo/base`: created `12:46:14Z`, detected `12:46:22.279Z`, delay 8 s 279 ms (interval 30 s); request log lines `pulls?head=dctellya:demo/forge-probe-4&base=demo/base&state=all`, `actions/runs?head_sha=`, `commits/<sha>/status`, all `status=200`; no `check-runs`; `grep -c` of token patterns and `Authorization` in the run output: 0. Probe PRs #25 to #28 were closed and their branches deleted (`demo/base` is the only `demo/*` branch left). The first probes found two defects the recorded fixtures could not (keychain prompt on a binary it has not seen, native-tls roots). | m4-forge-core-and-pr-detection | done (m4 integration) |
| F-198 | 2026-10-29 | Pushed head — `f4de0fe` = empty `git commit-tree` child of the tip a2eb508 (tip carries `[skip ci]`, which would skip the `pull_request` run; the child message has no marker), `demo/forge-checks-1`, push exit 0 | m4-forge-live-walk-2 | done (m4 integration) |
| F-199 | 2026-10-29 | PR — #29 `demo/forge-checks-1` -> `demo/base`, created with `--label demo:red` 2026-10-29T14:47:06Z | m4-forge-live-walk-2 | done (m4 integration) |
| F-200 | 2026-10-29 | Red run — 40894949421 failure (created 14:47:09Z, updated 14:47:17Z); a second run 40894948984 for the same sha was `cancelled` by the workflow's `cancel-in-progress` (opened and labeled fire together); `gh run watch 40894949421 --exit-status` returned the failure at once (exit 1) | m4-forge-live-walk-2 | done (m4 integration) |
| F-201 | 2026-10-29 | Red live test — `badge ci_red`; `prs` 29 open; `checks` demo-gate actions_run failed; `check.failed` id `github:check:29:40894949421:failed`; `rework_windows` 1 m4 gate opened_by that id, closed_by NULL, pr 29; `missions` live `fixing` | m4-forge-live-walk-2 | done (m4 integration) |
| F-202 | 2026-10-29 | Label removed — 2026-10-29T14:47:45Z; no new head needed (`unlabeled` triggers the workflow) | m4-forge-live-walk-2 | done (m4 integration) |
| F-203 | 2026-10-29 | Green run — 40895054822 success (created 14:47:47Z, updated 14:47:53Z); `gh run watch --exit-status` exit 0 | m4-forge-live-walk-2 | done (m4 integration) |
| F-204 | 2026-10-29 | Green live test (same store) — `badge ci_green`; `checks` passed; `check.passed` id `github:check:29:40895054822:passed`; window 1 closed_by `github:check:29:40895054822:passed`; `missions` `at_gate`; same PR 29 | m4-forge-live-walk-2 | done (m4 integration) |
| F-205 | 2026-10-29 | Merge — `gh pr merge 29 --squash --delete-branch` by dctellya, mergedAt 2026-10-29T14:48:12Z, merge commit `36f287b84fe4d9a1deea3b081bb96ccee114b152`; the branch is gone on origin (`git ls-remote` empty) | m4-forge-live-walk-2 | done (m4 integration) |
| F-206 | 2026-10-29 | Merge probe — `badge human_merged decided_by Ruleset { id: 26112925, name: "demo-gate" } requires_human true merge_offered false`; `prs` 29 merged merged_by dctellya; one `handed_to_cd` (id `github:pr:29:handed_to_cd`, role human, lane gate, sha 36f287b, url `.../pulls/29`, at 2026-10-29T14:48:12.000Z); `missions` live complete, completed_at 2026-10-29T14:48:12.000Z | m4-forge-live-walk-2 | done (m4 integration) |
| F-207 | 2026-10-29 | Policy probe — separate store, same merged PR: `decided_by ConfigOverride("policy") requires_human false merge_offered false` | m4-forge-live-walk-2 | done (m4 integration) |
| F-208 | 2026-10-29 | Counts — `SELECT COUNT(*) FROM prs WHERE head='demo/forge-checks-1'` 1; `gh pr list --head demo/forge-checks-1 --state all` one number (29); `human_actions` kind merge 0; `unknown.event` from git/github 0; attestations 0 and reviews 0 (the demo-gate run uploads no attestation artifact, so the `handed_to_cd` event has an empty attestation list) | m4-forge-live-walk-2 | done (m4 integration) |
| F-209 | 2026-10-29 | Unit rerun — `cargo test -p observatory-core --test forge --test forge_merge`: 16 passed (forge), 13 passed (forge_merge), exit 0, real 7.036 s | m4-forge-live-walk-2 | done (m4 integration) |
| F-210 | 2026-10-29 | VAL-GATE-012 — Unit: `checks_come_from_actions_runs_and_commit_statuses` tests/forge.rs:426. Live: `/tmp/walk2-red-live.txt` request lines `GET .../actions/runs?head_sha=f4de0fe...` and `GET .../commits/f4de0fe.../status`; `checks` row demo-gate actions_run failed; `gh api .../actions/runs?head_sha=f4de0fe...` returned 40894949421 failure (and 40894948984 cancelled), the stored conclusion agrees with the failure run | m4-forge-live-walk-2 | done (m4 integration) |
| F-211 | 2026-10-29 | VAL-GATE-017 — Unit: `red_required_check_opens_a_fix_cycle_on_the_affected_card` tests/forge.rs:767. Live: `check.failed` lane gate source github url `.../actions/runs/40894949421`, badge `ci_red` | m4-forge-live-walk-2 | done (m4 integration) |
| F-212 | 2026-10-29 | VAL-GATE-020 — Unit: `fix_cycle_keeps_the_same_pr` tests/forge.rs:904. Live: PR 29 across red and green, `prs` count 1, `gh pr list` one number | m4-forge-live-walk-2 | done (m4 integration) |
| F-213 | 2026-10-29 | VAL-GATE-021 — Unit: `green_check_closes_the_rework_window_and_the_badge_turns_green` tests/forge.rs:854. Live: window closed_by `github:check:29:40895054822:passed`, badge `ci_green`; the second poll ran 8 s after the run finished (updatedAt 14:47:53Z, poll 14:48:01Z), inside the 30 s bound | m4-forge-live-walk-2 | done (m4 integration) |
| F-214 | 2026-10-29 | VAL-GATE-015 — Unit: `rule_attribution_names_the_ruleset` tests/forge_merge.rs:339. Live: `decided_by Ruleset { id: 26112925, name: "demo-gate" } requires_human true`, id equal to `gh api repos/dctellya/droid-observatory/rulesets` demo-gate | m4-forge-live-walk-2 | done (m4 integration) |
| F-215 | 2026-10-29 | VAL-GATE-016 — Unit: `merge_policy_override_hides_merge_and_names_the_config` tests/forge_merge.rs:417. Live: policy probe `decided_by ConfigOverride("policy") requires_human false merge_offered false`; run on the merged PR, so the open-and-green button state is unit evidence only | m4-forge-live-walk-2 | done (m4 integration) |
| F-216 | 2026-10-29 | VAL-GATE-027 — Unit: `policy_merged_detected_without_human_action` tests/forge_merge.rs:469, `handed_to_cd_is_emitted_once_with_merge_sha_pr_and_attestations` tests/forge_merge.rs:523. Live: one `handed_to_cd`, 0 `human_actions` merge rows; role is `human` not `external` (one login on this machine) | m4-forge-live-walk-2 | done (m4 integration) |
| F-217 | 2026-10-29 | VAL-GATE-028 — Unit: `a_merge_completes_the_mission_at_the_hand_over` tests/forge_merge.rs:618. Live: `missions` complete, completed_at 2026-10-29T14:48:12.000Z equals the `handed_to_cd` at | m4-forge-live-walk-2 | done (m4 integration) |
| F-218 | 2026-10-29 | rule-24 decision, owner rule 1520): the forge source is `off` without a token or the owner flag. The keychain is consulted only when `GITHUB_TOKEN` is unset or empty AND `FORGE_LIVE=1` is set; a non-empty `GITHUB_TOKEN` wins as at 3c73008 and leaves the keychain alone; with neither the token source is `off`, the shell starts no `github` poll thread (no store open, no request) and logs `github.token source=off reason=no GITHUB_TOKEN and FORGE_LIVE unset` with no value; the Gate lane keeps its pending placeholder. The keychain path is the owner's own `FORGE_LIVE=1` launch from his terminal. Code: `core::secrets::resolve_forge_token` (keychain closure behind the flag), `forge_source_enabled`, `TokenSource::Off`, `OFF_REASON`; the shell's `github::spawn` returns without a thread when `forge_source_enabled_in_env()` is false. `resolve_github_token` (env first, then keychain) is unchanged and still pinned by `the_environment_wins_and_the_keychain_is_not_consulted`; `github_token()` and `github_token_with_source()` now go through the flag, so `GitHubClient::from_config` cannot reach the keychain either. Launched processes: the e2e harness (`apps/observatory-app/e2e/lib/env.ts` `appEnv()`) sets `GITHUB_TOKEN` to a PAT-shaped value built at run time by concatenation (never a literal) unless the variable is already set; `scripts/tests/attach-smoke.sh`, `make app-dev` and every other launch need no change because the default is off. **Alternatives considered:** only the harness fake token (rejected: `make app-dev` and every other launch from a worker would still read the keychain); `[github]` off in `config/observatory.toml` (rejected: a config key is another thing to forget, the environment is the owner's launch path anyway); a CLI flag on the app (rejected: Tauri's launch path has no argv hook worth adding for this). Evidence: `cargo test -p observatory-core --test secrets` (the red/green test `the_forge_source_is_off_without_a_token_or_the_owner_flag`: fake keychain records 0 calls when both variables are unset or `GITHUB_TOKEN` is empty, 1 call with `FORGE_LIVE=1`, 0 calls when `GITHUB_TOKEN` is set); `make app-dev` with both variables unset logged `github.token source=off reason=no GITHUB_TOKEN and FORGE_LIVE unset` and no `github_started`, `github_poll` or `github.request` line; the e2e suite with the harness token: 32 cases, 0 failures. | m4-forge-off-without-flag | done (m4 integration) |
| F-219 | 2026-10-29 | Row (tool fact, library/tooling.md; repeats the owner's 1343 and 1520 facts): the macOS keychain ACL is per binary hash. A rebuilt app or test binary prompts again for `droid-observatory/github-token`; "Always Allow" binds the secret to a throwaway build and does not carry to the next build. Hence no process a worker starts reaches the keychain (tests, the app under e2e, scripts, the browser host); only the owner's `FORGE_LIVE=1` launch does. | m4-forge-off-without-flag | done (m4 integration) |
| F-220 | 2026-10-29 | Row (cycle rule, as widened by inbox 1520): "no worker test reads the keychain" (1343) is now "no process a worker starts reads the keychain", enforced by mechanism (off by default plus the harness fake token) and not by a sentence. A worker whose launched process prompts the owner's GUI records the row as owner-run, does not wait, and counts the live clause as not run whatever the answer. Applies to `m4-merge-ui-and-403`, `m4-forge-live-walk-2` and the M4 user-testing validator. | m4-forge-off-without-flag | done (m4 integration) |
| F-221 | 2026-10-29 | Row (observation, e2e, 2026-10-29): the first `make e2e` on the green tree exited 2 after 285 s (`Spec Files: 4 passed, 2 failed`: five `gate.e2e.ts` cases failed with "the card never reached the build lane" after 1m44s, and `panel.e2e.ts` "renders the Panel within the configured latency" read 4161 ms against the 3000 ms limit). Two full reruns of the same binary on a clean run directory passed, 6 spec files, 32 cases, 82 s each: one with `GITHUB_TOKEN=` empty (forge off), one with the harness fake token (junit.xml 32 `<testcase>`, 0 `<failure>`); the gate spec alone on a clean directory passed both ways (6 cases, 5 to 6 s). Two gate-only runs on the directory the failed run left behind failed again (the board showed the PR already "handed to CD" from the earlier run: the store is reused, so `make e2e` removes it first). The cause of the first failure is not proven (a loaded machine is the suspect: the Panel case is a timing case). No B-row: not reproduced. | m4-forge-off-without-flag | done (m4 integration) |
| F-222 | 2026-10-29 | `Forge` gains `policy(target)`, `reviews(pr)`, `attestations(pr)`, `token_owner()` and `merge(pr, method)` (architecture §4.6 lists them). `policy` reads `GET /rules/branches/{target}`, names the ruleset from `GET /rulesets` (`ruleset {id}` when that list is unreadable) and falls back to `GET /branches/{target}/protection` (404 is no protection): `RuleAttribution::{Ruleset{id,name}, BranchProtection, ConfigOverride(text), NoRule}`. `requires_human` is "a `pull_request` rule applies"; the required checks are the contexts of the `required_status_checks` rules. `[github].merge_policy` is applied by `MergePolicy::with_override`: `auto` keeps the forge's answer unless auto-merge is on for the PR, `human` forces `requires_human` and names `ConfigOverride("human")`, `policy` clears it and names `ConfigOverride("policy")`; the required checks stay. | m4-forge-rulesets-and-merge | done (m4 integration) |
| F-223 | 2026-10-29 | `Poll` carries `reviews`, `attestations` and `gate: Option<GateFacts>` (policy in force, one `RequiredCheck` per required name with its conclusion, `RequiredMissing` when nothing reports it, review count, attestation ids, `pr_open`). `GateGroup` gains `decided_by`, `requires_human`, `merge_offered`, `required_checks`, `reviews`, `attestation_ids` (all `#[serde(default)]`), filled by `GateGroup::with_facts`. `merge_offered` = PR open, a person decides, badge `ci_green` and every required check `passed` (`merge_button_shows_only_when_policy_is_human_and_checks_are_green`). The shell's `gate_changed` log line and window event now carry the header with the facts and fire when any of it changes (a policy flip, not only the badge). Policy, reviews and attestations that cannot be read degrade (warn, no facts); a failure there never fails the poll, except the attestations of a merged PR (below). | m4-forge-rulesets-and-merge | done (m4 integration) |
| F-224 | 2026-10-29 | `Forge::merge` PUTs `{"merge_method": ...}` to `/pulls/{n}/merge`, logged as `github.request{method="PUT", url, status}` like every request (no body, no header). A 403 that is not the rate limit is `ForgeError::Forbidden { pr, target, missing_permission: "pull_requests:write" }`; one PUT, no retry. `HttpTransport::put` has a default that refuses, so a read-only transport needs nothing. | m4-forge-rulesets-and-merge | done (m4 integration) |
| F-225 | 2026-10-29 | `handed_to_cd` (F-028 amendment): when the poll sees `PrState::Merged` and every required check is `passed` (all observed checks when the policy is unreadable) it emits `github:pr:<n>:handed_to_cd` once: `merge_sha`, `pr_number`, `attestation_ids`, `at` = `merged_at`, pointer = the PR API URL, `role = Human` when `merged_by` equals the token owner (`GET /user`, read once) else `External`. `merged_by` comes from `GET /pulls/{n}` (the list omits it); that call, the token-owner call and the attestations of a merged PR fail the poll instead of degrading, because the event is announced once and a wrong role or empty id list would stay. Branch deletion and anything after the merge are not observed. | m4-forge-rulesets-and-merge | done (m4 integration) |
| F-226 | 2026-10-29 | attestations: `Forge::attestations` lists the artifacts of the newest passed run of each workflow on the head (`GET /actions/runs/{id}/artifacts`, 404 is none) and asks `GET /attestations/{digest}` for each artifact digest (404 is none); the id is the forge's `id` when it gives one, else `<digest>#<position>`. `verified` is false: nothing verifies a bundle here. The demo repository has no artifacts, so its attestations are an empty list (count 0), as VAL-GATE-014 wants shown. | m4-forge-rulesets-and-merge | done (m4 integration) |
| F-227 | 2026-10-29 | store: migration `0004_attestation_ids.sql` adds `attestations.attestation_id` (unique per PR). `record_poll` writes `prs.merged_by`, one `reviews` row per GitHub review id and one `attestations` row per (PR, id), and on a `handed_to_cd` event sets `missions.state = 'complete'`, `completed_at` = the event's `at` (only while `completed_at` is NULL). Inserting a `handed_to_cd` now writes `events.sha` = the merge sha; reading it joins the attestation ids of its PR, so the round trip is exact (the lossy case noted by `m1-view-models-core` is closed). `Store::events` and `Store::event` both fill the ids. | m4-forge-rulesets-and-merge | done (m4 integration) |
| F-228 | 2026-10-29 | Evidence (live, 2026-10-29, read-only, `GITHUB_TOKEN="$(gh auth token)"`, ignored test `the_live_gate_names_the_rule_and_records_a_merge` against PR #24 `demo/smoke-1 -> demo/base`, closed): requests `pulls?head=...`, `actions/runs?head_sha=`, `commits/<sha>/status`, `actions/runs/40853894595/artifacts`, `pulls/24/reviews`, `rules/branches/demo/base`, `rulesets`, all `status=200`, no check-runs; policy `auto`: `decided_by Some(Ruleset { id: 26112925, name: "demo-gate" }) requires_human true`; policy `policy`: `decided_by Some(ConfigOverride("policy")) requires_human false merge_offered false`; the PR is closed so `merge_offered` is false in both; `grep -c` for `Authorization`, `gho_` and `github_pat` over both outputs: 0. | m4-forge-rulesets-and-merge | done (m4 integration) |
| F-229 | 2026-10-29 | Row (rule 24, F-NEW): the GitHub token resolves environment first. A non-empty `GITHUB_TOKEN` is the token and the keychain `droid-observatory/github-token` is not read at all (zero calls on the injected keychain); the keychain is read only when the variable is unset or empty, and a refusing keychain then is the answer (no token, the refusal). The owner's Q2 is unchanged: the read-only PAT stays in the keychain. `core::secrets::resolve_github_token(env, keychain_closure)` returns the token and a `TokenSource` (`env`, `keychain`, `none`); the shell reads it once at start (one secrets call per app process, never per poll or per mission) and logs `github.token source=env\|keychain\|none`, never the value. Alternatives: keychain first with the item's access list pre-authorised (rejected: the list binds to one binary hash, see the tool fact); a config switch for the order (rejected: a setting for a problem the environment variable already solves). | m4-forge-token-env-first | done (m4 integration) |
| F-230 | 2026-10-29 | Row (tool fact): the macOS keychain access list is per binary code hash. Every rebuilt test or app binary prompts anew for the item, and "Always Allow" binds the secret to a throwaway executable (the `forge_live-1607b840b0124955` prompt of 2026-10-29 13:41). Reading the keychain from a test or a dev build is therefore never a check a worker can complete. | m4-forge-token-env-first | done (m4 integration) |
| F-231 | 2026-10-29 | Row (cycle rule): no worker test reads the keychain. A test that needs the real token is an owner-run row (attended class, M3 item 18): `#[ignore]`, returns early printing `skipped: FORGE_LIVE unset (owner-run)` unless `FORGE_LIVE=1`, takes the token from `GITHUB_TOKEN` only and never calls the keychain. A worker whose check would prompt the owner's GUI marks the row owner-run and moves on; a prompt that already appeared makes the check "not run", whatever the answer. Every `cargo test --workspace` test goes through an injected fake: resolution takes the keychain as a closure, and `a_service_without_an_item_is_none_not_an_error` runs on keyring's mock store. The renamed `the_live_demo_pair_is_polled_with_the_owner_run_token` was not run. | m4-forge-token-env-first | done (m4 integration) |
| F-232 | 2026-10-29 | the Gate group on the Gate lane (the template's final lane) and in Details: `GateGroup::view(&template)` in `crates/observatory-core/src/view/gate.rs` returns `GateGroupView` (badge text, PR, required checks with their conclusion, `reviews: N`, `ruleset: <name>` or `decided by: ...`, `attestations: N`, the merge offer, the `handed to CD` reading `handed to CD: <sha7> · PR #<n> · attestations <ids\|none> · merged by human\|policy`, and one line per clock of the final lane: `closed at <final event>` for the clock with an end, `starts at <sha7>; not computed here` for the platform's clock). `ui/src/gate.rs` renders it as is. Counts of 0 are shown (`reviews: 0`, `attestations: 0`). | m4-gate-group-ui | done (m4 integration) |
| F-233 | 2026-10-29 | the fix cycle's return arc: `Card::return_arc(&template)` (core, `card.rs`) is `from` the template's final lane `to` `template.home(Role::Worker)` with `cycle` and `cycle N`, `active` while no green check has come since the red one. The card keeps `cycle N` after the green check (the arc turns from amber to muted). Nothing names a lane: `LaneId`s and `template.name_of`, no `7`, no enum path. | m4-gate-group-ui | done (m4 integration) |
| F-234 | 2026-10-29 | Merge visibility is `GateGroupView.merge` (`Some` only when `GateGroup.merge_offered`, which `with_facts` already computed: PR open, a human decides, every required check passed, badge `CI green`). `GateGroup::with_polled` overlays the poll on the events a scrubber keeps and withholds Merge unless those events are green too. The button is drawn and has no handler: the confirmation, the press and the 403 are `m4-merge-ui-and-403`. | m4-gate-group-ui | done (m4 integration) |
| F-235 | 2026-10-29 | `gate_changed` now carries `GateChanged { mission_id, gate }` (was the bare `GateGroup`) and the shell keeps each mission's last header in `GateCache`, read by the new command `gate_group(mission_id)`, so a board opened after the change still shows the group. The `gate_changed` log line lists `required_checks` (`name=conclusion`), `reviews`, `ruleset`, `attestations` and `attestation_ids` (VAL-GATE-014). Architecture §7 command list gains `gate_group` (orchestrator to record). | m4-gate-group-ui | done (m4 integration) |
| F-236 | 2026-10-29 | e2e: `--features e2e` builds the command `e2e_gate_poll(scenario_name)` (`red`, `green`, `merged`) that feeds a scenario of PR #24 through `apply_poll` (store `record_poll`, `events_appended`, `gate_changed`) on the first live mission. The release build has no such command. `lib/env.ts` sets `[github] poll_interval_s = 3600` so a real poll cannot race it. This is a stand-in for the GitHub transport, not a recorded GitHub answer: the live walk (`m4-forge-live-walk-2`) proves the real poll, this proves the window. | m4-gate-group-ui | done (m4 integration) |
| F-237 | 2026-10-29 | kit work: heading count spans `agent-count`, `overlay-human-count`, `tray-count`, `details-milestone`, `details-row-count` take `class="mono"` (their five lines are gone from `kit-pending.txt`, now comments only); ThemeSwitch marks the active choice with `.on` (`current` signal, kit rule 7); every page (library, board) starts with the kit's page head (`.pagehead`, `.eyebrow` breadcrumb, h1 title, mono muted subtitle; `ui/src/pagehead.rs`, F-106); the two padding overrides at `style.css` (`.library th, .library td` 6px 8px and the Panel/Agents/Details `th, td` 3px 8px) are back to the kit's `10px` (F-103/F-104 reverted, their two rows deleted from `kit-departures.md`). `make check-kit`: `check-kit ok VAL-UI-001 [3 departures recorded]`. | m4-gate-group-ui | done (m4 integration) |
| F-238 | 2026-10-29 | rule 27 amendment (owner item 17): the gate cache is keyed by the CODE paths. `scripts/gate.sh` hashes the working tree (uncommitted and untracked files included) minus the excluded set `GATE_EXCLUDE_DIRS="build-log reports docs"` and every `*.md` outside `GATE_CODE_ROOTS="crates apps scripts config .github"`; the set is written once in the script and, with the reason, in `config/observatory.example.toml`. Records live in `reports/gate/<code_tree>/<name>.json` and keep `fingerprint` (never `key`: gitleaks and Droid-Shield read a `key` field holding 40 hex characters as an API key) and add `code_tree` and `tree` (the old key: build-log/ and reports/gate/ out only). `fingerprint` hashes the code tree, the toolchain and the command. A run that finds the code paths equal to a recorded gate prints `cached, proven at ...` then `gate <name>: HIT code tree <code_tree>` (and, when `tree` moved, that non-code paths changed since the proof); a miss prints `gate <name>: miss <code_tree>, running` then `gate <name>: MISS, code files that differ from the newest record (...)` and the `git diff --stat` between the newest record's `code_tree` and the current one, restricted to the code paths by construction. `scripts/gate.sh --diff <rev>` prints the same list from a revision. `hits.log` field 3 is the code tree. Evidence: `scripts/tests/gate_test.sh` (red `a7add22`, green `560143b`). | m4-gate-key-and-ci-paths | done (m4 integration) |
| F-239 | 2026-10-29 | owner item 17 (c): `make integrate` uses the same key (`scripts/integrate.sh`, `6ab5efd`). It prints the gate's HIT or MISS lines and, on a miss, the summary reason is `gate.sh --diff <branch>` for each merged branch (code files only; it listed docs and reports before). Evidence: `scripts/tests/integrate_test.sh`, the `code paths` scenario (red `6bb4982`). | m4-gate-key-and-ci-paths | done (m4 integration) |
| F-240 | 2026-10-29 | B-056 (owner item 17 (d)): `.github/workflows/mission-branch.yml` has `on.push.paths-ignore` (`reports/**`, `build-log/**`, `docs/**`, `*.md`, `ai/**/*.md`, `.factory/**/*.md`) and the `rust` job's `test` step runs `cargo test -p observatory-cli` only (the core tests run once, in `coverage`). `scripts/check-ci-jobs.sh` pins both. Warm measurement on `wt/m4` (run 40851767262, attempt 2, same commit as the cold attempt 1): 144 s against the 3-minute budget; the previous warm run (40222605337) was 221 s, so there are not two consecutive over-budget runs and the budget stays 3 minutes (`reports/ci/2026-10-29-m4-gate-key.md`). | m4-gate-key-and-ci-paths | done (m4 integration) |
| F-241 | 2026-10-29 | the `[skip ci]` rule (F-137) ends: the path filter exists, so a report-, log- or docs-only push no longer needs `[skip ci]` in its subject; the marker stays harmless. A push that also touches a code path runs the checks whatever its subject. | m4-gate-key-and-ci-paths | done (m4 integration) |
| F-242 | 2026-10-29 | `ingest::git::GitSource` (architecture §4.6) reads the repository with read-only `git` (no new dependency). `GitSource::mission_branches` is `for-each-ref refs/heads/mission/`; `snapshot()` returns the mission branch and head, every `git worktree list --porcelain` entry (with the milestone of a `wt/<m>` branch) and the `wt/*` branches with their tips; `integration_events()` yields one `integration.merged` per `git log --first-parent --merges` commit (role orchestrator, `SourcePtr::Git { repo, sha }`, id `git:<sha>:0`, `at` = committer date); `gate_command_events(&[FastCheckRun])` yields `gate_command.run` per run and `.passed` / `.failed` when the run has a verdict (role external, `SourcePtr::GitHub { url }` of the run, ids `github:<run_id>:0` and `:1`). Lanes are `template.lane_of(kind, role)`, no lane name or number in the code. `GitSource` does not implement the `Source` trait (its output is events, not raw lines); `SourceKind::Git` stays declared. `Store::set_mission_branch` and `Store::mission_branch` read and write `missions.branch` (column exists, no migration). | m4-git-source-core | done (m4 integration) |
| F-243 | 2026-10-29 | milestone attribution of a merge: the branch named in the subject (`Integrate m3: wt/m3`, git's `Merge branch 'wt/m4'`), else the `wt/*` branch whose first-parent line holds the merged tip, nearest to its own tip first (a later milestone branched from the merge does not claim it). A merge no `wt/*` branch made (the pull requests merged on `main` before the mission) has `milestone_id = NULL`. On the real repository: 6 such merges, s, m0 (two), m1, m2, m3 attributed. | m4-git-source-core | done (m4 integration) |
| F-244 | 2026-10-29 | run attribution: exact match only, `milestone_id` of the merge whose sha is the run's `headSha`; a run on any other commit has `milestone_id = NULL`, because a nearest-ancestor guess would file the run in a milestone that did not integrate it. Cancelled, skipped and unfinished runs get `gate_command.run` and no verdict (`RunConclusion::from_github`: `success` passed; `failure`, `timed_out`, `startup_failure` failed). | m4-git-source-core | done (m4 integration) |
| F-245 | 2026-10-29 | Evidence: `crates/observatory-core/tests/git_source.rs` (temp repository built by the test, the three named tests plus a store round trip with `unknown_count = 0`); the ignored `the_real_repository_yields_lane6_events_and_no_unknown` on this repository and `gh run list --workflow mission-branch` (21 runs, 53 lane-6 events, 0 unknown). | m4-git-source-core | done (m4 integration) |
| F-246 | 2026-10-29 | orchestrator ruling under rule 24 (2026-10-29, recorded at the next boundary): `GitSource::gate_command_events` attributes a fast-check run to the milestone whose `wt/<m>` first-parent merge is the run's head or the nearest first-parent ancestor of the head, the milestone integrated last (one `git rev-list --first-parent --merges <head>` per distinct run head, the first merge a milestone made wins; no per-event git call). A run whose head has no `wt/<m>` merge behind it, is not a full object name, or is not in the clone keeps `milestone_id` null. Real repository (`GIT_SOURCE_RUNS_JSON=/tmp/runs.json cargo test -p observatory-core --test git_source -- --ignored --nocapture`, 21 runs of `gh run list --workflow mission-branch --branch mission/observatory-v1`): 21 of 21 runs carry a milestone (s 3, m0 7, m1 4, m2 4, m3 3, counted from the pasted output, one m3 run has no verdict); the exact-head rule gave a milestone only to the 5 runs whose head is a merge commit (38e56174, cc05aa55, f66a9f50, c2fb42ac, 7385b629). Evidence: `runs_on_commits_after_a_merge_are_attributed_to_the_milestone_integrated_last` (red 6e1590b, green 6419db9), and run 104 of `gate_command_runs_are_lane6_events_attributed_to_the_milestone` now expects the milestone of the last merge (changed in the red commit). **Alternatives considered:** attribute only a run whose head is the merge commit (rejected: the first version; 16 of the 21 runs sit on commits after a merge, so the Integrate lane showed them under no milestone); one `git merge-base --is-ancestor` per run and merge (rejected: runs times merges git calls, where one first-parent walk per head answers the nearest merge); attribute by commit date (rejected: a clock, not the history; rebased or backdated commits land in the wrong section). | m4-git-source-run-attribution | done (m4 integration) |
| F-247 | 2026-10-29 | rule 28 amendment (owner item 16c): the integration's long work is one target, `make integrate-long M=<m>` (`scripts/integrate-long.sh`). Jobs run in lanes, lanes in parallel, the jobs of a lane in order: lane `cargo` is `coverage` (`make coverage`, so alone on the cargo lock and cached by `scripts/gate.sh`), then the warm timings (`cargo check -p observatory-core`, `cargo test -p observatory-core`, `trunk build` when the UI crate and trunk exist); lane `network` is `ci`, which finds the `mission-branch.yml` run of the head (or of an earlier commit with the same code tree, since a report- or docs-only push starts no run) and follows it with `gh run watch --exit-status`. One result file, `reports/integrate-long/<code_tree>.json` (schema in the script header): per job the exit code, wall time, log path and whether it was reused; the wall time of the run, the worst exit code, a fingerprint of code tree, toolchain and job set. The target exits with the worst code. A record with exit 0 and the same fingerprint is a HIT that runs nothing; after a red run only the failed jobs rerun; `FORCE=1` reruns all. It is started as a background job and its completion is the message (the result table and the last 20 lines of every failed job's log), so nothing is tailed or polled (item 16 (b), (c)). Evidence: `scripts/tests/integrate_long_test.sh` (red `5ff494a`, green `29277ad`; 59 checks: parallel lanes by overlapping timestamps, lane order, worst exit code, HIT under 2 s, docs commit keeps the HIT, code commit loses it, rerun of failed jobs only, interrupt writes no record, stub-`gh` CI cases). | m4-integrate-long-target | done (m4 integration) |
| F-248 | 2026-10-29 | owner item 16c: `make integrate` reads the result for the merged code tree into the boundary report (`## Long work`: the job table and the gate summary of each job that has one, e.g. the coverage `TOTAL` line) and into the summary line `long work: OK\|RED\|not run`; `INTEGRATE_LONG=1` runs it inside the target (branch pushed first); a pending result does not stop the boundary, a red one fails the target after the boundary commit; `reports/integrate-long/` is exempt from the clean-tree check so the second `make integrate` (the refresh after the background job) runs. Evidence: `scripts/tests/integrate_test.sh`, the four `long work` scenarios (red `53c4cf2`, green `46fe0c4`). Runbook: `docs/runbooks/integrate-long.md`. | m4-integrate-long-target | done (m4 integration) |
| F-249 | 2026-10-29 | fix to F-NEW (gate key, `m4-gate-key-and-ci-paths`): `scripts/gate.sh --diff <rev>` printed nothing and exited 2 when an excluded file (a docs file) differed between `<rev>` and HEAD, because `git rm --cached` refuses a path whose index copy differs from both the file and HEAD; an empty diff reads as "same code tree", so `make integrate`'s miss reason lost its file list and the new `ci` job would have accepted any run. The cached removals are forced (`-f`). Evidence: `scripts/tests/gate_test.sh`, scenario `--diff <rev> where an excluded file differs` (red `8532b69`, green `29277ad`). | m4-integrate-long-target | done (m4 integration) |
| F-250 | 2026-10-29 | tooling and test policy (rule 26, measured on 2026-10-29). `scripts/tests/kit_lint_test.sh` failed 4 asserts (`pend1`, `pend2`) and so did `make check-scripts`. Measured with `scripts/tests/kit_lint_test.sh` in a throwaway `git worktree` per commit: `2b3c492^` (726493f) exit 1 with the 4 failures; `e821ff8^` (a5efd55) exit 1 with the same 4; `4723ad8^` exit 0; `4723ad8` (`feat(app): gate group, return arc, page head, kit fixes`, m4-gate-group-ui) exit 1. The breaking commit is `4723ad8`, not `e821ff8` (a staging-only commit): it fixed the five heading-count spans and emptied `kit-pending.txt` to comments, and the two test cases planted their violations through those five lines. The lint (`scripts/check-kit.py`) did not regress: it still fails an unrecorded violation (an unlisted sans number cell in panel.rs, an unlisted padding or width override, a stale departure row) and a listed line that no longer violates; the test was stale. The fix removes `pend1` and `pend2` and adds `pend3` (an unlisted planted violation is red and prints its pending key), `pend4` (the same violation listed in `kit-pending.txt` is `check-kit ok`), `pend5` (a listed line that no longer violates is red naming `kit-pending.txt`) and a check that the real `kit-pending.txt` holds comments only. `4723ad8` ran no `make check-scripts`; the gap lasted from it to the first worker that ran the target (ecf45169). **Alternatives considered:** restoring a pending line to keep `pend1`/`pend2` (rejected: the list is empty on purpose, owner answer Q4); changing the lint (rejected: it was right). Measured: `make check-scripts` exit 0 in 1m32.844s, `make check-contract` exit 0 in 5.012s (22 ok, 9 skipped, 0 FAIL). | m4-kit-lint-test-fix | done (m4 integration) |
| F-251 | 2026-10-29 | owner Q4 ruling applied to `.card`: `ui/style.css:48` `.card` is back to the kit's `padding: 12px 14px; gap: 8px` (it was `6px 10px` and `4px`, found by `make check-kit` in `m4-ui-kit-lint`). Its two F-NEW rows are gone from `kit-departures.md`, which now lists the F-105 rows (`.observatory`, `.library`, `.details`) and the F-103/F-104 rows (`m4-gate-group-ui` reverts those). This closes the "approve or revert" question of the `m4-ui-kit-lint` row: reverted. | m4-kit-pending-fixes | done (m4 integration) |
| F-252 | 2026-10-29 | ten mono fixes (kit rule 2): `class="mono"` on `agents.rs` `agent-stream-rows` and `agent-stream-markers`, `attach.rs` session id `<code>`, `controls.rs` feature mark `<span>`, `launch.rs` `hosted-pid`, `hosted-session`, `<code>{ack.request_id}</code>` and `live-output-total`, `raw.rs` `raw-line-kind` and `raw-line-event`; their ten lines are deleted from `kit-pending.txt`, which keeps the five UT round 3 heading-count lines for `m4-gate-group-ui`. `make check-kit`: `check-kit ok VAL-UI-001 [5 departures recorded; pending: 5 known mono violations in kit-pending.txt]`. Claims (rule 30): none; VAL-UI-001 had already passed and is not in this feature's `fulfills`. | m4-kit-pending-fixes | done (m4 integration) |
| F-253 | 2026-10-29 | the Merge press: `HumanAction::Merge { pr_number, target }` (store kind `merge`, state `requested` then `merged`, `refused` or `failed`, `target` text `pr:<n>`, event `approval.given`, overlay human, lane = the template's final lane). One press is one `human_actions` row, one `approval.given` event and one request. `control::merge::press(forge, pr, allowed)` (core) makes the request, picks the method from the policy's allowed set (squash, merge, rebase) and maps a 403 to `MergeState::Refused` with the architecture §4.6 text (`Merge of PR #n into <target> refused: token lacks pull_requests:write. Merge in GitHub; the badge updates from the next poll.`); no retry, no extra request, the poll continues at its interval. `Store::set_human_action_state` moves the row. | m4-merge-ui-and-403 | done (m4 integration) |
| F-254 | 2026-10-29 | `GateFacts.target` (from `pr.base`), `GateGroup.target`, `MergeOffer { pr_number, target, label }` and `MergeConfirmation` (`Merge PR #n into <target>?`, `Confirm merge`, `Cancel`). A header that does not know its target offers no Merge. | m4-merge-ui-and-403 | done (m4 integration) |
| F-255 | 2026-10-29 | config `[github].api_base` (default `https://api.github.com`, in `observatory.example.toml` and `DOCUMENTED_KEYS`); `GitHubClient::with_token` applies it. Needed for the e2e stub and the live walk proxy; production leaves it unset. | m4-merge-ui-and-403 | done (m4 integration) |
| F-256 | 2026-10-29 | architecture §7 `merge` command: payload `{ mission_id, pr_number, target, confirm }`, answer `MergePressed { state, message }` (was `PrMergedByHuman` in the parent text). `confirm` is the dialog's answer: the first click only opens the confirmation in the window (no command call), and a call with `confirm: false` is refused without a request or a row. Shell: `merge.rs` `MergeDesk` (the open PR and allowed methods per mission, noted by the poll thread; the token is set once by the poll thread, no second keychain read), `press_merge` logs `merge_requested` and `merge_result` and records the overlay row through `actions::record`. | m4-merge-ui-and-403 | done (m4 integration) |
| F-257 | 2026-10-29 | e2e: `lib/forge.ts` `startForgeStub` (loopback forge on 3147 answering the merge PUT 403) and `startForgeProxy` (reads go to api.github.com, the merge PUT gets the 403); `E2E_CONFIG_FILE` in `lib/env.ts` replaces the generated config; `specs/walk-*.walk.ts` are the live walk, not matched by the `.e2e.ts` glob. 3142-3149 are the mission's scratch ports; 3147 is now taken by the stub. | m4-merge-ui-and-403 | done (m4 integration) |
| F-258 | 2026-10-29 | tooling change (rule 26, re-verified on 2026-10-29 by running the two scripts' own self-tests): the three quirks seen at the M3 integration are fixed, test first. (1) `scripts/integrate-staging.py` validates every staging file before it plans or writes anything: each `##` heading that is not one of the five sections is reported as `file:line` for all files in one pass and the script exits 2; `--check` validates only and `--rev REV` reads the files of a git revision, which `scripts/integrate.sh` runs on every branch still to merge before the first `git merge` (exit 2, nothing merged, no gate run). The comma marker forms `Row (F-NEW),` and `B-NEW,` are dropped like the colon form. (2) The `scripts/usage-report.py` Gates table takes a run when `started_at` < `--to` or `finished_at` <= `--to` and it did not finish before the window start (a run that straddles either edge is in); the table's caption `Gate window rule:` states it. (3) `re-issued waits` counts `WaitForScript` calls (any `action` but `kill`) on a `toolCallId` already waited on; kills are the separate row `worker pauses (kills)`; both rows carry their definition. Real M3 window (`--from 2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z`): `re-issued waits: 0`, `worker pauses (kills): 1`, `WaitForScript calls in the window: 1` (the 15:35:59Z kill); the same window ended at 19:55:00Z reads kills 2 (the owner's "2 in the log") and lists the integrator's two full gate runs (`0c7b6f23` 195.592 s, `a3adb9b3` 85.676 s; 19 gate runs against 13). Evidence: `scripts/tests/integrate_test.sh` (red `c81d76e`: 15 failed; green `141bfab`: all passed, `make check-integrate` 18.051 s) and `scripts/tests/usage_test.py` (red `51dfa38`: failures 3, errors 3; green `5f23cbc`: 38 tests OK, 1.557 s). **Alternatives considered:** keep exit 1 with `unfiled: N` for a stray heading (rejected: it printed no line number and the merge had already run); reject only headings that hold bullets (rejected: an empty stray heading is still a mis-filed section). | m4-tooling-fixes | done (m4 integration) |
| F-259 | 2026-10-29 | rule 28 amendment (owner item 14b, F-151): `make check-kit` is the UI-kit lint (`scripts/check-kit.py`, Python stdlib, 0.07 s). It reads `apps/observatory-app/ui` (`src/*.rs`, `style.css`, `index.html`, `theme.css`) against `ai/design/ui-kit-v2/` and prints `check-kit ok\|FAIL VAL-UI-001`. Rules: (1) `ui/theme.css` equals the kit's, no literal colour, no `font-family`, no inline spacing or type style outside it; (2) text that is identifier-, timestamp- or number-shaped (the rt3 `scan.js` shapes, applied to the `view!` templates: literal text by the scan's regex, Rust expressions by the names they read, `<time>` always) sits in an element with a mono-face class (any class that sets `font-family: var(--mono)` or `tabular-nums` in the kit or in `style.css`), a number-shaped `td` carries `n`, and every `<table>` sits in `.lib`; (3, 6) a `padding`, `margin` or `gap` on a selector the kit also styles, `padding` on any `th`/`td`, any `max-width` other than 1180 px, any `inset` and a font size off the 11/12/14/15/22 scale is a departure that must be a row of `apps/observatory-app/kit-departures.md` (selector, property, kit value, actual, reason, SPEC row), and a row whose departure is gone, or whose values are stale, is red. Wired: `check-all` (cached by the gate, code paths), a `kit` job in `mission-branch.yml` (in `mission-checks.needs`, required by `check-ci-jobs.sh`), and the self-test `scripts/tests/kit_lint_test.sh` in `check-scripts`. Paths: the app's UI lives in `apps/observatory-app/ui/`, not `apps/observatory-app/src` and `style.css` as the feature text says. | m4-ui-kit-lint | done (m4 integration) |
| F-260 | 2026-10-29 | departure found by the lint and not in §9 before: `ui/style.css:48`, `.card { padding: 6px 10px; gap: 4px }` against the kit's `.card { padding: 12px 14px; gap: 8px }` (kit rule 6, board and stall-banner cards). Two rows of `kit-departures.md` with SPEC row `F-NEW`. Owner decision needed: approve as a departure, or revert in `m4-gate-group-ui` (then the two rows go). | m4-ui-kit-lint | done (m4 integration) |
| F-261 | 2026-10-29 | the kit table after `m4-gate-group-ui` lists F-105 only (`.observatory`, `.library`, `.details`: 3 rows, one approved departure) once that feature reverts F-103 and F-104 (rows `.library th, .library td` and the `.panel th ... .details td` group go with the override) and the `.card` question above is closed. | m4-ui-kit-lint | done (m4 integration) |
| F-262 | 2026-10-29 | rule A.3 of `docs/owner-feedback/inbox/2026-10-29-1408-orchestrator-boundary-cost.md`), tooling change (rule 26, verified on 2026-10-29 by `scripts/tests/usage_rewrites_test.py` and a regeneration of the M3 window into `/tmp`): the `Lead context rewrites per milestone` part of `scripts/usage-report.py` (in `scripts/usage_rewrites.py`) prints `orchestrator turns per feature boundary: <turns> / <boundaries> = <ratio> (target 3)`. `<turns>` is the section's own lead-turn count (the `Lead turns` column: orchestrator log turns in the window). `<boundaries>` is the count of `worker_completed` entries of the mission's `progress_log.jsonl` in the window, the runner's feature returns (validator sessions included, they return a hand-off too; `worker_failed` is not one; `handoffs/*.json` is not read because the report already reads `progress_log.jsonl`; on 2026-10-29 the directory holds 118 files and the log 117 `worker_completed` entries). With no boundary the ratio reads `n/a`. When the window holds a compaction marker, a second line `before compaction: <turns> turns / <boundaries> boundaries = <ratio>, <USD-eq> USD-eq per turn; after: ... (marker: <kind> <time>)` splits at the earliest marker: a `mission_resumed` entry of the progress_log (rule B has the orchestrator record its compaction as one) or a lead context rewrite turn of the table above it; a `mission_resumed` wins a tie. With no marker the one line stands alone. Command (writes nothing under `reports/usage`; the M3 window the report was generated with): `python3 scripts/usage-report.py --milestone m3 --cross-check --cap orchestrator=30 --from 2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z --now 2026-10-29T10:34:11Z --out /tmp/m3-new.md` (what `make usage M=m3 CAPS='orchestrator=30' USAGE_ARGS='--from ... --to ...'` runs, but `--out` keeps `reports/usage/m3.md` as it is; wall 24.3 s). **Alternatives considered:** count `handoffs/*.json` (rejected: the file count and the log differ by one, and the report reads the progress_log everywhere else); count only features of `features.json` (rejected: a validator hand-off costs the orchestrator turns like any other); split at every marker (rejected: the M3 window holds 8 rewrites and the log 17 resumes in all, so it would give fragments; the owner asked for before and after one compaction); split at the last marker (rejected: a late cache expiry would leave an after-side of one turn). | m4-usage-boundary-turns | done (m4 integration) |
| F-263 | 2026-10-29 | tooling change (rule 26, verified on 2026-10-29 by the scripts' own tests and a regeneration of the M3 window on the real logs): `scripts/usage-report.py` gains six sections, in three new modules (`usage_idle.py`, `usage_testing.py`, `usage_rewrites.py`, standard library only). (1) `Owner interventions` (item 6): the `mission_resumed` records of `progress_log.jsonl`, and from the orchestrator transcript the answer to each `AskUser` call and each `StartMissionRun` with a `messageToWorker`, each with its time and the feature it unblocked (the session it resumes, else the next `worker_started`). (2) `Idle time by cause` and `Idle gaps` (item 8): a gap between two consecutive events (a model turn of any session in the Droid log, a `progress_log.jsonl` record) over 15 minutes is idle time, with the cause `usage limit` (throttled pause, or `[LLM] Chat route failure` `paymentRequired`), `host` (ECONNREFUSED, EADDRNOTAVAIL, ENOBUFS, ENETDOWN, ENETUNREACH, or a route failure with another reason), `owner` (owner pause or intervention) or `unexplained` (listed with its bounds). (3) `Host health` and `Model call faults` (item 12): `sysctl kern.boottime` pasted, uptime at generation and at the window end against the 49.7-day macOS TCP clock limit (new `--boottime` option for tests), the count of stalled or retried model calls (`[LLM] Chat route failure`, `"retried":true`, `"providerRotated":true`, usage limit excluded) and one `host verdict:` line. (4) `User testing rounds` and `User testing flow groups`: cost per round (validator session plus every subagent under it) and per flow group (a Task of the validator, matched by `callingToolUseId` to its `description`), one `user testing:` line. (5) `Lead context rewrites` and `Lead context rewrites per milestone` (item 19c): the lead's turns whose cache write exceeds the cache read by at least the context size (median nonzero cache read of the lead's turns in the window, measured), with tokens, USD-eq and the share of the lead's cost. (6) The `weekly allowance share:` line (item 7): the window's cost per assertion times the assertions the next milestone claims in `fulfills` (`--next-milestone`, default the next `m<N>` of `features.json`; its order is not chronological), for the orchestrator to state before it dispatches the next milestone. Real M3 window (`--from 2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z`): 7 owner interventions (5 `mission_resumed`, 1 AskUser answer, 1 `messageToWorker`); host gap 2026-10-25T09:12:16Z to 10:18:07Z (65.85 min, EADDRNOTAVAIL 1, ECONNREFUSED 5) is `host`, 10 stalled or retried calls, verdict `inflated`; 17 flow groups in 4 user-testing rounds, 56.47 USD-eq (validators 16.71, subagents 39.76); 8 rewrites of 194 lead turns (context 132 984 tokens), 20.44 USD-eq, 29.7 % of the lead's 68.81; `weekly allowance share: m4 claims 39 assertions x 5.55 USD-eq per assertion (m3 window, 38 fulfilled) = 216.53 USD-eq`, which is above the M4 cap of 200 (D-038). Evidence: `scripts/tests/usage_idle_test.py`, `usage_testing_test.py`, `usage_rewrites_test.py` and `usage_test.py`, red then green per addition (commits in the handoff). **Alternatives considered:** count only `progress_log.jsonl` and turn events and leave the failure lines out of the cause (rejected: the 23 October 16:11 to 18:36 local gap would read `unexplained` although the Droid log holds two `paymentRequired` failures in it; the table keeps the evidence so the owner can see either reading); a fixed 136 000-token context for the rewrites (rejected: the context differs by window and model; it is measured); the next milestone as the next block in `features.json` (rejected: the file lists m4, m5, final, s, m0 to m3 in that order). | m4-usage-idle-host-and-rewrites | done (m4 integration) |
| F-264 | 2026-10-29 | tooling change (rule 26, verified on 2026-10-29 by the scripts' own tests and a regeneration of `reports/usage/m3.md`): the plan prices of the owner's answer (pre-roci, under the owner's delegation of 2026-10-29) are in `config/prices/2026-10-29.toml`, a new dated price file that keeps the four rows of `config/prices/2026-10-22.toml` byte for byte (that file is untouched) and adds `kimi-k3` 3.00 / 15.00 / 0.30 / 3.00, `gpt-6.1-sol` 2.00 / 10.00 / 0.10 / 2.00, `gpt-6-sol` 2.00 / 10.00 / 0.20 / 2.50 and `claude-opus-5-5-fast` 4.0 / 20.0 / 0.20 / 5.0 (input / output / cache read / cache write per million, USD-eq) as `plan_*_per_m` keys; the `claude-opus-5-5-fast` row carries `note = "assumed = opus-5-5 until the owner confirms"` (the owner gave no price; it is priced as `claude-opus-5-5`). The file header names the inbox file and carries the provenance line verbatim: the observer's price table of 25-oct-2026 (resume/roci/tools/milestones.py), secondary public sources, used for every milestone reading S to M3; the owner has seen those readings; not estimates, list prices at the date named. `scripts/usage-report.py` reads the newest dated file of `config/prices/` (`--prices-dir`), or the one `--price-table <date, name or path>` names (`--prices <file>` still wins); `Models that ran` gains a `Note` column and prints `Price provenance:`; `Mission to date by role` prints `Price provenance:` and `Price notes:` (the noted model, its turns to date and USD-eq, and the note). `Feature size` reads the budget from the feature's `turnBudget` key or, when the key is absent (the mission runner rewrites `features.json` and drops it), from the text `turnBudget <N>` of the description. Regenerated M3 window (`make usage M=m3 CAPS='milestone=110 orchestrator=30' USAGE_ARGS='--from 2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z'`, 22.80 s, cross-check identical on 80 sessions): `Unpriced turns` 0 (was 373), Mission to date total 990.09 -> 1007.00 USD-eq (+16.91, the 373 turns priced; 3 `claude-opus-5-5-fast` turns are 1.06 of it), window total 210.98 unchanged. Evidence: `scripts/tests/usage_test.py` (`UsageReportPriceFiles`, `RepoPriceTables`, `UsageReportFeatureBudgets`) and `scripts/tests/usage-from-log.sh`, red then green. **Alternatives considered:** write the plan prices into `config/prices/2026-10-22.toml` (rejected: that file is the verified 22-oct table the M5 oracle and `config/observatory.example.toml` `price_table` name; a new rule 3 change is a new dated file); change `price_table` in `config/observatory.example.toml` to the new file (rejected: the app's cost model keeps the 22-oct table until a decision says otherwise, the usage tooling reads the newest file by itself); price `claude-opus-5-5-fast` as unpriced (rejected: the owner told to price it as `claude-opus-5-5` and mark it). | m4-usage-plan-prices | done (m4 integration) |
| F-265 | 2026-10-29 | tooling change (rule 26, verified on 2026-10-29 by the scripts' own tests and a regeneration of `reports/usage/m3.md`): `scripts/usage-report.py` and `scripts/usage-from-log.sh` carry owner items 5, 6 (first half) and 16 (pricing). (1) `Cost by token class`: per role and for the milestone, tokens and USD-eq by class (input, cache write, cache read, output) with the share of tokens and of cost. (2) `Cost per assertion and per feature`: the window total over the assertions with status `passed` and `validatedAtMilestone` equal to the milestone (`validation-state.json` has no timestamps, so the milestone is its window; `--validation-state <path>` or the mission directory's file) beside the same total over the milestone's `completed` features, and the ids of the assertions. (3) `Feature size` lists every feature of the milestone in `features.json` (also those that never ran, 0 turns) with `Turn budget`, `Over budget` and USD-eq (subagents included). (4) `Subagent classes`: a row per class (orchestrator, validator, worker subagents) with sessions, turns, USD-eq, the declared model and purpose read from `handoffs/*.json` and `validation/<milestone>/**/synthesis*.json`, `No declared model` and `Off declared model` counts, and the bullet `no declared model: N of M subagent sessions`. (5) `Models that ran`: every model with a turn in the window priced at the plan price (a price row with the four `plan_*_per_m` keys) or the list price of `config/prices/<date>.toml`; a model that ran in the window with no price row, or a zero input or output rate, stops the run with exit 2 naming the model; `scripts/usage-from-log.sh --prices <toml>` does the same check and its rows gain `cache_write`, which the report's cross-check compares. Real M3 window (`make usage M=m3 CAPS='milestone=110 orchestrator=30' USAGE_ARGS='--from 2026-10-24T21:48:56Z --to 2026-10-25T18:19:14Z'`, 23.18 s, cross-check identical on 80 sessions): window total 210.98 USD-eq and mission to date 990.09 unchanged; cost by class for the milestone cache write 55.1 % of cost on 6.3 % of tokens; 38 assertions, 5.55 USD-eq each, 22 completed features, 9.59 each; subagent classes 5 orchestrator subagent sessions (27.47) and 33 validator subagent sessions (58.59), all 38 with no declared model. Evidence: `scripts/tests/usage_test.py` and `scripts/tests/usage-from-log.sh`, red then green per addition (commits in the handoff). **Alternatives considered:** fail on any unpriced model in the whole log (rejected: 373 turns of `kimi-k3`, `gpt-6-sol`, `gpt-6.1-sol` and `claude-opus-5-5-fast` before the window have no price and the published M3 numbers must stay reproducible; they stay in `Unpriced turns` of Mission to date and a line names them); price from the sidecar credits (rejected: credits are a different unit, D-029). | m4-usage-token-classes | done (m4 integration) |
| F-266 | 2026-10-29 | closes owner Q5 and BACKLOG B-093: `control::dedupe_paused` (F-072) is wired into the hosted pause path. A hosted Pause leaves two `mission.paused` rows, the ack-derived one (`control::ack_event`, id `<session>:<offset>:1073741824`) and the stream's own (the cancelled turn the runner reports after its response); the app's own action row (`app:<n>:0`, human overlay, and its `human_actions` row) is a third row of another kind and stays. The ack arrives first, so the rule cannot run at write time. It runs after a stream `mission.paused` is stored: `Store::retract_echoed_pauses(mission)` runs `dedupe_paused` over the mission's `mission.paused` and `mission.resumed` events (ack ids named by the new `control::is_ack`) and deletes the ack rows the stream reports before the next `mission.resumed`. The host pump (`host.rs` `store`) calls it, drops the ack from the batch it has not flushed, and tells the UI through the new `EventBatch.retracted_events` (serde default, so older payloads read) which `merge` applies with `ingest::drop_events`. Option chosen from B-093: the host-side retract, not a view-time filter: the store then holds one row, so replay, Details and the Panel counts agree. No store schema change, no migration, no threshold (so nothing in `config/observatory.example.toml`). Alternative considered: a view-time filter in the board fold (rejected: the store would still hold two rows and every reader would need the filter). A lone ack (the stream never reports the pause) is kept. Evidence: `crates/observatory-core/tests/hosted_pause.rs`, `tests/event_batches.rs`. | m4-wire-dedupe-paused | done (m4 integration) |

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
