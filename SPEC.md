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
| M3 intervention | 35 | 7 | 60 | Human-overlay semantics over M2 controls; wording only. |
| M4 branch, gate, fix loop | 75 | 15 | 120 | Two adapters (git, GitHub), the demo branch pair, a live PR loop; drops branch-deletion detection, `deploy.completed` and two merge kinds (F-028). |
| M5 time, waste, cost | 105 | 21 | 120 | Priority: engineering cycle time redefined, stage times per phase, waste measured as 8 and displayed as 5, both clocks, the Panel. |
| Total | 630 | 75 (M1–M5) | 630 | M1–M5 sum 375 (planning numbers, not measurements); 630 stays the hard stop for the mission, and the final PR and fixes come out of the unspent remainder. |

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
  over the M0 events: Plan 443, Contract 69, Build 424, Scrutiny 87, User testing 122, Integrate 134,
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
golden is authoritative): Plan 443 / Contract 69 / Build 424 / Scrutiny 87 / User testing 122 / Integrate 134 /
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

## 10. Evolution log

One row per integrated milestone, written at integration from the workers' and validators'
reports.

| Milestone | Integrated | What landed | Tests green | Usage spent | Deviations | Unknown events left |
|---|---|---|---|---|---|---|
| S | 2026-10-23 | Cargo workspace (`observatory-core`, `observatory-cli`, `observatory-app`), `Cargo.lock`; canonical `Event` with 61 PRD kinds plus `Unknown`; `JsonlSource`, capture header and the Unknown-only `mapping::v1`; `observatory-cli replay` (`unknown events: 2027` on the S fixture); Leptos UI and Tauri window with nine empty lanes; S fixture (orchestrator session plus 25 children, gitleaks clean); `machete` and `coverage` jobs, tag check in `release.yml`, pinned bundle tools; D-011 | `cargo test --workspace --all-features`: 41 passed, 0 failed; `make check` exit 0 on the merge commit `38e5617`; coverage 94.48 % lines on `observatory-core` (reported, not enforced); 20 of 20 VAL-SKEL assertions passed; **`mission-checks` red on the first run** (run 39919592562, `rust` job: the Linux runner lacks `glib-2.0`, which the Tauri shell needs), see `reports/ci/2026-10-23-s.md` | 13.21 USD-eq of the 40 cap on workers and validators (`reports/usage/s.md`); orchestrator share not exposed | §3 M0 and §5.2 amended (mapping version from the fixture header, D-012); §3 M4 acceptance amended (owner merges, app surfaces 403, D-014); §8 hybrid backlog withdrawn (D-015); D-011 (deny exceptions) awaits owner confirmation; kimi-k3 episode at the boundary (D-018) | 2027 on the S fixture (expected: the S mapper knows no kind); 0 required from the M0 fixture |
| S, second pass (M0 pre-work) | 2026-10-23 | `wt/m0` merged as `cc05aa5` (`Integrate m0 CI fix and S fixture redaction`): Linux `rust` job without the Tauri shell and `wt/**` trigger (D-022); `scripts/redact-fixture.sh`, `make redact-check` and the redacted S fixture (D-020); owner decisions D-019, D-020, D-021, D-011 and D-018 addenda, rules 14, 21, 22; hook `script` field | `make check` exit 0 on `cc05aa5`; `mission-checks` green, run 39928967884, 2 min 56 s; user-testing round 2 on `claude-opus-5-5`: 20 of 20 VAL-SKEL passed | S unchanged at 13.21 USD-eq of 40; round-2 validators 2.69; orchestrator delta 18.94 USD-eq (`reports/usage/s.md`) | S row above: "`mission-checks` red" is superseded by this row; D-022 (Linux CI excludes the shell, B-024) | 2027 on the S fixture (unchanged); 0 required from the M0 fixture |
| M0 | 2026-10-23 | Redacted M0 capture (this mission's own sessions: root plus 55 children, 112 fixture files, gitleaks clean); sidecar reader and session tree with role resolution; `replay` over the root and its children, `--root-only`; mapping v1 (30 observed kinds, 31 defined-not-observed, golden `.events.json`, `unknown events: 0`); SQLite pointer store, `inspect` and `replay --store`; gate rule from the exit code; redactor fix for JSON escapes; coverage job enforced at 80 %; D-023 | `make check` exit 0 on the merge commit `f66a9f5`; 128 tests passed, 0 failed (`cargo test --workspace`); coverage 94.21 % lines on `observatory-core` (floor 80, `COVERAGE_MIN` "80" in `mission-branch.yml`); `mission-checks` green, run 39953923391, 1 min 45 s (`reports/ci/2026-10-23-m0.md`); scrutiny round 2 PASS at `88866ef`; user testing round 1: 26 of 27 passed, VAL-PROC-039 failed until the D-020 addendum written at this integration | Workers and validators 33.28 USD-eq plus orchestrator subagents 15.77 plus this pass, 49.05 priced of the 60 cap; orchestrator own delta 66.87 USD-eq against 32 (209 %); `reports/usage/m0.md` | D-023 added; D-020 addendum (third rule, hash tables, redactor fix); M0 orchestrator cap 32 (F-024); `source_dir` stored redacted; CI warm budget 3 min exceeded on `wt/m0` (run 39942226174, 3 min 58 s, cold rust job 3 min 40 s), met on the mission branch (1 min 45 s); orchestrator cap exceeded (deviation, rule 9) | 0 on the S and M0 fixtures (S 661 events, M0 1279 events over 8735 records) |
| M1 | 2026-10-24 | `wt/m1` merged as `c2fb42a` (`Integrate m1: board, replay, attach, usage-log source`): seven phase lanes in mapping v1 and the golden (D-024), `view` models (board, card, details, agents, panel, filters, scrubber, raw-line scrub), attach watcher with a SQLite-backed live path and `Attach::poll`, usage-log source with `turn.usage` events (D-029), the Leptos board with Details and agents views (D-030: go), the Tauri host with live attach and an open-by-path field, hook duration logging (D-021 addendum), README UI build command; session names by `user` tag then id; D-030 | `make check` exit 0 on `c2fb42a`, real 1 m 32.700 s: 262 passed, 0 failed, 0 ignored; `cargo llvm-cov` alone exit 0, real 2 m 1.731 s, 93.89 % lines on `observatory-core`; `mission-checks` green, run 40061513431, 3 min 48 s (over the 3 min warm budget; `reports/ci/2026-10-24-m1.md`); scrutiny and user-testing rounds passed in `user-testing-m1`; clean-clone UI build 56.95 s and 61.04 s | Workers, validators and orchestrator subagents 110.38 USD-eq of the 110 cap (workers 37.05, validators 48.53, subagents 24.80; plus 3.44 for the M0 validation re-run in the window); orchestrator own delta 115.74 USD-eq against 22 (526 %); throttled 255.4 min (two `unrecoverable_usage_402` windows); compile share 1.74 % lower bound; `reports/usage/m1.md` | D-030 added (Leptos go); D-023, D-024, D-029 addenda; golden refrozen (2394 events, `25d91c53…`); usage-log extract fixture added; M1 cap exceeded by 0.38 and orchestrator cap exceeded (deviations, rule 9); CI warm budget 3 min exceeded (B-052); the M0 `.capture.json` header gained a `usage_log` block and a refreshed `gitleaks` field (per-file redaction rows and all fixture and child hashes unchanged); `0001_init.sql` was edited in place twice during M1, which is forbidden from M2 on (AGENTS.md) | 0 on the S and M0 fixtures (S 661 events, M0 1279 session events over 8735 records, plus 1115 `turn.usage` events); a live attach of a real `droid exec` session shows one extra unknown event until `m2-fix-rewritten-line-resync` lands (B-051) |

**Frozen from the M0 integration (D-020, D-023), refrozen at the M1 integration.** Fixture and golden hashes
(`shasum -a 256`, bytes at the M1 merge commit `c2fb42a` plus this pass's one edit to the M0 `.capture.json`); a
change needs a decision file:

| File | SHA-256 |
|---|---|
| `fixtures/streams/2026-10-22-observatory-v1-m0.jsonl` | `ab23da5510478bb1c519adb78e1f47f959398477bfd802a9329bc71318a9a58c` |
| `fixtures/streams/2026-10-22-observatory-v1-m0.settings.json` | `efef27eb7fe491e2cef90bd83634230e4b845a742d00141658c88dd6154076ec` |
| `fixtures/streams/2026-10-22-observatory-v1-m0.capture.json` | `4865d01dd8a1efefff751b55716cef0b8d37e916a2411228b163840c79b4b400` |
| `fixtures/streams/2026-10-22-observatory-v1-m0.events.json` (golden, 2394 events) | `25d91c53b580f8f5f3f1a62ecaa977a24dca0b61f5caf86e296182e8f23a5750` |
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

**Amended by F-028 (D-024).** The golden hash `f055657a46af2713f50d1963dab684495d5f3770225f942adfcbb28e8331f3e1`
above was superseded at the M1 integration: `2026-10-22-observatory-v1-m0.events.json` was regenerated
at Step 0 of `m1-view-models-core`, for the seven phase lanes and the `overlay` field, and the
new SHA-256 is recorded in the M1 row. The fixture `.jsonl` and `.settings.json`
hashes of both fixtures and the `.children/` hashes stay as frozen (the `.capture.json` change is listed above). Expected phase counts on the
regenerated M0 golden (the committed golden is authoritative): Plan 443, Contract 69, Build 424,
Scrutiny 87, User testing 122, Integrate 134, Gate 0; 70 human overlays.

The per-file hashes of the `.children/` directories are `redaction.files[]` in each `.capture.json`
and the pairs are in the D-020 addendum. Mapping v1 golden counts: M0 1279 events (root only 474), S 661
events (root only 364); `gate_command.run/passed/failed` 35/34/1 (D-023).
