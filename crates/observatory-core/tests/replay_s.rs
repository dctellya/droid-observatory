//! The skeleton mapper over the S capture: every raw line becomes one `Unknown` event.

use std::path::{Path, PathBuf};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{JsonlSource, RawRecord, Source, mapping_version_of};
use observatory_core::mapping::{Mapper, MappingVersion, SessionMeta};
use observatory_core::model::{Event, EventKind, Lane, Role, SessionId, SourcePtr};

const SLUG: &str = "2026-10-22-observatory-v1-s";

fn fixture(extension: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams")
        .join(format!("{SLUG}.{extension}"))
}

fn read_all(source: &mut JsonlSource) -> Vec<RawRecord> {
    let mut records = Vec::new();
    for _ in 0..100_000 {
        records.extend(source.next_batch().expect("batch reads"));
        if source.at_eof() {
            records.extend(source.drain_partial());
            return records;
        }
    }
    panic!("source never reached the end of the file");
}

fn replay(path: &Path) -> (Vec<RawRecord>, Vec<Event>) {
    let mut source = JsonlSource::open(path, &IngestConfig::default()).expect("fixture opens");
    let version = mapping_version_of(path).expect("capture header is readable");
    let mut mapper = Mapper::new(SessionMeta::root(source.root_session_id().clone()), version);
    let records = read_all(&mut source);
    let events = records
        .iter()
        .flat_map(|record| mapper.map(record))
        .collect();
    (records, events)
}

#[test]
fn replay_counts_every_line_of_a_capture() {
    let path = fixture("jsonl");
    let lines = std::fs::read(&path)
        .unwrap()
        .split_inclusive(|b| *b == b'\n')
        .count();

    let (records, events) = replay(&path);

    assert_eq!(records.len(), lines);
    assert_eq!(events.len(), lines);
    assert!(lines > 0);
    assert!(
        events
            .iter()
            .all(|event| matches!(event.kind, EventKind::Unknown { .. })),
        "the S mapper knows no kind but Unknown"
    );
}

#[test]
fn every_replayed_event_points_at_its_own_line() {
    let path = fixture("jsonl");
    let file = std::fs::read(&path).unwrap();

    let (_, events) = replay(&path);

    for event in &events {
        let SourcePtr::Jsonl {
            byte_offset,
            byte_len,
            ..
        } = &event.source
        else {
            panic!("replayed events point into a Jsonl file");
        };
        let start = *byte_offset as usize;
        let line = &file[start..start + *byte_len as usize];
        let raw: serde_json::Value = serde_json::from_slice(line).expect("fixture lines are JSON");
        let EventKind::Unknown { raw_type } = &event.kind else {
            panic!("expected Unknown");
        };
        assert_eq!(raw["type"].as_str(), Some(raw_type.as_str()));
        assert_eq!(
            event.id.0,
            format!("{}:{start}:0", event.session_id.as_ref().unwrap())
        );
    }
}

#[test]
fn replayed_events_carry_the_root_session_the_orchestrator_lane_and_the_header_version() {
    let (_, events) = replay(&fixture("jsonl"));

    let first = events.first().unwrap();
    assert_eq!(
        first.session_id,
        Some(SessionId("e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c".to_owned()))
    );
    assert_eq!(first.role, Role::Orchestrator);
    assert!(events.iter().all(|e| e.lane == Lane::Orchestrator));
    assert!(
        events
            .iter()
            .all(|e| e.mapping_version == MappingVersion::V1)
    );
}

#[test]
fn event_time_follows_the_record_and_never_runs_backwards() {
    let (_, events) = replay(&fixture("jsonl"));

    assert!(events.windows(2).all(|pair| pair[0].at <= pair[1].at));
    let timed = events
        .iter()
        .find(|event| event.at.to_rfc3339_millis() == "2026-10-22T10:18:11.451Z");
    assert!(timed.is_some(), "the first message keeps its own timestamp");
}

#[test]
fn a_line_that_is_not_json_is_unknown_and_unparseable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, "not json\n[1,2]\n{\"no_type\":true}\n").unwrap();

    let (_, events) = replay(&path);

    let raw_types: Vec<_> = events
        .iter()
        .map(|event| match &event.kind {
            EventKind::Unknown { raw_type } => raw_type.as_str(),
            other => panic!("expected Unknown, got {other:?}"),
        })
        .collect();
    assert_eq!(
        raw_types,
        ["<unparseable>", "<unparseable>", "<unparseable>"]
    );
}

#[test]
fn the_mapping_version_comes_from_the_capture_header_else_latest() {
    assert_eq!(
        mapping_version_of(&fixture("jsonl")).unwrap(),
        MappingVersion::V1
    );

    let dir = tempfile::tempdir().unwrap();
    let bare = dir.path().join("2026-01-01-bare.jsonl");
    std::fs::write(&bare, "{\"type\":\"message\"}\n").unwrap();
    assert_eq!(mapping_version_of(&bare).unwrap(), MappingVersion::LATEST);

    std::fs::write(
        dir.path().join("2026-01-01-bare.capture.json"),
        "{\"schema\":1,\"droid_version\":\"9.9.9\"}",
    )
    .unwrap();
    assert_eq!(mapping_version_of(&bare).unwrap(), MappingVersion::LATEST);
}
