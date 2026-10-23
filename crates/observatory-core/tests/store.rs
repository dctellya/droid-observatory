//! The SQLite store over the M0 capture: pointers only, never raw text (architecture §6, I3).

#![cfg(feature = "store")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{
    JsonlSource, SessionTree, Source, mapping_version_of, read_capture, sidecar_path,
};
use observatory_core::mapping::TreeMapper;
use observatory_core::model::{Event, EventId, EventKind, SourcePtr};
use observatory_core::store::{CaptureInfo, Store, StoreError};
use rusqlite::Connection;
use serde_json::Value;

const SLUG: &str = "2026-10-22-observatory-v1-m0";

fn streams_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams")
        .canonicalize()
        .expect("the fixtures directory exists")
}

fn m0_root() -> PathBuf {
    streams_dir().join(format!("{SLUG}.jsonl"))
}

/// Maps the whole tree under `root` and writes it through `store` the way `replay` does.
fn replay_into(store: &mut Store, root: &Path, root_only: bool) -> Vec<Event> {
    let cfg = IngestConfig::default();
    let tree = SessionTree::discover(root, &cfg).expect("the capture opens");
    let capture = CaptureInfo::from_session_file(root).expect("the header reads");
    store
        .record_tree(&tree, &capture, root_only)
        .expect("sessions are recorded");
    let version = mapping_version_of(root).expect("header is readable");
    let mut source = if root_only {
        JsonlSource::open(root, &cfg)
    } else {
        JsonlSource::open_tree(&tree, &cfg)
    }
    .expect("the session files open");
    let mut mapper = TreeMapper::new(&tree, version);

    let mut events = Vec::new();
    loop {
        let mut batch = source.next_batch().expect("batch reads");
        let at_eof = source.at_eof();
        if at_eof {
            batch.extend(source.drain_partial());
        }
        let mut mapped: Vec<Event> = batch.iter().flat_map(|rec| mapper.map(rec)).collect();
        if at_eof {
            mapped.extend(mapper.finish());
        }
        store.insert_events(&mapped).expect("events are stored");
        events.extend(mapped);
        if at_eof {
            break;
        }
    }
    store
        .mark_tailed(tree.mission_id())
        .expect("sessions are marked read");
    events
}

fn indexed_m0() -> (tempfile::TempDir, PathBuf, Vec<Event>) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("observatory.sqlite");
    let mut store = Store::open(&db).expect("the store opens");
    let events = replay_into(&mut store, &m0_root(), false);
    (dir, db, events)
}

fn jsonl_pointer(event: &Event) -> (&Path, u64, u32) {
    match &event.source {
        SourcePtr::Jsonl {
            path,
            byte_offset,
            byte_len,
        } => (path, *byte_offset, *byte_len),
        other => panic!("a session event has a jsonl pointer, not {other:?}"),
    }
}

/// The lines of a file by independent means: split on `\n`, offsets counted by hand.
fn lines_with_offsets(bytes: &[u8]) -> Vec<(u64, &[u8])> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            lines.push((start as u64, &bytes[start..index]));
            start = index + 1;
        }
    }
    if start < bytes.len() {
        lines.push((start as u64, &bytes[start..]));
    }
    lines
}

