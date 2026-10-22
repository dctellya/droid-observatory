# PRD: droid-observatory

| Field | Value |
|---|---|
| Status | Draft v0.5 for review |
| Date | 2026-10-22 |
| Owner | dct |
| Author | Droid (ideation session, see `ai/session/2026-10-22-session.md`) |
| Changes from v0.1 to v0.4 | See Appendix C |

---

## 1. Summary

droid-observatory is a Rust desktop application that shows how a Factory Mission moves through the engineering build cycle. It renders the mission as a board whose lanes are the orchestrated states of a mission, from the human's intent through orchestration, building, validation, integration, the human gate, and the outside ecosystem (PR, CI, deploy). Alongside the flow it measures what the flow costs: cycle time, waste by cause, tokens and money per sub-droid and per model.

The observatory attaches to missions started from any terminal on this machine (it watches `~/.factory/sessions/` and tails each new session file as it grows), replays finished ones from the same files, and can also host a mission itself (it spawns `droid exec --mission` and consumes the JSON-RPC event stream, which adds the control channel). Attach is the primary live mode; hosting is the secondary path. It never copies the event data into a second source of truth: every lane entry is a view over the raw JSONL line it came from.

A mission is not complete when the workers stop. It is complete when the gate is green and the merge has happened.

## 2. Delivery model

This section defines the units the board shows. Everything else follows from it.

- **A mission is a business feature: the unit that ships.** For this project, v1 of the observatory. A mission ends in **one PR into `main`**, outcome-sized, with the evidence attached. The full attested CI runs on that PR; CD runs from `main`.
- **Milestones are internal iterations**, not shippable. Each milestone runs in its own worktree and is integrated into a **`mission/<name>` branch**, the mission's interface to `main`. Fast checks (tests, lint, the gate command) run on that branch to keep it healthy. Nothing on it is shippable or pretends to be.
- **The mission branch is short-lived by construction.** It dies at the merge. That is what keeps it from becoming a long-lived `dev` branch.
- **Features** are the orchestrator's work items inside a milestone. Workers build them, validators judge them, and metrics are computed at feature granularity, then rolled up to the milestone and the mission.
- **The fix loop after CI red reopens the mission on its branch.** It does not open a new PR. The same PR goes green or stays red.

On the board: **cards are milestones**; the gate group in lane 8 is the mission's one PR; lane 6 is integration into `mission/<name>`; lane 8 is the PR.

**Models per role** (decision D-006; `ai/RULES.md` rule 19): orchestrator `claude-fable-5-1`, workers `claude-sonnet-5-5`, validators `claude-opus-5-5`. Reasoning effort per role is declared in the spec. A change of model is a decision file.

## 3. Problem

Mission Control (CLI and Factory App) shows progress, workers, and a log. It does not show:

- the mission's full life, including what happens after the workers stop (PR, CI, attestations, deploy, red builds that need a fix);
- where time goes: active work versus waiting on a human, waiting on validation, waiting on merge, rework after a blocking finding, retries and conflicts;
- what each role and model consumed, so that model and reasoning-effort choices can be compared with real numbers;
- a single place to act when the mission needs a human: approve, answer, redirect, merge.

Running missions without that view means managing agents blind on the two dimensions that matter most for an engineering organization: flow and cost.

## 4. Goals

1. Show every milestone of a mission across the nine lanes defined in section 7, live and in replay, with features visible inside each card.
2. Show per card: validation state (red/green), current agent, gate state, and a Details view with review notes, validation criteria, commits, and raw event links.
3. Let the human act from the board: pause/resume the orchestrator, send instructions, answer escalations, approve, mark a feature complete or skipped, and merge the PR when policy says a human merges.
4. Measure cycle time, waste by cause, mission overhead, and bottlenecks per feature, per milestone, per role, and per mission.
5. Measure cost: tokens per sub-droid, converted to Factory Standard Credits when the stream exposes them, otherwise money via a dated price table that every displayed cost references.
6. Treat CI red on the mission PR as a counted rework loop: reopen the mission on its branch, loop the affected card back into the Workers lane, keep the same PR.

## 5. Non-goals (v1)

- Replacing Mission Control for planning. The observatory starts after a plan exists (it may launch the mission, but the plan conversation stays in Droid).
- Multi-user or server deployment. v1 is a single-user desktop app reading local data and the user's own GitHub access.
- Writing to Factory's hosted API beyond what the local `droid` process already does.
- Editing code or reviewing diffs inside the app. It links out to the worktree, commit, or PR.
- Supporting CI systems other than GitHub Actions in v1 (adapter boundary is defined so others can follow).

## 6. Users

**Mission owner (primary).** The engineer acting as project manager of agents. Wants to see progress, act when needed, and know what the mission cost in time and money.

**Reviewer / gatekeeper.** Opens the Details view, reads scrutiny findings and QA results, watches the outcome video, and merges or sends back.

**Engineering lead (reader).** Compares missions and models over time: cycle time, waste, cost per feature. Does not intervene.

## 7. Lanes

The board has nine lanes. Lanes 1–6 are fed by the mission stream, 7–8 by GitHub and human actions, 9 by session metadata. Each event carries `session_id`, `role`, `model`, `timestamp`, `duration`, and a pointer to its raw JSONL line.

