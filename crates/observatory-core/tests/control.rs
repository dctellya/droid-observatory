//! Controls (architecture §8, F-028): every human action is one human-overlay event and one
//! `human_actions` row; stall detection reads its threshold from the config.
//!
//! Lanes are addressed through the default engineering template's `LaneId`s (F-041, D-035):
//! the first lane of the template for mission-level controls, the request's lane for
//! approve and deny.

use std::path::PathBuf;
use std::time::Duration;

use observatory_core::config::{Config, InterventionConfig};
use observatory_core::control::stall::{self, FeatureActivity};
use observatory_core::control::{self, HumanAction, RunbookPrompt};
use observatory_core::ingest::rpc::HostControl;
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, MilestoneId, MissionId, ModelId, Overlay,
    Role, SessionId, SourcePtr, Timestamp,
};
use observatory_core::store::{EventScope, HostedRun, Store, TimeRange};
use observatory_core::view::{BoardState, DeclaredModels, ViewConfig, details};

const SECRET_MESSAGE: &str = "zq-message-text-that-must-not-persist";
const SECRET_ANSWER: &str = "zq-answer-text-that-must-not-persist";
const SECRET_REASON: &str = "zq-skip-reason-that-must-not-persist";

fn template() -> CycleTemplate {
    CycleTemplate::engineering()
}

fn mission() -> MissionId {
    MissionId("mission-1".to_owned())
}

fn at(minute: u32, second: u32) -> Timestamp {
    Timestamp::parse(&format!("2026-10-25T10:{minute:02}:{second:02}.000Z")).expect("valid time")
}

/// An event of the mission's own stream: a question, a permission request, a stall candidate.
fn stream_event(kind: EventKind, role: Role, minute: u32, offset: u64) -> Event {
    let template = template();
    Event {
        id: EventId(format!("mission-1:{offset}:0")),
        mission_id: mission(),
        milestone_id: Some(MilestoneId("m3".to_owned())),
        feature_id: Some(FeatureId("m3-control-core".to_owned())),
        lane: template.lane_of(&kind, role),
        overlay: kind.overlay(),
        kind,
        role,
        session_id: Some(SessionId("mission-1".to_owned())),
        model: None,
        at: at(minute, 0),
        duration: None,
        usage: None,
        source: SourcePtr::Jsonl {
            path: PathBuf::from("fixtures/session-1.jsonl"),
            byte_offset: offset,
            byte_len: 80,
        },
        mapping_version: MappingVersion::V1,
    }
}

fn question() -> Event {
    stream_event(EventKind::QuestionAsked, Role::Orchestrator, 1, 100)
}

/// A permission request raised by a worker: it sits in the worker's home lane, not the first.
fn request() -> Event {
    stream_event(EventKind::PermissionRequested, Role::Worker, 2, 200)
}

fn scratch_store(dir: &tempfile::TempDir) -> Store {
    let mut store = Store::open(&dir.path().join("observatory.sqlite")).expect("store opens");
    store
        .record_hosted(&HostedRun {
            mission_name: "hosted".into(),
            session: SessionId("mission-1".to_owned()),
            spool: dir.path().join("hosted.jsonl"),
            droid_version: None,
            started_at: "2026-10-25T09:00:00.000Z".into(),
        })
        .expect("hosted rows");
    store
}

fn the_eight_actions() -> Vec<HumanAction> {
    vec![
        HumanAction::Pause,
        HumanAction::Resume,
        HumanAction::message(SECRET_MESSAGE),
        HumanAction::answer(question().id, SECRET_ANSWER),
        HumanAction::Approve {
            request_event: request().id,
        },
        HumanAction::Deny {
            request_event: request().id,
        },
        HumanAction::MarkComplete {
            feature_id: FeatureId("m3-control-core".to_owned()),
        },
        HumanAction::mark_skipped(FeatureId("m3-e2e-harness".to_owned()), SECRET_REASON),
    ]
}

fn request_for(action: &HumanAction) -> Option<Event> {
    match action {
        HumanAction::Answer { .. } => Some(question()),
        HumanAction::Approve { .. } | HumanAction::Deny { .. } => Some(request()),
        _ => None,
    }
}

