//! Replay of files held in memory, the way the browser host reads a file-picker selection:
//! no directory, no store, the same mapping as the CLI (architecture §7 `BrowserBackend`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use observatory_core::ingest::replay_files;
use observatory_core::model::{Event, SourcePtr};
use observatory_core::view::{BoardState, ViewConfig, tray_count};

const M0: &str = "2026-10-22-observatory-v1-m0";
const S: &str = "2026-10-22-observatory-v1-s";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The files of one capture as a picker hands them over: `fixtures/streams/<slug>.*` and
/// `fixtures/streams/<slug>.children/*`. With `flat`, only the file names are kept, as with a
/// multi-file selection.
fn capture_files(slug: &str, flat: bool) -> BTreeMap<PathBuf, Vec<u8>> {
    let dir = Path::new("fixtures/streams");
    let mut files = BTreeMap::new();
    let mut take = |relative: PathBuf| {
        let bytes = std::fs::read(repo_root().join(&relative)).expect("the fixture is committed");
        let name = if flat {
            PathBuf::from(relative.file_name().expect("a file"))
        } else {
            relative
        };
        files.insert(name, bytes);
    };
    for extension in ["jsonl", "settings.json", "capture.json"] {
        take(dir.join(format!("{slug}.{extension}")));
    }
    let children = dir.join(format!("{slug}.children"));
    for entry in std::fs::read_dir(repo_root().join(&children)).expect("the children exist") {
        let name = entry.expect("an entry").file_name();
        take(children.join(name));
    }
    files
}

fn golden() -> Vec<Event> {
    let text =
        std::fs::read_to_string(repo_root().join(format!("fixtures/streams/{M0}.events.json")))
            .expect("the golden file is committed");
    let events: Vec<Event> =
        serde_json::from_str(&text).expect("the golden file is a list of events");
    // Plain replay never reads the usage log; its turn.usage events are checked in usage_log.rs.
    events
        .into_iter()
        .filter(|e| e.kind.prd_name() != "turn.usage")
        .collect()
}

#[test]
fn the_m0_files_replay_from_memory_to_the_golden_events() {
    let replay = replay_files(capture_files(M0, false)).expect("the M0 capture opens");

    assert_eq!(replay.events.len(), 1279);
    assert_eq!(replay.events, golden());
    let summary = &replay.summary;
    assert_eq!(summary.name, "observatory-v1");
    assert_eq!(summary.event_count, 1279);
    assert_eq!(summary.unknown_count, 0);
    assert_eq!(summary.record_count, 8735);
    assert_eq!(summary.session_count, 56);
    assert_eq!(summary.mapping_version, "v1");
    assert_eq!(summary.warning, None);
    assert_eq!(summary.started_at, "2026-10-23T09:21:59Z");
}

#[test]
fn the_s_files_replay_from_memory_with_no_unknown_events() {
    let replay = replay_files(capture_files(S, false)).expect("the S capture opens");

    assert_eq!(replay.summary.event_count, 661);
    assert_eq!(replay.summary.unknown_count, 0);
    assert_eq!(replay.summary.record_count, 4636);
}

#[test]
fn a_selection_without_directories_finds_the_same_tree() {
    let nested = replay_files(capture_files(M0, false)).expect("nested");
    let flat = replay_files(capture_files(M0, true)).expect("flat");

    assert_eq!(flat.summary.event_count, nested.summary.event_count);
    assert_eq!(flat.summary.session_count, 56);
    assert_eq!(flat.summary.name, "observatory-v1");
    let ids = |events: &[Event]| -> Vec<String> {
        events.iter().map(|event| event.id.0.clone()).collect()
    };
    assert_eq!(ids(&flat.events), ids(&nested.events));
}