| # | Lane | What it holds | Source | Event kinds |
|---|---|---|---|---|
| 1 | **Intent** | Mission prompt, spec, acceptance criteria; later human re-prompts | stream (user ↔ orchestrator), mission file | `intent.created`, `intent.amended`, `question.asked` (orchestrator → human), `intent.answered`, `spec.versioned` |
| 2 | **Orchestrator** | Plan, feature breakdown, milestones, task assignment, replans | stream (orchestrator session) | `plan.created`, `feature.defined`, `milestone.defined`, `task.assigned`, `task.skipped`, `plan.revised`, `escalation.raised`, `milestone.completed`, `mission.paused`, `mission.resumed`, `mission.completed`, `context.compacted` |
| 3 | **Workers** | One sub-lane per worker (or per feature when the mission names them) | stream (worker child sessions) | `subagent.spawned`, `task.started`, `file.touched`, `tests.run`, `commit.created`, `task.done`, `subagent.finished`, `tool.error`, `permission.requested`, `permission.granted`, `permission.denied`, `hook.blocked`, `usage.reported`, `model.changed` |
| 4 | **Validators, scrutiny** | Review findings, blocking vs non-blocking, re-work requested and resolved | stream (scrutiny validator sessions) | `finding.blocking`, `finding.non_blocking`, `rework.requested`, `rework.resolved`, `scrutiny.passed` |
| 5 | **Validators, user testing** | QA runs against the built thing, scenario results | stream (user-testing validator sessions) | `qa.scenario.passed`, `qa.scenario.failed`, `qa.evidence` (screenshot, video) |
| 6 | **Integration** | Worktree merges into `mission/<name>`, conflicts, fast checks. Wording is "integrated in mission", never "merged" | stream (orchestrator/worker git activity) | `integration.merged`, `integration.conflict`, `integration.conflict_resolved`, `gate_command.run`, `gate_command.passed`, `gate_command.failed` |
| 7 | **Human** | Approvals, answers to escalations, review of the outcome (the video), the merge of the PR | app actions, stream attention events | `approval.requested`, `approval.given`, `question.answered`, `review.outcome`, `pr.merged_by_human` |
| 8 | **Outside** | The mission PR into `main`: opened, reviews, checks, attestations, merge, deploys. Empty for the observatory itself until its final PR | GitHub adapter | `pr.opened`, `pr.review_requested`, `check.started`, `check.passed`, `check.failed`, `check.required_missing`, `ruleset.blocked`, `attestation.recorded`, `attestation.verified`, `pr.merged_by_policy`, `deploy.completed` |
| 9 | **Time, Waste, Cost and models** | Tokens and usage per role, which model did what; budget meter; elapsed, stuck, and wasted time; mission overhead | derived from all lanes + session metadata | computed, not evented |

**`unknown.event`** is a first-class kind in every lane. It is counted, surfaced in an "unmapped" tray, and never dropped. M0's exit criterion is driving its count to zero on the capture.

Lane 9 is rendered as a panel that doubles as the mission's budget meter. It is the lane the other eight are measured against.

`context.compacted` is kept in lane 2 because it explains sudden quality drops in the sessions that follow it. `hook.blocked` is the write guard firing; it is counted under retry and failure.

## 8. Board model

**Card = one milestone.** The milestone is the spec phase: the orchestrator plans it, workers build its features in a worktree, validators judge it, and it integrates into `mission/<name>`. Each card lists its features with their individual state, and metrics roll up feature → milestone → mission.

A card moves left to right. Its position is the lane where its most recent open activity is. The Details view shows the full history across lanes.

**Gate group.** Lane 8 holds one group per mission: the mission PR. All cards of the mission share that gate state; the badge sits on the group. Until the PR exists, lane 8 shows the mission branch with its fast-check status as a pre-gate placeholder.

**Fix loop.** When CI goes red on the mission PR, the observatory reopens the mission on `mission/<name>` with a cycle tagged `fix` on the milestone(s) whose features the fix touches. Those cards are drawn with a loop arrow back into lane 3. The PR stays the same. The loop is counted as rework in lane 9.

**Mission completion.** Mission state is `complete` only when the PR is merged with CI green. Until then it is `planning`, `building`, `validating`, `at gate`, or `fixing`.

### 8.1 Card anatomy

```text
┌──────────────────────────────────────────────────────┐
│ M2  Host                                    [Details]│
│ features 4/6 done · worktree wt/m2 · mission/obs-v1  │
│ ● validation: RED (1 blocking)  ◆ branch: fast checks ✓│
│ now: worker-3 (claude-sonnet-5-5) on F-014 · 00:14:22 │
│ cycle 2h 10m · waste 38m (rework 22m, human 16m)      │
│ 412k tok · $1.84 (prices 2026-10-22)                  │
└──────────────────────────────────────────────────────┘

Gate group (lane 8):
┌── PR #41  mission/observatory-v1 → main ─────────────┐
│ ◆ gate: HUMAN REQUIRED · checks 7/7 ✓ · attested ✓     │
│ cards: M0 M1 M2 M3 M4 M5                      [Merge]  │
└────────────────────────────────────────────────────────┘
```

- **Validation badge** (card): GREEN when scrutiny passed and user-testing scenarios passed for all features on the latest build; RED when any blocking finding or failed scenario is open; GREY when not yet validated.
- **Branch badge** (card): fast checks on `mission/<name>` passed / failed / running for this milestone's integration.
- **Gate badge** (group): `pending` (no PR), `checks running`, `review requested`, `human required`, `ruleset blocked`, `policy-merged`, `CI green`, `CI red`.
- **Now**: the agent currently holding a feature of this card (role, model, session id, feature, time on task). Empty when idle, with the idle reason (waiting on human, waiting on validation, queued).
- **Merge** button (group): only visible when the gate policy requires a human merge and checks are green.