fn query_strings(db: &Path, sql: &str) -> Vec<String> {
    let conn = Connection::open(db).unwrap();
    let mut statement = conn.prepare(sql).unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn query_count(db: &Path, sql: &str) -> i64 {
    Connection::open(db)
        .unwrap()
        .query_row(sql, [], |row| row.get(0))
        .unwrap()
}

#[test]
fn store_pointer_reads_back_the_exact_raw_line() {
    let (_dir, db, events) = indexed_m0();
    let store = Store::open(&db).unwrap();
    assert!(!events.is_empty());

    let mut checked_files = BTreeSet::new();
    let picks = [0, events.len() / 3, events.len() / 2, events.len() - 1];
    let child = events
        .iter()
        .position(|event| jsonl_pointer(event).0 != m0_root())
        .expect("the capture has events from a child session");
    for index in picks.into_iter().chain([child]) {
        let event = &events[index];
        let pointer = store.pointer(&event.id).expect("the pointer is stored");
        assert_eq!(pointer, event.source);

        let SourcePtr::Jsonl {
            path,
            byte_offset,
            byte_len,
        } = pointer
        else {
            panic!("jsonl pointer expected");
        };
        let read_back = JsonlSource::read_at(&path, byte_offset, byte_len).unwrap();

        let file = std::fs::read(&path).unwrap();
        let expected = lines_with_offsets(&file)
            .into_iter()
            .find(|(offset, _)| *offset == byte_offset)
            .map(|(_, line)| line)
            .expect("a line starts at the stored offset");
        assert_eq!(read_back, expected, "{}", event.id);
        assert_eq!(read_back.len(), byte_len as usize);
        assert!(!read_back.ends_with(b"\n"), "no trailing newline");
        checked_files.insert(path);
    }
    assert!(checked_files.len() >= 2, "root and child were both read");
}

/// Strings of at least this many characters in a raw line are specific enough to search for.
const NEEDLE_CHARS: usize = 40;

fn text_values(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) if text.chars().count() >= NEEDLE_CHARS => {
            out.push(text.chars().take(NEEDLE_CHARS).collect());
        }
        Value::Array(items) => items.iter().for_each(|item| text_values(item, out)),
        Value::Object(map) => map.values().for_each(|item| text_values(item, out)),
        _ => {}
    }
}

#[test]
fn store_never_contains_raw_line_text() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("observatory.sqlite");
    let mut store = Store::open(&db).unwrap();
    replay_into(&mut store, &m0_root(), false);

    // The store is still open: the WAL holds what a reader would see next to the main file.
    let mut stored = std::fs::read(&db).unwrap();
    let wal = dir.path().join("observatory.sqlite-wal");
    if let Ok(bytes) = std::fs::read(&wal) {
        stored.extend(bytes);
    }
    let stored = String::from_utf8_lossy(&stored);

    let mut needles = Vec::new();
    let root = std::fs::read(m0_root()).unwrap();
    for (index, (_, line)) in lines_with_offsets(&root).into_iter().enumerate() {
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if index % 7 == 0 {
            text_values(&value["message"], &mut needles);
        }
        needles.push(String::from_utf8_lossy(&line[..line.len().min(60)]).into_owned());
    }
    let children = streams_dir().join(format!("{SLUG}.children"));
    let sidecars = std::fs::read_dir(&children)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".settings.json"))
        .chain([sidecar_path(&m0_root())]);
    let mut prompts = 0;
    for path in sidecars {
        let sidecar: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        if let Some(prompt) = sidecar["systemPrompt"].as_str() {
            needles.push(prompt.chars().take(NEEDLE_CHARS).collect());
            prompts += 1;
        }
    }
    assert!(prompts > 0, "the capture has sessions with a systemPrompt");
    assert!(needles.len() > 100, "{} needles", needles.len());

    // The capture's source directory is a header value the `captures` row keeps on purpose.
    let header = read_capture(&m0_root()).unwrap().unwrap();
    let source_dir = header.source_dir.expect("the header names its source dir");
    let found: Vec<&String> = needles
        .iter()
        .filter(|needle| !source_dir.contains(needle.as_str()))
        .filter(|needle| stored.contains(needle.as_str()))
        .collect();
    assert!(found.is_empty(), "raw text in the store: {found:?}");
}