#[test]
fn each_control_emits_one_human_overlay_event_with_actor_human() {
    let template = template();
    let dir = tempfile::tempdir().unwrap();
    let mut store = scratch_store(&dir);
    store
        .insert_events(&[question(), request()])
        .expect("stream events");

    for action in the_eight_actions() {
        let request = request_for(&action);
        let done = store
            .record_human_action(&action, &mission(), at(5, 0), &template, request.as_ref())
            .expect("recorded");

        let event = &done.event;
        assert_eq!(event.overlay, Some(Overlay::Human), "{action:?}");
        assert_eq!(event.role, Role::Human, "{action:?}");
        assert_eq!(
            event.source,
            SourcePtr::App {
                action_id: done.action_id
            },
            "{action:?}"
        );
        let expected_lane = match &action {
            HumanAction::Approve { .. } | HumanAction::Deny { .. } => {
                request.as_ref().unwrap().lane.clone()
            }
            _ => template.first().clone(),
        };
        assert_eq!(event.lane, expected_lane, "{action:?}");
        let expected_kind = match &action {
            HumanAction::Answer { .. } => EventKind::QuestionAnswered,
            _ => EventKind::ApprovalGiven,
        };
        assert_eq!(event.kind, expected_kind, "{action:?}");
    }

    let app_events: Vec<Event> = store
        .events(&EventScope::Mission(mission()), TimeRange::ALL)
        .unwrap()
        .into_iter()
        .filter(|event| matches!(event.source, SourcePtr::App { .. }))
        .collect();
    assert_eq!(app_events.len(), 8, "one event per control");

    let rows = store.human_actions(&mission()).unwrap();
    let table: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| (row.kind.as_str(), row.state.as_str()))
        .collect();
    assert_eq!(
        table,
        [
            ("pause", "sent"),
            ("resume", "sent"),
            ("message", "sent"),
            ("answer", "sent"),
            ("approve", "granted"),
            ("deny", "denied"),
            ("mark_complete", "sent"),
            ("mark_skipped", "sent"),
        ]
    );
    let answer = &rows[3];
    assert_eq!(
        answer.payload_ptr.as_deref(),
        Some(question().id.0.as_str())
    );
    assert_eq!(
        rows[4].payload_ptr.as_deref(),
        Some(request().id.0.as_str())
    );
    assert_eq!(
        rows[5].payload_ptr.as_deref(),
        Some(request().id.0.as_str())
    );
    assert_eq!(rows[6].target.as_deref(), Some("m3-control-core"));
    assert_eq!(rows[7].target.as_deref(), Some("m3-e2e-harness"));
    for (row, event) in rows.iter().zip(&app_events) {
        assert_eq!(
            event.source,
            SourcePtr::App { action_id: row.id },
            "the event points at its row"
        );
    }
}

#[test]
fn record_builds_the_event_without_a_store() {
    let template = template();
    let event = control::record(
        &HumanAction::Pause,
        &mission(),
        at(3, 0),
        7,
        &template,
        None,
    );

    assert_eq!(event.source, SourcePtr::App { action_id: 7 });
    assert_eq!(event.lane, *template.first());
    assert_eq!(event.session_id, None);
    assert_eq!(event.mapping_version, MappingVersion::LATEST);
}

#[test]
fn stall_flag_raises_after_threshold_of_no_active_time() {
    let threshold = Duration::from_secs(10 * 60);
    let activity = [FeatureActivity {
        feature_id: FeatureId("m3-control-core".to_owned()),
        last_active_at: at(0, 0),
        last_kind: EventKind::ToolError,
        repeated: false,
        child_growing: false,
    }];

    let flags = stall::evaluate(&activity, at(10, 0), threshold);

    assert_eq!(flags.len(), 1);
    assert_eq!(flags[0].feature_id, FeatureId("m3-control-core".to_owned()));
    assert_eq!(flags[0].idle_for, Duration::from_secs(10 * 60));

    let later = stall::evaluate(&activity, at(15, 30), threshold);
    assert_eq!(later[0].idle_for, Duration::from_secs(15 * 60 + 30));
}

