//! Details, agent panel and card "now" view-models (FR-D1, FR-A1..A4): the figures the UI draws
//! for the Details view, the agent panel, the per-agent stream and the card's `now:` line.

use std::path::{Path, PathBuf};

use observatory_core::config::MissionConfig;
use observatory_core::ingest::SessionInfo;
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, MilestoneId, MissionId, ModelId, Role,
    SessionId, SourcePtr, Timestamp, TokenUsage,
};
use observatory_core::view::{
    AgentStatus, BoardState, DeclaredModels, Filter, IdleReason, MarkerKind, ViewConfig,
    WorkerGrouping, agent_entries, agent_stream, details, session_name,
};

fn engineering() -> CycleTemplate {
    CycleTemplate::engineering()
}

fn golden() -> Vec<Event> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.events.json");
    let text = std::fs::read_to_string(path).expect("the golden file is committed");
    serde_json::from_str(&text).expect("the golden file is a list of events")
}

fn at(second: u32) -> Timestamp {
    Timestamp::parse(&format!(
        "2026-10-22T10:{:02}:{:02}.000Z",
        second / 60,
        second % 60
    ))
    .expect("valid time")
}

fn event(kind: EventKind, role: Role, session: &str, second: u32, milestone: &str) -> Event {
    Event {
        id: EventId(format!("{session}:{second}:0")),
        mission_id: MissionId("mission".to_owned()),
        milestone_id: Some(MilestoneId(milestone.to_owned())),
        feature_id: Some(FeatureId(format!("{milestone}-feature"))),
        lane: engineering().lane_of(&kind, role),
        overlay: kind.overlay(),
        kind,
        role,
        session_id: Some(SessionId(session.to_owned())),
        model: Some(ModelId("model-a".to_owned())),
        at: at(second),
        duration: None,
        usage: None,
        source: SourcePtr::Jsonl {
            path: PathBuf::from(format!("fixtures/{session}.jsonl")),
            byte_offset: u64::from(second) * 100,
            byte_len: 90,
        },
        mapping_version: MappingVersion::V1,
    }
}

fn declared() -> DeclaredModels {
    DeclaredModels::from_config(&MissionConfig::default())
}

fn session_info(id: &str, role: Role, tag: Option<&str>, effort: Option<&str>) -> SessionInfo {
    SessionInfo {
        id: SessionId(id.to_owned()),
        parent: None,
        role,
        model: None,
        tag: tag.map(str::to_owned),
        effort: effort.map(str::to_owned),
    }
}

fn fold(events: &[Event], second: Option<u32>) -> BoardState {
    BoardState::fold(
        events,
        &ViewConfig {
            filter: Filter::default(),
            grouping: WorkerGrouping::PerWorker,
            at: second.map(at),
        },
        &engineering(),
    )
}

#[test]
fn a_timeline_row_names_the_milestone_that_owns_its_event() {
    let events = vec![
        event(EventKind::TaskStarted, Role::Worker, "w1", 1, "m1"),
        event(EventKind::TaskStarted, Role::Worker, "w2", 2, "m2"),
    ];

    let m2 = details(&events, &MilestoneId("m2".to_owned()));

    assert_eq!(m2.timeline.len(), 1);
    assert_eq!(
        m2.timeline[0].milestone,
        Some(MilestoneId("m2".to_owned())),
        "a sub-lane event opens the Details of this milestone"
    );
}

#[test]
fn every_row_of_the_m0_details_carries_its_phase_lane_and_milestone() {
    let events = golden();
    let milestone = events.iter().find_map(|e| e.milestone_id.clone()).unwrap();

    let details = details(&events, &milestone);

    for row in &details.timeline {
        assert_eq!(row.milestone.as_ref(), Some(&milestone));
        let source = events.iter().find(|e| e.id == row.event_id).unwrap();
        assert_eq!(row.lane, source.lane);
        assert_eq!(row.overlay, source.overlay);
    }
}

