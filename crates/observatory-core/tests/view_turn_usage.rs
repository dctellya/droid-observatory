//! `turn.usage` is Panel data: every board consumer reads the same figures with and without the
//! usage log loaded; only the Panel's turn columns read the turns (rule 24 decision 2026-10-24).

use std::path::{Path, PathBuf};

use observatory_core::config::MissionConfig;
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, LaneId, MilestoneId, MissionId, ModelId,
    Role, SessionId, SourcePtr, Timestamp,
};
use observatory_core::view::{
    AgentStatus, BoardState, BoardView, DeclaredModels, Filter, FilterOptions, Scrubber,
    ViewConfig, WorkerGrouping, agent_stream, details, lane_counts, overlay_markers, tray_count,
};

fn lane(id: &str) -> LaneId {
    LaneId::from(id)
}

fn engineering() -> CycleTemplate {
    CycleTemplate::engineering()
}

fn golden() -> Vec<Event> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.events.json");
    let text = std::fs::read_to_string(path).expect("the golden file is committed");
    serde_json::from_str(&text).expect("the golden file is a list of events")
}

fn is_turn(event: &Event) -> bool {
    matches!(event.kind, EventKind::TurnUsage { .. })
}

fn without_turns(events: &[Event]) -> Vec<Event> {
    events.iter().filter(|e| !is_turn(e)).cloned().collect()
}

fn declared() -> DeclaredModels {
    DeclaredModels::from_config(&MissionConfig::default())
}

fn cfg(grouping: WorkerGrouping) -> ViewConfig {
    ViewConfig {
        filter: Filter::default(),
        grouping,
        at: None,
    }
}

fn board(events: &[Event], grouping: WorkerGrouping) -> BoardView {
    BoardView::build(
        events,
        &cfg(grouping),
        &declared(),
        "mission/x",
        &engineering(),
    )
}

#[test]
fn turn_usage_events_do_not_appear_in_lane_rows_or_cards() {
    let with = golden();
    let without = without_turns(&with);
    assert_eq!(with.len(), 2394);
    assert_eq!(with.iter().filter(|e| is_turn(e)).count(), 1115);

    for grouping in [WorkerGrouping::PerWorker, WorkerGrouping::PerFeature] {
        let a = board(&with, grouping);
        let b = board(&without, grouping);

        let lane_events: Vec<u64> = a.lanes.iter().map(|lane| lane.event_count).collect();
        assert_eq!(lane_events, [437, 69, 424, 93, 122, 134, 0], "{grouping:?}");
        assert_eq!(a.tray_unknown, 0);
        assert!(a.tray.is_empty());
        assert_eq!(a.event_count, 1279);
        assert_eq!(a.lanes, b.lanes, "rows, sub-lanes and cards, {grouping:?}");
        assert_eq!(a.agents, b.agents);
        assert_eq!(a.gate, b.gate);
        assert_eq!(a.overlay_human, b.overlay_human);
        assert_eq!(a.digest(), b.digest());
        assert_eq!(a.panel.elapsed_ms, b.panel.elapsed_ms);
        for (x, y) in a.panel.roles.iter().zip(&b.panel.roles) {
            assert_eq!(
                (x.role, x.tokens, &x.observed),
                (y.role, y.tokens, &y.observed)
            );
        }

        let sa = BoardState::fold(&with, &cfg(grouping), &engineering());
        let sb = BoardState::fold(&without, &cfg(grouping), &engineering());
        assert_eq!(sa.sublanes, sb.sublanes, "{grouping:?}");
        assert_eq!(sa.cards, sb.cards);
        assert_eq!(sa.lane_counts.to_vec(), [437, 69, 424, 93, 122, 134, 0]);
    }

    assert_eq!(
        lane_counts(&with, &engineering()),
        lane_counts(&without, &engineering())
    );
    assert_eq!(tray_count(&with), 0);
    assert_eq!(
        overlay_markers(&with, observatory_core::model::Overlay::Human).len(),
        overlay_markers(&without, observatory_core::model::Overlay::Human).len()
    );
}

