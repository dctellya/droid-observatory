//! Migration `0002_lane_text.sql`: the store lane column goes from the position 1..7 to the
//! template lane id, and the run-level overlay is spelled `officer` (F-041, D-035).

#![cfg(feature = "store")]

use std::path::Path;

use observatory_core::model::{LaneId, Overlay};
use observatory_core::store::{EventScope, Store, TimeRange};
use rusqlite::{Connection, params};

const INIT: &str = include_str!("../src/store/migrations/0001_init.sql");
const IDS: [&str; 7] = [
    "plan",
    "contract",
    "build",
    "scrutiny",
    "user_testing",
    "integrate",
    "gate",
];

/// A store as the M2 binary left it: only `0001` applied, one event per lane number.
fn store_before_the_feature(db: &Path) {
    let conn = Connection::open(db).unwrap();
    conn.execute_batch(INIT).unwrap();
    conn.execute_batch(
        "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
         INSERT INTO schema_version VALUES (1, '2026-10-24T00:00:00.000Z');
         INSERT INTO mapping_versions VALUES ('v1', '0.247.*', '2026-10-23');
         INSERT INTO missions (id, name, state, started_at) VALUES ('m', 'm', 'building', 't');",
    )
    .unwrap();
    for number in 1..=7_i64 {
        let overlay = (number == 1).then_some("commander");
        conn.execute(
            "INSERT INTO events (id, mission_id, lane, overlay, kind, mapping_version, role, at, \
             source, path, byte_offset, byte_len, sub) \
             VALUES (?1, 'm', ?2, ?3, 'intent.created', 'v1', 'orchestrator', \
             '2026-10-22T10:00:00.000Z', 'jsonl', 'a.jsonl', ?2, 1, 0)",
            params![format!("s:{number}:0"), number, overlay],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO usage VALUES ('s:3:0', 10, 20, 30, 0, NULL)",
        [],
    )
    .unwrap();
}

#[test]
fn migration_0002_converts_seven_lane_numbers_to_their_ids() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("observatory.sqlite");
    store_before_the_feature(&db);

    let store = Store::open(&db).unwrap();
    let events = store
        .events(
            &EventScope::Mission(observatory_core::model::MissionId("m".into())),
            TimeRange::ALL,
        )
        .unwrap();

    let mut lanes: Vec<(String, u64)> = events
        .iter()
        .map(|event| (event.lane.to_string(), 1))
        .collect();
    lanes.sort();
    let mut wanted: Vec<(String, u64)> = IDS.iter().map(|id| ((*id).to_owned(), 1)).collect();
    wanted.sort();
    assert_eq!(lanes, wanted, "one row per lane, now by id");

    let by_number: Vec<LaneId> = {
        let mut ordered: Vec<_> = events.iter().collect();
        ordered.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        ordered.iter().map(|event| event.lane.clone()).collect()
    };
    assert_eq!(
        by_number,
        IDS.map(LaneId::from),
        "position n became the nth id"
    );

    let first = events
        .iter()
        .find(|e| e.lane == LaneId::from("plan"))
        .unwrap();
    assert_eq!(
        first.overlay,
        Some(Overlay::Officer),
        "commander rows read as officer"
    );

    let conn = Connection::open(&db).unwrap();
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        version, 4,
        "0002 converts the lanes, 0003 and 0004 follow it"
    );
    let usage: i64 = conn
        .query_row("SELECT COUNT(*) FROM usage", [], |r| r.get(0))
        .unwrap();
    assert_eq!(usage, 1, "the usage row survived the table rebuild");
    let violations: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(violations, 0);
    let overlays: Vec<Option<String>> = conn
        .prepare("SELECT DISTINCT overlay FROM events ORDER BY overlay")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(overlays, [None, Some("officer".to_owned())]);
    let indexes: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'events' \
             AND name LIKE 'events_%' ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        indexes,
        ["events_feature", "events_mission_at", "events_ptr"]
    );
}

#[test]
fn the_events_table_takes_any_lane_id_and_refuses_the_legacy_overlay() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("observatory.sqlite");
    drop(Store::open(&db).unwrap());
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
    let insert = |lane: &str, overlay: Option<&str>| {
        conn.execute(
            "INSERT INTO events (id, mission_id, lane, overlay, kind, mapping_version, role, at, source) \
             VALUES (?1, 'm', ?2, ?3, 'x', 'v1', 'worker', 't', 'app')",
            params![format!("e{lane}{overlay:?}"), lane, overlay],
        )
    };
    assert!(insert("a_ninth_lane", Some("officer")).is_ok());
    assert!(insert("plan", Some("human")).is_ok());
    assert!(
        insert("plan", Some("commander")).is_err(),
        "writers spell officer"
    );
    assert!(insert("plan", Some("robot")).is_err());
    let cause = conn.execute(
        "INSERT INTO rework_windows (feature_id, opened_by, cause) VALUES ('f', 'e', 'any_lane')",
        [],
    );
    assert!(cause.is_ok(), "rework_windows.cause holds a lane id");
}

#[test]
fn a_store_read_without_the_migration_shows_the_old_position_as_an_unknown_id() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("observatory.sqlite");
    store_before_the_feature(&db);

    let store = Store::open_existing(&db).unwrap();
    let event = store
        .event(&observatory_core::model::EventId("s:3:0".into()))
        .unwrap();
    assert_eq!(event.lane, LaneId::from("3"));
    assert_eq!(
        observatory_core::model::CycleTemplate::engineering().lane_text(&event.lane),
        "3"
    );
}