#[test]
fn a_card_is_held_until_the_task_is_done_and_then_names_why_it_idles() {
    let events = vec![
        event(EventKind::TaskStarted, Role::Worker, "w1", 1, "m1"),
        event(EventKind::TaskDone, Role::Worker, "w1", 4, "m1"),
        event(EventKind::TaskStarted, Role::Worker, "w2", 5, "m2"),
        event(EventKind::SubagentFinished, Role::Worker, "w2", 6, "m2"),
    ];

    let running = fold(&events, Some(2));
    let held = running.cards[0].now.as_ref().expect("w1 holds m1 at 2 s");
    assert_eq!(held.session_id.0, "w1");
    assert_eq!(held.model, Some(ModelId("model-a".to_owned())));
    assert_eq!(held.feature, Some(FeatureId("m1-feature".to_owned())));
    assert_eq!(running.cards[0].idle, None, "someone holds the card");

    let done = fold(&events, Some(4));
    assert_eq!(done.cards[0].now, None, "task.done releases the card");
    assert_eq!(
        done.cards[0].idle,
        Some(IdleReason::WaitingOnValidation),
        "work is done and no scrutiny has passed"
    );

    let end = fold(&events, None);
    assert_eq!(end.cards[1].now, None);
    assert_eq!(
        end.cards[1].idle,
        Some(IdleReason::Queued),
        "no work was done on m2 and nobody holds it"
    );
}

#[test]
fn a_card_waits_on_the_human_while_a_question_is_open_and_is_idle_free_when_green() {
    let asked = event(
        EventKind::QuestionAsked,
        Role::Orchestrator,
        "root",
        3,
        "m1",
    );
    let events = vec![
        event(EventKind::TaskStarted, Role::Worker, "w1", 1, "m1"),
        event(EventKind::TaskDone, Role::Worker, "w1", 2, "m1"),
        asked,
    ];
    assert_eq!(
        fold(&events, None).cards[0].idle,
        Some(IdleReason::WaitingOnHuman)
    );

    let mut green = events.clone();
    green.push(event(
        EventKind::ScrutinyPassed,
        Role::ScrutinyValidator,
        "v1",
        4,
        "m1",
    ));
    green.push(event(
        EventKind::QaScenarioPassed,
        Role::UserTestingValidator,
        "u1",
        5,
        "m1",
    ));
    let board = fold(&green, None);
    assert_eq!(
        board.cards[0].idle, None,
        "a finished milestone has no idle reason"
    );
}

#[test]
fn agent_entries_name_sessions_by_tag_or_id_and_compare_models_to_the_declared_one() {
    let usage = TokenUsage {
        input: 1,
        output: 2,
        cache_read: 3,
        cache_write: 4,
        credits: None,
    };
    let mut reported = event(EventKind::UsageReported, Role::Worker, "w1", 3, "m1");
    reported.usage = Some(usage);
    let mut other = event(EventKind::TaskStarted, Role::Worker, "w2", 4, "m1");
    other.model = Some(ModelId("claude-sonnet-5-5".to_owned()));
    let events = vec![
        event(EventKind::TaskStarted, Role::Worker, "w1", 1, "m1"),
        event(EventKind::ContextCompacted, Role::Worker, "w1", 2, "m1"),
        reported,
        other,
    ];
    let sessions = vec![
        session_info("w1", Role::Worker, Some("worker-1"), Some("high")),
        session_info("w2", Role::Worker, None, None),
    ];
    let board = fold(&events, None);

    let entries = agent_entries(&board.agents, &sessions, &declared(), false);

    assert_eq!(entries.len(), 2);
    let w1 = &entries[0];
    assert_eq!(w1.name, "worker-1", "the tag wins over the id");
    assert_eq!(w1.effort.as_deref(), Some("high"));
    assert_eq!(w1.model, Some(ModelId("model-a".to_owned())));
    assert_eq!(w1.declared, Some(ModelId("claude-sonnet-5-5".to_owned())));
    assert!(w1.drifted, "model-a is not the declared worker model");
    assert_eq!(w1.tokens, Some(usage));
    assert_eq!(w1.markers, 1);
    assert_eq!(w1.status, AgentStatus::Active);
    let w2 = &entries[1];
    assert_eq!(w2.name, "w2", "no tag: the session id");
    assert!(!w2.drifted);
    assert_eq!(w2.effort, None);
    assert_eq!(
        session_name(&sessions, &SessionId("w1".to_owned())),
        "worker-1"
    );
    assert_eq!(session_name(&sessions, &SessionId("zz".to_owned())), "zz");
}

