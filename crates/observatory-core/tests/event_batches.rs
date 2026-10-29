//! The shell's side of attach (architecture §7): events reach the UI in batches flushed every
//! `[ui].event_batch_ms`, sessions are announced with their names, the watch limits are
//! reported, and what the UI receives equals a replay of the same file.

mod common;

use std::time::{Duration, Instant};

use observatory_core::ingest::{EventBatcher, order_key};
use observatory_core::model::{Event, MissionId};

const BATCH: Duration = Duration::from_millis(100);

fn golden() -> Vec<Event> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.events.json");
    let text = std::fs::read_to_string(path).expect("the golden file is committed");
    serde_json::from_str(&text).expect("the golden file is a list of events")
}

#[test]
fn event_batches_flush_within_event_batch_ms() {
    let events = golden();
    let mission = events[0].mission_id.clone();
    let start = Instant::now();
    let mut batcher = EventBatcher::new(BATCH);
    assert_eq!(batcher.due_in(start), None, "nothing is pending yet");

    let ms = Duration::from_millis;
    batcher.push(&mission, events[0..2].to_vec(), 0, start);
    batcher.push(&mission, events[2..3].to_vec(), 1, start + ms(30));
    batcher.push(&mission, events[3..5].to_vec(), 2, start + ms(60));

    assert_eq!(batcher.due_in(start + ms(60)), Some(ms(40)));
    assert!(
        batcher.flush_due(start + ms(99)).is_empty(),
        "the oldest pending event is younger than the batch interval"
    );

    let flushed = batcher.flush_due(start + BATCH);
    assert_eq!(flushed.len(), 1, "one batch per mission, not one per push");
    assert_eq!(flushed[0].mission_id, mission);
    assert_eq!(flushed[0].events, events[0..5].to_vec(), "arrival order");
    assert_eq!(flushed[0].unknown_total, 2, "the latest total is reported");
    assert_eq!(batcher.due_in(start + BATCH), None);
    assert!(batcher.flush_due(start + BATCH * 2).is_empty());
}

#[test]
fn a_retraction_drops_the_pending_events_it_covers_and_travels_with_the_batch() {
    use observatory_core::ingest::Retraction;
    use observatory_core::model::SourcePtr;

    let events: Vec<Event> = golden()
        .into_iter()
        .filter(|event| matches!(event.source, SourcePtr::Jsonl { .. }))
        .take(12)
        .collect();
    let mission = events[0].mission_id.clone();
    let offset = |event: &Event| match &event.source {
        SourcePtr::Jsonl { byte_offset, .. } => *byte_offset,
        _ => unreachable!(),
    };
    let below = offset(&events[8]);
    let retraction = Retraction {
        session_id: events[0].session_id.clone().unwrap(),
        below_offset: below,
    };
    let start = Instant::now();
    let mut batcher = EventBatcher::new(BATCH);
    batcher.push(&mission, events[..10].to_vec(), 0, start);
    batcher.retract(&mission, retraction.clone(), 0, start);
    batcher.push(&mission, events[10..].to_vec(), 0, start);

    let flushed = batcher.flush_all();
    assert_eq!(flushed.len(), 1);
    assert_eq!(flushed[0].retracted, vec![retraction]);
    let kept: Vec<&Event> = events[..10]
        .iter()
        .filter(|event| offset(event) == 0 || offset(event) >= below)
        .chain(&events[10..])
        .collect();
    assert_eq!(flushed[0].events.iter().collect::<Vec<_>>(), kept);
}

#[test]
fn a_later_push_waits_for_its_own_interval() {
    let events = golden();
    let mission = events[0].mission_id.clone();
    let start = Instant::now();
    let mut batcher = EventBatcher::new(BATCH);

    batcher.push(&mission, events[0..1].to_vec(), 0, start);
    assert_eq!(batcher.flush_due(start + BATCH).len(), 1);

    let later = start + Duration::from_millis(250);
    batcher.push(&mission, events[1..2].to_vec(), 0, later);
    assert!(
        batcher
            .flush_due(later + BATCH - Duration::from_millis(1))
            .is_empty()
    );
    assert_eq!(
        batcher.flush_due(later + BATCH)[0].events,
        events[1..2].to_vec()
    );
}

#[test]
fn missions_flush_apart_and_flush_all_empties_the_batcher() {
    let events = golden();
    let (a, b) = (MissionId("a".into()), MissionId("b".into()));
    let start = Instant::now();
    let mut batcher = EventBatcher::new(BATCH);
    batcher.push(&a, events[0..1].to_vec(), 0, start);
    batcher.push(
        &b,
        events[1..2].to_vec(),
        3,
        start + Duration::from_millis(50),
    );

    let first = batcher.flush_due(start + BATCH);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].mission_id, a);

    let rest = batcher.flush_all();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].mission_id, b);
    assert_eq!(rest[0].unknown_total, 3);
    assert!(batcher.flush_all().is_empty());
}

