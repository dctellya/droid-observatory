# D-024: Lanes are the seven mission phases; human and commander actions are overlays; the cycle ends at `handed_to_cd`

**Date:** 2026-10-23
**Status:** accepted and implemented in `m1-view-models-core` (mapping::v1 lane rules, `Lane`, `Overlay`, `HandedToCd`, store 0001 amended, golden regenerated; M0 golden SHA-256 `c10b27ae67be62c6ace2d38d445a035d5c219a13b9ed38812cc0f27cffce635d`, refrozen in SPEC §10 at the M1 integration, merge commit `c2fb42acf97883c1e1aa0458ebbc26fbc938bad2`: the golden was `25d91c53b580f8f5f3f1a62ecaa977a24dca0b61f5caf86e296182e8f23a5750`, 2394 events, and is `cecf727d0944ff7e8ddf306a8aeca6cbc32022bd2ff2eb7e22050fa7b72106c7` after D-032, refrozen at the M2 integration `7385b62`)

## Context

PRD §7 placed events on nine lanes by the event's source: Intent, Orchestrator, Workers,
Validators-scrutiny, Validators-user-testing, Integration, Human, Outside, and Time/Waste/Cost/models.
That is a map of who acts, not of where the work is. The owner re-scoped the product on 2026-10-23
(SPEC §9 F-028): Roci is a mission watcher that measures the waste and effectiveness (cycle time, cost)
of the engineering cycle inside a mission and across a run, from the spec to the pull request's checks
going green. Release and observe belong to the delivery platform; the two meet at the merge. The
engineering cycle is the phases a feature passes through, so the lanes should be those phases.

The timing is deliberate. Mapping v1 and its golden were frozen at the M0 integration (D-023,
`f66a9f5`), but nothing consumes the lane yet: no persistent store has been written (`replay` writes
none without `--store`), the board does not exist (the M1 UI is unwritten), and no metric reads a lane.
Re-opening the lane column now costs one regeneration of the golden; after M1 it would cost a mapping
version and a second fixture. D-023 stays the record of the kinds, pointers, roles and sidecar; this
file amends its lane column (the amendment lands in D-023 with `m1-view-models-core`).

## Decision

### 1. The seven phases, the overlays and the Panel

- **Lanes (one axis, owner order):** 1 Plan, 2 Contract, 3 Build, 4 Scrutiny, 5 User testing,
  6 Integrate, 7 Gate. Serde names: `plan`, `contract`, `build`, `scrutiny`, `user_testing`, `integrate`,
  `gate`. Every event keeps exactly one lane; the Panel is not a lane.
- **Roles are colour and attribution** on each event; they no longer pick the lane except as the
  fallback "home" of a role-neutral kind (Orchestrator → Plan, Worker and Unresolved → Build,
  ScrutinyValidator → Scrutiny, UserTestingValidator → User testing, Human → Plan, External → Gate;
  `Lane::home(role)`, renamed from `for_role`).
- **Overlays, not lanes:** human interventions and commander actions are markers on the event
  (`Event.overlay: Option<Overlay>`, `Overlay { Human, Commander }`, serialised only when present).
  `Commander` is declared with no producer: no commander role or run exists in any capture, so it is
  "defined, not observed" until one does.
- **The Panel** replaces the old lane 9: engineering cycle time per feature, milestone and mission
  with stage times per phase, waste (measured as 8 causes, displayed as 5 groups), cost per role, and
  drift against the declared models. The old lane 8 (Outside) folds into Gate.
- **`handed_to_cd { merge_sha, pr_number, attestation_ids }`** is the cycle's one final event, emitted
  once per PR when the required checks are green and `state = merged` is observed. It replaces
  `pr.merged_by_human`, `pr.merged_by_policy` and `deploy.completed`: 61 − 3 + 1 = 59 kinds, 30 observed
  on the M0 capture and 29 defined-not-observed (31 at the D-023 freeze). Its payload lives in the kind
  (like `Unknown { raw_type }`); the source is `SourcePtr::GitHub { url }` (the PR API URL); the role is
  `Human` when the merger's login equals the token owner, otherwise `External`.
- **Two clocks, named on the board and never confused.** Engineering cycle time is Roci's: the feature's
  `feature.defined` to the `handed_to_cd` of the PR that carries it (milestone and mission: the first
  `feature.defined` to `handed_to_cd`; `integration.merged` into the mission branch is an intermediate
  mark). Lead time to production is the delivery platform's: a label and a link, "delivery platform's
  clock, starts at `<merge_sha>`", named and computed nowhere in Roci.

### 2. The kind → lane rule, in evaluation order

This is the rule applied inside `mapping::v1::Rec::push`; the table in §3 is its authoritative form
(copied from `architecture.md` §5 and SPEC §5.2).