#[test]
fn stall_flag_suggests_the_matching_runbook() {
    let threshold = Duration::from_secs(60);
    let flag_for = |last_kind: EventKind, repeated: bool, child_growing: bool| -> RunbookPrompt {
        let activity = [FeatureActivity {
            feature_id: FeatureId("f".to_owned()),
            last_active_at: at(0, 0),
            last_kind,
            repeated,
            child_growing,
        }];
        let mut flags = stall::evaluate(&activity, at(30, 0), threshold);
        assert_eq!(flags.len(), 1);
        flags.remove(0).runbook
    };

    let waiting = ("Waiting on human: answer it", "#waiting-on-human");
    for kind in [EventKind::QuestionAsked, EventKind::PermissionRequested] {
        let prompt = flag_for(kind, false, false);
        assert_eq!((prompt.prompt, prompt.anchor), waiting);
    }
    let silent = flag_for(EventKind::SubagentSpawned, false, false);
    assert_eq!(
        (silent.prompt, silent.anchor),
        (
            "Worker silent: check the child file, retry or skip",
            "#worker-silent"
        )
    );
    let blocked = flag_for(EventKind::HookBlocked, true, false);
    assert_eq!(
        (blocked.prompt, blocked.anchor),
        (
            "Agent hits the write guard: forward-only instruction",
            "#hook-blocked"
        )
    );
    let conflict = flag_for(EventKind::IntegrationConflict, false, false);
    assert_eq!(
        (conflict.prompt, conflict.anchor),
        ("Two worktrees touched one file", "#integration-conflict")
    );
    let long_tool = flag_for(EventKind::TestsRun, false, false);
    assert_eq!(
        (long_tool.prompt, long_tool.anchor),
        (
            "Long tool call: wait one more threshold, then pause and inspect",
            "#long-tool-call"
        )
    );
    // A single hook block and a spawn whose child still grows are the generic case.
    assert_eq!(
        flag_for(EventKind::HookBlocked, false, false).anchor,
        "#long-tool-call"
    );
    assert_eq!(
        flag_for(EventKind::SubagentSpawned, false, true).anchor,
        "#long-tool-call"
    );
}

#[test]
fn no_alert_for_waits_under_threshold() {
    let threshold = Duration::from_secs(20 * 60);
    let activity = [
        FeatureActivity {
            feature_id: FeatureId("fresh".to_owned()),
            last_active_at: at(59, 0),
            last_kind: EventKind::QuestionAsked,
            repeated: false,
            child_growing: false,
        },
        FeatureActivity {
            feature_id: FeatureId("just-under".to_owned()),
            last_active_at: at(40, 1),
            last_kind: EventKind::TestsRun,
            repeated: false,
            child_growing: false,
        },
    ];

    let flags = stall::evaluate(&activity, at(59, 59), threshold);

    assert!(flags.is_empty(), "{flags:?}");
    assert!(stall::evaluate(&[], at(59, 59), threshold).is_empty());
}

#[test]
fn stall_threshold_comes_from_the_config() {
    let default = Config::default();
    assert_eq!(default.intervention.stall_threshold_min, 20);
    assert_eq!(
        stall::threshold(&default.intervention),
        Duration::from_secs(20 * 60)
    );
    let short = InterventionConfig {
        stall_threshold_min: 1,
        ..InterventionConfig::default()
    };
    assert_eq!(stall::threshold(&short), Duration::from_secs(60));
}

#[test]
fn activity_leaves_out_finished_features_and_counts_repeated_hook_blocks() {
    let mut done = stream_event(EventKind::TaskDone, Role::Worker, 0, 10);
    done.feature_id = Some(FeatureId("done".to_owned()));
    let mut blocked_a = stream_event(EventKind::HookBlocked, Role::Worker, 1, 20);
    blocked_a.feature_id = Some(FeatureId("stuck".to_owned()));
    let mut blocked_b = stream_event(EventKind::HookBlocked, Role::Worker, 2, 30);
    blocked_b.feature_id = Some(FeatureId("stuck".to_owned()));

    let activity = stall::activity_of(&[done, blocked_a, blocked_b]);

    assert_eq!(activity.len(), 1);
    assert_eq!(activity[0].feature_id, FeatureId("stuck".to_owned()));
    assert_eq!(activity[0].last_active_at, at(2, 0));
    assert!(activity[0].repeated);
}

