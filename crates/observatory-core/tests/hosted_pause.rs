//! A hosted Pause (M3): the runner answers the control request and then reports the cancelled
//! turn, so the spool holds two records that both stand for `mission.paused`, and the app
//! recorded its own human action. One Pause stays one `mission.paused` row; the human action
//! row is untouched (D-035 overlays).

#![cfg(all(feature = "rpc", feature = "store"))]

use std::path::Path;

use observatory_core::config::IngestConfig;
use observatory_core::control::{self, HumanAction};
use observatory_core::ingest::rpc::{RpcMapper, RpcSource, Spool, spool_path};
use observatory_core::ingest::{RawRecord, Source};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventKind, MissionId, Overlay, SessionId, Timestamp,
};
use observatory_core::store::{EventScope, HostedRun, Store, TimeRange};
use serde_json::{Value, json};

const LAUNCH: &str = "launch-0001";
const PAUSE_REQUEST: &str = "pause-1";

fn line(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).unwrap();
    bytes.push(b'\n');
    bytes
}

fn response(id: &str, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "type": "response", "factoryApiVersion": "1.0.0",
        "factoryProtocolVersion": "1.274.0", "id": id, "result": result,
    })
}

fn cancelled_turn() -> Value {
    json!({
        "jsonrpc": "2.0", "type": "notification", "factoryApiVersion": "1.0.0",
        "factoryProtocolVersion": "1.274.0", "method": "droid.session_notification",
        "params": { "sessionId": "s-1", "notification": {
            "type": "agent_turn_completed", "reason": "cancelled", "turnId": "t-1",
        } },
    })
}

/// Spools the lines of one hosted Pause and returns the records read back.
fn spooled_pause(dir: &Path) -> Vec<RawRecord> {
    let path = spool_path(dir, LAUNCH);
    let mut spool = Spool::create(&path).unwrap();
    for value in [
        response("1", json!({ "sessionId": "s-1", "settings": {} })),
        response(PAUSE_REQUEST, json!({})),
        cancelled_turn(),
    ] {
        spool.append_line(line(&value).trim_ascii_end()).unwrap();
    }
    let mut source = RpcSource::open(&path, &IngestConfig::default()).unwrap();
    let mut records = source.next_batch().unwrap();
    records.extend(source.drain_partial());
    records
}

fn paused(events: &[Event]) -> Vec<&Event> {
    events
        .iter()
        .filter(|event| event.kind == EventKind::MissionPaused)
        .collect()
}

#[test]
fn one_hosted_pause_records_one_mission_paused_row_and_keeps_the_human_action() {
    let dir = tempfile::tempdir().unwrap();
    let template = CycleTemplate::engineering();
    let mission = MissionId(LAUNCH.to_owned());
    let session = SessionId(LAUNCH.to_owned());
    let mut store = Store::open(&dir.path().join("observatory.sqlite")).unwrap();
    store
        .record_hosted(&HostedRun {
            mission_name: "hosted".into(),
            session: session.clone(),
            spool: spool_path(dir.path(), LAUNCH),
            droid_version: None,
            started_at: "2026-10-29T09:00:00.000Z".into(),
        })
        .unwrap();
    let human = store
        .record_human_action(
            &HumanAction::Pause,
            &mission,
            Timestamp::parse("2026-10-29T09:10:00.000Z").unwrap(),
            &template,
            None,
        )
        .unwrap();

    let mut mapper = RpcMapper::new(LAUNCH, MappingVersion::LATEST);
    let mut removed = Vec::new();
    for record in spooled_pause(dir.path()) {
        let mut events = mapper.map(&record);
        let acknowledges_pause = String::from_utf8_lossy(&record.bytes).contains(PAUSE_REQUEST);
        if acknowledges_pause {
            events.extend(control::ack_event(
                &HumanAction::Pause,
                &mission,
                &session,
                record.arrived_at.unwrap_or_else(Timestamp::now),
                record.ptr.clone(),
                &template,
                None,
            ));
        }
        store.insert_events(&events).unwrap();
        removed.extend(store.retract_echoed_pauses(&mission).unwrap());
    }

    let stored = store
        .events(&EventScope::Mission(mission.clone()), TimeRange::ALL)
        .unwrap();
    let pauses = paused(&stored);
    assert_eq!(pauses.len(), 1, "{pauses:?}");
    assert!(
        !control::is_ack(&pauses[0].id),
        "the stream's own row stays, not the ack's: {}",
        pauses[0].id
    );
    assert_eq!(removed.len(), 1, "one ack row was retracted: {removed:?}");
    assert!(control::is_ack(&removed[0]), "{}", removed[0]);
    let overlay = stored
        .iter()
        .find(|event| event.id == human.event.id)
        .expect("the human action's overlay event is still stored");
    assert_eq!(overlay.overlay, Some(Overlay::Human));
    assert_eq!(store.human_actions(&mission).unwrap().len(), 1);
}