**Lane rule:** lane is a function of `(kind, role, session
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
timestamp" needs cross-session state the stateless `map_record` cannot see; it is a v2 note here, not work.

### 3. The authoritative v1 lane table

**Authoritative v1 lane table (D-024).** Columns: PRD lane before F-028; phase; overlay; count on the
M0 golden expected at the M1 regeneration (`jq` over `fixtures/streams/2026-10-22-observatory-v1-m0.events.json`,
1279 events; the committed M1 golden is authoritative); reason. `R` = role home (rule 5 of §2, subject to rules 3–4).

| Kind | PRD lane | Phase | Overlay | M0 | Reason |
|---|---|---|---|---|---|
| intent.created | 1 Intent | 1 Plan fixed | Human | 1 | the mission prompt; role stays Orchestrator (D-023) |
| intent.amended | 1 Intent | 1 Plan fixed | Human | 44 | typed root prompt |
| question.asked | 1 Intent | 1 Plan fixed | Human | 13 | pulls the human in; starts waiting-on-human |
| intent.answered | 1 Intent | 1 Plan fixed | Human | 12 | the human's answer |
| spec.versioned | 1 Intent | 2 Contract fixed | none | 0 | a spec version is the contract's artefact |
| plan.created | 2 Orch | 1 Plan fixed | none | 1 | decomposition |
| feature.defined | 2 Orch | 1 Plan fixed | none | 6 | cycle start; all 6 on M0 come from the scrutiny validator session `6eb88099` at 17:35:44Z, none for `m0-*` (see the clocks in §1); D-032: from a scrutiny or user-testing validator, that validator's phase |
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

### 4. Owner answers (2026-10-23)

| # | Question | Answer |
|---|---|---|
| 1a | Which events land in Contract? | Contract sessions are children spawned by a `Task` whose description matches `^Contract\b\|[Vv]alidation contract`. A v1 heuristic, recorded here and in the mapping decision. |
| 1b | How is the merger recorded? | One `handed_to_cd`, the merger attributed by role (human or external). `pr.merged_by_policy` is not kept beside it. |
| 1c | What does M4 cut? | `deploy.completed` and branch-deletion detection. The `demo/*` cleanup stays a `gh` step of the run, not an observed event. |
| 1d | Where do human-overlay events land? | In a fixed lane by kind in v1: `intent.*`, `question.*`, `approval.*` → Plan; `permission.*` → the asking session's phase; `review.outcome` → Gate; `approval.given` with `human_actions.kind = merge` → Gate. Placing them by the feature's phase at that instant is a v2 note (BACKLOG). |
| 1e | When does the engineering cycle start? | `feature.defined`, else the feature's first `task.started`, with the count of fallbacks reported. On the M0 capture only 6 of 13 features have a `feature.defined`, all emitted at 17:35:44Z by the scrutiny validator session `6eb88099`; none of the runner-started `m0-*` features has one.; D-032: from a scrutiny or user-testing validator, that validator's phase |

### 5. Implementation note (`m1-view-models-core`, 2026-10-23)

- `EventKind::fixed_lane()` is the phase a kind has whoever emits it (`None` for the role-neutral
  kinds); `EventKind::lane(role)` adds rule 2 (orchestrator `commit.created` → Integrate) and rule 5;
  `EventKind::overlay()` is `Some(Human)` for the 13 human-facing kinds. The mapping applies rules 3
  and 4 on top (`mapping::v1::lane_of`), never to fixed kinds and never to `Unknown`.
- The integration session is told by the session's own feature id, which is known only after its
  first prompt; the held `session_start` event (`subagent.spawned`) is placed when it is released.
- `SessionMeta.contract` is set by `SessionTree` from the parent's `Task` `description`
  (`mapping::is_contract_task`).
- Result on the M0 golden (`jq` over the committed file; after D-032, 443 / 87 before it): Plan 437, Contract 69, Build 424,
  Scrutiny 93, User testing 122, Integrate 134, Gate 0 (sum 1279); 70 `overlay: human`; 57 lane strings
  unchanged; event count, kinds, ids, offsets, `at` and order unchanged.

## Alternatives considered

- **A `mapping::v2` for the phase lanes, v1 kept frozen.** Rejected: a mapping version belongs to a Droid
  release (SPEC §5.2), not to a product re-scope. v2 would need its own fixture and golden to prove
  itself, and v1 would stay the version selected for the 0.247 captures, so the board would still need
  the phase table. With no consumer of lanes yet, one regeneration of the golden at M1 is cheaper and
  leaves a single mapping to maintain.
- **Keep nine lanes and add a "phase" column.** Rejected: two axes for one fact, and the board would
  keep a Human lane and an Outside lane that the owner's cycle does not have.
- **Contract left empty on M0 until a capture has a contract-writer role (1a).** Rejected by the owner:
  no PRD kind and no role identifies contract writing, so Contract would be empty on the only capture
  there is. Without the heuristic the counts are (after D-032) Plan 437, Build 493, Scrutiny 93, User testing 122,
  Integrate 134, Contract 0.
- **`pr.merged_by_policy` kept as a second Gate kind (1b).** Rejected: the distinction is who merged,
  which role attribution already carries, and a second kind doubles the final-event paths.
- **Branch-deletion detection as the PR's closing housekeeping (1c).** Rejected: it happens after the
  merge, which is out of scope.
- **Human events placed in the phase their feature is in at that instant (1d).** Deferred to v2: it
  needs cross-session state the stateless `map_record` cannot see and would make a human event's lane
  depend on replay order. A v2 note, recorded in BACKLOG; not work in v1.
- **A `feature.defined` ids rule amended before the reference CSV (1e).** Rejected for v1: the fallback
  to `task.started` is a metrics rule, not a mapping change, and keeps the frozen ids.

## Consequences

- **Golden regeneration and hash refreeze at M1.** Step 0 of `m1-view-models-core` writes the red tests
  first, then regenerates `fixtures/streams/2026-10-22-observatory-v1-m0.events.json` with
  `OBSERVATORY_UPDATE_GOLDEN=1 cargo test -p observatory-core --test mapping` and a reviewer read. Serde
  names `scrutiny` and `user_testing` are unchanged, so 57 of the 1279 golden events keep a byte-identical
  `lane`; 1222 change string; 70 gain `"overlay":"human"`; ids, offsets, `at` and order are untouched.
  The old golden hash `f055657a46af2713f50d1963dab684495d5f3770225f942adfcbb28e8331f3e1` is superseded
  at the M1 integration and the new SHA-256 goes to SPEC §10; the fixture `.jsonl`, `.capture.json` and
  `.settings.json` hashes stay.
- **Expected phase counts on the M0 golden** (`jq` of the committed golden is authoritative; after D-032): Plan 437,
  Contract 69, Build 424, Scrutiny 93, User testing 122, Integrate 134, Gate 0 (sum 1279); 70 human
  overlays (1 created + 44 amended + 12 answered + 13 asked). Gate holds nothing on a session capture;
  until M4 the Gate header card shows `gate: pending (no PR)`, the mission branch, the last
  `gate_command.*` verdict from Integrate and the two clock labels, with no value.
- **Store.** `0001_init.sql` is amended in place: `lane INTEGER NOT NULL CHECK (lane BETWEEN 1 AND 7)`
  and a nullable `overlay TEXT CHECK (overlay IN ('human','commander'))` after `lane` (22 → 23 columns).
  SQLite cannot alter a CHECK, so a `0002` would be a table rebuild for zero users; forward-only
  migrations start with the first shipped store.
- **Tests renamed or added** (M1 Step 0): `lanes_are_nine_in_prd_order` → `lanes_are_the_seven_phases_in_order`;
  `empty_board_has_nine_lanes_and_no_cards` → `empty_board_has_seven_lanes_a_panel_and_no_cards`;
  `prd_names_are_unique_and_every_kind_has_a_lane` drops its `TimeWasteCost` clause; new:
  `role_neutral_kinds_take_the_role_home_phase`, `gate_command_lands_in_integrate_for_every_role`,
  `orchestrator_commits_land_in_integrate`, `integration_session_events_land_in_integrate`,
  `contract_session_events_land_in_contract`, `human_facing_kinds_carry_the_human_overlay`,
  `handed_to_cd_carries_merge_sha_pr_number_and_attestation_ids`.
- **Frozen lists.** D-023's defined-not-observed kinds (29 after this decision) and the `Task` input
  fields for roles and ids (`subagent_type`, `description`) are frozen as of M0 (F-031).
- **Scope moves.** M4 keeps PR, checks and the fix loop, ends at `handed_to_cd` and drops
  `deploy.completed`, branch-deletion detection and the two merge kinds; M5 gains the Panel, the cycle
  time redefinition, the 8 → 5 waste display and both clocks. Re-caps in SPEC §2 (F-031).
- **Waste.** Measured as 8 causes (SPEC §5.6, the CSV, the D-010 series: 182 worst case), displayed as 5
  groups: waiting on human (question, permission, merge), waiting on validation, rework, retry, idle
  (queued, unscheduled). The owner's reference CSV and the oracle are unchanged.
- **Post-merge is out of scope.** Deploys, flags and fix-forward Issues are the delivery platform's;
  Roci's last event is `handed_to_cd`. The final PR of this mission has its own `handed_to_cd` after the
  mission, when the owner merges it (BACKLOG).

## Links

- [D-023](D-023-mapping-v1.md): mapping v1 and the frozen kinds, roles and sidecar; this decision amends its lane column.
- [D-020](D-020-pii-in-fixtures-is-redacted-at-capture.md): the fixture hash tables; the golden hash is
  refrozen at the M1 integration, the fixture hashes stay.
- SPEC §1, §3, §5.2, §9 F-028, §10; `architecture.md` §5.

## Addendum 2026-10-25 (m3 integration, m3-fix-ut-core-naming-and-docs)

the seven phases are now the default cycle template `config/cycles/engineering.toml` (F-041, D-035); the `Lane` enum is history.
