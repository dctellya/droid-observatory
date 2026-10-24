# D-032: A `feature.defined` emitted by a validator session sits in that validator's phase

**Date:** 2026-10-24
**Status:** accepted and implemented in `m2-mapping-scrutiny-feature-defined` (owner decision 31, a `mapping::v1` addendum, `mapping_version` stays `v1`); merged at the M2 integration `7385b62`, golden refrozen `cecf727d…`

## Context

`mapping::v1` (D-023, D-024) gives `feature.defined` the fixed phase Plan: the kind marks the start of the
engineering cycle (D-024 rule 1e) and belongs to the orchestrator's planning. The rule is applied by the
kind, whoever emits it.

The M0 capture shows the cost of that. The kind is produced when a `Task` call names a feature for the first
time in a session (`mapping/v1/mod.rs`, `Tool::Task`). All six `feature.defined` events of the capture come
from the scrutiny validator session `6eb88099-278d-4ddb-9f5d-23f1255cdbc5` at 17:35:44Z, when it spawns
reviewers for the features it checks. That validator defines nothing: it names features it reviews. The golden
placed the six in Plan, so the Plan lane carried six events of a session whose every other event sits in
Scrutiny (BACKLOG B-042). The owner decided on 2026-10-24 that the lane follows the emitting session.

Counts on the committed M0 golden (`jq`, before the change): `feature_defined` 6, of which role
`scrutiny_validator` 6, `user_testing_validator` 0, `orchestrator` 0, `worker` 0; the S capture has none.

## Decision

1. The lane of a `feature.defined` follows the role of the session that emits it:
   scrutiny validator → Scrutiny; user-testing validator → User testing; orchestrator, worker and every other
   role → Plan, unchanged. The rule is one `match` at the top of `lane_of` in `mapping::v1`; `EventKind`
   keeps `Plan` as the kind's own phase (`EventKind::lane` and `fixed_lane` are untouched), so the model
   table and its tests do not change and the exception lives next to the other session-dependent rules.
2. The user-testing branch has no case in the fixtures (count 0). It is covered by the hand-made test
   `a_feature_defined_follows_the_phase_of_the_session_that_emits_it`, which maps one `Task` call in a session
   of each role.
3. The `mapping_version` stays `v1`. The change moves a lane string, not a kind, an id, a pointer or a
   timestamp, and there is no new Droid release behind it (the argument of D-024 against a `v2`).
4. The golden `fixtures/streams/2026-10-22-observatory-v1-m0.events.json` is regenerated with
   `OBSERVATORY_UPDATE_GOLDEN=1 cargo test -p observatory-core --test mapping`. The fixture `.jsonl`, the
   `.capture.json`, the `.settings.json`, the usage-log extract and the `.children/` files keep their hashes.

## Alternatives considered

- **Keep Plan, as "where the feature was conceived".** Rejected by the owner. It reads the kind's name
  instead of the board: a lane shows what the session that emitted the event is doing, and a validator
  that names a feature in a `Task` description is reviewing it. Plan kept six events of a session that has no
  other Plan activity, and a Plan card for a scrutiny-only feature would look planned, not reviewed. The
  cycle clock is not affected either way: it reads the kind, not the lane.
- **Use `Lane::home(role)` for every role's `feature.defined`.** Rejected: it is a different rule, not a
  generalisation of this one. `Lane::home(Role::Worker)` is `Build` (`model/lane.rs`), so the kind would
  split across Build, Scrutiny and User testing by role where this decision keeps one exception: the two
  validator roles follow their phase, every other role stays in Plan. `home` is documented as the fallback
  "when nothing else fixes the event's phase", and `feature.defined` fixes one: Plan, the start of the
  cycle (D-024 rule 1e); routing it through the role fallback discards the kind's own phase. The M0 capture
  has no worker or user-testing `feature.defined` (counts above), so the two rules agree on the golden, but
  the hand-made role test pins the difference: under `Lane::home` its worker case would sit in Build, and
  any worker `feature.defined` in a later capture would move between Build and Plan, changing the per-lane
  totals the golden and the lane-total tests pin (Plan 437, Build 424, Scrutiny 93). The owner's decision
  covers validator sessions only.
- **A Build sub-lane (or a "validator-defined" sub-lane under Plan).** Rejected: Build is the workers'
  phase (one sub-lane per worker session, VAL-BOARD-006) and the validator is not a worker; a sub-lane under
  Plan keeps the events on the wrong axis and adds a view concept for six events. Lanes are the seven phases of
  D-024 and sub-lanes are a Build-only device.
- **Drop the `feature.defined` of validators (no event).** Rejected: the Task call is understood and the
  event carries the feature id and the clock start fallback; dropping a recognised shape is not what rule 18
  allows for data that exists.
- **A `mapping::v2`.** Rejected for the reason in D-024: a version bump is for a new Droid release, and v2
  would need its own fixture and golden for a one-string change.

## Consequences

- The golden changes in exactly six elements, each by `lane` `plan` → `scrutiny` (`jq` comparison of the old
  and the new file with `del(.lane)`: 0 elements differ). Ids, offsets, `at`, order, the 35/34/1 gate counts and
  `tests.run` 83 are unchanged; the file length is still 2394 elements, unknown 0. Per-lane totals of the
  replay (without `turn.usage`): Plan 443 → 437, Scrutiny 87 → 93, the other lanes unchanged, sum 1279. Over
  the whole golden (with `turn.usage`): Plan 750 → 744, Scrutiny 151 → 157.
- The golden sha256 changes from `25d91c53b580f8f5f3f1a62ecaa977a24dca0b61f5caf86e296182e8f23a5750` to
  `cecf727d0944ff7e8ddf306a8aeca6cbc32022bd2ff2eb7e22050fa7b72106c7`. SPEC §10 is refrozen by
  `m3-integrate-m2`; the fixture `.jsonl`, `.capture.json` and `.settings.json` hashes are not touched.
- Tests that pinned the lane totals move with it: `m0_events_fall_into_the_seven_phases_with_human_overlays`,
  the board and turn-usage view tests and the store lane-count test (443/87 → 437/93).
- VAL-BOARD-010 (role filter, per lane) and VAL-BOARD-004 (lane counts) are re-run against the new golden;
  VAL-MAP-027 asserts the six events in Scrutiny and the unchanged rest.
- The `feature.defined` row of D-023's kinds table and of D-024's kinds table states Plan without the
  exception; the integrator adds the reference to this decision (shared files, `m3-integrate-m2`). The
  rule table in `mapping/v1/kinds.rs` already carries it.