## 9. Functional requirements

### 9.1 Board (FR-B)

- FR-B1: Render the nine lanes with milestone cards positioned by current lane.
- FR-B2: Lane 3 shows sub-lanes per worker, or per feature when the mission names them. Toggle between the two groupings.
- FR-B3: Lane 8 shows the mission PR as one gate group containing all cards; before the PR exists, it shows the mission branch with fast-check status.
- FR-B4: Fix loops are drawn as a visible return arc from lane 8 to lane 3 on the affected card(s), with the cycle count on the card.
- FR-B5: Filter by milestone, feature, role, model, validation state, gate state.
- FR-B6: Live mode updates within 1 s of the event arriving; replay mode has a time scrubber.
- FR-B7: An "unmapped" tray shows the count of `unknown.event` and opens their raw lines.

### 9.2 Details (FR-D)

Opened via the Details button on a card. Tabs:

- FR-D1 **Timeline**: every event for the milestone and its features across all lanes, each with role, model, duration, and a link that opens the raw JSONL line.
- FR-D2 **Review notes**: scrutiny findings (blocking / non-blocking, status: open / resolved), user-testing scenario results with evidence (screenshots, video).
- FR-D3 **Validation criteria**: the acceptance criteria from the plan, per feature, and which events satisfied each.
- FR-D4 **Commits and integration**: commits, worktree, integration into `mission/<name>`, conflicts, fast-check results.
- FR-D5 **Gate**: mission PR link, reviews, checks, attestations, merge policy and which rule decided, merge history.
- FR-D6 **Cost and waste**: per-feature and per-role token and cost breakdown for this milestone, waste split by cause, rework windows listed.

### 9.3 Agent attribution (FR-A)

- FR-A1: Show which agent (role, session id, model) is working on each feature now, surfaced on the card.
- FR-A2: Agent panel lists all orchestrator, worker, and validator sessions with status (active, idle, waiting, finished), current feature, model, and tokens so far.
- FR-A3: Selecting an agent shows its live output stream.
- FR-A4: `model.changed` and `context.compacted` are marked on the agent's timeline.

### 9.4 Human intervention (FR-H)

All actions are logged as lane 7 events with the human as actor.

- FR-H1: Pause / resume the orchestrator.
- FR-H2: Send a message to the orchestrator (redirect, re-plan request, scope change).
- FR-H3: Answer an open question or approve/deny a permission request raised by any sub-droid.
- FR-H4: Mark a feature complete or skipped (relayed to the orchestrator as an instruction).
- FR-H5: Stall detection: a feature with no active time for a configurable threshold is flagged; the flag suggests the matching troubleshooting prompt (frozen mission, stuck worker, repeated failure).
- FR-H6: Interventions are expected to be rare. The UI must not nudge the user to intervene when the mission is progressing (no alerts on normal waits under threshold).

### 9.5 Mission branch and gate (FR-G)

- FR-G1: Identify the mission branch `mission/<name>` and the worktree of each milestone.
- FR-G2: Record fast-check runs on the mission branch as lane 6 `gate_command.*` events, attributed to the milestone being integrated.
- FR-G3: Detect the mission PR (`mission/<name>` → `main`) when it opens; poll check runs, required checks, reviews, rulesets, and attestations.
- FR-G4: Read the merge policy for `main` (GitHub branch protection / rulesets plus an app-level override) and decide whether a human merge is required. Show which rule decided.
- FR-G5: When a human merge is required and checks are green, show the Merge button on the gate group. Merge via GitHub API with the repository's allowed merge method. Confirm before merging, naming PR and target branch.
- FR-G6: When policy merges automatically, flip the badge to `policy-merged` on detection.
- FR-G7: On CI red, reopen the mission on `mission/<name>` with a `fix` cycle on the affected milestone(s); no new PR. If the orchestrator is still running, instruct it; if it has ended, launch a follow-up mission on the same branch pre-filled with the feature spec, failing check output, and original session ids.
- FR-G8: On merge, mark the mission `complete` and record that the mission branch is to be deleted (or detect its deletion).

### 9.6 Metrics (FR-M)

Definitions in section 10. All metrics are available per feature, per milestone, per role, per model, and per mission, and exportable as CSV/JSON.

- FR-M1: Cycle time per feature, per milestone, per mission, and per lane.
- FR-M2: Waste by cause, including waiting on merge and mission overhead reported separately.
- FR-M3: Bottlenecks: lanes and roles with the longest queue time and highest WIP; queued-behind-WIP idle is a bottleneck signal, unscheduled idle is not.
- FR-M4: Tokens per sub-droid, per model, per feature.
- FR-M5: Cost in Factory Standard Credits when exposed by the stream; otherwise in money from the dated price table. Every cost display shows the price-table date it used.
- FR-M6: Budget meter: mission cost so far against an optional budget set at launch.
- FR-M7: Model comparison: for the same role, cost and first-pass active time per completed feature by model.

### 9.7 Mission lifecycle (FR-L)

