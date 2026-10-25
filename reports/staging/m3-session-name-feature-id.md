# Staging: m3-session-name-feature-id

Feature `m3-session-name-feature-id` (F-038, B-041), worktree `observatory-wt/m3`, branch `wt/m3`.

## SPEC §9

- F-038 status: decided; code landed at `m3-session-name-feature-id` (D-036). Name rule: feature id, else user tag, else
  session id. The feature id is read from the user messages before the first assistant message
  (`assigned to execute feature "<id>"`). M0 capture: 15 of 56 sessions have one. Golden `.events.json` unchanged
  (sha256 `cecf727d0944ff7e8ddf306a8aeca6cbc32022bd2ff2eb7e22050fa7b72106c7`; events carry no session name).

## Decisions

- D-036 (`docs/decisions/D-036-session-name-feature-id.md`) is new; `docs/decisions/README.md` was regenerated here, the
  integrator re-runs `make decisions-index` after merging the other M3 decision files.

## BACKLOG

- B-041: done (this feature).
- B-NEW: VAL-BOARD-039/040 say `sessions.name`; the store column is `sessions.tag` and holds the display name since M1.
  Reason: no migration was written for a rename; validators query `SELECT id, tag AS name FROM sessions`, or the contract
  wording changes to `tag`.
- B-NEW: the live store (`LiveIngest`) writes no `sessions` rows, so VAL-BOARD-040's `SELECT ... FROM sessions` for an
  attached session has nothing to read. Reason: only `record_tree` (replay) and the hosted recorder write rows.
- B-NEW: `Attach::watched` re-reads each file head every poll to recompute names; cache a found feature id per tail if
  the 64-session watch shows cost. Reason: measured nothing yet.

## Deferred clauses

- VAL-BOARD-040 (live `droid exec --tag` and a runner-started worker, tauri-smoke) was not run: it needs a scratch
  mission. Core behaviour is covered by fixture and temp-directory tests; the live rename path is the existing
  `announce_changes` name poll. This session's own file carries the sentence at record 3.

## Library

- `library/droid-sessions.md`: section "Where a worker session's feature id is recoverable" (field path, example, oracle).
- Evidence screenshots: `evidence/m3-session-name-feature-id/*.png` in the mission directory.