#[test]
fn mark_skipped_relays_as_an_instruction_to_the_orchestrator() {
    let template = template();
    let feature = FeatureId("m3-e2e-harness".to_owned());
    let action = HumanAction::mark_skipped(feature.clone(), SECRET_REASON);

    let event = control::record(&action, &mission(), at(5, 0), 3, &template, None);
    let relay = control::relay(&action, Some(SECRET_REASON)).expect("a skip is relayed");

    assert_eq!(event.overlay, Some(Overlay::Human));
    assert_eq!(event.feature_id, Some(feature));
    match relay {
        HostControl::Message { text } => {
            assert!(text.contains("m3-e2e-harness"), "{text}");
            assert!(text.contains(SECRET_REASON), "{text}");
            assert!(text.to_lowercase().contains("skip"), "{text}");
        }
        other => panic!("a skip relays as a message to the orchestrator, got {other:?}"),
    }
    assert_eq!(
        control::relay(&HumanAction::Pause, None),
        Some(HostControl::Pause)
    );
    assert_eq!(
        control::relay(&HumanAction::Resume, None),
        Some(HostControl::Resume)
    );
    assert_eq!(
        control::relay(
            &HumanAction::MarkComplete {
                feature_id: FeatureId("x".into())
            },
            None
        ),
        None,
        "mark complete is a record only"
    );
}

#[test]
fn free_text_is_stored_as_length_only() {
    let template = template();
    let dir = tempfile::tempdir().unwrap();
    let mut store = scratch_store(&dir);
    store.insert_events(&[question(), request()]).unwrap();
    for action in the_eight_actions() {
        let request = request_for(&action);
        store
            .record_human_action(&action, &mission(), at(5, 0), &template, request.as_ref())
            .unwrap();
    }

    let message = HumanAction::message(SECRET_MESSAGE);
    assert_eq!(
        message,
        HumanAction::Message {
            text_len: u32::try_from(SECRET_MESSAGE.chars().count()).unwrap()
        }
    );
    let payload = serde_json::to_string(&message).unwrap();
    assert!(!payload.contains(SECRET_MESSAGE), "{payload}");
    assert!(payload.contains("text_len"), "{payload}");

    let db = dir.path().join("observatory.sqlite");
    let conn = rusqlite::Connection::open(&db).unwrap();
    let columns: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('human_actions') ORDER BY cid")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        columns,
        [
            "id",
            "mission_id",
            "kind",
            "target",
            "at",
            "state",
            "payload_ptr"
        ]
    );
    drop(conn);

    for file in ["observatory.sqlite", "observatory.sqlite-wal"] {
        let Ok(bytes) = std::fs::read(dir.path().join(file)) else {
            continue;
        };
        for text in [SECRET_MESSAGE, SECRET_ANSWER, SECRET_REASON] {
            assert!(
                !bytes
                    .windows(text.len())
                    .any(|window| window == text.as_bytes()),
                "{text} found in {file}"
            );
        }
    }
}

#[test]
fn host_acks_become_overlay_events_pointing_at_the_ack_line() {
    let template = template();
    let spool = SourcePtr::Rpc {
        path: PathBuf::from("hosted/launch-1.jsonl"),
        byte_offset: 4096,
        byte_len: 64,
    };
    let session = SessionId("mission-1".to_owned());
    let cases = [
        (HumanAction::Pause, Some(EventKind::MissionPaused)),
        (HumanAction::Resume, Some(EventKind::MissionResumed)),
        (HumanAction::message("hi"), Some(EventKind::IntentAmended)),
        (
            HumanAction::answer(question().id, "yes"),
            Some(EventKind::IntentAnswered),
        ),
        (
            HumanAction::Approve {
                request_event: request().id,
            },
            Some(EventKind::PermissionGranted),
        ),
        (
            HumanAction::Deny {
                request_event: request().id,
            },
            Some(EventKind::PermissionDenied),
        ),
        (
            HumanAction::mark_skipped(FeatureId("f".into()), "r"),
            Some(EventKind::IntentAmended),
        ),
        (
            HumanAction::MarkComplete {
                feature_id: FeatureId("f".into()),
            },
            None,
        ),
    ];
    for (action, kind) in cases {
        let request = request_for(&action);
        let event = control::ack_event(
            &action,
            &mission(),
            &session,
            at(6, 0),
            spool.clone(),
            &template,
            request.as_ref(),
        );
        assert_eq!(event.as_ref().map(|e| e.kind.clone()), kind, "{action:?}");
        if let Some(event) = event {
            assert_eq!(event.overlay, Some(Overlay::Human));
            assert_eq!(event.role, Role::Human);
            assert_eq!(event.source, spool);
            assert_eq!(event.at, at(6, 0));
            assert_eq!(event.session_id, Some(session.clone()));
            assert!(event.id.0.starts_with("mission-1:4096:"), "{}", event.id);
        }
    }
}

