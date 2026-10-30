//! A session file whose first line Droid rewrites longer, in place, after the tail has read
//! it (the live-attach defect of `droid exec` 0.247: `session_start` grows by 34 bytes). The
//! tail must read the new line from its start, not resume in the middle of it.
#![cfg(feature = "watch")]

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::{Collected, append, config, m0_lines, pump, replayed_ids, session_start};
use observatory_core::config::Config;
use observatory_core::ingest::watch::{Attach, LiveIngest};
use observatory_core::ingest::{Retraction, merge_ordered, retract_stale};
use observatory_core::model::{Event, EventKind, SessionId, SourcePtr};
use observatory_core::store::{EventScope, Store, TimeRange};
use tempfile::TempDir;

const SESSION: &str = "rewritten";
const POLL_MS: u64 = 50;
const LIMIT: Duration = Duration::from_secs(10);
/// How much longer the rewritten first line is (the growth seen on `droid exec` 0.247).
const GROWTH: usize = 34;
const LINES: usize = 40;
const LINES_BEFORE: usize = 4;

fn session_path(dir: &Path) -> PathBuf {
    dir.join("-home-user-project")
        .join(format!("{SESSION}.jsonl"))
}

/// The first line as Droid first writes it, then the same line with `GROWTH` more bytes.
fn first_lines() -> (Vec<u8>, Vec<u8>) {
    let short = session_start(SESSION, None);
    let mut line: serde_json::Value = serde_json::from_slice(&short).unwrap();
    line["title"] = "x".repeat(GROWTH - r#","title":"""#.len()).into();
    let mut long = line.to_string().into_bytes();
    long.push(b'\n');
    assert_eq!(long.len(), short.len() + GROWTH);
    (short, long)
}

/// The file after the rewrite: the longer first line, then the lines of the M0 root.
fn rewritten_file() -> Vec<u8> {
    let mut bytes = first_lines().1;
    for line in m0_lines(LINES).into_iter().skip(1) {
        bytes.extend(line);
    }
    bytes
}

/// The file as the tail first finds it: the short first line and the lines Droid wrote
/// straight after it, so events are out before the rewrite.
fn file_before_the_rewrite() -> Vec<u8> {
    let mut bytes = first_lines().0;
    for line in m0_lines(LINES_BEFORE).into_iter().skip(1) {
        bytes.extend(line);
    }
    bytes
}

/// What the board does with a batch: drop what it retracts, then add what is new.
fn fold(held: &mut Vec<Event>, retracted: &[Retraction], incoming: Vec<Event>) {
    retract_stale(held, retracted);
    merge_ordered(held, incoming);
}

fn unknown_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
        .count()
}

fn ids(events: &[Event]) -> Vec<String> {
    events.iter().map(|event| event.id.to_string()).collect()
}

#[test]
fn a_first_line_rewritten_longer_under_the_tail_yields_no_unknown_event() {
    let dir = TempDir::new().unwrap();
    let path = session_path(dir.path());
    append(&path, &first_lines().0);

    let mut attach = Attach::start(&config(dir.path(), POLL_MS)).unwrap();
    let mut collected = Collected::default();
    collected.absorb(attach.poll());
    assert_eq!(
        attach.watched().len(),
        1,
        "the tail has read the short line"
    );

    std::fs::write(&path, rewritten_file()).unwrap();
    pump(&mut attach, &mut collected, LIMIT, |attach, _| {
        attach.at_eof()
    });
    collected.events.extend(attach.finish());

    assert_eq!(
        unknown_count(&collected.events),
        0,
        "{:?}",
        collected.warnings
    );
    assert!(
        collected
            .events
            .iter()
            .all(|event| event.at.to_rfc3339_millis().as_str() >= "1971"),
        "no event is timed before 1971"
    );
    assert_eq!(ids(&collected.events), replayed_ids(&path, SESSION));
    assert!(collected.warnings.is_empty(), "{:?}", collected.warnings);
}

fn live_config(dir: &Path) -> Config {
    let mut cfg = Config::default();
    cfg.ingest.factory_sessions_dir = dir.to_string_lossy().into_owned();
    cfg.ingest.factory_logs_dir = dir.join("no-factory-logs").to_string_lossy().into_owned();
    cfg.ingest.jsonl_poll_ms = POLL_MS;
    cfg.ui.event_batch_ms = POLL_MS;
    cfg
}

