//! The control bar and the stall banner as view-models (FR-H1 to FR-H6, architecture §8), and
//! the host-side events they rest on. Lanes are addressed by `LaneId` through the template.

use std::path::PathBuf;
use std::time::Duration;

use observatory_core::control::stall::{self, StallFlag};
use observatory_core::control::{self, HumanAction, RunbookPrompt};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, MissionId, Overlay, Role, SessionId,
    SourcePtr, Timestamp,
};
use observatory_core::view::{
    ControlBar, ControlMode, HostedState, HostedStatus, PermissionRow, QuestionRow, RunState,
    StallBanner, StallFlagView, TERMINAL_INSTRUCTION, terminal_text,
};

fn at(minute: u32) -> Timestamp {
    Timestamp::parse(&format!("2026-10-25T10:{minute:02}:00.000Z")).expect("valid time")
}

fn event(kind: EventKind, feature: Option<&str>, minute: u32, offset: u64) -> Event {
    let template = CycleTemplate::engineering();
    Event {
        id: EventId(format!("mission-1:{offset}:0")),
        mission_id: MissionId("mission-1".to_owned()),
        milestone_id: None,
        feature_id: feature.map(|name| FeatureId(name.to_owned())),
        lane: template.first().clone(),
        overlay: kind.overlay(),
        kind,
        role: Role::Orchestrator,
        session_id: Some(SessionId("mission-1".to_owned())),
        model: None,
        at: at(minute),
        duration: None,
        usage: None,
        source: SourcePtr::Rpc {
            path: PathBuf::from("spool.jsonl"),
            byte_offset: offset,
            byte_len: 60,
        },
        mapping_version: MappingVersion::V1,
    }
}

fn running() -> HostedStatus {
    let mut status = HostedStatus::starting("mission-1");
    status.initialised();
    status
}

#[test]
fn a_running_hosted_mission_offers_pause_and_message_but_not_resume() {
    let bar = ControlBar::hosted(&running(), Some(&[]));

    assert_eq!(bar.mode, ControlMode::Hosted);
    assert_eq!(bar.state, RunState::Running);
    assert!(bar.pause_enabled && bar.message_enabled);
    assert!(!bar.resume_enabled);
    assert_eq!(bar.terminal_instruction, None);
}

#[test]
fn the_last_pause_or_resume_event_decides_between_pause_and_resume() {
    let paused = [event(EventKind::MissionPaused, None, 1, 10)];
    let bar = ControlBar::hosted(&running(), Some(&paused));
    assert_eq!(bar.state, RunState::Paused);
    assert!(bar.resume_enabled && !bar.pause_enabled);

    let resumed = [
        event(EventKind::MissionPaused, None, 1, 10),
        event(EventKind::MissionResumed, None, 2, 20),
    ];
    let bar = ControlBar::hosted(&running(), Some(&resumed));
    assert_eq!(bar.state, RunState::Running);
    assert!(bar.pause_enabled && !bar.resume_enabled);
}

#[test]
fn without_the_events_both_pause_and_resume_stay_available() {
    let bar = ControlBar::hosted(&running(), None);

    assert!(bar.pause_enabled && bar.resume_enabled);
}

#[test]
fn a_run_that_has_not_started_or_has_exited_offers_nothing() {
    let starting = ControlBar::hosted(&HostedStatus::starting("mission-1"), Some(&[]));
    assert_eq!(starting.state, RunState::Starting);
    assert!(!starting.pause_enabled && !starting.resume_enabled && !starting.message_enabled);

    let mut status = running();
    status.exited(Some(0));
    assert_eq!(status.state, HostedState::Exited { code: Some(0) });
    let exited = ControlBar::hosted(&status, Some(&[]));
    assert_eq!(exited.state, RunState::Exited);
    assert!(!exited.pause_enabled && !exited.resume_enabled && !exited.message_enabled);
}

#[test]
fn the_bar_lists_the_questions_and_permission_requests_that_wait() {
    let mut status = running();
    status.questions = vec![QuestionRow {
        event_id: "mission-1:300:0".into(),
        prompts: vec!["Which one?".into()],
    }];
    status.permissions = vec![PermissionRow {
        request_id: "perm-1".into(),
        event_id: "mission-1:200:1073741825".into(),
        tools: vec!["Execute".into()],
        can_approve: true,
        can_deny: false,
    }];

    let bar = ControlBar::hosted(&status, Some(&[]));

    assert_eq!(bar.questions.len(), 1);
    assert_eq!(bar.permissions.len(), 1);
    assert!(bar.permissions[0].can_approve && !bar.permissions[0].can_deny);
}

