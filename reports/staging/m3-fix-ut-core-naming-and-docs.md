# Staging: m3-fix-ut-core-naming-and-docs

Feature `m3-fix-ut-core-naming-and-docs` (fix 1 of 3 after M3 user-testing round 1), worktree `observatory-wt/m3`,
branch `wt/m3`. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

none

## Decisions

- D-024 (amend): the seven phases are now the default cycle template `config/cycles/engineering.toml` (F-041, D-035); the `Lane` enum is history.
- D-032 (amend): the Scrutiny rule for `feature.defined` is the template rule `kind_follows_emitter` (D-035).
- D-036 (amend): runner tag list gains `mission-orchestrator` (Droid 0.249 writes it first on an orchestrator session, before `exec`), in `IngestConfig::default()` and `config/observatory.example.toml`; a live orchestrator is named by its user tag again and a rename to `mission-orchestrator` is refused.

## BACKLOG

- B-NEW: `control::dedupe_paused` has no production caller; a hosted pause stores two `mission.paused` rows (the ack-derived `…:1073741824` from `host.rs` `control::ack_event` and the stream's own), so the board and Details show both. Options: a view-time filter in `view/board.rs` fold and `details` (drop the ack row when a non-ack `mission.paused` follows before `mission.resumed`, about 40 LOC plus a red view test) or a host-side retract of the ack event when the stream line arrives (`Store::retract_events`). It cannot be applied at write time because the ack arrives before the stream line. Orchestrator decision: backlog for M4. Reason: found at M3 user testing, no contract assertion covers it, and the choice between the two options is a small design decision.
- B-NEW: the offline spool replay stamps three events with the replay wall time instead of the recorded time (counts are unaffected). Reason: found at M3 user testing; no M3 assertion covers it and counts are unaffected.
- B-NEW: the Library row of a hosted scratch mission shows the mission id (`0f91442d`) instead of its user tag `scratch`. Reason: found at M3 user testing; cosmetic, no M3 assertion covers it, and the name rule for hosted missions belongs with the M4 Library work.

## Deferred clauses

none

## Library

- Droid 0.249 writes the runner tag `mission-orchestrator` before `exec` on an orchestrator session's sidecar (tags `[mission-orchestrator, exec, <user tag>, mission-session{role:orchestrator}]`); 0.247 captures (the M0 fixture) never carry it. The M0 store holds 0 sessions tagged with any runner tag, `mission-orchestrator` included (replay of `2026-10-22-observatory-v1-m0.jsonl` into a temporary store, 56 sessions).
