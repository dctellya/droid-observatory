//! Both hosts read the usage-log extract the capture header names, so the Panel's usage block
//! and the metrics have the turns (rule 24 decision, 2026-10-24; F-034).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use observatory_core::ingest::replay_files;
use observatory_core::model::{Event, EventKind};

const M0: &str = "2026-10-22-observatory-v1-m0";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden_turns() -> Vec<Event> {
    let text =
        std::fs::read_to_string(repo_root().join(format!("fixtures/streams/{M0}.events.json")))
            .expect("the golden file is committed");
    let events: Vec<Event> = serde_json::from_str(&text).expect("a list of events");
    events
        .into_iter()
        .filter(|event| matches!(event.kind, EventKind::TurnUsage { .. }))
        .collect()
}

type Counts = (u64, u64, u64, Option<String>, String);
type Shape = (
    String,
    Counts,
    String,
    String,
    Option<String>,
    Option<String>,
);

/// What a turn is, without the path its pointer names (the host's own copy of the extract) and
/// without `reasoning` and `total_input`, which the store does not keep.
fn shapes(events: &[Event]) -> Vec<Shape> {
    let mut found: Vec<Shape> = events
        .iter()
        .filter_map(|event| {
            let EventKind::TurnUsage {
                input,
                output,
                cache_read,
                model,
                log_ts,
                ..
            } = &event.kind
            else {
                return None;
            };
            Some((
                event.id.0.clone(),
                (
                    *input,
                    *output,
                    *cache_read,
                    model.as_ref().map(ToString::to_string),
                    log_ts.to_rfc3339_millis(),
                ),
                event.at.to_rfc3339_millis(),
                event.lane.to_string(),
                event.feature_id.as_ref().map(ToString::to_string),
                event.milestone_id.as_ref().map(ToString::to_string),
            ))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[cfg(feature = "store")]
#[test]
fn app_replay_loads_the_usage_log_named_in_the_capture_header() {
    use observatory_core::config::IngestConfig;
    use observatory_core::store::Store;

    let dir = tempfile::tempdir().expect("scratch directory");
    let mut store = Store::open(&dir.path().join("observatory.db")).expect("store opens");
    let root = repo_root().join(format!("fixtures/streams/{M0}.jsonl"));

    let opened = store
        .replay_file(&root, &IngestConfig::default())
        .expect("the capture replays");

    let expected = shapes(&golden_turns());
    assert_eq!(expected.len(), 1115);
    assert_eq!(
        shapes(&opened.events),
        expected,
        "the replay holds the turns"
    );
    assert_eq!(opened.summary.warning, None);
    assert_eq!(
        opened.summary.event_count, 1279,
        "the turns are not board events"
    );

    let again = store
        .replay_file(&root, &IngestConfig::default())
        .expect("a second replay");
    assert_eq!(
        again.events.len(),
        opened.events.len(),
        "replaying twice adds nothing"
    );

    let reopened = store
        .open_mission(&opened.summary.mission_id)
        .expect("the mission reopens from its rows");
    assert!(shapes(&reopened.events) == expected, "the store keeps them");
}

/// The files of the M0 capture as a picker hands them over, with or without the extract.
fn picked(with_usage_log: bool) -> BTreeMap<PathBuf, Vec<u8>> {
    let dir = Path::new("fixtures/streams");
    let mut files = BTreeMap::new();
    let mut take = |relative: PathBuf| {
        let bytes = std::fs::read(repo_root().join(&relative)).expect("committed");
        files.insert(relative, bytes);
    };
    for extension in ["jsonl", "settings.json", "capture.json"] {
        take(dir.join(format!("{M0}.{extension}")));
    }
    if with_usage_log {
        take(dir.join(format!("{M0}.usage-log.jsonl")));
    }
    let children = dir.join(format!("{M0}.children"));
    for entry in std::fs::read_dir(repo_root().join(&children)).expect("children") {
        take(children.join(entry.expect("an entry").file_name()));
    }
    files
}

#[test]
fn browser_replay_loads_the_usage_log_named_in_the_capture_header() {
    let replay = replay_files(picked(true)).expect("the capture opens");

    assert!(shapes(&replay.events) == shapes(&golden_turns()));
    assert_eq!(
        replay.summary.event_count, 1279,
        "the turns are not board events"
    );
    assert_eq!(
        replay.summary.session_count, 56,
        "the extract is not a session"
    );
    assert_eq!(replay.summary.warning, None);
    assert_eq!(replay.summary.unknown_count, 0);
}

#[test]
fn a_selection_without_the_named_usage_log_replays_without_turns_and_says_so() {
    let replay = replay_files(picked(false)).expect("the capture opens");

    assert!(shapes(&replay.events).is_empty());
    assert_eq!(replay.summary.event_count, 1279);
    let warning = replay
        .summary
        .warning
        .expect("the missing extract is named");
    assert!(
        warning.contains(&format!("{M0}.usage-log.jsonl")),
        "{warning}"
    );
}