- FR-L1: Attach (primary live mode): watch `~/.factory/sessions/` with a filesystem notifier, detect any new session JSONL (a mission started from any terminal on this machine), tail it as it grows and render it within 1 s, named by its `--tag` or session id. No control channel.
- FR-L2: Replay a finished mission from JSONL files. Same adapter as attach: replay is attach at end-of-file, with the scrubber.
- FR-L3: Host (secondary path): launch a mission from the app: choose repo, mission file, mission branch name, models and reasoning effort for orchestrator / worker / validator, autonomy flags, optional budget. The app spawns `droid exec --mission --input-format stream-jsonrpc --output-format stream-jsonrpc`; this is the only mode with a control channel (pause, message, answer).
- FR-L4: Mission library: list past missions with headline metrics.

## 10. Metric definitions

### 10.1 Time states

Each feature, at every instant, is in exactly one time state:

| State | Definition |
|---|---|
| **Active** | A worker, validator, or orchestrator is doing something for the feature: tool call in flight or model generating. |
| **Waiting on human** | A question or approval is open and nobody is acting (lane 7 open, lanes 2–6 idle for this feature). Sub-causes: *question*, *permission*, *waiting on merge* (checks green, policy requires a human, nobody has merged). |
| **Waiting on validation** | Work is done, validator not started. |
| **Retry and failure** | Time in tool errors, `hook.blocked`, re-runs of the same command, and conflict resolution. |
| **Idle, queued** | Feature is planned and ready but blocked behind another feature (WIP limit, dependency, no free worker). A bottleneck signal. |
| **Idle, unscheduled** | Feature exists but nothing is scheduled and nothing is blocking it. Not a bottleneck signal. |

Rework is not a separate time state. It is a window (10.2) that overlays active, retry, and waiting time.

### 10.2 Rework windows

A **rework window** opens at a blocking signal and closes at its resolution event, per signal:

| Opens | Closes |
|---|---|
| `finding.blocking` | `rework.resolved` |
| `qa.scenario.failed` | `qa.scenario.passed` for the same scenario |
| `integration.conflict` | `integration.conflict_resolved` |
| `check.failed` (mission PR) | `check.passed` for the same check |

Overlapping windows on the same feature merge into one. A window with no closing event stays open until mission end and is reported as unresolved.

- **Rework time** = active time inside rework windows.
- **First-pass active time** = active time outside any rework window.

This bounds rework so that legitimate new work after a fix is not counted as waste.

### 10.3 Mission overhead

Orchestrator time before the first `feature.defined`, replanning time not attributable to one feature, and `context.compacted` recovery belong to no card. They are reported as **mission overhead**, a separate line in lane 9, never apportioned silently. An optional explicit apportionment (equal split across features) can be switched on for comparisons and is labeled as such.

### 10.4 Derived measures

- **Cycle time** (feature) = `feature.defined` → feature's last `task.done` with validation green and integrated. **Cycle time** (milestone) = `milestone.defined` → `milestone.completed`. **Cycle time** (mission) = `intent.created` → `pr.merged` (human or policy) with CI green. Stage cycle times are computed per lane.
- **Waste** = total elapsed − first-pass active time, split by cause: waiting on human (question, permission, merge), waiting on validation, rework, retry and failure, idle queued, idle unscheduled. Mission overhead is reported next to waste, not inside it. Reported per feature, per milestone, and per role.
- **Bottleneck** = lane or role with the highest sum of waiting plus queued time across features in the window, and the lane with the highest WIP relative to its throughput.
- **Fix loops** = count of `fix` cycles per milestone and per mission.

### 10.5 Cost

- Token counts are taken from `usage.reported` events per session (input, output, cache read, cache write when available).
- If the stream reports Factory Standard Credits, credits are the primary unit.
- Otherwise cost = Σ tokens × price per model and token class, from the **price table**: a dated config file (`~/.droid-observatory/prices/<YYYY-MM-DD>.toml`). The board always shows the date of the table it used, and a mission keeps the table it started with unless the user re-prices it explicitly.
- Cost is shown next to waste so that a slow, cheap model and a fast, expensive one can be compared on the same panel.

Price table example (values illustrative, table dated):

```toml
# ~/.droid-observatory/prices/2026-10-22.toml
effective = "2026-10-22"
currency = "USD"

[models."claude-fable-5-1"]
input_per_m = 0.0
output_per_m = 0.0
cache_read_per_m = 0.0
cache_write_per_m = 0.0

[models."claude-opus-5-5"]
input_per_m = 0.0
output_per_m = 0.0

[models."claude-sonnet-5-5"]
input_per_m = 0.0
output_per_m = 0.0

[models."claude-haiku-4-5-20251001"]
input_per_m = 0.0
output_per_m = 0.0
```

The shipped default table carries the vendor prices on its date; the user edits and re-dates it.

## 11. Data sources and architecture

### 11.1 Sources

| Source | Mode | Provides |
|---|---|---|
| `~/.factory/sessions/<cwd-slug>/<session-id>.jsonl` (+ `.settings.json`), watched | **Attach (primary live mode)** | The observatory watches `~/.factory/sessions/` with a filesystem notifier, detects any new session JSONL (a mission started from any terminal on this machine), tails it as it grows and renders it in real time, named by its `--tag` or session id. Worker and validator events via the child session files. These are the canonical raw lines that lane entries link to. Read-only: no control channel. |
| Same files, finished | Replay | Same adapter: replay is attach at end-of-file, plus the time scrubber. |
| `droid exec --mission --input-format stream-jsonrpc --output-format stream-jsonrpc` | Host (secondary live mode) | Every orchestrator event; token usage; permission and question requests; the control channel (pause, message, answer). Started from the app's launch screen. |
| Git (local) | Live, polled | Mission branch, milestone worktrees, integration commits, fast-check results. |
| GitHub REST/GraphQL (user token) | Live, polled | The mission PR, reviews, check runs, required checks, rulesets, attestations, merges, deploy statuses. |
| App actions | Live | Lane 7 human events. |

