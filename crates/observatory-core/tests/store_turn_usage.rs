//! The `board` command reads a store that holds the usage-log turns: its digest is the digest
//! of the plain replay, and the mission's event counts leave the turns out (rule 24 decision
//! 2026-10-24).

#![cfg(feature = "store")]

use std::path::{Path, PathBuf};

use observatory_core::config::{IngestConfig, MissionConfig};
use observatory_core::model::{Event, EventKind};
use observatory_core::store::Store;
use observatory_core::view::{BoardView, DeclaredModels, ViewConfig, branch_of};

fn streams_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/streams")
}

fn golden() -> Vec<Event> {
    let text =
        std::fs::read_to_string(streams_dir().join("2026-10-22-observatory-v1-m0.events.json"))
            .expect("the golden file is committed");
    serde_json::from_str(&text).expect("the golden file is a list of events")
}

fn digest_of(events: &[Event], name: &str) -> observatory_core::view::BoardDigest {
    let mission = MissionConfig::default();
    BoardView::build(
        events,
        &ViewConfig::default(),
        &DeclaredModels::from_config(&mission),
        &branch_of(&mission, name),
    )
    .digest()
}

#[test]
fn a_store_holding_the_turns_gives_the_plain_replay_digest() {
    let dir = tempfile::tempdir().expect("scratch directory");
    let mut store = Store::open(&dir.path().join("observatory.db")).expect("store opens");
    let root = streams_dir().join("2026-10-22-observatory-v1-m0.jsonl");
    let plain = store
        .replay_file(&root, &IngestConfig::default())
        .expect("the capture replays");
    let plain_digest = digest_of(&plain.events, &plain.summary.name);
    assert_eq!(plain.events.len(), 1279);

    let turns: Vec<Event> = golden()
        .into_iter()
        .filter(|event| matches!(event.kind, EventKind::TurnUsage { .. }))
        .collect();
    assert_eq!(store.insert_events(&turns).expect("turns are stored"), 1115);

    let opened = store
        .open_mission(&plain.summary.mission_id)
        .expect("the mission opens");
    assert_eq!(
        opened.events.len(),
        1279 + 1115,
        "the store holds the turns"
    );
    assert_eq!(
        digest_of(&opened.events, &opened.summary.name),
        plain_digest,
        "the board command's digest does not see them"
    );
    assert_eq!(opened.summary.event_count, 1279);
    let rows = store.missions().expect("the library reads");
    assert_eq!(rows[0].event_count, 1279);
}
