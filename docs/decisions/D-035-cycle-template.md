# D-035: The cycle is a template read at start-up, not a `Lane` enum

**Date:** 2026-10-25
**Status:** accepted and implemented in `m3-cycle-template-refactor` (`model/cycle.rs`, `config/cycles/engineering.toml`, migration `0002_lane_text.sql`)

## Context

F-041 (owner item 32): the cycle is data, not code. D-024 fixed seven lanes as a Rust enum (`Lane::ALL: [Lane; 7]`)
and every consumer indexed it: the `event_kinds!` lane column, `lane_of` in mapping v1, the board digest
(`[u64; 7]`), `BoardState::lane_counts`, the card tracker, the CLI lane text and the store column
(`lane INTEGER CHECK (lane BETWEEN 1 AND 7)`). A team whose cycle is not the engineering one (four steps, other
names) needed a code change in about 36 files. The same pass records the owner's item 12: the role that runs a
cycle is the `officer`, not the `commander`.

## Decision

1. `model/cycle.rs` holds `CycleTemplate { name, lanes: Vec<LaneSpec>, final_event, clocks, rules }`,
   `LaneSpec { id: LaneId, name, owner_role, home_for, kinds, grouping, clocks }` and `LaneId(String)`.
   The default is `config/cycles/engineering.toml`, embedded with `include_str!`: ids `plan, contract, build,
   scrutiny, user_testing, integrate, gate`, final event `handed_to_cd`. `[cycle].template` in the config is
   `engineering` or a file path; a missing or invalid file falls back to `engineering` with a warning that names
   the file. `from_toml` validates (unique ids, known kinds, a kind in one lane, every role at home in one lane,
   the final event in a lane).
2. Kinds belong to lanes by the template (`LaneSpec.kinds`), not by a column of `event_kinds!`. The session facts of
   D-024 rules 2 to 4 and D-032 are `[rules]` of the template (`integration_session`, `contract_session`,
   `orchestrator_commit`, `kind_follows_emitter`) as lane ids. D-032 is kept as decided: a kind in
   `kind_follows_emitter` follows its emitter only for validator roles (`Role::is_validator`).
3. Mapping v1, `BoardView`, `BoardDigest` (`lanes`, `lane_events`, `lane_cards` are vectors in template order),
   `BoardState`, `Card`, the scrubber counts, the gate placeholder (clock labels from the final lane's `clocks`), the
   new `view::post_mortem_input` (one summary per lane, one reading per clock), the CLI and the Leptos UI iterate
   `template.lanes`. `rg 'Lane::' crates apps` returns no hit; lane names live in `model/cycle.rs` tests and
   `config/cycles/engineering.toml` only.
4. Migration `0002_lane_text.sql` rebuilds `events` with `lane TEXT NOT NULL`, converting 1..7 to the engineering
   ids, and `rework_windows.cause` to free text (`integration` becomes `integrate`). `0001_init.sql` is untouched.
   The runner turns foreign keys off around a migration (SQLite ignores the pragma inside a transaction) and runs
   `foreign_key_check` before it commits. A read-only open of an unmigrated store shows the old number as an
   unknown id instead of failing.
5. Officer (owner item 12): `Overlay::Commander` becomes `Overlay::Officer`. Intake reads `commander` as `officer`
   for M3 (serde alias, `Overlay::parse`, the migration's `CASE`); only `officer` is written; the store CHECK is
   `('human','officer')`. The alias is removed in the next milestone after M3.
6. Evidence: the M0 golden is byte-identical (`sha256 cecf727d0944ff7e8ddf306a8aeca6cbc32022bd2ff2eb7e22050fa7b72106c7`,
   2394 events); the `four_step` suite renders `BoardView`, `BoardDigest`, `Panel` and the post-mortem input of a
   four-lane template with other names through `core::view` with no code change; a default-template guard pins the
   seven headers in order.

## Alternatives

- **Keep the enum with a config-only display-name map.** Rejected: names change, lane count, order, owners and the
  final event do not, so a four-step cycle still needs code in every `match` over `Lane`.
- **Keep the INTEGER position column.** Rejected: a position is meaningless once the template can reorder or
  shorten the lanes; stored rows would silently change lane with the template. The id is stable, the position is
  not. Cost accepted: one table rebuild.
- **Fixed-size arrays sized by a const generic.** Rejected: the lane count is known at start-up only.

## Consequences

- A new cycle is a TOML file; adding a lane name to code is a defect (`AGENTS.md` Conventions).
- The UI cannot read a file: the shell sends the template (`cycle_template` command); the browser host keeps the
  compiled-in one.
- `rework_windows.cause` now holds a lane id; M5 writes ids, not the four legacy words.
- Replay, the board and live attach use the configured template; `replay_files` (the browser file picker) and
  `Store::replay_file` keep the engineering default and have `_with` variants for a given template.
- The measurement gap of AGENTS item 33 is unchanged and stays a discovered issue (no new measurement here).