Attach is primary because missions are started from the terminal in practice; the launch screen is the secondary path. Hosting is the only mode with a control channel; attach and replay are read-only views over the same files.

Open items: the exact JSON-RPC event schema for missions (how worker child sessions, feature ids, milestone ids, and validator roles are identified) is not fully documented, and where `--tag` is recorded (session settings file or first stream line) is unconfirmed. M0 captures a real mission stream, confirms both, and freezes the mapping from raw events to lane events as a versioned module with fixtures.

### 11.2 Components

```text
┌─────────────────────────── Tauri 2 shell ───────────────────────────┐
│  UI (webview)                                                        │
│   board · details · agents · metrics · launch · settings            │
└──────────────▲───────────────────────────────────────────────────────┘
               │ Tauri commands + events (serde types shared with core)
┌──────────────┴───────────────────────────────────────────────────────┐
│ observatory-core (Rust lib)                                          │
│  ingest/   jsonl_watch · jsonl_tail (attach; replay = attach at EOF) │
│            droid_rpc (host) · git · github                           │
│  model/    Event · Feature · Milestone · Agent · Lane mapping        │
│            (versioned by Droid version)                              │
│  metrics/  time states · rework windows · overhead · cycle · waste · │
│            bottleneck · cost                                         │
│  store/    SQLite index: events(ptr→jsonl file,offset), features,    │
│            milestones, agents, pr, checks, prices                    │
│  control/  pause · message · answer · mark · merge                   │
└──────────────────────────────────────────────────────────────────────┘
```

- **The lane is a view, not a copy.** The store indexes events with a pointer to the raw JSONL line (file, byte offset). Opening an event in the UI reads the raw line on demand.
- `observatory-core` has no UI dependency and is testable from fixtures (recorded mission streams). A thin CLI (`observatory replay <session>`) exercises it headlessly and is the first thing that works.
- The GitHub adapter is behind a trait so other forges and CI systems can follow.

### 11.3 Security

- GitHub token and Factory credentials stay in the OS keychain (Tauri keyring plugin). Never written to the store or logs.
- Raw JSONL may contain secrets from tool output; the app renders it read-only and never uploads it.
- Merge is the only outward-facing write. It requires an explicit confirmation naming the PR and target branch.

### 11.4 Telemetry export (Grafana Cloud)

Two kinds of telemetry leave the app, both over OpenTelemetry OTLP (`http/protobuf`) to a Grafana Cloud stack, both off by default so the app works fully offline (decision D-004):

- **App telemetry**: `tracing` spans for ingest, mapping, metrics, and control; errors as span events.
- **Mission metrics**: the lane-9 series, so the budget meter and waste can be charted and alerted on outside the desktop window. Metric names are a contract: `observatory.mission.cycle_seconds`, `observatory.feature.cycle_seconds`, `observatory.waste_seconds`, `observatory.rework_windows`, `observatory.fix_loops`, `observatory.tokens.{input,output,cache_read,cache_write}`, `observatory.cost_usd`, `observatory.unknown_events`.

Contract (owner, D-010): OTLP over HTTP to `https://otlp-gateway-prod-eu-west-2.grafana.net/otlp` (`/v1/metrics`, `/v1/traces`, `/v1/logs`). The `Authorization=Basic …` header value is read at runtime from the macOS keychain item with service `droid-observatory/otlp-headers` and account = current user; never written to config, the store or logs; if the item is missing, telemetry stays off and the UI says so. Resource attributes: `service.name=droid-observatory`, `service.version`. Metric labels only from `mission`, `milestone`, `role`, `model`, `cause`; never session id, event id or free text. Counters and histograms only, no per-event gauges. The spec states the number of active series the app adds and keeps it under 200 (B-017). Config keys and defaults: `config/observatory.example.toml` `[telemetry]`. CI telemetry is deferred (B-002).

## 12. Technology options and recommendation

Constraint: Rust. Preference: Tauri 2. Needs: data-dense board, live updates, charts, markdown and video rendering in Details, long-lived subprocess management, SQLite.

| Option | Pros | Cons |
|---|---|---|
| **A. Tauri 2 + Leptos (Rust, CSR via Trunk)** | Single language; shared serde types between core and UI with no bindings; fine-grained reactivity suits a live board; webview gives markdown, video, CSS layout for free. | Charting ecosystem thinner than JS; WASM/Trunk toolchain adds build steps; smaller community. **Droids have far less Leptos in their training than React or Svelte**, so worker rework on UI tasks is likely higher. **The WASM/Trunk toolchain inside a Mission is a new failure mode** (toolchain install, target, dev server) that validators may struggle to diagnose. |
| B. Tauri 2 + Dioxus | Same single-language benefit; React-like ergonomics; growing fast. | Dioxus also ships its own desktop renderer, which blurs the "why Tauri" story; same training-data and toolchain concerns as A. |
| C. Tauri 2 + TypeScript (Svelte or React) | Richest charting and UI libraries; fastest to a polished UI; best-known stack for Droids. | Two languages; types must be generated (ts-rs/specta) and kept in sync; the user's stated preference is Rust. |
| D. iced (native) | Pure Rust, no webview, small binary, Elm architecture fits an event-driven board. | Markdown, video, and rich text are weak; complex layouts cost more effort; no Tauri plugin ecosystem (keyring, updater, tray). |
| E. egui (native, immediate mode) | Trivial live updates and plots (egui_plot); excellent for dashboards. | Looks like a tool, not an app; text-heavy Details view (review notes, markdown) is awkward; accessibility and theming limited. |