#[test]
fn store_schema_has_the_pointer_index_and_no_payload_column() {
    let (_dir, db, _) = indexed_m0();

    let tables: BTreeSet<String> = query_strings(
        &db,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    )
    .into_iter()
    .collect();
    for table in [
        "schema_version",
        "captures",
        "mapping_versions",
        "missions",
        "sessions",
        "milestones",
        "features",
        "events",
        "usage",
        "rework_windows",
        "prs",
        "checks",
        "reviews",
        "attestations",
        "human_actions",
        "prices",
    ] {
        assert!(tables.contains(table), "table {table} missing: {tables:?}");
    }

    let columns = query_strings(&db, "SELECT name FROM pragma_table_info('events')");
    assert_eq!(
        columns,
        [
            "id",
            "mission_id",
            "session_id",
            "milestone_id",
            "feature_id",
            "lane",
            "kind",
            "raw_type",
            "mapping_version",
            "role",
            "model",
            "at",
            "duration_ms",
            "source",
            "path",
            "byte_offset",
            "byte_len",
            "sub",
            "repo",
            "sha",
            "url",
            "action_id"
        ]
    );

    let index = query_strings(
        &db,
        "SELECT sql FROM sqlite_master WHERE name = 'events_ptr'",
    );
    assert_eq!(
        index,
        ["CREATE UNIQUE INDEX events_ptr ON events(path, byte_offset, sub) WHERE path IS NOT NULL"]
    );
    for name in ["events_mission_at", "events_feature"] {
        let found = query_count(
            &db,
            &format!("SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = '{name}'"),
        );
        assert_eq!(found, 1, "{name}");
    }
    assert_eq!(query_strings(&db, "PRAGMA journal_mode"), ["wal"]);
    assert!(query_count(&db, "SELECT max(version) FROM schema_version") >= 1);
    assert_eq!(
        query_count(&db, "SELECT count(*) FROM events WHERE path IS NULL"),
        0
    );
}

#[test]
fn store_records_the_mapping_version_and_the_capture_header() {
    let (_dir, db, events) = indexed_m0();

    assert_eq!(
        query_strings(
            &db,
            "SELECT version || '|' || droid_version_range FROM mapping_versions"
        ),
        ["v1|0.247.*"]
    );
    assert!(
        query_count(
            &db,
            "SELECT count(*) FROM mapping_versions WHERE frozen_at <> ''"
        ) == 1
    );
    let header = read_capture(&m0_root()).unwrap().unwrap();
    assert_eq!(header.droid_version.as_deref(), Some("0.247.0"));
    let captures = query_strings(
        &db,
        "SELECT droid_version || '|' || header_path FROM captures",
    );
    assert_eq!(captures.len(), 1, "{captures:?}");
    assert!(captures[0].starts_with("0.247.0|"), "{captures:?}");
    assert!(
        captures[0].ends_with("2026-10-22-observatory-v1-m0.capture.json"),
        "{captures:?}"
    );
    assert_eq!(
        query_count(
            &db,
            "SELECT count(*) FROM events WHERE mapping_version <> 'v1'"
        ),
        0
    );
    assert_eq!(
        query_count(&db, "SELECT count(*) FROM events"),
        i64::try_from(events.len()).unwrap()
    );
}

#[test]
fn store_has_one_session_row_per_jsonl_with_parent_and_role() {
    let (_dir, db, _) = indexed_m0();
    let header: Value = serde_json::from_slice(
        &std::fs::read(streams_dir().join(format!("{SLUG}.capture.json"))).unwrap(),
    )
    .unwrap();
    let expected_sessions = header["sessions"].as_array().unwrap();

    let conn = Connection::open(&db).unwrap();
    let mut statement = conn
        .prepare(
            "SELECT id, parent_id, role, path, size_bytes, tailed_to FROM sessions ORDER BY id",
        )
        .unwrap();
    let rows: Vec<(String, Option<String>, String, String, i64, i64)> = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    assert_eq!(rows.len(), expected_sessions.len());
    let root_id = header["sessions"][0]["id"].as_str().unwrap();
    for (id, parent, role, path, size, tailed) in &rows {
        let expected = expected_sessions
            .iter()
            .find(|session| session["id"] == id.as_str())
            .unwrap_or_else(|| panic!("{id} is not in the header"));
        assert_eq!(role, expected["role"].as_str().unwrap(), "{id}");
        assert!(path.ends_with(".jsonl"), "{path}");
        assert_eq!(
            std::fs::metadata(path).unwrap().len(),
            u64::try_from(*size).unwrap()
        );
        assert_eq!(size, tailed, "a finished replay read each file to its end");
        if id == root_id {
            assert_eq!(parent, &None);
        } else {
            assert!(parent.is_some(), "{id} has a parent");
        }
    }
    assert_eq!(
        query_count(
            &db,
            "SELECT count(*) FROM sessions WHERE role = 'unresolved'"
        ),
        0
    );
    assert!(
        query_count(
            &db,
            "SELECT count(*) FROM sessions WHERE tag IS NOT NULL AND tag <> ''"
        ) > 0
    );
}