#[test]
fn scrubber_stops_and_filter_options_ignore_turn_usage() {
    let with = golden();
    let without = without_turns(&with);

    let (a, b) = (Scrubber::new(&with), Scrubber::new(&without));
    assert_eq!(a.stops(), b.stops());
    assert!(a == b, "the same stops, one per distinct event time");
    assert!(FilterOptions::of(&with) == FilterOptions::of(&without));
}

#[test]
fn details_row_count_is_unchanged_when_the_usage_log_is_loaded() {
    let with = golden();
    let without = without_turns(&with);
    let milestone = MilestoneId("s".to_owned());

    let a = details(&with, &milestone);
    let b = details(&without, &milestone);

    assert_eq!(a.timeline.len(), 251);
    assert_eq!(a, b);
}

fn at(second: u32) -> Timestamp {
    Timestamp::parse(&format!(
        "2026-10-22T10:{:02}:{:02}.000Z",
        second / 60,
        second % 60
    ))
    .expect("valid time")
}

fn turn_kind() -> EventKind {
    EventKind::TurnUsage {
        input: 1,
        output: 2,
        cache_read: 3,
        reasoning: 0,
        total_input: 4,
        model: None,
        log_ts: at(0),
    }
}

fn event(kind: EventKind, role: Role, session: &str, second: u32) -> Event {
    Event {
        id: EventId(format!("{session}:{second}:0")),
        mission_id: MissionId("mission".to_owned()),
        milestone_id: Some(MilestoneId("m1".to_owned())),
        feature_id: Some(FeatureId("m1-feature".to_owned())),
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

#[test]
fn agent_stream_and_status_ignore_turn_usage() {
    let events = vec![
        event(EventKind::TaskStarted, Role::Worker, "w1", 1),
        event(EventKind::TaskDone, Role::Worker, "w1", 3),
        event(turn_kind(), Role::Worker, "w1", 4),
        event(turn_kind(), Role::Worker, "w2", 5),
    ];

    let stream = agent_stream(&events, &SessionId("w1".to_owned()), None);
    let state = BoardState::fold(&events, &cfg(WorkerGrouping::PerWorker), &engineering());

    assert_eq!(stream.rows.len(), 2, "the turn is not a row of the stream");
    assert_eq!(
        state.agents.len(),
        1,
        "a session with only turns is no agent"
    );
    let agent = &state.agents[0];
    assert_eq!(
        agent.status,
        AgentStatus::Idle,
        "a turn never revives an idle agent"
    );
    assert_eq!(agent.event_count, 2);
    assert_eq!(state.event_count, 2);
    assert_eq!(
        state.cards[0].event_count, 2,
        "card bookkeeping skips the turn"
    );
    assert_eq!(state.cards[0].last_at, at(3));
    assert_eq!(
        state.lane_counts[engineering().index(&lane("build")).unwrap()],
        2
    );
}

#[test]
fn panel_counts_turn_tokens_per_role_when_the_log_is_loaded() {
    let with = golden();
    let without = without_turns(&with);

    let panel = board(&with, WorkerGrouping::PerWorker).panel;
    let plain = board(&without, WorkerGrouping::PerWorker).panel;

    let totals = [
        (Role::Orchestrator, 307, 77_540, 529_212, 38_166_855),
        (Role::Worker, 635, 75_403, 923_696, 45_998_989),
        (Role::ScrutinyValidator, 64, 154, 88_643, 3_198_549),
        (Role::UserTestingValidator, 109, 165_488, 121_197, 4_839_929),
    ];
    for (role, turns, input, output, cache_read) in totals {
        let row = panel.roles.iter().find(|r| r.role == role).unwrap();
        assert_eq!(
            (
                row.turns,
                row.turn_input,
                row.turn_output,
                row.turn_cache_read
            ),
            (turns, input, output, cache_read),
            "{role:?}"
        );
    }
    for row in &plain.roles {
        assert_eq!(row.turns, 0, "no log, no turn columns: {:?}", row.role);
        assert_eq!(
            (row.turn_input, row.turn_output, row.turn_cache_read),
            (0, 0, 0)
        );
    }
    for (x, y) in panel.roles.iter().zip(&plain.roles) {
        assert_eq!(x.tokens, y.tokens, "the sidecar tokens stay as they were");
    }
}