**Recommendation: Option A, Tauri 2 + Leptos, with M1 as an explicit go/no-go.**

Reasoning: the hard part of this product is `observatory-core` (ingest, mapping, metrics), not the UI. Keeping the UI in Rust means the metric types, lane mapping, and event model are used directly in the view with no translation layer, which is where drift and bugs would otherwise appear. The webview covers the rich-content needs (markdown findings, QA video, PR rendering) that native options handle poorly. Charting is the one real gap; lane 9 needs stacked bars and timelines, achievable as SVG from Leptos signals, with a JS chart library as a documented escape hatch.

**Go/no-go criterion (evaluated at M1 completion):** the board renders the M0 replay (all nine lanes, cards positioned correctly, Details timeline open) with **at most 3 worker sessions of rework** after the first scrutiny pass, and the Trunk/WASM build succeeds from a clean checkout with the documented command. If either fails, switch to Option C for M2 onward, keeping `observatory-core` unchanged; the Tauri command boundary is designed to make that swap possible. (Threshold of 3 agreed 2026-10-22; may be adjusted at planning time.)

## 13. Milestones (seed of the mission plan)

**One mission, six milestones, one branch `mission/observatory-v1`, one PR at the end.** Each milestone runs in its own worktree and integrates into the mission branch with fast checks. M0 must complete before the orchestrator plans M1 onward in detail, because the stream schema is unknown until the capture exists.

| # | Name | Outcome | Validation |
|---|---|---|---|
| M0 | Stream capture and mapping | Capture one real mission run (`droid exec --mission` on a sample repo); write the raw→lane event mapper with fixtures; `observatory replay` CLI prints lane events. Freeze the mapping, versioned by Droid version. | Fixture tests pass; `unknown.event` count on the capture is zero; every event kind in section 7 that appears in the capture is mapped. **Gate for planning M1–M5 in detail.** |
| M1 | Board in replay and attach | Tauri 2 + Leptos app renders the nine lanes and milestone cards from a session JSONL; Details timeline tab; scrubber; unmapped tray. Attach: a filesystem watcher on `~/.factory/sessions/` detects any new session file, tails it as it grows and renders it live, named by its `--tag` or session id; replay is attach at end-of-file, same adapter. Agent panel with current agent per feature; `model.changed` and `context.compacted` markers. | Replay of the M0 capture shows all milestones and features with correct lane positions versus the capture's known outcome. A mission started in a terminal appears on the board, and each of its events shows, within 1 s. Agent attribution matches orchestrator logs. **Leptos go/no-go evaluated here (section 12).** |
| M2 | Host | Launch screen (secondary path); spawn `droid exec --mission --input-format stream-jsonrpc --output-format stream-jsonrpc` from the app; pause, message and answer controls over the RPC input channel; live output per agent from the host stream. | A hosted mission shows events within 1 s; pause, message and answer reach the orchestrator and its reaction appears on the board; the same mission attached from its JSONL shows the same lane events. |
| M3 | Intervention | Lane 7 semantics for every control (M2 pause/resume, message, answer; plus approve/deny, mark complete/skipped); stall flag with suggested prompts. | Each action produces a lane 7 event and the orchestrator reacts. |
| M4 | Mission branch, gate, and fix loop | Git adapter (mission branch, worktrees, fast checks → lane 6); GitHub adapter; gate group in lane 8; gate badge; merge policy with rule attribution; Merge button; CI red → `fix` cycle on the affected card, same PR. | Tested against a **private demo repository with a PR flow, CI, and rulesets** (the observatory itself has no PR flow until the end of the mission). A red check on the demo PR opens a fix cycle; a green check with human policy shows Merge; merging via the app updates the badge. |
| M5 | Time, waste, cost | Time-state engine; rework windows; mission overhead; cycle, waste, bottleneck; tokens per sub-droid; dated price table; budget meter; model comparison; CSV/JSON export. | Metrics recomputed from the M0 capture match a **human-produced reference spreadsheet** computed by hand once from the same capture, within rounding. The spreadsheet is a mission input, not something validators invent. |

Optional insurance before the mission starts: a manual ~20-minute capture of a small mission, so the orchestrator has something to look at while planning M0. It is not a structural step.

Dogfooding starts the moment M1 lands: the mission that builds the observatory is started from a terminal, and the observatory attaches to its session files, so M2–M5 are watched live without hosting. Its own lane 8 stays empty until the final PR `mission/observatory-v1` → `main`.

## 14. Risks and open questions