#[test]
fn an_empty_push_leaves_nothing_pending() {
    let mut batcher = EventBatcher::new(BATCH);
    batcher.push(&MissionId("a".into()), Vec::new(), 0, Instant::now());
    assert_eq!(batcher.due_in(Instant::now()), None);
    assert!(batcher.flush_all().is_empty());
}

#[test]
fn replay_order_is_time_then_session_then_offset() {
    let events = golden();
    let mut shuffled: Vec<Event> = events.iter().rev().cloned().collect();
    shuffled.sort_by_key(order_key);
    let ids = |list: &[Event]| list.iter().map(|e| e.id.to_string()).collect::<Vec<_>>();
    assert_eq!(
        ids(&shuffled),
        ids(&events),
        "the golden is in replay order"
    );
}

#[cfg(all(feature = "watch", feature = "store"))]
mod live {
    use std::collections::BTreeMap;
    use std::sync::mpsc;
    use std::thread;

    use super::*;
    use crate::common::{append, byte_offset, m0_lines, session_start};
    use observatory_core::config::Config;
    use observatory_core::ingest::IngestError;
    use observatory_core::ingest::replay_files;
    use observatory_core::ingest::watch::LiveIngest;
    use observatory_core::model::EventKind;
    use observatory_core::store::Store;
    use tempfile::TempDir;

    const SESSION: &str = "live-session";

    /// The latency tests measure wall time; run side by side they would measure each other.
    static LATENCY: std::sync::Mutex<()> = std::sync::Mutex::new(());
    const APPEND_GAP: Duration = Duration::from_millis(20);
    const LINES: usize = 200;

    fn percentile(sorted: &[Duration], per_cent: usize) -> Duration {
        sorted[(sorted.len() * per_cent / 100).min(sorted.len() - 1)]
    }

    fn config(dir: &std::path::Path) -> Config {
        let mut cfg = Config::default();
        cfg.ingest.factory_sessions_dir = dir.to_string_lossy().into_owned();
        cfg
    }

    fn session_path(dir: &std::path::Path, id: &str) -> std::path::PathBuf {
        dir.join("-home-user-project").join(format!("{id}.jsonl"))
    }

    fn live(dir: &TempDir, store_dir: &TempDir) -> LiveIngest {
        let store = Store::open(&store_dir.path().join("live.sqlite")).unwrap();
        LiveIngest::start(&config(dir.path()), store).unwrap()
    }

    fn summary(event: &Event) -> (String, String, String, String, u64) {
        (
            event.id.to_string(),
            event.kind.prd_name().into_owned(),
            event.lane.to_string(),
            event.at.to_rfc3339_millis(),
            byte_offset(event),
        )
    }

    #[test]
    fn appended_lines_reach_the_ui_in_batches_within_one_second() {
        let _alone = LATENCY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = TempDir::new().unwrap();
        let store_dir = TempDir::new().unwrap();
        let mut live = live(&dir, &store_dir);
        let path = session_path(dir.path(), SESSION);
        let mut lines = m0_lines(LINES);
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

        let mut flushed_at: BTreeMap<u64, Instant> = BTreeMap::new();
        let mut received: Vec<Event> = Vec::new();
        let mut batches = 0;
        let mut writer_done = false;
        let mut quiet = 0;
        let started = Instant::now();
        while quiet < 3 && started.elapsed() < Duration::from_secs(60) {
            let step = live.step();
            let now = Instant::now();
            writer_done = writer_done || done_rx.try_recv().is_ok();
            quiet = if writer_done && step.batches.is_empty() {
                quiet + 1
            } else {
                0
            };
            for batch in step.batches {
                batches += 1;
                for event in batch.events {
                    flushed_at.entry(byte_offset(&event)).or_insert(now);
                    received.push(event);
                }
            }
        }
        let appended = writer.join().unwrap();

        let mut deltas: Vec<Duration> = appended
            .iter()
            .filter_map(|(offset, at)| Some(flushed_at.get(offset)?.duration_since(*at)))
            .collect();
        deltas.sort();
        let (p50, p95, max) = (
            percentile(&deltas, 50),
            percentile(&deltas, 95),
            *deltas.last().unwrap(),
        );
        println!(
            "append to flush over {total} lines at {} ms gap in {batches} batches: p50={p50:?} p95={p95:?} max={max:?}",
            APPEND_GAP.as_millis()
        );
        assert!(max < Duration::from_secs(1), "slowest line took {max:?}");
        assert!(
            batches < received.len(),
            "events are batched, not sent one by one"
        );
    }