#[test]
fn resync_does_not_duplicate_events_in_the_store_or_the_batches() {
    let dir = TempDir::new().unwrap();
    let store_dir = TempDir::new().unwrap();
    let store_path = store_dir.path().join("resync.sqlite");
    let path = session_path(dir.path());
    append(&path, &file_before_the_rewrite());

    let store = Store::open(&store_path).unwrap();
    let mut live = LiveIngest::start(&live_config(dir.path()), store).unwrap();
    let mut batches = Vec::new();
    let started = Instant::now();
    let mut discovered = false;
    while started.elapsed() < LIMIT && !(discovered && !batches.is_empty()) {
        let step = live.step();
        discovered |= !step.discovered.is_empty();
        batches.extend(step.batches);
    }
    assert!(!batches.is_empty(), "events were out before the rewrite");

    std::fs::write(&path, rewritten_file()).unwrap();
    let expected = replayed_ids(&path, SESSION);
    let mut board: Vec<Event> = Vec::new();
    let mut seen = 0;
    while started.elapsed() < LIMIT {
        let step = live.step();
        assert!(step.errors.is_empty(), "{:?}", step.errors);
        assert!(step.warnings.is_empty(), "{:?}", step.warnings);
        batches.extend(step.batches);
        board.clear();
        for batch in &batches {
            fold(&mut board, &batch.retracted, batch.events.clone());
        }
        seen = board.len();
        if seen >= expected.len() {
            break;
        }
    }
    batches.extend(live.finish());
    drop(live);
    assert_eq!(seen, expected.len(), "the board reached the replay");

    let batch_ids: Vec<String> = batches.iter().flat_map(|b| ids(&b.events)).collect();
    let distinct: BTreeSet<_> = batch_ids.iter().collect();
    assert_eq!(
        batch_ids.len(),
        distinct.len(),
        "an id is batched twice: {batch_ids:?}"
    );
    board.clear();
    for batch in &batches {
        fold(&mut board, &batch.retracted, batch.events.clone());
    }
    assert_eq!(
        ids(&board),
        expected,
        "the board after retractions is the replay"
    );
    assert_eq!(unknown_count(&board), 0);

    let store = Store::open_existing(&store_path).unwrap();
    let stored = store
        .events(
            &EventScope::Session(SessionId(SESSION.to_owned())),
            TimeRange::ALL,
        )
        .unwrap();
    let mut stored_ids = ids(&stored);
    stored_ids.sort();
    let mut expected_sorted = expected;
    expected_sorted.sort();
    assert_eq!(
        stored_ids, expected_sorted,
        "the store holds each event once, none stale"
    );
}

#[test]
fn the_first_line_event_pointer_follows_the_rewrite() {
    let dir = TempDir::new().unwrap();
    let store_dir = TempDir::new().unwrap();
    let store_path = store_dir.path().join("pointer.sqlite");
    let path = session_path(dir.path());
    append(&path, &file_before_the_rewrite());

    let store = Store::open(&store_path).unwrap();
    let mut live = LiveIngest::start(&live_config(dir.path()), store).unwrap();
    let mut batches = Vec::new();
    let started = Instant::now();
    while started.elapsed() < LIMIT && batches.is_empty() {
        batches.extend(live.step().batches);
    }
    assert!(!batches.is_empty(), "events were out before the rewrite");

    std::fs::write(&path, rewritten_file()).unwrap();
    let expected = replayed_ids(&path, SESSION);
    let mut board: Vec<Event> = Vec::new();
    while started.elapsed() < LIMIT {
        let step = live.step();
        assert!(step.errors.is_empty(), "{:?}", step.errors);
        batches.extend(step.batches);
        board.clear();
        for batch in &batches {
            fold(&mut board, &batch.retracted, batch.events.clone());
        }
        if board.len() >= expected.len() {
            break;
        }
    }
    batches.extend(live.finish());
    drop(live);

    let batch_ids: Vec<String> = batches.iter().flat_map(|b| ids(&b.events)).collect();
    let distinct: BTreeSet<_> = batch_ids.iter().collect();
    assert_eq!(batch_ids.len(), distinct.len(), "an id is batched twice");

    let file = std::fs::read(&path).unwrap();
    let first_line = file.split(|byte| *byte == b'\n').next().unwrap().len();
    assert_eq!(first_line, first_lines().0.len() - 1 + GROWTH);

    let store = Store::open_existing(&store_path).unwrap();
    let stored = store
        .events(
            &EventScope::Session(SessionId(SESSION.to_owned())),
            TimeRange::ALL,
        )
        .unwrap();
    let mut stored_ids = ids(&stored);
    stored_ids.sort();
    let mut expected_sorted = expected;
    expected_sorted.sort();
    assert_eq!(stored_ids, expected_sorted, "no id lost or stored twice");

    let mut at_zero = 0;
    for event in &stored {
        let SourcePtr::Jsonl {
            byte_offset,
            byte_len,
            ..
        } = &event.source
        else {
            panic!(
                "a session event without a jsonl pointer: {:?}",
                event.source
            );
        };
        let start = usize::try_from(*byte_offset).unwrap();
        let line = &file[start..start + usize::try_from(*byte_len).unwrap()];
        assert!(
            serde_json::from_slice::<serde_json::Value>(line).is_ok(),
            "the pointer of {} ({byte_offset}, {byte_len}) does not read back as JSON",
            event.id
        );
        if *byte_offset == 0 {
            at_zero += 1;
            assert_eq!(
                usize::try_from(*byte_len).unwrap(),
                first_line,
                "the first line event keeps its pre-rewrite length"
            );
            let pointer = store.pointer(&event.id).unwrap();
            assert_eq!(pointer, event.source);
        }
    }
    assert!(at_zero > 0, "the first line produced an event");
}