| Risk / question | Mitigation |
|---|---|
| Mission JSON-RPC event schema is not fully documented; mapping to features, milestones, and validator roles may be heuristic. | M0 captures and freezes a versioned mapping with fixtures; `unknown.event` is counted and shown, never dropped; M0 is the gate before detailed planning. |
| Leptos: less Droid training data than React/Svelte; WASM/Trunk toolchain is a new failure mode inside a mission. | Core first and headless; M1 go/no-go with a written criterion; swap to TypeScript at the Tauri command boundary. |
| Factory Standard Credits may not be exposed per session in the stream. | Price table fallback is first-class; the UI always names the unit and the price-table date. |
| Attestation formats vary (GitHub artifact attestations, SLSA provenance, custom). | v1 reads GitHub artifact attestations and check-run conclusions (`attestation.recorded`), verifies via GitHub's attestation API where possible (`attestation.verified`); others appear as opaque recorded events with a link. |
| Merge policy lives in several places (branch protection, rulesets, CODEOWNERS, org policy). | v1 reads branch protection and rulesets, plus an explicit app-level override per repo; the Details gate tab shows which rule decided. |
| A `fix` cycle after the orchestrator has ended needs a follow-up mission with the right context on the same branch. | The fix launch pre-fills the mission file with the feature spec, the failing check output, the mission branch, and the original session ids. |
| Local JSONL files may change format between Droid versions. | The mapper is versioned by the Droid version observed in the session settings file; replay of old captures stays in the test suite. |
| Filesystem notifiers coalesce or miss events (macOS FSEvents latency, a session file created before the watcher starts). | The watcher is paired with a poll at `[ingest].jsonl_poll_ms`; on start the sessions directory is scanned and files already growing are attached; attach latency is measured in M1 acceptance, not assumed. |
| `--tag` may not be recoverable from the session files. | M0 confirms where it is recorded; if absent, sessions are named by id and the first `mission.started` payload. |
| M4 cannot be tested on the observatory repo itself. | Private demo repository with PR flow, CI, and rulesets, prepared before M4. |
| M5 has no oracle unless a human produces one. | Reference spreadsheet from the M0 capture is a listed mission input. |
| Webview charting in Rust is thinner than JS. | Scoped chart set (stacked bars, timeline, sparkline) built as SVG components; JS chart lib as a documented escape hatch. |

## 15. Acceptance criteria for this PRD

The PRD is accepted when the reviewer agrees that:

1. The delivery model (mission = shippable business feature, milestones = internal iterations on `mission/<name>`, one PR) is stated correctly.
2. The nine lanes, their sources, and their event kinds are correct and complete, with `unknown.event` as a first-class kind.
3. Card = milestone, features inside the card, one gate group per mission PR, and the fix loop reopening the mission on its branch match the intended workflow.
4. The time states, bounded rework windows, first-pass definition, mission overhead, waiting on merge, and the queued/unscheduled idle split are the ones to measure against.
5. Attach as the primary live mode, replay from the same files, hosting as the secondary path with the control channel, plus git and GitHub adapters is the right v1 data source set.
6. The stack recommendation (Tauri 2 + Leptos) with the M1 go/no-go criterion is approved, or a different option is chosen.
7. The milestone plan (M0 as planning gate, demo repo for M4, human reference for M5) is acceptable as the seed of the mission plan.

## Appendix A: Canonical event

```rust
pub struct Event {
    pub id: EventId,
    pub mission_id: MissionId,
    pub milestone_id: Option<MilestoneId>,
    pub feature_id: Option<FeatureId>,
    pub lane: Lane,                 // Intent | Orchestrator | Workers | Scrutiny | UserTesting | Integration | Human | Outside
    pub kind: EventKind,            // every kind in section 7, plus Unknown { raw_type: String }
    pub role: Role,                 // Orchestrator | Worker | ScrutinyValidator | UserTestingValidator | Human | External
    pub session_id: Option<SessionId>,
    pub model: Option<ModelId>,
    pub at: Timestamp,
    pub duration: Option<Duration>,
    pub usage: Option<TokenUsage>,  // input, output, cache_read, cache_write, credits
    pub source: SourcePtr,          // Jsonl { path, byte_offset } | Git { repo, sha } | GitHub { url } | App
}

pub struct ReworkWindow {
    pub feature_id: FeatureId,
    pub opened_by: EventId,         // blocking signal
    pub closed_by: Option<EventId>, // resolution event; None = unresolved at mission end
    pub cause: ReworkCause,         // Scrutiny | UserTesting | Integration | Gate
}
```

## Appendix B: Glossary

- **Mission**: a business feature, the unit that ships. One PR into `main`.
- **Mission branch**: `mission/<name>`, the mission's interface to `main`. Short-lived; dies at the merge.
- **Milestone**: an internal iteration inside a mission; runs in its own worktree; integrates into the mission branch; one card.
- **Feature**: the orchestrator's work item inside a milestone; the granularity at which metrics are computed.
- **Sub-droid**: any agent session spawned for the mission: orchestrator, worker, scrutiny validator, user-testing validator.
- **Integration**: landing a milestone worktree on the mission branch, with fast checks. Not a merge in the gate sense.
- **Gate**: the mission PR into `main`, merged by a human or by policy, triggering attested CI and possibly CD.
- **Fix cycle**: a mission cycle tagged `fix` on the affected milestone(s) after CI red at the gate. Same branch, same PR.
- **Rework window**: the span from a blocking signal to its resolution on one feature.
- **First-pass active time**: active time outside any rework window.
- **Mission overhead**: orchestrator time attributable to no feature; reported separately.
- **Waste**: elapsed time that is not first-pass active time, split by cause.