#[test]
fn a_stream_pause_replaces_the_acked_one() {
    let template = template();
    let ptr = |offset| SourcePtr::Rpc {
        path: PathBuf::from("hosted/launch-1.jsonl"),
        byte_offset: offset,
        byte_len: 64,
    };
    let session = SessionId("mission-1".to_owned());
    let acked = control::ack_event(
        &HumanAction::Pause,
        &mission(),
        &session,
        at(6, 0),
        ptr(10),
        &template,
        None,
    )
    .unwrap();
    let mut streamed = stream_event(EventKind::MissionPaused, Role::Orchestrator, 6, 20);
    streamed.at = at(6, 1);
    let resumed = control::ack_event(
        &HumanAction::Resume,
        &mission(),
        &session,
        at(8, 0),
        ptr(30),
        &template,
        None,
    )
    .unwrap();
    let lone = control::ack_event(
        &HumanAction::Pause,
        &mission(),
        &session,
        at(9, 0),
        ptr(40),
        &template,
        None,
    )
    .unwrap();

    let ack_ids = [acked.id.clone(), lone.id.clone()].into_iter().collect();
    let kept = control::dedupe_paused(
        vec![acked, streamed.clone(), resumed.clone(), lone.clone()],
        &ack_ids,
    );

    let ids: Vec<&EventId> = kept.iter().map(|event| &event.id).collect();
    assert_eq!(ids, [&streamed.id, &resumed.id, &lone.id]);
}

#[test]
fn human_overlay_events_render_in_board_and_details() {
    let template = template();
    let mut events = vec![question(), request()];
    for (index, action) in the_eight_actions().into_iter().enumerate() {
        let request = request_for(&action);
        let mut event = control::record(
            &action,
            &mission(),
            at(5, u32::try_from(index).unwrap()),
            index as u64 + 1,
            &template,
            request.as_ref(),
        );
        event.milestone_id = Some(MilestoneId("m3".to_owned()));
        events.push(event);
    }
    events.sort_by_key(|event| event.at);

    let declared = DeclaredModels {
        orchestrator: ModelId("o".to_owned()),
        worker: ModelId("w".to_owned()),
        validator: ModelId("v".to_owned()),
    };
    let board = observatory_core::view::BoardView::build(
        &events,
        &ViewConfig::default(),
        &declared,
        "mission/x",
        &template,
    );
    let state = BoardState::fold(&events, &ViewConfig::default(), &template);

    let markers: Vec<_> = board
        .overlay_human
        .markers
        .iter()
        .filter(|marker| marker.event_id.0.starts_with("app:"))
        .collect();
    assert_eq!(markers.len(), 8, "the overlay filter lists every control");
    let first_lane = board
        .lanes
        .iter()
        .find(|lane| lane.lane == *template.first())
        .unwrap();
    let on_first_lane: u64 = first_lane
        .groups
        .iter()
        .flat_map(|group| &group.rows)
        .filter(|row| row.event_id.0.starts_with("app:"))
        .count() as u64;
    assert_eq!(
        on_first_lane, 6,
        "pause, resume, message, answer, complete, skip sit on the first lane"
    );
    let request_lane = board
        .lanes
        .iter()
        .find(|lane| lane.lane == request().lane)
        .unwrap();
    let approvals = request_lane
        .groups
        .iter()
        .flat_map(|group| &group.rows)
        .filter(|row| row.event_id.0.starts_with("app:"))
        .count();
    assert_eq!(approvals, 2, "approve and deny sit on the request's lane");
    assert!(state.human_overlays >= 8);

    let milestone = MilestoneId("m3".to_owned());
    let timeline = details(&events, &milestone).timeline;
    let app_rows: Vec<_> = timeline
        .iter()
        .filter(|row| matches!(row.pointer, SourcePtr::App { .. }))
        .collect();
    assert_eq!(app_rows.len(), 8);
    assert!(
        app_rows
            .iter()
            .all(|row| row.overlay == Some(Overlay::Human))
    );
}