#[test]
fn storing_the_same_events_twice_adds_nothing() {
    let (_dir, db, events) = indexed_m0();
    let mut store = Store::open(&db).unwrap();
    let before = query_count(&db, "SELECT count(*) FROM events");

    let added = store.insert_events(&events).unwrap();

    assert_eq!(added, 0);
    assert_eq!(query_count(&db, "SELECT count(*) FROM events"), before);
}

#[test]
fn replaying_twice_keeps_one_capture_one_mission_and_one_row_per_session() {
    let (_dir, db, _) = indexed_m0();
    let mut store = Store::open(&db).unwrap();
    let events = replay_into(&mut store, &m0_root(), false);

    assert_eq!(query_count(&db, "SELECT count(*) FROM captures"), 1);
    assert_eq!(query_count(&db, "SELECT count(*) FROM missions"), 1);
    let sessions = SessionTree::discover(&m0_root(), &IngestConfig::default())
        .unwrap()
        .sessions()
        .len();
    let sessions = i64::try_from(sessions).unwrap();
    assert_eq!(query_count(&db, "SELECT count(*) FROM sessions"), sessions);
    assert_eq!(
        query_count(&db, "SELECT count(*) FROM events"),
        i64::try_from(events.len()).unwrap()
    );
}

#[test]
fn an_unknown_event_id_is_an_error_naming_the_id() {
    let (_dir, db, _) = indexed_m0();
    let store = Store::open(&db).unwrap();

    let err = store
        .pointer(&EventId("no-such-session:0:0".into()))
        .unwrap_err();

    assert!(matches!(err, StoreError::UnknownEvent { .. }), "{err}");
    assert!(err.to_string().contains("no-such-session:0:0"), "{err}");
}

#[test]
fn unknown_count_counts_the_unknown_events_of_a_mission() {
    let dir = tempfile::tempdir().unwrap();
    let streams = streams_dir();
    let root = dir.path().join(format!("{SLUG}.jsonl"));
    let mut bytes = std::fs::read(m0_root()).unwrap();
    bytes.extend_from_slice(b"not json\n{\"type\":\"future_kind\"}\n");
    std::fs::write(&root, bytes).unwrap();
    for extension in ["settings.json", "children", "capture.json"] {
        std::os::unix::fs::symlink(
            streams.join(format!("{SLUG}.{extension}")),
            dir.path().join(format!("{SLUG}.{extension}")),
        )
        .unwrap();
    }
    let mut store = Store::open(&dir.path().join("store.sqlite")).unwrap();

    let events = replay_into(&mut store, &root, false);

    let mission = SessionTree::discover(&root, &IngestConfig::default())
        .unwrap()
        .mission_id()
        .clone();
    let unknown = events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
        .count();
    assert_eq!(unknown, 2);
    assert_eq!(store.unknown_count(&mission).unwrap(), 2);
}

#[test]
fn opening_a_store_twice_applies_the_schema_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("nested/dir/observatory.sqlite");

    drop(Store::open(&db).expect("parent directories are created"));
    drop(Store::open(&db).expect("an existing store opens"));

    assert_eq!(query_count(&db, "SELECT count(*) FROM schema_version"), 1);
    assert_eq!(query_count(&db, "SELECT count(*) FROM mapping_versions"), 1);
}
