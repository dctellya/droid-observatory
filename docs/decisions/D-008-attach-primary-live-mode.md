# D-008: Attach is the primary live mode

**Date:** 2026-10-22
**Status:** accepted

## Context

Missions are started from the terminal in practice (`droid exec --mission ...` in a
project's directory). Every session, including the child sessions of workers and
validators, is written as it runs to `~/.factory/sessions/<cwd-slug>/<id>.jsonl`. PRD v0.3
treated hosting (the app spawns `droid exec`) as the live mode and attach as an afterthought,
which made a launch screen a prerequisite for watching anything live (M2) and left the
dogfooding statement ("from M1 onward") without a mechanism.

## Decision

Attach is the primary live mode. The observatory watches `~/.factory/sessions/` with a
filesystem notifier, detects any new session JSONL, tails it as it grows and renders it
within 1 s, named by its `--tag` or session id. Replay is attach at end-of-file: one adapter
(`jsonl_watch` + `jsonl_tail`), one code path. Hosting remains as the secondary path and the
only mode with a control channel (pause, message, answer over the JSON-RPC input stream).

Milestones: attach moves into M1 with replay; M1 acceptance adds "start a mission in a
terminal, see its events on the board within 1 s". M2 becomes hosting only. The agent panel
and the `model.changed` / `context.compacted` markers move to M1 because they are read-only
views over the same events. PRD v0.4, §1, §9.7, §11.1, §11.2, §13, §14, §15.

## Alternatives

- Host first (v0.3): every observed mission must be launched from the app; nothing can be
  watched until the launch screen, process management and RPC plumbing exist.
- Attach by explicit file pick only, no watcher: works, but the user has to find the session
  id for each mission and misses child sessions.

## Consequences

- Dogfooding starts the moment M1 lands: M2–M5 of the mission that builds the observatory
  are watched by attaching to its own session files.
- New risk rows: notifier coalescing or missed events (paired with a poll at
  `[ingest].jsonl_poll_ms` and a startup scan of the directory); `--tag` recoverability
  from the session files (confirmed in M0).
- `config/observatory.example.toml` `[ingest]` gains `watch_sessions_dir`.
- The Leptos go/no-go (D-002) is unchanged: it judges the board rendering, not ingest.