#[test]
fn unknown_lines_are_counted_and_their_raw_line_opens_from_the_pointer() {
    let mut files = capture_files(S, false);
    let root = PathBuf::from(format!("fixtures/streams/{S}.jsonl"));
    let appended = ["this is not json", r#"{"type":"future_kind"}"#];
    let bytes = files.get_mut(&root).expect("the root is present");
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    for line in appended {
        bytes.extend_from_slice(line.as_bytes());
        bytes.push(b'\n');
    }

    let replay = replay_files(files).expect("the copy opens");

    assert_eq!(replay.summary.unknown_count, 2);
    assert_eq!(tray_count(&replay.events), 2);
    let board = BoardState::fold(&replay.events, &ViewConfig::default());
    assert_eq!(board.tray_unknown, 2);
    let raw: Vec<&[u8]> = board
        .tray
        .iter()
        .map(|entry| replay.raw_line(&entry.pointer).expect("the pointer reads"))
        .collect();
    assert_eq!(raw, [appended[0].as_bytes(), appended[1].as_bytes()]);
    let raw_types: Vec<&str> = board.tray.iter().map(|e| e.raw_type.as_str()).collect();
    assert_eq!(raw_types, ["<unparseable>", "future_kind"]);
}

#[test]
fn a_pointer_outside_the_selection_has_no_raw_line() {
    let replay = replay_files(capture_files(S, false)).expect("the S capture opens");
    let lost = SourcePtr::Jsonl {
        path: PathBuf::from("elsewhere.jsonl"),
        byte_offset: 0,
        byte_len: 3,
    };

    assert_eq!(replay.raw_line(&lost), None);
    let beyond = SourcePtr::Jsonl {
        path: PathBuf::from(format!("fixtures/streams/{S}.jsonl")),
        byte_offset: u64::MAX - 1,
        byte_len: 9,
    };
    assert_eq!(replay.raw_line(&beyond), None);
    assert_eq!(replay.raw_line(&SourcePtr::App { action_id: 1 }), None);
}

#[test]
fn a_selection_with_no_root_session_is_refused() {
    let mut files = capture_files(S, false);
    files.retain(|path, _| !path.to_string_lossy().ends_with(&format!("{S}.jsonl")));
    files.retain(|path, _| !path.to_string_lossy().contains(".children/"));

    let error = replay_files(files).expect_err("no session file is left");

    assert!(error.to_string().contains("no session file"), "{error}");
    assert!(replay_files(BTreeMap::new()).is_err());
}

#[test]
fn a_capture_header_for_another_droid_version_warns_and_maps_with_the_latest() {
    let mut files = capture_files(S, false);
    let capture = PathBuf::from(format!("fixtures/streams/{S}.capture.json"));
    let text = String::from_utf8(files[&capture].clone()).expect("utf-8");
    files.insert(capture, text.replace("0.247.0", "9.9.9").into_bytes());

    let replay = replay_files(files).expect("the capture opens");

    let warning = replay.summary.warning.expect("a warning");
    assert!(warning.contains("unknown Droid version 9.9.9"), "{warning}");
    assert_eq!(replay.summary.mapping_version, "v1");
    assert_eq!(replay.summary.unknown_count, 0);
}

/// The replay of one session file whose sidecar carries `tags_json`.
fn replay_tagged(tags_json: &str) -> observatory_core::ingest::MemoryReplay {
    let files = BTreeMap::from([
        (
            PathBuf::from("probe/p-1.jsonl"),
            b"{\"type\":\"session_start\",\"id\":\"p-1\",\"title\":\"t\"}\n".to_vec(),
        ),
        (
            PathBuf::from("probe/p-1.settings.json"),
            format!(r#"{{"tags":{tags_json}}}"#).into_bytes(),
        ),
    ]);
    replay_files(files).expect("the session opens")
}

#[test]
fn memory_replay_names_sessions_by_the_same_rule() {
    let tagged = replay_tagged(r#"[{"name":"exec"},{"name":"t"}]"#);
    assert_eq!(tagged.summary.name, "t");
    assert_eq!(tagged.sessions[0].tag.as_deref(), Some("t"));

    let runner_only = replay_tagged(r#"[{"name":"exec"}]"#);
    assert_eq!(runner_only.summary.name, "p-1");
    assert_eq!(runner_only.sessions[0].tag.as_deref(), Some("p-1"));
}