## Appendix C: Changes from v0.1

| Area | v0.1 | v0.2 |
|---|---|---|
| Delivery model | Implicit; one PR per feature | New section 2: mission = shippable business feature; milestones = internal iterations on `mission/<name>`; one PR per mission; branch dies at merge |
| Card | Mission feature | Milestone; features listed inside; metrics at feature granularity rolled up |
| Gate | Cards grouped by the PR that carries them | One gate group per mission (the mission PR); pre-gate placeholder shows the mission branch fast-check status |
| Fix loop | New mission cycle, possibly new PR | Reopens the mission on its branch; same PR; loops the affected card(s) |
| Event kinds | Base set | Added per review: lane 1 (`question.asked`, `intent.answered`, `spec.versioned`), lane 2 (`milestone.completed`, `task.skipped`, `mission.paused/resumed/completed`, `context.compacted`), lane 3 (`subagent.spawned/finished`, `permission.*`, `hook.blocked`, `usage.reported`, `model.changed`), lane 4 (`rework.resolved`), lane 6 (`gate_command.passed/failed`, `integration.conflict_resolved`), lane 8 (`pr.review_requested`, `check.required_missing`, `ruleset.blocked`, `attestation.verified`); `unknown.event` first-class |
| Rework | All active time after first blocking signal | Bounded windows from signal to resolution; overlapping windows merge; unresolved windows reported |
| First-pass active time | Before first blocking signal | Active time outside any rework window |
| Mission overhead | Not addressed | Reported separately; optional labeled apportionment |
| Waiting on merge | Not addressed | Sub-cause under waiting on human |
| Idle | One state | Split: queued (bottleneck signal) vs unscheduled |
| Cycle time | Feature only | Feature, milestone, mission (ends at `pr.merged` with CI green) |
| Stack | Leptos recommended | Leptos recommended with two named risks and an M1 go/no-go criterion |
| Price table | Old model ids | Current ids: `claude-fable-5-1`, `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-haiku-4-5-20251001`; dated |
| Milestones | M0–M5 | Same order; one mission on `mission/observatory-v1`; M0 is the gate before detailed planning; M4 uses a private demo repo; M5 needs a human-produced reference spreadsheet; manual pre-capture optional |
| FR-G | PR per feature | Mission branch identification, fast checks → lane 6, mission PR detection, rule attribution, fix on same branch, completion on merge |

### v0.2 → v0.3 (repo readiness, 2026-10-22)

| Area | v0.2 | v0.3 |
|---|---|---|
| Telemetry | Not addressed | New §11.4: Grafana Cloud over OTLP for app telemetry and lane-9 mission metrics; metric names as contract; off by default (D-004) |
| Models per role | Not stated | §2: orchestrator `claude-fable-5-1`, workers `claude-sonnet-5-5`, validators `claude-opus-5-5` (D-006) |
| Repository | None | Readiness scaffold: CI gate (D-003), ruleset (D-007), Renovate (D-005), hooks, config contract, decisions, backlog |

### v0.3 → v0.4 (attach is the primary live mode, 2026-10-22)

Source: `ai/prd/review-2026-10-22.md` item 5; decision D-008.

| Area | v0.3 | v0.4 |
|---|---|---|
| Live mode | Host (spawn `droid exec --mission`) primary; attach listed second | §11.1: **Attach** is the primary live mode: watch `~/.factory/sessions/`, detect any new session JSONL, tail it, render within 1 s, named by `--tag` or session id. Replay is attach at end-of-file, same adapter. Host is the secondary path and the only mode with a control channel |
| FR-L | L1 launch, L2 attach, L3 replay | L1 attach, L2 replay, L3 host (launch screen) |
| M1 | Board in replay | Board in replay **and attach**; agent panel and `model.changed` / `context.compacted` markers move here; acceptance adds "a mission started in a terminal shows on the board within 1 s" |
| M2 | Live host and agents | Host only: launch screen, spawn, pause/message/answer over the RPC channel, live output per agent |
| M3 | Intervention | Unchanged scope; stated as lane 7 semantics over the M2 controls plus approve/deny, mark complete/skipped, stall flag |
| Dogfooding | From M1 onward (unspecified how) | Via attach the moment M1 lands; M2–M5 watched without hosting |
| Risks | — | Added: notifier coalescing/missed events (poll fallback, startup scan); `--tag` recoverability (confirmed in M0) |

### v0.4 → v0.5 (telemetry contract, 2026-10-22)

Source: owner message in `ai/session/2026-10-22-session.md` entry 17; decision D-010.

| Area | v0.4 | v0.5 |
|---|---|---|
| §11.4 endpoint and credential | Region placeholder; header from env or keychain | Fixed gateway `otlp-gateway-prod-eu-west-2`; header only from the keychain item `droid-observatory/otlp-headers`; missing item = telemetry off, shown in the UI |
| §11.4 labels | `feature`, `session`, `class`, `unit`, `price_table`, `status` among attributes | Only `mission`, `milestone`, `role`, `model`, `cause`; splits moved into metric names (`tokens.*`, `cost_usd`) |
| §11.4 instruments | Gauge `active_agents` | Counters and histograms only; `active_agents` not exported; `unknown_events` added |
| §11.4 budget | Not stated | Spec states the series count, under 200 (B-017) |
| Resource attributes | `deployment.environment`, `service.namespace` too | `service.name`, `service.version` only |
