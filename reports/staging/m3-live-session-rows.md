# Staging: m3-live-session-rows

Feature `m3-live-session-rows` (fix after `m3-session-name-feature-id`, VAL-BOARD-040), worktree `observatory-wt/m3`,
branch `wt/m3`.

## SPEC §9

- VAL-BOARD-040 store clause: a live-attached session has a `sessions` row from discovery (`record_tree` already ran on
  every new tree; a characterization test pins it) and its `tag` (the display name, D-036) follows the name:
  `LiveIngest::announce_changes` writes it through `Store::set_session_name`. The earlier staging row "the live store
  writes no sessions rows" was wrong about discovery and right about renames; this feature closes the rename gap.
- Name policy (`watch::SessionNames`): the first name seen is kept, one change is accepted, a second change or a runner
  tag (`[ingest].runner_tags`) is refused, each refusal reported once. `LiveStep.renamed` / `LiveStep.refused` carry the
  entries; the host logs `session_renamed` (old, new, at) at INFO and `session_rename_refused` (kept, attempted, at) at
  WARN next to `session_discovered`.
- After `record_tree` (a later session joined the tree) `LiveIngest` writes back the names it holds, so a refused change
  never reaches the store.
- No migration: `sessions.tag` already exists. Golden `.events.json` unchanged (events carry no session name).

## Decisions

- D-036 consequence 3 amended in words only (no file edit here): the second `session_discovered` for a renamed session is
  accompanied by `session_renamed`; a later change is refused, not announced.

## BACKLOG

- B-NEW: cache the found feature id per tail in `Attach::watched` (re-read on every poll today). Reason: `session_name`
  reads the sidecar and the file head of every watched session each `jsonl_poll_ms`; a found feature id never changes,
  so it can be kept once read (no measurement taken yet).

## Deferred clauses

- VAL-BOARD-040 (a) and (b) as tauri-smoke runs (a real `droid exec --tag` and a runner-started worker) were not run in
  this feature; the core behaviour is covered by `crates/observatory-core/tests/live_session_rows.rs` over a temp
  sessions directory with a real `LiveIngest` and store. `inspect` has no name listing; validators read
  `SELECT id, tag AS name FROM sessions` with `sqlite3 'file:<db>?immutable=1'` as for VAL-BOARD-037.

## Library

- `library/attach-watcher.md`: add "A watched session's name is decided by `SessionNames` (once, never a runner tag); the
  store `tag` follows it; log lines `session_renamed` and `session_rename_refused`".
