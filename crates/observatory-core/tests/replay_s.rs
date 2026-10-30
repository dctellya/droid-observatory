//! The mapper over the S capture: every line is read, every event points at its own line, and
//! times, roles and lanes follow the session.

use std::path::{Path, PathBuf};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{JsonlSource, RawRecord, SessionTree, Source, mapping_version_of};
use observatory_core::mapping::{Mapper, MappingVersion, SessionMeta, TreeMapper};
use observatory_core::model::{
    CycleTemplate, Event, EventKind, LaneId, Role, SessionId, SourcePtr,
};

fn lane(id: &str) -> LaneId {
    LaneId::from(id)
}

fn engineering() -> CycleTemplate {
    CycleTemplate::engineering()
}

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
    let mut events: Vec<Event> = records
        .iter()
        .flat_map(|record| mapper.map(record))
        .collect();
    events.extend(mapper.finish());
    (records, events)
}

#[test]
fn replay_reads_every_line_of_a_capture() {
    let path = fixture("jsonl");
    let lines = std::fs::read(&path)
        .unwrap()
        .split_inclusive(|b| *b == b'\n')
        .count();

    let (records, events) = replay(&path);

    assert_eq!(records.len(), lines);
    assert!(lines > 0);
    assert!(!events.is_empty());
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
        if let EventKind::Unknown { raw_type } = &event.kind {
            assert_eq!(raw["type"].as_str(), Some(raw_type.as_str()));
        }
        let prefix = format!("{}:{start}:", event.session_id.as_ref().unwrap());
        let sub = event.id.0.strip_prefix(&prefix).expect("id names its line");
        assert!(sub.parse::<u32>().is_ok(), "{}", event.id);
    }
}

#[test]
fn replayed_events_carry_the_root_session_the_orchestrator_role_and_the_header_version() {
    let (_, events) = replay(&fixture("jsonl"));

    let first = events.first().unwrap();
    assert_eq!(
        first.session_id,
        Some(SessionId("e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c".to_owned()))
    );
    assert_eq!(first.role, Role::Orchestrator);
    assert!(events.iter().all(|e| e.role == Role::Orchestrator));
    assert!(
        events
            .iter()
            .all(|e| e.lane == engineering().lane_of(&e.kind, e.role))
    );
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

fn lines_of(path: &Path) -> usize {
    std::fs::read(path)
        .unwrap()
        .split_inclusive(|b| *b == b'\n')
        .count()
}

#[test]
fn a_tree_replay_maps_every_session_under_its_own_role_and_lane() {
    let path = fixture("jsonl");
    let cfg = IngestConfig::default();
    let tree = SessionTree::discover(&path, &cfg).expect("the S capture opens");
    let mut source = JsonlSource::open_tree(&tree, &cfg).expect("every session file opens");
    let mut mapper = TreeMapper::new(&tree, mapping_version_of(&path).unwrap());

    let records = read_all(&mut source);
    let mut events: Vec<Event> = records
        .iter()
        .flat_map(|record| mapper.map(record))
        .collect();
    events.extend(mapper.finish());

    let expected: usize = tree.sessions().iter().map(|s| lines_of(&s.path)).sum();
    assert_eq!(records.len(), expected);
    assert!(
        expected > lines_of(&path),
        "the children add lines to the root file"
    );
    for event in &events {
        let session = tree.get(event.session_id.as_ref().unwrap()).unwrap();
        assert_eq!(event.role, session.role);
        match engineering().fixed_lane(&event.kind) {
            Some(fixed) => assert_eq!(&event.lane, fixed),
            None => assert!(
                [
                    engineering().lane_of(&event.kind, session.role),
                    lane("contract"),
                    lane("integrate")
                ]
                .contains(&event.lane),
                "{} in {:?}",
                event.kind.prd_name(),
                event.lane
            ),
        }
        assert_eq!(event.mission_id, *tree.mission_id());
    }
    assert!(events.iter().any(|e| e.role == Role::Worker));
    assert!(events.iter().all(|e| e.role != Role::Unresolved));
}