    /// Droid keeps its session file open while it writes; the notifier does not report every write
    /// of a long-lived handle, so the tail cannot rely on it alone.
    #[test]
    fn lines_written_through_one_open_handle_flush_within_half_a_poll_and_a_batch() {
        use std::io::Write;

        let _alone = LATENCY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = TempDir::new().unwrap();
        let store_dir = TempDir::new().unwrap();
        let path = session_path(dir.path(), SESSION);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"").unwrap();
        let mut live = live(&dir, &store_dir);
        let mut lines = m0_lines(LINES);
        lines[0] = session_start(SESSION, None);

        let writer = thread::spawn(move || {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            let mut appended = Vec::with_capacity(lines.len());
            let mut offset = 0_u64;
            for line in &lines {
                appended.push((offset, Instant::now()));
                file.write_all(line).unwrap();
                file.flush().unwrap();
                offset += line.len() as u64;
                thread::sleep(APPEND_GAP);
            }
            appended
        });

        let mut flushed_at: BTreeMap<u64, Instant> = BTreeMap::new();
        let started = Instant::now();
        let mut quiet = 0;
        while quiet < 12 && started.elapsed() < Duration::from_secs(60) {
            let step = live.step();
            let now = Instant::now();
            quiet = if step.batches.is_empty() && writer.is_finished() {
                quiet + 1
            } else {
                0
            };
            for event in step.batches.into_iter().flat_map(|batch| batch.events) {
                flushed_at.entry(byte_offset(&event)).or_insert(now);
            }
        }
        let appended = writer.join().unwrap();
        // The first lines wait for the session to be found, which takes up to one poll interval.
        let mut deltas: Vec<Duration> = appended
            .iter()
            .filter(|(_, at)| at.duration_since(started) > Duration::from_secs(1))
            .filter_map(|(offset, at)| Some(flushed_at.get(offset)?.duration_since(*at)))
            .collect();
        deltas.sort();
        let max = *deltas.last().unwrap();
        println!(
            "open handle, {} events: p50={:?} p95={:?} max={max:?}",
            deltas.len(),
            percentile(&deltas, 50),
            percentile(&deltas, 95),
        );
        assert!(
            max < Duration::from_millis(450),
            "slowest line took {max:?}"
        );
    }

    #[test]
    fn what_the_ui_receives_equals_a_replay_of_the_same_file() {
        let dir = TempDir::new().unwrap();
        let store_dir = TempDir::new().unwrap();
        let mut live = live(&dir, &store_dir);
        let path = session_path(dir.path(), SESSION);
        let mut lines = m0_lines(300);
        lines[0] = session_start(SESSION, None);
        let mut bytes = Vec::new();
        for line in &lines {
            append(&path, line);
            bytes.extend_from_slice(line);
        }

        let mut received: Vec<Event> = Vec::new();
        let started = Instant::now();
        let mut quiet = 0;
        while quiet < 3 && started.elapsed() < Duration::from_secs(30) {
            let step = live.step();
            quiet = if step.batches.is_empty() {
                quiet + 1
            } else {
                0
            };
            received.extend(step.batches.into_iter().flat_map(|batch| batch.events));
        }
        received.extend(live.finish().into_iter().flat_map(|batch| batch.events));
        received.sort_by_key(order_key);

        let oracle = replay_files(BTreeMap::from([(path.clone(), bytes)])).unwrap();
        assert!(!oracle.events.is_empty());
        assert_eq!(
            received.iter().map(summary).collect::<Vec<_>>(),
            oracle.events.iter().map(summary).collect::<Vec<_>>(),
            "same ids, kinds, lanes, times and offsets in the same order"
        );
        let unknown = |events: &[Event]| {
            events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::Unknown { .. }))
                .count() as u64
        };
        assert_eq!(unknown(&received), oracle.summary.unknown_count);
    }

    #[test]
    fn a_new_session_is_announced_with_its_tag_or_its_id() {
        let dir = TempDir::new().unwrap();
        let store_dir = TempDir::new().unwrap();
        let mut live = live(&dir, &store_dir);
        assert!(live.step().discovered.is_empty());

        let tagged = session_path(dir.path(), "tagged-id");
        append(&tagged, &session_start("tagged-id", None));
        std::fs::write(
            tagged.with_extension("settings.json"),
            r#"{"tags":[{"name":"release-hardening"}]}"#,
        )
        .unwrap();
        append(
            &session_path(dir.path(), "plain-id"),
            &session_start("plain-id", None),
        );

        let mut names = BTreeMap::new();
        let started = Instant::now();
        while names.len() < 2 && started.elapsed() < Duration::from_secs(10) {
            for row in live.step().discovered {
                names.insert(row.session_id.0, row.name);
            }
        }
        assert_eq!(
            names,
            BTreeMap::from([
                ("plain-id".to_owned(), "plain-id".to_owned()),
                ("tagged-id".to_owned(), "release-hardening".to_owned()),
            ])
        );
        let status = live.status();
        assert_eq!(status.watched.len(), 2);
        assert_eq!(status.unwatched, 0);
    }

    #[test]
    fn a_tag_written_after_the_session_started_renames_it() {
        let dir = TempDir::new().unwrap();
        let store_dir = TempDir::new().unwrap();
        let mut live = live(&dir, &store_dir);
        let path = session_path(dir.path(), "late-tag");
        append(&path, &session_start("late-tag", None));

        let mut first = Vec::new();
        let started = Instant::now();
        while first.is_empty() && started.elapsed() < Duration::from_secs(10) {
            first = live.step().discovered;
        }
        assert_eq!(first[0].name, "late-tag");

        std::fs::write(
            path.with_extension("settings.json"),
            r#"{"tags":[{"name":"after-the-fact"}]}"#,
        )
        .unwrap();
        let mut renamed = Vec::new();
        let started = Instant::now();
        while renamed.is_empty() && started.elapsed() < Duration::from_secs(10) {
            renamed = live.step().discovered;
        }
        assert_eq!(renamed[0].session_id.0, "late-tag");
        assert_eq!(renamed[0].name, "after-the-fact");
    }

    #[test]
    fn the_status_counts_sessions_not_watched_and_keeps_one_too_large_warning() {
        let dir = TempDir::new().unwrap();
        let store_dir = TempDir::new().unwrap();
        let mut cfg = config(dir.path());
        cfg.ingest.max_watched_sessions = 2;
        cfg.ingest.max_session_file_mb = 1;
        cfg.ingest.tail_chunk_kb = 256;
        for id in ["s1", "s2", "s3"] {
            let path = session_path(dir.path(), id);
            append(&path, &session_start(id, None));
            std::thread::sleep(Duration::from_millis(15));
        }
        let big = session_path(dir.path(), "big");
        append(&big, &session_start("big", None));
        let mut size = 0;
        for line in m0_lines(1000).into_iter().skip(1) {
            if size > 1024 * 1024 {
                break;
            }
            size += line.len();
            append(&big, &line);
        }
        let store = Store::open(&store_dir.path().join("limits.sqlite")).unwrap();
        let mut live = LiveIngest::start(&cfg, store).unwrap();

        let mut warned = Vec::new();
        for _ in 0..4 {
            warned.extend(live.step().warnings);
        }
        let status = live.status();
        assert_eq!(status.watched.len(), 2, "the two newest are watched");
        assert_eq!(status.unwatched, 2);
        let too_large = warned
            .iter()
            .filter(|warning| matches!(warning, IngestError::TooLarge { .. }))
            .count();
        assert_eq!(too_large, 1, "warned once: {warned:?}");
        assert_eq!(
            status
                .warnings
                .iter()
                .filter(|text| text.contains("big.jsonl"))
                .count(),
            1,
            "the status keeps it for the UI: {:?}",
            status.warnings
        );
    }
}

#[test]
fn a_batch_retracts_events_by_id_and_old_payloads_still_read() {
    use observatory_core::ingest::{EventBatch, drop_events};

    let mut held = golden();
    let gone = vec![held[3].id.clone(), held[7].id.clone()];
    let before = held.len();

    assert_eq!(drop_events(&mut held, &gone), 2);
    assert_eq!(held.len(), before - 2);
    assert!(held.iter().all(|event| !gone.contains(&event.id)));
    assert_eq!(drop_events(&mut held, &gone), 0, "nothing left to drop");

    let batch = EventBatch {
        mission_id: MissionId("m".to_owned()),
        events: Vec::new(),
        unknown_total: 0,
        retracted: Vec::new(),
        retracted_events: gone.clone(),
    };
    let wire = serde_json::to_value(&batch).unwrap();
    assert_eq!(serde_json::from_value::<EventBatch>(wire).unwrap(), batch);
    let old = serde_json::json!({ "mission_id": "m", "events": [], "unknown_total": 0 });
    let read: EventBatch = serde_json::from_value(old).unwrap();
    assert!(
        read.retracted_events.is_empty(),
        "a payload without the field reads"
    );
}
