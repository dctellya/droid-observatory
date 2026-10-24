# D-031: A session file rewritten under the tail is read again and its stale events are retracted

**Date:** 2026-10-24
**Status:** accepted and implemented in `m2-fix-rewritten-line-resync`

## Context

`droid exec` 0.247 rewrites a live session file in place. Sampled every 2 ms in the live probe: the file
is 245 bytes (the `session_start` line), grows to 7665 bytes by two appended lines, then the first line
becomes 34 bytes longer (278 bytes plus its newline) and every later byte moves by 34. Sometimes the file is
truncated first, so the tail polls an empty file. The tail resumed at its stored offset, in the middle of the new
first line, and recorded the 33-byte rest as an `unknown.event` timed 1970 (VAL-BOARD-037). When events were already out (the
two appended lines produce `intent.created`), the length check raised `FileMoved` and the session stopped
being followed. Event ids are `<session>:<byte_offset>:<sub>`, so after the move the same lines have
other ids and the events handed out before point into bytes that are gone.

## Decision

1. **Detection** (`JsonlSource`): before each read the tail checks the bytes just before its offset: the
   newline that ended the last line and any unfinished line it holds back. If they differ, the file was
   rewritten under it. `length < offset` is the same event (truncate first). The tail restarts at offset 0.
2. **Replace, not skip** (`Attach`): the session's `Mapper` restarts (`Mapper::restart`) and the whole file is
   mapped again. The events handed out before are retracted: `Retraction { session_id, below_offset }` covers
   the events of lines after the first that start before the offset the tail had reached.
3. **The first line keeps its ids.** It is the line rewritten in place, so offset 0 does not move. The tail
   remembers the ids of the first-line events it handed out and does not hand them out again. No id leaves
   the ingest twice. The line is longer after the rewrite, so the tail hands the events read again to
   `AttachBatch.repointed` and `LiveIngest::step` refreshes their stored pointer with `Store::update_pointer`
   (`byte_len` and `path` change, the id does not).
4. **Store:** `Store::retract_events(session, below_offset)` deletes the covered events and their usage rows
   in one transaction, before the new events are inserted. `insert_events` stays `ON CONFLICT (id) DO NOTHING`.
5. **Batcher:** `EventBatcher::retract` removes the covered events still pending and adds the retraction to the
   batch; `EventBatch.retracted` (serde default, so older payloads still parse) tells the UI to drop what it
   already holds. `ingest::retract_stale` applies it, then `merge_ordered` (dedup by id) adds the new events;
   the UI capture does exactly that.
6. Nothing is dropped silently (rule 18): the re-read line is mapped like any other, so a line the rules do
   not know is still an `unknown.event`; the fragment case no longer arises.

## Alternatives

- **Check only the byte at `offset - 1` and re-sync from 0, skipping events whose id was already emitted.**
  Smallest change, and enough when only the first line was read. Rejected: with lines after the first
  already mapped, every later id changes, so the skip rule leaves the old ids in the store and the board and
  adds the new ones (each event twice, with pointers into moved bytes).
- **Dedup by the message `id` field instead of the byte offset.** Not every line has one, and ids derived
  from the pointer are the contract of `inspect`.
- **Report the session as `FileMoved` and stop following it** (the previous behaviour, kept for events out).
  Rejected: a normal `droid exec` run hits this, and the board would lose the live session.
- **Resume at the next line boundary after the stored offset.** Rejected: it guesses the shift. A rewrite that
  moves bytes by an amount other than the length of the first line drops or splits a line.

## Consequences

- A live attach of a real `droid exec` session lists the same ids and kinds as `observatory-cli replay
  --root-only` of the finished file, with `unknown events: 0`.
- Every poll reads a few more bytes (the last newline and the held-back line) to verify the prefix.
- The tail keeps the ids of the first-line events (a handful) for the session's life.
- The stored pointer of a first-line event follows the rewrite, so `inspect` and Details read the whole new
  line, not the first bytes of it (VAL-BOARD-038). The store schema is unchanged.
- A retraction costs the UI one pass over its held events.
- Usage-log events (`turn.usage`) are not file-based and are never retracted.