#[test]
fn a_rewrite_after_events_were_emitted_retracts_them_and_reads_the_file_again() {
    let dir = TempDir::new().unwrap();
    let path = session_path(dir.path());
    let mut attach = Attach::start(&config(dir.path(), POLL_MS)).unwrap();
    let mut collected = Collected::default();
    append(&path, &file_before_the_rewrite());
    pump(&mut attach, &mut collected, LIMIT, |attach, _| {
        attach.at_eof()
    });
    let mut board = std::mem::take(&mut collected.events);
    board.sort_by_cached_key(observatory_core::ingest::order_key);
    assert!(board.len() > 1, "events were out before the rewrite");

    std::fs::write(&path, rewritten_file()).unwrap();
    pump(&mut attach, &mut collected, LIMIT, |attach, _| {
        attach.at_eof()
    });
    collected.events.extend(attach.finish());

    assert!(collected.warnings.is_empty(), "{:?}", collected.warnings);
    let retracted: Vec<Retraction> = collected
        .retractions
        .iter()
        .map(|(_, r)| r.clone())
        .collect();
    assert_eq!(retracted.len(), 1, "{retracted:?}");
    assert_eq!(retracted[0].session_id, SessionId(SESSION.to_owned()));
    assert_eq!(
        retracted[0].below_offset,
        file_before_the_rewrite().len() as u64,
        "everything the tail had read"
    );
    let first_line_ids: BTreeSet<String> = ids(&board)
        .into_iter()
        .filter(|id| id.contains(":0:"))
        .collect();
    let again = ids(&collected.events);
    assert!(
        again.iter().all(|id| !first_line_ids.contains(id)),
        "an event of the unmoved first line is not emitted twice: {again:?}"
    );
    fold(&mut board, &retracted, collected.events);
    assert_eq!(ids(&board), replayed_ids(&path, SESSION));
}

#[test]
fn an_unfinished_line_that_changed_under_the_tail_is_read_again() {
    use observatory_core::ingest::{JsonlSource, Source};

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("unfinished.jsonl");
    std::fs::write(&path, "{\"a\":1}\n{\"b\"").unwrap();
    let mut source = JsonlSource::open(&path, &Default::default()).unwrap();
    assert_eq!(source.next_batch().unwrap().len(), 1);
    assert!(source.take_rewritten().is_none());

    std::fs::write(&path, "{\"a\":1}\n{\"c\":2}\n").unwrap();
    let lines: Vec<Vec<u8>> = source
        .next_batch()
        .unwrap()
        .into_iter()
        .map(|record| record.bytes)
        .collect();
    assert!(source.take_rewritten().is_some());
    assert!(source.take_rewritten().is_none(), "reported once");
    assert_eq!(lines, [b"{\"a\":1}".to_vec(), b"{\"c\":2}".to_vec()]);
}

#[test]
fn a_file_truncated_before_it_is_rewritten_is_read_from_its_start() {
    let dir = TempDir::new().unwrap();
    let path = session_path(dir.path());
    append(&path, &first_lines().0);
    let mut attach = Attach::start(&config(dir.path(), POLL_MS)).unwrap();
    let mut collected = Collected::default();
    collected.absorb(attach.poll());

    // Droid truncates the file before it writes the longer line; the tail can poll in between.
    std::fs::write(&path, b"").unwrap();
    collected.absorb(attach.poll());
    std::fs::write(&path, rewritten_file()).unwrap();
    pump(&mut attach, &mut collected, LIMIT, |attach, _| {
        attach.at_eof()
    });
    collected.events.extend(attach.finish());

    assert!(collected.warnings.is_empty(), "{:?}", collected.warnings);
    assert_eq!(unknown_count(&collected.events), 0);
    assert_eq!(ids(&collected.events), replayed_ids(&path, SESSION));
}
