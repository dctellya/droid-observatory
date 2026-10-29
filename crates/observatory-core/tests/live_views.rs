//! The store-backed side of the Tauri commands (architecture §7): `open_replay` writes a capture
//! through the store and answers with its events, the library reopens a mission from the rows
//! alone, `event_raw` re-reads a line by pointer, and live batches merge into a held list in
//! replay order.

use std::path::{Path, PathBuf};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{merge_ordered, order_key};
use observatory_core::model::{Event, EventId, SessionId};
use observatory_core::view::scrub;

fn golden() -> Vec<Event> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.events.json");
    let text = std::fs::read_to_string(path).expect("the golden file is committed");
    let events: Vec<Event> =
        serde_json::from_str(&text).expect("the golden file is a list of events");
    // Plain replay never reads the usage log; its turn.usage events are checked in usage_log.rs.
    events
        .into_iter()
        .filter(|e| e.kind.prd_name() != "turn.usage")
        .collect()
}

fn m0_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.jsonl")
        .canonicalize()
        .expect("the fixture exists")
}

fn ids(events: &[Event]) -> Vec<String> {
    events.iter().map(|e| e.id.to_string()).collect()
}

#[test]
fn merging_a_batch_keeps_replay_order_and_skips_known_ids() {
    let events = golden();
    let mut held: Vec<Event> = events[..10].to_vec();
    // A batch from another session whose events fall between those held, one repeated event.
    let mut incoming: Vec<Event> = events[20..30].to_vec();
    incoming.extend(events[5..7].to_vec());
    incoming.extend(events[10..20].iter().rev().cloned());

    let added = merge_ordered(&mut held, incoming);

    assert_eq!(added, 20, "the two repeated events are not added");
    assert_eq!(ids(&held), ids(&events[..30]));
    assert!(
        held.windows(2)
            .all(|pair| order_key(&pair[0]) <= order_key(&pair[1]))
    );
}

#[test]
fn merging_into_an_empty_list_sorts_the_batch() {
    let events = golden();
    let mut held = Vec::new();
    let reversed: Vec<Event> = events[..5].iter().rev().cloned().collect();
    assert_eq!(merge_ordered(&mut held, reversed), 5);
    assert_eq!(ids(&held), ids(&events[..5]));
}

#[cfg(feature = "store")]
mod store {
    use super::*;
    use observatory_core::store::Store;
    use tempfile::TempDir;

    fn summary(event: &Event) -> (String, String, String, String) {
        (
            event.id.to_string(),
            event.kind.prd_name().into_owned(),
            event.lane.to_string(),
            event.at.to_rfc3339_millis(),
        )
    }

    fn replayed() -> (TempDir, Store, observatory_core::ingest::OpenedMission) {
        let dir = TempDir::new().unwrap();
        let mut store = Store::open(&dir.path().join("views.sqlite")).unwrap();
        let opened = store
            .replay_file(&m0_root(), &IngestConfig::default())
            .unwrap();
        (dir, store, opened)
    }

    #[test]
    fn open_replay_through_the_store_lists_the_golden_events() {
        let (_dir, _store, opened) = replayed();
        let golden = golden();
        // The capture header names a usage-log extract: its turns come with the replay.
        let (turns, others): (Vec<&Event>, Vec<&Event>) =
            opened.events.iter().partition(|e| e.kind.is_panel_only());
        assert_eq!(turns.len(), 1115);
        assert_eq!(
            others.into_iter().map(summary).collect::<Vec<_>>(),
            golden.iter().map(summary).collect::<Vec<_>>()
        );
        assert_eq!(opened.summary.event_count, golden.len() as u64);
        assert_eq!(opened.summary.unknown_count, 0);
        assert_eq!(opened.summary.session_count, 56);
        assert_eq!(opened.summary.mission_id, golden[0].mission_id);
        assert!(!opened.live);
        assert_eq!(opened.sessions.len(), 56);
    }

    #[test]
    fn a_capture_header_path_opens_the_capture_it_belongs_to() {
        let dir = TempDir::new().unwrap();
        let mut store = Store::open(&dir.path().join("header.sqlite")).unwrap();
        let header = m0_root().with_extension("capture.json");
        let opened = store
            .replay_file(&header, &IngestConfig::default())
            .unwrap();
        assert_eq!(opened.summary.event_count, golden().len() as u64);
    }

    #[test]
    fn the_library_reopens_a_mission_from_the_store_alone() {
        let (_dir, store, opened) = replayed();
        let mission = opened.summary.mission_id.clone();

        let rows = store.missions().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event_count, opened.summary.event_count);

        let again = store.open_mission(&mission).unwrap();
        assert_eq!(ids(&again.events), ids(&opened.events));
        assert_eq!(again.summary.name, opened.summary.name);
        assert_eq!(again.summary.event_count, opened.summary.event_count);
        assert_eq!(again.summary.unknown_count, opened.summary.unknown_count);
        assert_eq!(again.summary.session_count, opened.summary.session_count);
        assert_eq!(
            again.summary.mapping_version,
            opened.summary.mapping_version
        );
        let roles = |sessions: &[observatory_core::ingest::SessionInfo]| {
            let mut all: Vec<_> = sessions
                .iter()
                .map(|s| (s.id.0.clone(), s.role, s.parent.clone()))
                .collect();
            all.sort_by(|a, b| a.0.cmp(&b.0));
            all
        };
        assert_eq!(roles(&again.sessions), roles(&opened.sessions));
    }

    #[test]
    fn a_session_leads_to_its_mission() {
        let (_dir, store, opened) = replayed();
        let child = opened
            .sessions
            .iter()
            .find(|s| s.parent.is_some())
            .expect("the capture has children");
        assert_eq!(
            store.mission_of_session(&child.id).unwrap(),
            Some(opened.summary.mission_id.clone())
        );
        assert_eq!(
            store
                .mission_of_session(&SessionId("not-a-session".into()))
                .unwrap(),
            None
        );
    }

    #[test]
    fn event_raw_reads_the_line_by_pointer_and_masks_it_for_display() {
        let (_dir, store, opened) = replayed();
        let event = &opened.events[0];
        let view = store.event_raw(&event.id).unwrap();
        assert_eq!(view.event_id, event.id);
        assert_eq!(view.pointer, event.source);
        let observatory_core::model::SourcePtr::Jsonl {
            path,
            byte_offset,
            byte_len,
        } = &event.source
        else {
            panic!("a session line");
        };
        let raw = std::fs::read(path).unwrap();
        let start = *byte_offset as usize;
        let line = &raw[start..start + *byte_len as usize];
        assert_eq!(view.line, scrub(line));

        let missing = store.event_raw(&EventId("nope:0:0".into()));
        assert!(missing.is_err());
    }
}