#[test]
fn an_attached_mission_records_the_control_and_points_to_the_terminal() {
    let asked = event(EventKind::QuestionAsked, None, 1, 30);
    let bar = ControlBar::attached(std::slice::from_ref(&asked));

    assert_eq!(bar.mode, ControlMode::Attached);
    assert_eq!(bar.state, RunState::Attached);
    assert_eq!(
        bar.terminal_instruction,
        Some("copy to the terminal running the mission")
    );
    assert_eq!(
        TERMINAL_INSTRUCTION,
        "copy to the terminal running the mission"
    );
    assert!(bar.pause_enabled && bar.resume_enabled && bar.message_enabled);
    assert_eq!(bar.questions.len(), 1);
    assert_eq!(bar.questions[0].event_id, asked.id.to_string());
    assert!(bar.permissions.is_empty());
}

#[test]
fn an_answered_question_leaves_the_attached_list() {
    let events = [
        event(EventKind::QuestionAsked, None, 1, 30),
        event(EventKind::QuestionAnswered, None, 2, 40),
    ];

    assert!(ControlBar::attached(&events).questions.is_empty());
}

#[test]
fn the_text_to_paste_is_the_message_or_the_instruction_of_the_control() {
    let message = HumanAction::message("use the staging table");
    assert_eq!(
        terminal_text(&message, "use the staging table"),
        "use the staging table"
    );
    assert_eq!(terminal_text(&HumanAction::Pause, ""), "Pause the mission.");
    assert_eq!(
        terminal_text(&HumanAction::Resume, ""),
        "Resume the mission."
    );
    let skip = HumanAction::mark_skipped(FeatureId("m3-x".into()), "out of scope");
    let text = terminal_text(&skip, "out of scope");
    assert!(text.contains("Skip feature m3-x") && text.contains("out of scope"));
    let approve = HumanAction::Approve {
        request_event: EventId("e".into()),
    };
    assert!(terminal_text(&approve, "").contains("terminal"));
}

#[test]
fn the_banner_names_the_feature_the_idle_minutes_the_prompt_and_the_runbook_link() {
    let flag = StallFlagView::from(&StallFlag {
        feature_id: FeatureId("m3-feature".into()),
        idle_for: Duration::from_secs(95),
        runbook: RunbookPrompt {
            prompt: "Waiting on human: answer it",
            anchor: "#waiting-on-human",
        },
    });

    let banner = StallBanner::of(&[flag]);

    assert_eq!(banner.rows.len(), 1);
    let row = &banner.rows[0];
    assert_eq!(row.feature_id.0, "m3-feature");
    assert_eq!(row.idle_minutes, 1);
    assert_eq!(row.prompt, "Waiting on human: answer it");
    assert_eq!(
        row.link,
        "docs/runbooks/stalled-mission.md#waiting-on-human"
    );
}

#[test]
fn no_flag_means_no_banner() {
    assert!(StallBanner::of(&[]).rows.is_empty());
}

#[test]
fn a_question_nobody_answered_stalls_the_mission_even_without_a_feature() {
    let events = [
        event(EventKind::TaskStarted, Some("m3-feature"), 0, 10),
        event(EventKind::TaskDone, Some("m3-feature"), 1, 20),
        event(EventKind::QuestionAsked, None, 2, 30),
    ];

    let activity = stall::activity_of(&events);
    let flags = stall::evaluate(&activity, at(5), Duration::from_secs(60));

    assert_eq!(flags.len(), 1);
    assert_eq!(flags[0].runbook.anchor, "#waiting-on-human");
}

#[test]
fn an_answered_mission_level_question_does_not_stall() {
    let events = [
        event(EventKind::QuestionAsked, None, 2, 30),
        event(EventKind::QuestionAnswered, None, 3, 40),
    ];

    assert!(stall::activity_of(&events).is_empty());
}

#[test]
fn a_permission_request_event_is_a_human_overlay_on_the_lane_of_the_asking_session() {
    let template = CycleTemplate::engineering();
    let lane = template.lane_of(&EventKind::TaskStarted, Role::Worker);
    let ptr = SourcePtr::Rpc {
        path: PathBuf::from("spool.jsonl"),
        byte_offset: 410,
        byte_len: 90,
    };

    let request = control::permission_requested_event(
        &MissionId("mission-1".into()),
        &SessionId("mission-1".into()),
        at(3),
        ptr,
        lane.clone(),
    );

    assert_eq!(request.kind, EventKind::PermissionRequested);
    assert_eq!(request.overlay, Some(Overlay::Human));
    assert_eq!(request.lane, lane);
    assert_eq!(request.id.to_string(), "mission-1:410:1073741825");
}
