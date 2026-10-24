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
use observatory_core::model::{Event, EventKind, SessionId};
use observatory_core::store::{EventScope, Store, TimeRange};
use tempfile::TempDir;

const SESSION: &str = "rewritten";
const POLL_MS: u64 = 50;
const LIMIT: Duration = Duration::from_secs(10);
/// How much longer the rewritten first line is (the growth seen on `droid exec` 0.247).
const GROWTH: usize = 34;
const LINES: usize = 40;

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
    append(&path, &first_lines().0);

    let store = Store::open(&store_path).unwrap();
    let mut live = LiveIngest::start(&live_config(dir.path()), store).unwrap();
    let mut batched: Vec<Event> = Vec::new();
    let started = Instant::now();
    let mut discovered = false;
    while !discovered && started.elapsed() < LIMIT {
        let step = live.step();
        discovered = !step.discovered.is_empty();
        batched.extend(step.batches.into_iter().flat_map(|batch| batch.events));
    }
    assert!(discovered, "the session was found");

    std::fs::write(&path, rewritten_file()).unwrap();
    let expected = replayed_ids(&path, SESSION);
    while batched.len() < expected.len() && started.elapsed() < LIMIT {
        let step = live.step();
        assert!(step.errors.is_empty(), "{:?}", step.errors);
        batched.extend(step.batches.into_iter().flat_map(|batch| batch.events));
    }
    batched.extend(live.finish().into_iter().flat_map(|batch| batch.events));
    drop(live);

    let batch_ids = ids(&batched);
    let distinct: BTreeSet<_> = batch_ids.iter().collect();
    assert_eq!(
        batch_ids.len(),
        distinct.len(),
        "an id is batched twice: {batch_ids:?}"
    );
    assert_eq!(batch_ids, expected);
    assert_eq!(unknown_count(&batched), 0);

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
        "the store holds each event once"
    );
}

#[test]
fn a_rewrite_after_events_were_emitted_is_reported_and_nothing_is_doubled() {
    let dir = TempDir::new().unwrap();
    let path = session_path(dir.path());
    let mut attach = Attach::start(&config(dir.path(), POLL_MS)).unwrap();
    let mut collected = Collected::default();
    append(&path, &first_lines().0);
    for line in m0_lines(LINES).into_iter().skip(1) {
        append(&path, &line);
    }
    pump(&mut attach, &mut collected, LIMIT, |attach, _| {
        attach.at_eof()
    });
    let before = ids(&collected.events);
    assert!(!before.is_empty());

    std::fs::write(&path, rewritten_file()).unwrap();
    pump(&mut attach, &mut collected, LIMIT, |_, collected| {
        !collected.warnings.is_empty()
    });

    assert!(
        matches!(
            collected.warnings.as_slice(),
            [observatory_core::ingest::IngestError::FileMoved { .. }]
        ),
        "{:?}",
        collected.warnings
    );
    assert_eq!(ids(&collected.events), before, "no event is emitted twice");
}

#[test]
fn an_unfinished_line_that_changed_under_the_tail_is_read_again() {
    use observatory_core::ingest::{JsonlSource, Source};

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("unfinished.jsonl");
    std::fs::write(&path, "{\"a\":1}\n{\"b\"").unwrap();
    let mut source = JsonlSource::open(&path, &Default::default()).unwrap();
    assert_eq!(source.next_batch().unwrap().len(), 1);
    assert!(!source.take_rewritten());

    std::fs::write(&path, "{\"a\":1}\n{\"c\":2}\n").unwrap();
    let lines: Vec<Vec<u8>> = source
        .next_batch()
        .unwrap()
        .into_iter()
        .map(|record| record.bytes)
        .collect();
    assert!(source.take_rewritten());
    assert!(!source.take_rewritten(), "reported once");
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
