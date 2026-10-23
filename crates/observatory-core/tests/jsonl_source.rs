//! `JsonlSource` reads a session file by byte offsets without ever writing it (architecture §4.2).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{IngestError, JsonlSource, RawRecord, Source};
use observatory_core::model::{SessionId, SourcePtr};

fn read_all(source: &mut JsonlSource) -> Vec<RawRecord> {
    let mut records = Vec::new();
    for _ in 0..10_000 {
        records.extend(source.next_batch().expect("batch reads"));
        if source.at_eof() {
            return records;
        }
    }
    panic!("source never reached the end of the file");
}

fn pointer(record: &RawRecord) -> (u64, u32) {
    match &record.ptr {
        SourcePtr::Jsonl {
            byte_offset,
            byte_len,
            ..
        } => (*byte_offset, *byte_len),
        other => panic!("a JsonlSource yields Jsonl pointers, got {other:?}"),
    }
}

fn append(path: &Path, text: &str) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

#[test]
fn lines_carry_byte_offsets_and_lengths_without_the_newline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, "{\"a\":1}\n\n{\"b\":\"é\"}\n").unwrap();

    let mut source = JsonlSource::open(&path, &IngestConfig::default()).unwrap();
    let records = read_all(&mut source);

    let pointers: Vec<_> = records.iter().map(pointer).collect();
    assert_eq!(pointers, [(0, 7), (8, 0), (9, 10)]);
    assert_eq!(records[2].bytes, "{\"b\":\"é\"}".as_bytes());
}

#[test]
fn lines_longer_than_the_chunk_are_reassembled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let long = format!("{{\"text\":\"{}\"}}", "x".repeat(5_000));
    std::fs::write(&path, format!("{long}\n{{\"b\":2}}\n")).unwrap();
    let cfg = IngestConfig {
        tail_chunk_kb: 1,
        ..IngestConfig::default()
    };

    let mut source = JsonlSource::open(&path, &cfg).unwrap();
    let records = read_all(&mut source);

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].bytes, long.as_bytes());
    assert_eq!(pointer(&records[1]), (long.len() as u64 + 1, 7));
}

#[test]
fn an_incomplete_trailing_line_waits_for_its_newline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, "{\"a\":1}\n{\"b\":").unwrap();

    let mut source = JsonlSource::open(&path, &IngestConfig::default()).unwrap();
    let first = read_all(&mut source);
    assert_eq!(first.len(), 1);

    append(&path, "2}\n");
    assert!(!source.at_eof(), "the file grew");
    let second = read_all(&mut source);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].bytes, b"{\"b\":2}");
    assert_eq!(pointer(&second[0]), (8, 7));
}

#[test]
fn drain_partial_returns_an_unterminated_last_line_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, "{\"a\":1}\n{\"b\":2}").unwrap();

    let mut source = JsonlSource::open(&path, &IngestConfig::default()).unwrap();
    assert_eq!(read_all(&mut source).len(), 1);

    let tail = source.drain_partial();
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].bytes, b"{\"b\":2}");
    assert_eq!(pointer(&tail[0]), (8, 7));
    assert!(source.drain_partial().is_empty());
}

#[test]
fn at_eof_is_false_until_every_line_was_returned() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, "{\"a\":1}\n").unwrap();

    let mut source = JsonlSource::open(&path, &IngestConfig::default()).unwrap();
    assert!(!source.at_eof());
    assert_eq!(source.next_batch().unwrap().len(), 1);
    assert!(source.at_eof());
    assert!(source.next_batch().unwrap().is_empty());
}

#[test]
fn session_id_comes_from_session_start_else_the_file_stem() {
    let dir = tempfile::tempdir().unwrap();
    let started = dir.path().join("renamed-copy.jsonl");
    std::fs::write(
        &started,
        "{\"type\":\"session_start\",\"id\":\"abc-123\"}\n",
    )
    .unwrap();
    let bare = dir.path().join("0f2c.jsonl");
    std::fs::write(&bare, "{\"type\":\"message\"}\n").unwrap();

    let cfg = IngestConfig::default();
    let mut source = JsonlSource::open(&started, &cfg).unwrap();
    assert_eq!(source.root_session_id(), &SessionId("abc-123".to_owned()));
    assert_eq!(read_all(&mut source)[0].session_id.0, "abc-123");

    let source = JsonlSource::open(&bare, &cfg).unwrap();
    assert_eq!(source.root_session_id(), &SessionId("0f2c".to_owned()));
}

#[test]
fn a_file_that_shrinks_is_reported_as_moved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, "{\"a\":1}\n{\"b\":2}\n").unwrap();

    let mut source = JsonlSource::open(&path, &IngestConfig::default()).unwrap();
    read_all(&mut source);
    std::fs::write(&path, "{\"c\":3}\n").unwrap();

    let err = source.next_batch().unwrap_err();
    assert!(matches!(err, IngestError::FileMoved { .. }), "{err}");
    assert!(err.to_string().contains("session.jsonl"), "{err}");
}

#[test]
fn opening_a_missing_file_fails_with_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.jsonl");

    let err = JsonlSource::open(&path, &IngestConfig::default())
        .err()
        .expect("a missing file is an error");

    assert!(matches!(err, IngestError::Io { .. }), "{err}");
    assert!(err.to_string().contains("absent.jsonl"), "{err}");
}

#[test]
fn read_at_returns_the_line_a_pointer_names() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, "{\"a\":1}\n{\"b\":2}\n").unwrap();

    let line = JsonlSource::read_at(&path, 8, 7).unwrap();

    assert_eq!(line, b"{\"b\":2}");
}

#[test]
fn reading_never_changes_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let text = "{\"a\":1}\n{\"b\":2}";
    std::fs::write(&path, text).unwrap();

    let mut source = JsonlSource::open(&path, &IngestConfig::default()).unwrap();
    read_all(&mut source);
    source.drain_partial();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}
