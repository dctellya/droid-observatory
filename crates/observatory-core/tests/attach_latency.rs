//! Attach latency (SPEC §3 M1, architecture §14): a harness thread appends the M0 capture to a
//! session file line by line while the ingest loop attaches, maps and stores what it finds. The
//! time from the append of a line to the moment its events are readable in the store must stay
//! under one second.
#![cfg(all(feature = "watch", feature = "store"))]

mod common;

use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use common::{append, byte_offset, config, m0_lines, replayed_ids, session_start};
use observatory_core::ingest::watch::Attach;
use observatory_core::model::SessionId;
use observatory_core::store::{CaptureInfo, EventScope, Store, TimeRange};
use tempfile::TempDir;

const LINES_ENV: &str = "OBSERVATORY_LATENCY_LINES";
const DEFAULT_LINES: usize = 200;
/// Gap between two appended lines: a fast burst compared with a live session.
const APPEND_GAP: Duration = Duration::from_millis(20);
const BUDGET: Duration = Duration::from_millis(1000);
const SESSION: &str = "latency-session";

fn lines_to_append() -> usize {
    std::env::var(LINES_ENV)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(DEFAULT_LINES)
}

fn percentile(sorted: &[Duration], per_cent: usize) -> Duration {
    sorted[(sorted.len() * per_cent / 100).min(sorted.len() - 1)]
}

#[test]
fn attach_latency_under_one_second_on_fixture_replayed_live() {
    let dir = TempDir::new().unwrap();
    // The default config: the interval and chunk size users get.
    let cfg = config(
        dir.path(),
        observatory_core::config::IngestConfig::default().jsonl_poll_ms,
    );
    let mut attach = Attach::start(&cfg).unwrap();
    let store_dir = TempDir::new().unwrap();
    let mut store = Store::open(&store_dir.path().join("latency.sqlite")).unwrap();

    let path = dir
        .path()
        .join("-home-user-project")
        .join(format!("{SESSION}.jsonl"));
    let mut lines = m0_lines(lines_to_append());
    lines[0] = session_start(SESSION, None);
    let total = lines.len();

    let (done_tx, done_rx) = mpsc::channel();
    let writer = {
        let path = path.clone();
        thread::spawn(move || {
            let mut appended = Vec::with_capacity(lines.len());
            let mut offset = 0_u64;
            for line in &lines {
                appended.push((offset, Instant::now()));
                append(&path, line);
                offset += line.len() as u64;
                thread::sleep(APPEND_GAP);
            }
            done_tx.send(()).unwrap();
            appended
        })
    };

    let mut visible: HashMap<u64, Instant> = HashMap::new();
    let mut stored = 0;
    let mut produced = 0;
    let started = Instant::now();
    let mut writer_done = false;
    let mut quiet_polls = 0;
    let mut warnings = Vec::new();
    while quiet_polls < 3 && started.elapsed() < Duration::from_secs(60) {
        attach.wait();
        let batch = attach.poll();
        for tree in &batch.trees {
            let capture = CaptureInfo::from_session_file(&tree.root().path).unwrap();
            store.record_tree(tree, &capture, false).unwrap();
        }
        stored += store.insert_events(&batch.events).unwrap();
        let now = Instant::now();
        produced += batch.events.len();
        warnings.extend(batch.warnings);
        for event in &batch.events {
            visible.entry(byte_offset(event)).or_insert(now);
        }
        writer_done = writer_done || done_rx.try_recv().is_ok();
        quiet_polls = if writer_done && batch.events.is_empty() {
            quiet_polls + 1
        } else {
            0
        };
    }
    let appended = writer.join().unwrap();

    // A line that maps to no event (a repeated todo list, say) has nothing to see in the store;
    // the comparison with a replay below proves none was lost.
    let mut deltas: Vec<Duration> = appended
        .iter()
        .filter_map(|(offset, at)| Some(visible.get(offset)?.duration_since(*at)))
        .collect();
    deltas.sort();
    let (p50, p95, max) = (
        percentile(&deltas, 50),
        percentile(&deltas, 95),
        *deltas.last().unwrap(),
    );
    println!(
        "attach latency over {total} lines at {} ms gap: p50={p50:?} p95={p95:?} max={max:?}",
        APPEND_GAP.as_millis()
    );

    assert!(warnings.is_empty(), "attach warned: {warnings:?}");
    assert_eq!(stored, produced, "every event is new to the store");
    let in_store = store
        .events(
            &EventScope::Session(SessionId(SESSION.to_owned())),
            TimeRange::ALL,
        )
        .unwrap();
    assert_eq!(
        in_store.len(),
        produced,
        "the store holds what attach mapped"
    );
    let mut stored_ids: Vec<String> = in_store.iter().map(|e| e.id.to_string()).collect();
    let mut replayed = replayed_ids(&path, SESSION);
    stored_ids.sort();
    replayed.sort();
    assert_eq!(
        stored_ids, replayed,
        "attach and replay map the same events"
    );
    assert!(max < BUDGET, "slowest line took {max:?}, budget {BUDGET:?}");
}