#[test]
fn at_the_end_of_a_replay_every_agent_is_finished() {
    let events = golden();
    let board = fold(&events, None);

    let live = agent_entries(&board.agents, &[], &declared(), false);
    let ended = agent_entries(&board.agents, &[], &declared(), true);

    assert_eq!(ended.len(), 56);
    assert!(ended.iter().all(|a| a.status == AgentStatus::Finished));
    assert!(
        live.iter().any(|a| a.status != AgentStatus::Finished),
        "the fold alone leaves the orchestrator open"
    );
}

#[test]
fn the_agent_stream_lists_one_session_in_time_order_with_its_markers() {
    let events = golden();
    let session = events
        .iter()
        .filter(|e| e.kind == EventKind::ContextCompacted)
        .find_map(|e| e.session_id.clone())
        .expect("the capture compacts a context");
    let expected: Vec<&Event> = events
        .iter()
        .filter(|e| e.session_id.as_ref() == Some(&session))
        .filter(|e| !e.kind.is_panel_only())
        .collect();

    let stream = agent_stream(&events, &session, None);

    assert_eq!(stream.session_id, session);
    assert_eq!(stream.rows.len(), expected.len());
    assert_eq!(
        stream.rows.first().map(|r| &r.event_id),
        Some(&expected[0].id)
    );
    assert_eq!(
        stream.rows.last().map(|r| &r.event_id),
        Some(&expected[expected.len() - 1].id)
    );
    assert!(stream.rows.windows(2).all(|pair| pair[0].at <= pair[1].at));
    let compacted = expected
        .iter()
        .filter(|e| e.kind == EventKind::ContextCompacted)
        .count();
    assert_eq!(
        stream
            .markers
            .iter()
            .filter(|m| m.kind == MarkerKind::ContextCompacted)
            .count(),
        compacted
    );
    assert!(
        stream
            .markers
            .iter()
            .all(|m| stream.rows.iter().any(|r| r.event_id == m.event_id))
    );
}

#[test]
fn the_agent_stream_stops_at_the_scrubbed_instant() {
    let events = vec![
        event(EventKind::TaskStarted, Role::Worker, "w1", 1, "m1"),
        event(EventKind::ContextCompacted, Role::Worker, "w1", 3, "m1"),
        event(EventKind::TaskDone, Role::Worker, "w1", 5, "m1"),
        event(EventKind::TaskStarted, Role::Worker, "w2", 2, "m1"),
    ];

    let stream = agent_stream(&events, &SessionId("w1".to_owned()), Some(at(4)));

    assert_eq!(
        stream.rows.len(),
        2,
        "w2 is another session, task.done is later"
    );
    assert_eq!(stream.markers.len(), 1);
    assert_eq!(stream.markers[0].at, at(3));
}

#[test]
fn the_board_view_carries_the_agents_at_the_scrubbed_instant() {
    use observatory_core::view::BoardView;
    let events = golden();
    let cfg = |second: Option<Timestamp>| ViewConfig {
        filter: Filter::default(),
        grouping: WorkerGrouping::PerWorker,
        at: second,
    };

    let full = BoardView::build(
        &events,
        &cfg(None),
        &declared(),
        "mission/x",
        &engineering(),
    );
    let first = events[0].at;
    let early = BoardView::build(
        &events,
        &cfg(Some(first)),
        &declared(),
        "mission/x",
        &engineering(),
    );

    assert_eq!(full.agents.len(), 56);
    assert!(early.agents.len() < full.agents.len());
    assert_eq!(early.agents.len(), 1, "only the orchestrator has started");
}
