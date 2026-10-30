//! `Notes::from_store`: what the mapped events do not carry and the store does: the check a gate
//! window is about (`rework_windows.check_name`) and the question an app answer closes
//! (`human_actions.payload_ptr`).

mod metrics_support;

use metrics_support::{EventExt, ev, t};
use observatory_core::control::HumanAction;
use observatory_core::metrics::{
    Notes, ReworkKind, WaitKind, events_of_feature, partition, rework_windows,
};
use observatory_core::model::SessionId;
use observatory_core::model::{CycleTemplate, EventKind, FeatureId, MissionId, Role};
use observatory_core::store::{HostedRun, Store};
use rusqlite::{Connection, params};

const MISSION: &str = "mission-1";

fn scratch_store(dir: &tempfile::TempDir) -> (Store, std::path::PathBuf) {
    let db = dir.path().join("observatory.sqlite");
    let mut store = Store::open(&db).expect("store opens");
    store
        .record_hosted(&HostedRun {
            mission_name: "hosted".into(),
            session: SessionId(MISSION.to_owned()),
            spool: dir.path().join("hosted.jsonl"),
            droid_version: None,
            started_at: "2026-10-30T09:00:00.000Z".into(),
        })
        .expect("hosted rows");
    (store, db)
}

#[test]
fn notes_from_store_pair_gate_checks_by_name_and_close_a_question_with_the_app_answer() {
    let template = CycleTemplate::engineering();
    let mission = MissionId(MISSION.to_owned());
    let dir = tempfile::tempdir().unwrap();
    let (mut store, db) = scratch_store(&dir);
    let ask = ev("ask", EventKind::QuestionAsked, 100)
        .feature("f")
        .by(Role::Orchestrator);
    let lint_red = ev("lint-red", EventKind::CheckFailed, 200)
        .feature("f")
        .by(Role::External);
    let gate_red = ev("gate-red", EventKind::CheckFailed, 210)
        .feature("f")
        .by(Role::External);
    let lint_green = ev("lint-green", EventKind::CheckPassed, 300)
        .feature("f")
        .by(Role::External);
    let gate_green = ev("gate-green", EventKind::CheckPassed, 400)
        .feature("f")
        .by(Role::External);
    store
        .insert_events(&[
            ask.clone(),
            lint_red.clone(),
            gate_red.clone(),
            lint_green.clone(),
            gate_green.clone(),
        ])
        .unwrap();
    let recorded = store
        .record_human_action(
            &HumanAction::answer(ask.id.clone(), "yes"),
            &mission,
            t(250),
            &template,
            Some(&ask),
        )
        .unwrap();
    assert_eq!(recorded.event.id.0, format!("app:{}:0", recorded.action_id));
    let conn = Connection::open(&db).unwrap();
    for (red, green, check) in [
        (&lint_red, &lint_green, "lint"),
        (&gate_red, &gate_green, "demo-gate"),
    ] {
        conn.execute(
            "INSERT INTO rework_windows (feature_id, opened_by, closed_by, cause, pr_number, check_name) \
             VALUES ('f', ?1, ?2, 'gate', 1, ?3)",
            params![red.id.0, green.id.0, check],
        )
        .unwrap();
    }
    let events = vec![
        ask.clone(),
        recorded.event.clone(),
        lint_red.clone(),
        gate_red.clone(),
        lint_green.clone(),
        gate_green.clone(),
    ];

    let notes = Notes::from_store(&store, &mission, &events).unwrap();

    assert_eq!(notes.key(&lint_red), Some("lint"));
    assert_eq!(notes.key(&gate_green), Some("demo-gate"));
    let windows = rework_windows(&events, &template, &notes, t(500));
    let kinds: Vec<(ReworkKind, Option<&str>, i64)> = windows
        .iter()
        .map(|w| {
            (
                w.kind,
                w.key.as_deref(),
                (w.to.0 - w.from.0).whole_seconds(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            (ReworkKind::Check, Some("lint"), 100),
            (ReworkKind::Check, Some("demo-gate"), 190)
        ],
        "a pass of another check closes nothing"
    );
    let mine = events_of_feature(&events, &FeatureId("f".to_owned()), &notes);
    let waiting = partition(&mine, t(500))
        .into_iter()
        .find(|i| {
            matches!(
                i.state,
                observatory_core::metrics::TimeState::WaitingHuman(WaitKind::Question)
            )
        })
        .expect("the question is waiting on the human");
    assert_eq!(waiting.to, recorded.event.at, "closed by the app's answer");
}
