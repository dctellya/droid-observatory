//! View-models over events: the board (cards, sub-lanes, filters, scrubber, tray), the agent
//! panel, Details, the library and the B-006 token scrub (architecture §7).
//!
//! Real input is the committed golden of the M0 capture (1279 events); the synthetic cases build
//! small event lists whose answer is known by construction.

use std::path::{Path, PathBuf};

use observatory_core::ingest::SessionInfo;
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, LaneId, MilestoneId, MissionId, ModelId,
    Overlay, Role, SessionId, SourcePtr, Timestamp, TokenUsage,
};
use observatory_core::view::{
    AgentStatus, BoardState, Filter, GateBadge, MarkerKind, RowGroup, ValidationBadge, ViewConfig,
    WorkerGrouping, agent_markers, card_lane, details, lane_counts, overlay_markers, raw_line_view,
    scrub, scrub_to, tray_count,
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
    let events: Vec<Event> =
        serde_json::from_str(&text).expect("the golden file is a list of events");
    // Plain replay never reads the usage log; its turn.usage events are checked in usage_log.rs.
    events
        .into_iter()
        .filter(|e| e.kind.prd_name() != "turn.usage")
        .collect()
}

fn config(filter: Filter, at: Option<Timestamp>) -> ViewConfig {
    ViewConfig {
        filter,
        grouping: WorkerGrouping::PerWorker,
        at,
    }
}

fn all(events: &[Event]) -> BoardState {
    BoardState::fold(events, &config(Filter::default(), None), &engineering())
}

// --- synthetic events ---------------------------------------------------------------------

fn at(second: u32) -> Timestamp {
    Timestamp::parse(&format!(
        "2026-10-22T10:{:02}:{:02}.000Z",
        second / 60,
        second % 60
    ))
    .expect("valid time")
}

struct Build(Event);

impl Build {
    fn new(kind: EventKind, role: Role, session: &str, second: u32) -> Self {
        Self(Event {
            id: EventId(format!("{session}:{second}:0")),
            mission_id: MissionId("mission".to_owned()),
            milestone_id: None,
            feature_id: None,
            lane: engineering().lane_of(&kind, role),
            overlay: kind.overlay(),
            kind,
            role,
            session_id: Some(SessionId(session.to_owned())),
            model: None,
            at: at(second),
            duration: None,
            usage: None,
            source: SourcePtr::Jsonl {
                path: PathBuf::from(format!("fixtures/{session}.jsonl")),
                byte_offset: u64::from(second) * 100,
                byte_len: 90,
            },
            mapping_version: MappingVersion::V1,
        })
    }

    fn milestone(mut self, id: &str) -> Self {
        self.0.milestone_id = Some(MilestoneId(id.to_owned()));
        self
    }

    fn feature(mut self, id: &str) -> Self {
        self.0.feature_id = Some(FeatureId(id.to_owned()));
        self
    }

    fn model(mut self, id: &str) -> Self {
        self.0.model = Some(ModelId(id.to_owned()));
        self
    }

    fn usage(mut self, usage: TokenUsage) -> Self {
        self.0.usage = Some(usage);
        self
    }

    fn done(self) -> Event {
        self.0
    }
}

fn worker(kind: EventKind, session: &str, second: u32, milestone: &str) -> Event {
    Build::new(kind, Role::Worker, session, second)
        .milestone(milestone)
        .feature(&format!("{milestone}-feature"))
        .done()
}

// --- cards (FR-B1) ------------------------------------------------------------------------

#[test]
fn card_sits_in_lane_of_most_recent_open_activity() {
    let mut events = vec![
        worker(EventKind::TaskStarted, "w1", 1, "m1"),
        Build::new(
            EventKind::FindingNonBlocking,
            Role::ScrutinyValidator,
            "v1",
            2,
        )
        .milestone("m1")
        .done(),
        Build::new(EventKind::ScrutinyPassed, Role::ScrutinyValidator, "v1", 3)
            .milestone("m1")
            .done(),
        worker(EventKind::TaskDone, "w1", 4, "m1"),
        worker(EventKind::SubagentFinished, "w1", 5, "m1"),
    ];
    assert_eq!(
        card_lane(&events, &engineering()),
        lane("scrutiny"),
        "passed, done and finished close; the finding is the last open activity"
    );

    events.push(
        Build::new(EventKind::QaEvidence, Role::UserTestingValidator, "u1", 6)
            .milestone("m1")
            .done(),
    );
    assert_eq!(card_lane(&events, &engineering()), lane("user_testing"));

    events.push(
        Build::new(EventKind::GateCommandRun, Role::Worker, "w2", 7)
            .milestone("m1")
            .done(),
    );
    events.push(
        Build::new(EventKind::GateCommandPassed, Role::Worker, "w2", 8)
            .milestone("m1")
            .done(),
    );
    assert_eq!(
        card_lane(&events, &engineering()),
        lane("integrate"),
        "a passed gate command closes, the run before it was open"
    );

    events.push(
        Build::new(EventKind::CheckStarted, Role::External, "gh", 9)
            .milestone("m1")
            .done(),
    );
    events.push(
        Build::new(EventKind::CheckPassed, Role::External, "gh", 10)
            .milestone("m1")
            .done(),
    );
    assert_eq!(
        card_lane(&events, &engineering()),
        lane("gate"),
        "a Gate kind that awaits someone is open, a passed check is not"
    );

    assert_eq!(
        card_lane(&[], &engineering()),
        lane("plan"),
        "a milestone with no activity waits in Plan"
    );
    let only_closed = [worker(EventKind::TaskDone, "w1", 1, "m1")];
    assert_eq!(
        card_lane(&only_closed, &engineering()),
        lane("build"),
        "with nothing open the card stays where its last event is"
    );
}

#[test]
fn the_board_has_one_card_per_milestone_in_the_lane_of_its_last_open_event() {
    let events = vec![
        Build::new(EventKind::MilestoneDefined, Role::Orchestrator, "root", 1)
            .milestone("m1")
            .done(),
        Build::new(EventKind::MilestoneDefined, Role::Orchestrator, "root", 2)
            .milestone("m2")
            .done(),
        worker(EventKind::FileTouched, "w1", 3, "m1"),
        worker(EventKind::TaskDone, "w1", 4, "m1"),
        Build::new(
            EventKind::QaScenarioPassed,
            Role::UserTestingValidator,
            "u1",
            5,
        )
        .milestone("m2")
        .done(),
        Build::new(EventKind::QaEvidence, Role::UserTestingValidator, "u1", 6)
            .milestone("m2")
            .done(),
    ];

    let board = all(&events);

    let cards: Vec<(String, LaneId)> = board
        .cards
        .iter()
        .map(|card| (card.milestone.to_string(), card.lane.clone()))
        .collect();
    assert_eq!(
        cards,
        [
            ("m1".to_owned(), lane("build")),
            ("m2".to_owned(), lane("user_testing"))
        ]
    );
    assert_eq!(board.cards[0].event_count, 3);
    assert_eq!(board.cards[0].last_at, at(4));
    assert_eq!(
        board.cards[0].features,
        [FeatureId("m1-feature".to_owned())]
    );
}

#[test]
fn every_milestone_of_the_m0_capture_has_a_card_in_its_open_lane() {
    let events = golden();

    let board = all(&events);

    let mut milestones: Vec<&MilestoneId> = events
        .iter()
        .filter_map(|e| e.milestone_id.as_ref())
        .collect();
    milestones.sort();
    milestones.dedup();
    assert!(milestones.len() >= 2, "{milestones:?}");
    assert_eq!(board.cards.len(), milestones.len());
    for card in &board.cards {
        let mine: Vec<&Event> = events
            .iter()
            .filter(|e| e.milestone_id.as_ref() == Some(&card.milestone))
            .collect();
        let expected = mine
            .iter()
            .rev()
            .find(|e| is_open_by_the_book(e))
            .map_or_else(
                || mine.last().map(|e| e.lane.clone()),
                |e| Some(e.lane.clone()),
            )
            .expect("a card has events");
        assert_eq!(card.lane, expected, "card {}", card.milestone);
    }
}

/// The VAL-BOARD-005 wording, written out without the implementation: phases 1 to 6 are open
/// unless the kind ends `.done`, `.passed`, `.merged` or `.finished`; in Gate only the kinds
/// that wait for someone are.
fn is_open_by_the_book(event: &Event) -> bool {
    let name = event.kind.prd_name().into_owned();
    if event.lane == lane("gate") {
        return [
            "pr.review_requested",
            "check.started",
            "check.failed",
            "check.required_missing",
            "ruleset.blocked",
        ]
        .contains(&name.as_str());
    }
    ![".done", ".passed", ".merged", ".finished"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

// --- scrubber (FR-B6) ---------------------------------------------------------------------

#[test]
fn scrubber_replays_to_a_timestamp() {
    let events = golden();
    let middle = events[events.len() / 2].at;
    let expected = events.iter().filter(|e| e.at <= middle).count();

    let prefix = scrub_to(&events, middle);
    assert_eq!(prefix.len(), expected);
    assert!(prefix.iter().all(|e| e.at <= middle));
    assert!(events[prefix.len()..].iter().all(|e| e.at > middle));

    let board = BoardState::fold(
        &events,
        &config(Filter::default(), Some(middle)),
        &engineering(),
    );
    assert_eq!(board.event_count, expected as u64);
    assert_eq!(board.lane_counts, lane_counts(prefix, &engineering()));
    assert_eq!(board.now, Some(middle));
    for card in &board.cards {
        let mine: Vec<Event> = prefix
            .iter()
            .filter(|e| e.milestone_id.as_ref() == Some(&card.milestone))
            .cloned()
            .collect();
        assert_eq!(
            card.lane,
            card_lane(&mine, &engineering()),
            "card {}",
            card.milestone
        );
    }
    assert_eq!(board, all(prefix).with_now(Some(middle)));

    assert!(scrub_to(&events, at(0)).is_empty());
    assert_eq!(
        scrub_to(&events, events.last().unwrap().at).len(),
        events.len()
    );
}

// --- tray (FR-B7) -------------------------------------------------------------------------

#[test]
fn unmapped_tray_count_equals_unknown_events() {
    let mut events = vec![
        worker(EventKind::TaskStarted, "w1", 1, "m1"),
        Build::new(
            EventKind::Unknown {
                raw_type: "future_kind".to_owned(),
            },
            Role::Worker,
            "w1",
            2,
        )
        .done(),
        Build::new(
            EventKind::Unknown {
                raw_type: "message/audio".to_owned(),
            },
            Role::Unresolved,
            "w9",
            3,
        )
        .done(),
    ];

    let board = all(&events);

    assert_eq!(tray_count(&events), 2);
    assert_eq!(board.tray_unknown, 2);
    let tray: Vec<(&str, &SourcePtr)> = board
        .tray
        .iter()
        .map(|entry| (entry.raw_type.as_str(), &entry.pointer))
        .collect();
    assert_eq!(tray.len(), 2);
    assert_eq!(tray[0].0, "future_kind");
    assert_eq!(
        tray[0].1, &events[1].source,
        "the raw line opens from the pointer"
    );
    assert_eq!(board.tray[1].raw_type, "message/audio");
    assert_eq!(board.tray[1].event_id, events[2].id);

    events.truncate(2);
    assert_eq!(
        all(&events).tray_unknown,
        1,
        "scrubbing back shrinks the tray"
    );
}

#[test]
fn the_m0_board_agrees_with_the_replay_counts() {
    let events = golden();

    let board = all(&events);

    assert_eq!(events.len(), 1279, "the replay prints 1279 events");
    assert_eq!(board.event_count, 1279);
    assert_eq!(board.tray_unknown, 0, "the replay ends `unknown events: 0`");
    assert!(board.tray.is_empty());
    assert_eq!(board.lane_counts, [437, 69, 424, 93, 122, 134, 0]);
    assert_eq!(lane_counts(&events, &engineering()), board.lane_counts);
    assert_eq!(board.human_overlays, 70);
    assert_eq!(overlay_markers(&events, Overlay::Human).len(), 70);
    assert!(overlay_markers(&events, Overlay::Officer).is_empty());
}

// --- filters (FR-B5) ----------------------------------------------------------------------

#[test]
fn filter_by_milestone_keeps_only_that_milestones_events() {
    let events = golden();
    let milestone = events.iter().find_map(|e| e.milestone_id.clone()).unwrap();
    let filter = Filter {
        milestone: Some(milestone.clone()),
        ..Filter::default()
    };

    let board = BoardState::fold(&events, &config(filter, None), &engineering());

    let expected = events
        .iter()
        .filter(|e| e.milestone_id.as_ref() == Some(&milestone))
        .count();
    assert_eq!(board.event_count, expected as u64);
    assert_eq!(board.cards.len(), 1);
    assert_eq!(board.cards[0].milestone, milestone);
}

#[test]
fn filter_by_feature_keeps_only_that_features_events() {
    let events = golden();
    let feature = events.iter().find_map(|e| e.feature_id.clone()).unwrap();
    let filter = Filter {
        feature: Some(feature.clone()),
        ..Filter::default()
    };

    let board = BoardState::fold(&events, &config(filter, None), &engineering());

    let expected = events
        .iter()
        .filter(|e| e.feature_id.as_ref() == Some(&feature))
        .count();
    assert!(expected > 0);
    assert_eq!(board.event_count, expected as u64);
    assert!(
        board
            .cards
            .iter()
            .all(|card| card.features.contains(&feature))
    );
}

#[test]
fn filter_by_role_keeps_only_that_roles_events() {
    let events = golden();
    let filter = Filter {
        role: Some(Role::ScrutinyValidator),
        ..Filter::default()
    };

    let board = BoardState::fold(&events, &config(filter, None), &engineering());

    let expected = events
        .iter()
        .filter(|e| e.role == Role::ScrutinyValidator)
        .count();
    assert!(expected > 0 && expected < events.len());
    assert_eq!(board.event_count, expected as u64);
    assert!(
        board
            .agents
            .iter()
            .all(|agent| agent.role == Role::ScrutinyValidator)
    );
}

#[test]
fn filter_by_model_keeps_only_that_models_events() {
    let events = golden();
    let mut models: Vec<ModelId> = events.iter().filter_map(|e| e.model.clone()).collect();
    models.sort();
    models.dedup();
    assert!(models.len() >= 2, "{models:?}");
    let model = models[0].clone();
    let filter = Filter {
        model: Some(model.clone()),
        ..Filter::default()
    };

    let board = BoardState::fold(&events, &config(filter, None), &engineering());

    let expected = events
        .iter()
        .filter(|e| e.model.as_ref() == Some(&model))
        .count();
    assert_eq!(board.event_count, expected as u64);
    assert!(expected < events.len());
}

fn validation_events() -> Vec<Event> {
    let scrutiny = |kind, second, milestone: &str| {
        Build::new(kind, Role::ScrutinyValidator, "v1", second)
            .milestone(milestone)
            .done()
    };
    let qa = |kind, second, milestone: &str| {
        Build::new(kind, Role::UserTestingValidator, "u1", second)
            .milestone(milestone)
            .done()
    };
    vec![
        worker(EventKind::TaskStarted, "w3", 1, "m3"),
        scrutiny(EventKind::FindingBlocking, 2, "m1"),
        qa(EventKind::QaScenarioPassed, 3, "m1"),
        scrutiny(EventKind::ScrutinyPassed, 4, "m2"),
        qa(EventKind::QaScenarioPassed, 5, "m2"),
        qa(EventKind::QaScenarioFailed, 6, "m4"),
        scrutiny(EventKind::ScrutinyPassed, 7, "m5"),
    ]
}

#[test]
fn filter_by_validation_state_keeps_the_cards_with_that_badge() {
    let events = validation_events();
    let board = all(&events);
    let badges: Vec<(&str, ValidationBadge)> = board
        .cards
        .iter()
        .map(|card| (card.milestone.0.as_str(), card.validation))
        .collect();
    assert_eq!(
        badges,
        [
            ("m3", ValidationBadge::Grey),
            ("m1", ValidationBadge::Red),
            ("m2", ValidationBadge::Green),
            ("m4", ValidationBadge::Red),
            ("m5", ValidationBadge::Grey),
        ],
        "red: an open blocking finding or failed scenario; green: scrutiny and scenarios passed; \
         grey: not validated (m5 has no scenario yet)"
    );

    for (badge, expected) in [
        (ValidationBadge::Red, ["m1", "m4"].as_slice()),
        (ValidationBadge::Green, ["m2"].as_slice()),
        (ValidationBadge::Grey, ["m3", "m5"].as_slice()),
    ] {
        let filter = Filter {
            validation: Some(badge),
            ..Filter::default()
        };
        let board = BoardState::fold(&events, &config(filter, None), &engineering());
        let mut kept: Vec<&str> = board.cards.iter().map(|c| c.milestone.0.as_str()).collect();
        kept.sort_unstable();
        assert_eq!(kept, expected, "{badge:?}");
        assert!(board.cards.iter().all(|card| card.validation == badge));
    }
}

#[test]
fn a_blocking_finding_is_closed_by_a_later_pass_and_a_resolved_rework() {
    let mut events = vec![
        Build::new(EventKind::FindingBlocking, Role::ScrutinyValidator, "v1", 1)
            .milestone("m1")
            .done(),
    ];
    assert_eq!(all(&events).cards[0].validation, ValidationBadge::Red);

    events.push(
        Build::new(EventKind::ReworkResolved, Role::ScrutinyValidator, "v1", 2)
            .milestone("m1")
            .done(),
    );
    assert_eq!(all(&events).cards[0].validation, ValidationBadge::Grey);

    events.push(
        Build::new(EventKind::ScrutinyPassed, Role::ScrutinyValidator, "v1", 3)
            .milestone("m1")
            .done(),
    );
    events.push(
        Build::new(
            EventKind::QaScenarioPassed,
            Role::UserTestingValidator,
            "u1",
            4,
        )
        .milestone("m1")
        .done(),
    );
    assert_eq!(all(&events).cards[0].validation, ValidationBadge::Green);
}

#[test]
fn filter_by_gate_state_keeps_the_board_only_when_the_gate_matches() {
    let mut events = vec![worker(EventKind::TaskStarted, "w1", 1, "m1")];
    let pending = all(&events);
    assert_eq!(pending.gate.badge, GateBadge::Pending);
    assert_eq!(pending.gate.pr_number, None);

    let with_gate = |events: &[Event], badge| {
        BoardState::fold(
            events,
            &config(
                Filter {
                    gate: Some(badge),
                    ..Filter::default()
                },
                None,
            ),
            &engineering(),
        )
    };
    assert_eq!(with_gate(&events, GateBadge::Pending).event_count, 1);
    assert_eq!(with_gate(&events, GateBadge::CiRed).event_count, 0);
    assert!(with_gate(&events, GateBadge::CiRed).cards.is_empty());

    let gh = |kind, second| {
        Build::new(kind, Role::External, "gh", second)
            .milestone("m1")
            .done()
    };
    events.push(gh(EventKind::PrOpened, 2));
    assert_eq!(all(&events).gate.badge, GateBadge::ChecksRunning);
    events.push(gh(EventKind::CheckStarted, 3));
    events.push(gh(EventKind::CheckFailed, 4));
    assert_eq!(all(&events).gate.badge, GateBadge::CiRed);
    assert_eq!(with_gate(&events, GateBadge::CiRed).event_count, 4);
    assert_eq!(with_gate(&events, GateBadge::Pending).event_count, 0);

    events.push(gh(EventKind::CheckStarted, 5));
    events.push(gh(EventKind::CheckPassed, 6));
    assert_eq!(all(&events).gate.badge, GateBadge::CiGreen);
    events.push(
        Build::new(
            EventKind::HandedToCd {
                merge_sha: "abc".to_owned(),
                pr_number: 41,
                attestation_ids: vec![],
            },
            Role::External,
            "gh",
            7,
        )
        .done(),
    );
    let merged = all(&events);
    assert_eq!(merged.gate.badge, GateBadge::PolicyMerged);
    assert_eq!(merged.gate.pr_number, Some(41));
}

#[test]
fn the_gate_header_before_a_pr_shows_the_last_gate_command_verdict() {
    let events = golden();

    let board = all(&events);

    assert_eq!(board.gate.badge, GateBadge::Pending);
    assert_eq!(board.gate.gate_runs, 35);
    assert_eq!(
        board.gate.last_gate_command,
        Some(observatory_core::view::GateVerdict::Passed),
        "M0: gate_command.passed after 35 runs, 1 failed"
    );
}

#[test]
fn filters_compose_with_the_scrubber() {
    let events = golden();
    let middle = events[events.len() * 2 / 3].at;
    let filter = Filter {
        role: Some(Role::Worker),
        ..Filter::default()
    };

    let both = BoardState::fold(
        &events,
        &config(filter.clone(), Some(middle)),
        &engineering(),
    );

    let expected: Vec<Event> = events
        .iter()
        .filter(|e| e.role == Role::Worker && e.at <= middle)
        .cloned()
        .collect();
    assert_eq!(both.event_count, expected.len() as u64);
    assert_eq!(both.lane_counts, lane_counts(&expected, &engineering()));
    let filtered_first: Vec<Event> = events
        .iter()
        .filter(|e| e.role == Role::Worker)
        .cloned()
        .collect();
    let scrubbed_after = BoardState::fold(
        &filtered_first,
        &config(Filter::default(), Some(middle)),
        &engineering(),
    );
    assert_eq!(both.lane_counts, scrubbed_after.lane_counts);
    assert_eq!(both.cards, scrubbed_after.cards);

    let narrower = Filter {
        milestone: events.iter().find_map(|e| e.milestone_id.clone()),
        ..filter
    };
    let narrow = BoardState::fold(&events, &config(narrower, Some(middle)), &engineering());
    assert!(narrow.event_count <= both.event_count);
}

// --- Workers lane (FR-B2) -----------------------------------------------------------------

#[test]
fn the_build_lane_groups_per_worker_or_per_feature() {
    let events = vec![
        Build::new(EventKind::TaskStarted, Role::Worker, "w1", 1)
            .milestone("m1")
            .feature("m1-a")
            .done(),
        Build::new(EventKind::FileTouched, Role::Worker, "w1", 2)
            .milestone("m1")
            .feature("m1-a")
            .done(),
        Build::new(EventKind::TaskStarted, Role::Worker, "w2", 3)
            .milestone("m1")
            .feature("m1-b")
            .done(),
        Build::new(EventKind::FileTouched, Role::Worker, "w2", 4)
            .milestone("m1")
            .feature("m1-a")
            .done(),
        Build::new(EventKind::FileTouched, Role::Worker, "w3", 5).done(),
        Build::new(
            EventKind::FindingNonBlocking,
            Role::ScrutinyValidator,
            "v1",
            6,
        )
        .done(),
    ];
    let cfg = |grouping| ViewConfig {
        grouping,
        ..config(Filter::default(), None)
    };

    let per_worker = BoardState::fold(&events, &cfg(WorkerGrouping::PerWorker), &engineering());
    let workers: Vec<(&str, u64)> = per_worker
        .sublanes
        .iter()
        .map(|lane| (lane.key.as_str(), lane.event_count))
        .collect();
    assert_eq!(workers, [("w1", 2), ("w2", 2), ("w3", 1)]);
    assert!(
        per_worker
            .sublanes
            .iter()
            .all(|lane| lane.role == Some(Role::Worker))
    );

    let per_feature = BoardState::fold(&events, &cfg(WorkerGrouping::PerFeature), &engineering());
    let features: Vec<(&str, u64)> = per_feature
        .sublanes
        .iter()
        .map(|lane| (lane.key.as_str(), lane.event_count))
        .collect();
    assert_eq!(
        features,
        [("m1-a", 3), ("m1-b", 1), ("", 1)],
        "no feature: the empty key"
    );
    assert_eq!(
        per_worker
            .sublanes
            .iter()
            .map(|l| l.event_count)
            .sum::<u64>(),
        per_feature
            .sublanes
            .iter()
            .map(|l| l.event_count)
            .sum::<u64>(),
        "both groupings partition the same Build events"
    );
}

// --- agents (FR-A2, FR-A4) ----------------------------------------------------------------

#[test]
fn agent_panel_rows_carry_status_feature_model_tokens_and_markers() {
    let usage = TokenUsage {
        input: 10,
        output: 20,
        cache_read: 30,
        cache_write: 40,
        credits: None,
    };
    let events = vec![
        Build::new(EventKind::IntentCreated, Role::Orchestrator, "root", 1).done(),
        Build::new(EventKind::SubagentSpawned, Role::Worker, "w1", 2)
            .model("model-a")
            .done(),
        Build::new(EventKind::TaskStarted, Role::Worker, "w1", 3)
            .milestone("m1")
            .feature("m1-a")
            .model("model-a")
            .done(),
        Build::new(EventKind::ModelChanged, Role::Worker, "w1", 4)
            .milestone("m1")
            .feature("m1-a")
            .model("model-b")
            .done(),
        Build::new(EventKind::ContextCompacted, Role::Worker, "w1", 5)
            .milestone("m1")
            .feature("m1-a")
            .model("model-b")
            .done(),
        Build::new(EventKind::UsageReported, Role::Worker, "w1", 6)
            .milestone("m1")
            .feature("m1-a")
            .model("model-b")
            .usage(usage)
            .done(),
        Build::new(EventKind::SubagentFinished, Role::Worker, "w1", 7)
            .milestone("m1")
            .feature("m1-a")
            .model("model-b")
            .done(),
        Build::new(EventKind::SubagentSpawned, Role::Worker, "w2", 8).done(),
        Build::new(EventKind::QuestionAsked, Role::Orchestrator, "root", 9).done(),
    ];

    let board = all(&events);

    let ids: Vec<&str> = board
        .agents
        .iter()
        .map(|a| a.session_id.0.as_str())
        .collect();
    assert_eq!(ids, ["root", "w1", "w2"]);
    let root = &board.agents[0];
    assert_eq!(root.role, Role::Orchestrator);
    assert_eq!(root.status, AgentStatus::Waiting, "a question is open");
    let w1 = &board.agents[1];
    assert_eq!(w1.status, AgentStatus::Finished);
    assert_eq!(w1.feature, Some(FeatureId("m1-a".to_owned())));
    assert_eq!(w1.model, Some(ModelId("model-b".to_owned())));
    assert_eq!(w1.tokens, Some(usage));
    let markers: Vec<(MarkerKind, Timestamp)> = w1.markers.iter().map(|m| (m.kind, m.at)).collect();
    assert_eq!(
        markers,
        [
            (MarkerKind::ModelChanged, at(4)),
            (MarkerKind::ContextCompacted, at(5))
        ]
    );
    assert_eq!(board.agents[2].status, AgentStatus::Active);
    assert_eq!(board.agents[2].tokens, None);

    let session: Vec<Event> = events
        .iter()
        .filter(|e| e.session_id.as_ref().is_some_and(|s| s.0 == "w1"))
        .cloned()
        .collect();
    assert_eq!(agent_markers(&session), w1.markers);

    let card = &board.cards[0];
    assert_eq!(card.now, None, "w1 has finished: nobody holds m1");
    let mut running = events.clone();
    running.retain(|e| e.kind != EventKind::SubagentFinished);
    let now = all(&running).cards[0].now.clone().expect("w1 holds m1");
    assert_eq!(now.session_id.0, "w1");
    assert_eq!(now.role, Role::Worker);
    assert_eq!(now.feature, Some(FeatureId("m1-a".to_owned())));
}

#[test]
fn the_m0_agent_panel_lists_every_session_and_every_marker() {
    let events = golden();

    let board = all(&events);

    assert_eq!(board.agents.len(), 56, "1 orchestrator and 55 children");
    assert_eq!(
        board
            .agents
            .iter()
            .filter(|a| a.role == Role::Orchestrator)
            .count(),
        1
    );
    let markers: usize = board.agents.iter().map(|a| a.markers.len()).sum();
    let expected = events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::ModelChanged | EventKind::ContextCompacted
            )
        })
        .count();
    assert_eq!(markers, expected);
    assert_eq!(expected, 15, "11 context.compacted and 4 model.changed");
    let with_tokens = board.agents.iter().filter(|a| a.tokens.is_some()).count();
    assert_eq!(
        with_tokens,
        events
            .iter()
            .filter(|e| e.kind == EventKind::UsageReported)
            .count()
    );
}

// --- Details ------------------------------------------------------------------------------

#[test]
fn details_timeline_rows_carry_role_model_duration_and_pointer() {
    let mut done = worker(EventKind::TaskDone, "w1", 4, "m1");
    done.duration = Some(std::time::Duration::from_millis(1_500));
    done.model = Some(ModelId("model-a".to_owned()));
    let events = vec![
        worker(EventKind::TaskStarted, "w1", 1, "m1"),
        worker(EventKind::TaskStarted, "w2", 2, "m2"),
        done,
    ];

    let details = details(&events, &MilestoneId("m1".to_owned()));

    assert_eq!(details.milestone, MilestoneId("m1".to_owned()));
    assert_eq!(details.timeline.len(), 2);
    let row = &details.timeline[1];
    assert_eq!(row.kind, "task.done");
    assert_eq!(row.role, Role::Worker);
    assert_eq!(row.model, Some(ModelId("model-a".to_owned())));
    assert_eq!(row.duration_ms, Some(1_500));
    assert_eq!(row.pointer, events[2].source);
    assert_eq!(row.lane, lane("build"));
    assert_eq!(row.at, at(4));
    assert!(!row.unmapped);
}

#[test]
fn details_of_the_m0_milestones_cover_every_event_of_the_milestone() {
    let events = golden();
    let milestone = events.iter().find_map(|e| e.milestone_id.clone()).unwrap();

    let details = details(&events, &milestone);

    let expected = events
        .iter()
        .filter(|e| e.milestone_id.as_ref() == Some(&milestone))
        .count();
    assert_eq!(details.timeline.len(), expected);
    assert!(
        details
            .timeline
            .windows(2)
            .all(|pair| pair[0].at <= pair[1].at)
    );
}

#[test]
fn the_raw_line_view_shows_the_scrubbed_line_beside_its_pointer() {
    let event = worker(EventKind::TaskStarted, "w1", 1, "m1");
    let secret = token("ghp_", &"a".repeat(36));
    let raw = format!(r#"{{"type":"message","text":"use {secret} now"}}"#);

    let view = raw_line_view(&event, raw.as_bytes());

    assert_eq!(view.event_id, event.id);
    assert_eq!(view.kind, "task.started");
    assert_eq!(view.pointer, event.source);
    assert!(!view.line.text.contains(&secret));
    assert!(view.line.text.contains("[redacted:github_token]"));
    assert_eq!(view.line.masked, 1);
}

// --- B-006 scrub --------------------------------------------------------------------------

/// Built at run time so no source line looks like a credential to the secret scanners.
fn token(prefix: &str, body: &str) -> String {
    format!("{prefix}{body}")
}

#[test]
fn scrub_masks_known_token_shapes_without_altering_the_store() {
    let samples = [
        (token("ghp_", &"A1".repeat(18)), "github_token"),
        (token("github_pat_", &"b2".repeat(15)), "github_token"),
        (token("gho_", &"c3".repeat(18)), "github_token"),
        (token("sk-", &"d4".repeat(12)), "api_key"),
        (token("AKIA", "ABCDEFGHIJKLMNOP"), "aws_key"),
        (token("Bearer ", &"e5".repeat(12)), "bearer_token"),
        (token("Basic ", &"dXNlcjpwYXNz".repeat(2)), "basic_auth"),
        (token("xoxb-", "123456789012-abcdefghij"), "slack_token"),
        (
            format!(
                "{}.{}.{}",
                token("eyJ", "hbGciOiJIUzI1"),
                "f".repeat(12),
                "g".repeat(12)
            ),
            "jwt",
        ),
        (
            format!(
                "-----BEGIN {k} PRIVATE KEY-----\nMIIBVQIBADANBgkq\n-----END {k} PRIVATE KEY-----",
                k = "RSA"
            ),
            "private_key",
        ),
    ];
    for (secret, label) in &samples {
        let raw = format!("before {secret} after");

        let scrubbed = scrub(raw.as_bytes());

        assert_eq!(
            scrubbed.text,
            format!("before [redacted:{label}] after"),
            "{label}"
        );
        assert_eq!(scrubbed.masked, 1, "{label}");
        assert!(raw.contains(secret.as_str()), "the input is untouched");
    }

    let several = format!("{} and {} and {}", samples[0].0, samples[4].0, samples[3].0);
    assert_eq!(scrub(several.as_bytes()).masked, 3);

    let plain = br#"{"type":"message","text":"nothing to hide, sk- is short"}"#;
    let clean = scrub(plain);
    assert_eq!(clean.text.as_bytes(), plain);
    assert_eq!(clean.masked, 0);

    let invalid = [b'o', b'k', 0xff, b'!'];
    assert_eq!(
        scrub(&invalid).masked,
        0,
        "invalid UTF-8 is shown lossy, not rejected"
    );
}

#[cfg(feature = "store")]
mod stored {
    use observatory_core::config::IngestConfig;
    use observatory_core::ingest::{JsonlSource, SessionTree, Source, mapping_version_of};
    use observatory_core::mapping::TreeMapper;
    use observatory_core::store::{CaptureInfo, EventScope, Store, TimeRange};

    use super::*;

    fn m0_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.jsonl")
            .canonicalize()
            .expect("the M0 capture exists")
    }

    fn index_m0(db: &Path) -> Store {
        let root = m0_root();
        let cfg = IngestConfig::default();
        let tree = SessionTree::discover(&root, &cfg).unwrap();
        let mut store = Store::open(db).unwrap();
        store
            .record_tree(
                &tree,
                &CaptureInfo::from_session_file(&root).unwrap(),
                false,
            )
            .unwrap();
        let mut source = JsonlSource::open_tree(&tree, &cfg).unwrap();
        let mut mapper = TreeMapper::new(&tree, mapping_version_of(&root).unwrap());
        loop {
            let mut batch = source.next_batch().unwrap();
            let at_eof = source.at_eof();
            if at_eof {
                batch.extend(source.drain_partial());
            }
            let mut mapped: Vec<Event> = batch.iter().flat_map(|r| mapper.map(r)).collect();
            if at_eof {
                mapped.extend(mapper.finish());
            }
            store.insert_events(&mapped).unwrap();
            if at_eof {
                break;
            }
        }
        store
    }

    #[test]
    fn the_board_built_from_the_store_matches_the_board_built_from_the_golden() {
        let dir = tempfile::tempdir().unwrap();
        let store = index_m0(&dir.path().join("observatory.sqlite"));
        let golden = golden();
        let mission = golden[0].mission_id.clone();

        let stored = store
            .events(&EventScope::Mission(mission.clone()), TimeRange::ALL)
            .unwrap();

        assert_eq!(stored.len(), golden.len());
        assert_eq!(store.unknown_count(&mission).unwrap(), 0);
        let from_store = all(&stored);
        assert_eq!(from_store.event_count, 1279);
        assert_eq!(
            from_store.tray_unknown,
            store.unknown_count(&mission).unwrap()
        );
        assert_eq!(from_store.lane_counts, all(&golden).lane_counts);
        assert_eq!(from_store.cards.len(), all(&golden).cards.len());
    }

    #[test]
    fn scrubbing_a_stored_line_leaves_the_store_and_the_file_as_they_were() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("observatory.sqlite");
        let store = index_m0(&db);
        let golden = golden();
        let event = &golden[golden.len() / 2];
        let before = store.pointer(&event.id).unwrap();
        let SourcePtr::Jsonl {
            byte_offset,
            byte_len,
            ..
        } = &before
        else {
            panic!("a session event has a jsonl pointer");
        };
        let path = store_path(&db, &event.id);
        let file_before = std::fs::read(&path).unwrap();
        let raw =
            &file_before[*byte_offset as usize..(*byte_offset + u64::from(*byte_len)) as usize];

        let view = raw_line_view(event, raw);

        assert!(!view.line.text.is_empty());
        assert_eq!(store.pointer(&event.id).unwrap(), before);
        assert_eq!(std::fs::read(&path).unwrap(), file_before);
        let rows = |db: &Path| -> Vec<String> {
            let conn = rusqlite::Connection::open(db).unwrap();
            let mut statement = conn
                .prepare("SELECT id || kind || lane FROM events ORDER BY id")
                .unwrap();
            statement
                .query_map([], |r| r.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        let stored_rows = rows(&db);
        let _ = scrub(raw);
        assert_eq!(rows(&db), stored_rows);
        let text = std::fs::read(&db).unwrap();
        assert!(
            !String::from_utf8_lossy(&text).contains("[redacted:"),
            "no mask text reaches the database"
        );
    }

    fn store_path(db: &Path, id: &EventId) -> PathBuf {
        let conn = rusqlite::Connection::open(db).unwrap();
        let path: String = conn
            .query_row(
                "SELECT path FROM events WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        PathBuf::from(path)
    }
}

// --- render models of the board (the UI draws these as they are) ----------------------------

mod render {
    use std::collections::BTreeMap;

    use observatory_core::config::MissionConfig;
    use observatory_core::model::{
        Event, EventKind, LaneId, MilestoneId, Overlay, Role, Timestamp,
    };
    use observatory_core::view::{
        BoardView, DeclaredModels, Filter, FilterField, FilterOptions, GateBadge, GateVerdict,
        Scrubber, ValidationBadge, ViewConfig, WorkerGrouping, integrate_timeline,
        overlay_marker_rows,
    };

    use super::{engineering, golden, lane};

    const BRANCH: &str = "mission/observatory-v1";

    fn declared() -> DeclaredModels {
        DeclaredModels::from_config(&MissionConfig::default())
    }

    fn build(events: &[Event], cfg: &ViewConfig) -> BoardView {
        BoardView::build(events, cfg, &declared(), BRANCH, &engineering())
    }

    fn view_of(board: &BoardView, lane: LaneId) -> &observatory_core::view::LaneView {
        board
            .lanes
            .iter()
            .find(|view| view.lane == lane)
            .expect("the template has the lane")
    }

    #[test]
    fn panel_reports_tokens_per_role_and_model_drift_against_declared() {
        let events = golden();

        let board = build(&events, &ViewConfig::default());

        let panel = &board.panel;
        assert_eq!(panel.aria_label, "panel");
        let roles: Vec<Role> = panel.roles.iter().map(|row| row.role).collect();
        assert_eq!(
            roles,
            [
                Role::Orchestrator,
                Role::Worker,
                Role::ScrutinyValidator,
                Role::UserTestingValidator
            ]
        );
        let sessions: Vec<u64> = panel.roles.iter().map(|row| row.sessions).collect();
        assert_eq!(sessions, [1, 39, 7, 9], "the capture header's sessions");
        let declared_models = [
            "claude-fable-5.1",
            "claude-sonnet-5-5",
            "claude-opus-5-5",
            "claude-opus-5-5",
        ];
        for (row, declared) in panel.roles.iter().zip(declared_models) {
            assert_eq!(row.declared.0, declared);
            let mine = || events.iter().filter(|e| e.role == row.role);
            let usage: Vec<_> = mine()
                .filter(|e| e.kind == EventKind::UsageReported)
                .filter_map(|e| e.usage)
                .collect();
            assert_eq!(row.tokens.input, usage.iter().map(|u| u.input).sum::<u64>());
            assert_eq!(
                row.tokens.output,
                usage.iter().map(|u| u.output).sum::<u64>()
            );
            assert_eq!(
                row.tokens.cache_read,
                usage.iter().map(|u| u.cache_read).sum::<u64>()
            );
            assert_eq!(
                row.tokens.cache_write,
                usage.iter().map(|u| u.cache_write).sum::<u64>()
            );
            let mut seen: BTreeMap<String, u64> = BTreeMap::new();
            for model in mine().filter_map(|e| e.model.as_ref()) {
                *seen.entry(model.0.clone()).or_default() += 1;
            }
            let observed: BTreeMap<String, u64> = row
                .observed
                .iter()
                .map(|o| (o.model.0.clone(), o.events))
                .collect();
            assert_eq!(observed, seen, "{:?}", row.role);
            assert_eq!(
                row.drifted,
                seen.keys().any(|model| model != declared),
                "{:?}",
                row.role
            );
        }
        assert_eq!(panel.roles[1].tokens.input, 288_701);
        assert!(
            panel.roles.iter().any(|row| row.drifted),
            "the capture ran models other than the declared ones"
        );
        let (first, last) = (events.first().unwrap().at, events.last().unwrap().at);
        let elapsed = (last.0 - first.0).whole_milliseconds();
        assert_eq!(i128::from(panel.elapsed_ms.expect("elapsed")), elapsed);
    }

    #[test]
    fn human_overlay_filter_lists_the_human_markers() {
        let events = golden();

        let markers = overlay_marker_rows(&events, Overlay::Human);

        assert_eq!(markers.len(), 70);
        assert!(markers.iter().all(|marker| marker.lane == lane("plan")));
        let ids: Vec<&str> = markers.iter().map(|m| m.event_id.0.as_str()).collect();
        let expected: Vec<&str> = events
            .iter()
            .filter(|e| e.overlay == Some(Overlay::Human))
            .map(|e| e.id.0.as_str())
            .collect();
        assert_eq!(ids, expected);
        assert!(overlay_marker_rows(&events, Overlay::Officer).is_empty());

        let board = build(&events, &ViewConfig::default());
        assert_eq!(board.overlay_human.aria_label, "overlay-human");
        assert_eq!(board.overlay_human.markers.len(), 70);
        let plan = view_of(&board, lane("plan"));
        let marked = plan
            .groups
            .iter()
            .flat_map(|g| &g.rows)
            .filter(|row| row.overlay == Some(Overlay::Human))
            .count();
        assert_eq!(marked, 70, "each marker sits on its phase lane");
    }

    #[test]
    fn integrate_timeline_lists_gate_runs_per_milestone() {
        let events = golden();

        let groups = integrate_timeline(&events, &engineering());

        let rows: usize = groups.iter().map(|g| g.rows.len()).sum();
        assert_eq!(rows, 134);
        let runs: BTreeMap<Option<String>, u64> = groups
            .iter()
            .map(|g| (g.milestone.as_ref().map(|m| m.0.clone()), g.gate_runs))
            .collect();
        assert_eq!(runs.get(&Some("m0".to_owned())), Some(&18));
        assert_eq!(runs.get(&Some("s".to_owned())), Some(&26));
        assert_eq!(runs.get(&None), Some(&26));
        assert_eq!(runs.values().sum::<u64>(), 70);
        for group in &groups {
            assert!(group.rows.windows(2).all(|w| w[0].at <= w[1].at));
            assert!(group.rows.iter().all(|row| row.lane == lane("integrate")));
        }
    }

    #[test]
    fn the_gate_placeholder_names_the_pending_state_branch_verdict_and_clocks() {
        let events = golden();
        let last_verdict = events
            .iter()
            .rev()
            .find_map(|e| match e.kind.prd_name().as_ref() {
                "gate_command.passed" => Some(GateVerdict::Passed),
                "gate_command.failed" => Some(GateVerdict::Failed),
                _ => None,
            });

        let board = build(&events, &ViewConfig::default());

        let gate = view_of(&board, lane("gate"));
        assert_eq!(gate.event_count, 0);
        let placeholder = gate.placeholder.as_ref().expect("no PR: a placeholder");
        assert_eq!(placeholder.text, "gate: pending (no PR)");
        assert_eq!(placeholder.branch, BRANCH);
        assert_eq!(placeholder.last_gate_command, last_verdict);
        assert_eq!(placeholder.gate_runs, 35);
        let clocks: Vec<(&str, Option<&str>)> = placeholder
            .clocks
            .iter()
            .map(|c| (c.label.as_str(), c.value.as_deref()))
            .collect();
        assert_eq!(
            clocks,
            [
                ("engineering cycle time", None),
                ("lead time to production: delivery platform's clock", None)
            ]
        );
        assert_eq!(board.gate.badge, GateBadge::Pending);
        for view in board
            .lanes
            .iter()
            .filter(|l| l.lane != LaneId::from("gate"))
        {
            assert!(view.placeholder.is_none());
        }
    }

    #[test]
    fn the_m0_board_view_has_lane_counts_cards_rows_and_build_groups() {
        let events = golden();

        let board = build(&events, &ViewConfig::default());

        let counts: Vec<u64> = board.lanes.iter().map(|l| l.event_count).collect();
        assert_eq!(counts, [437, 69, 424, 93, 122, 134, 0]);
        for view in &board.lanes {
            let rows: usize = view.groups.iter().map(|g| g.rows.len()).sum();
            assert_eq!(rows as u64, view.event_count, "{}", view.name);
        }
        assert_eq!(board.event_count, 1279);
        assert_eq!(board.tray_unknown, 0);
        let milestones: Vec<&str> = board
            .lanes
            .iter()
            .flat_map(|l| l.cards.iter().map(|c| c.milestone.0.as_str()))
            .collect();
        assert_eq!(milestones.len(), 7);
        for view in &board.lanes {
            assert!(view.cards.iter().all(|c| c.lane == view.lane));
            assert_eq!(view.card_count, view.cards.len());
        }

        let build_lane = view_of(&board, lane("build"));
        assert!(build_lane.grouped);
        assert_eq!(
            build_lane.groups.len(),
            30,
            "worker sessions with Build events"
        );
        assert!(
            build_lane
                .groups
                .iter()
                .all(|g| g.role == Some(Role::Worker))
        );
        let per_feature = build(
            &events,
            &ViewConfig {
                grouping: WorkerGrouping::PerFeature,
                ..ViewConfig::default()
            },
        );
        let regrouped = view_of(&per_feature, lane("build"));
        assert_eq!(regrouped.event_count, build_lane.event_count);
        let features: std::collections::BTreeSet<_> = events
            .iter()
            .filter(|e| e.lane == lane("build"))
            .map(|e| e.feature_id.clone())
            .collect();
        assert_eq!(regrouped.groups.len(), features.len());
        assert!(regrouped.groups.iter().all(|g| g.role.is_none()));
        let integrate = view_of(&board, lane("integrate"));
        assert_eq!(
            integrate.groups.len(),
            integrate_timeline(&events, &engineering()).len(),
            "Integrate groups its rows per milestone"
        );
        assert!(!view_of(&board, lane("plan")).grouped);
    }

    #[test]
    fn the_board_view_follows_the_filter_and_the_scrubber_together() {
        let events = golden();
        let middle: Timestamp = events[events.len() / 2].at;
        let cfg = ViewConfig {
            filter: Filter {
                role: Some(Role::Worker),
                ..Filter::default()
            },
            at: Some(middle),
            ..ViewConfig::default()
        };

        let board = build(&events, &cfg);

        let expected: Vec<&Event> = events
            .iter()
            .filter(|e| e.role == Role::Worker && e.at <= middle)
            .collect();
        assert_eq!(board.event_count, expected.len() as u64);
        let rows: usize = board
            .lanes
            .iter()
            .flat_map(|l| &l.groups)
            .map(|g| g.rows.len())
            .sum();
        assert_eq!(rows, expected.len());
        for empty in [lane("plan"), lane("scrutiny"), lane("user_testing")] {
            assert_eq!(view_of(&board, empty.clone()).event_count, 0, "{empty:?}");
        }
        assert_eq!(board.overlay_human.markers.len(), 0);
    }

    #[test]
    fn filter_options_list_what_the_capture_holds_and_a_choice_parses_back() {
        let events = golden();

        let options = FilterOptions::of(&events);

        let values = |choices: &[observatory_core::view::Choice]| -> Vec<String> {
            choices.iter().map(|c| c.value.clone()).collect()
        };
        assert_eq!(
            values(&options.milestones),
            ["m0", "m1", "m2", "m3", "m4", "m5", "s"]
        );
        assert_eq!(options.features.len(), {
            let mut all: Vec<_> = events.iter().filter_map(|e| e.feature_id.clone()).collect();
            all.sort();
            all.dedup();
            all.len()
        });
        let expected_roles: Vec<String> = Role::ALL
            .iter()
            .filter(|role| events.iter().any(|e| e.role == **role))
            .map(|role| role.as_str().to_owned())
            .collect();
        assert_eq!(values(&options.roles), expected_roles);
        assert_eq!(expected_roles[..2], ["orchestrator", "worker"]);
        assert!(values(&options.models).contains(&"claude-sonnet-5-5".to_owned()));
        assert_eq!(values(&options.validation), ["red", "grey", "green"]);
        let labels: Vec<&str> = options
            .validation
            .iter()
            .map(|c| c.label.as_str())
            .collect();
        assert_eq!(labels, ["RED", "GREY", "GREEN"]);
        assert_eq!(values(&options.gate)[0], "pending");

        let mut filter = Filter::default();
        filter.set(FilterField::Role, "worker");
        filter.set(FilterField::Validation, "red");
        filter.set(FilterField::Gate, "ci_green");
        filter.set(FilterField::Milestone, "m0");
        filter.set(FilterField::Model, "claude-sonnet-5-5");
        filter.set(FilterField::Feature, "some-feature");
        assert_eq!(filter.role, Some(Role::Worker));
        assert_eq!(filter.validation, Some(ValidationBadge::Red));
        assert_eq!(filter.gate, Some(GateBadge::CiGreen));
        assert_eq!(filter.milestone, Some(MilestoneId("m0".to_owned())));
        filter.set(FilterField::Role, "");
        filter.set(FilterField::Validation, "not-a-state");
        assert_eq!(filter.role, None);
        assert_eq!(filter.validation, None);
        let names: Vec<&str> = FilterField::ALL.iter().map(|f| f.aria_label()).collect();
        assert_eq!(
            names,
            [
                "filter-milestone",
                "filter-feature",
                "filter-role",
                "filter-model",
                "filter-validation",
                "filter-gate"
            ]
        );
    }

    #[test]
    fn the_scrubber_stops_at_each_event_time_and_the_last_stop_is_live() {
        let events = golden();
        let scrubber = Scrubber::new(&events);

        let mut times: Vec<Timestamp> = events.iter().map(|e| e.at).collect();
        times.dedup();
        assert_eq!(scrubber.stops(), times.len());
        assert_eq!(scrubber.last_index(), times.len() - 1);
        assert_eq!(scrubber.at(0), Some(times[0]));
        assert_eq!(scrubber.at(scrubber.last_index()), None, "the end is live");
        assert_eq!(scrubber.at(1_000_000), None);
        assert_eq!(scrubber.index_of(None), scrubber.last_index());
        let middle = times[times.len() / 2];
        assert_eq!(scrubber.index_of(Some(middle)), times.len() / 2);
        let between = Timestamp(middle.0 + time::Duration::milliseconds(1));
        if between < times[times.len() / 2 + 1] {
            assert_eq!(scrubber.index_of(Some(between)), times.len() / 2);
        }
        assert_eq!(
            scrubber.index_of(Some(Timestamp(times[0].0 - time::Duration::hours(1)))),
            0
        );
        assert_eq!(scrubber.label(Some(middle)), middle.to_rfc3339_millis());
        assert_eq!(
            scrubber.label(None),
            times.last().unwrap().to_rfc3339_millis()
        );
        assert_eq!(Scrubber::new(&[]).stops(), 0);
        assert_eq!(Scrubber::new(&[]).label(None), "");
    }
}

#[test]
fn a_per_worker_sub_lane_is_headed_by_the_session_name() {
    let sessions = vec![SessionInfo {
        id: SessionId("2c04bd42".to_owned()),
        parent: None,
        role: Role::Worker,
        model: None,
        tag: Some("m0-integrate-s".to_owned()),
        effort: None,
    }];
    let group = |key: &str, role| RowGroup {
        key: key.to_owned(),
        label: key.to_owned(),
        role,
        rows: Vec::new(),
    };
    assert_eq!(
        group("2c04bd42", Some(Role::Worker)).title(&sessions),
        "m0-integrate-s"
    );
    assert_eq!(
        group("unknown-session", Some(Role::Worker)).title(&sessions),
        "unknown-session",
        "a session the list does not know keeps its id"
    );
    assert_eq!(
        group("2c04bd42", None).title(&sessions),
        "2c04bd42",
        "a feature or milestone group keeps its own label"
    );
}

#[test]
fn a_red_check_returns_the_card_to_the_build_lane_and_counts_one_cycle() {
    let template = engineering();
    let gh = |kind, second| {
        Build::new(kind, Role::External, "gh", second)
            .milestone("m1")
            .done()
    };
    let mut events = vec![
        worker(EventKind::TaskStarted, "w1", 1, "m1"),
        Build::new(EventKind::IntegrationMerged, Role::Orchestrator, "o1", 2)
            .milestone("m1")
            .done(),
        gh(EventKind::PrOpened, 3),
    ];
    let board = all(&events);
    assert_eq!(board.cards[0].cycle, 0, "no red check, no fix cycle");

    events.push(gh(EventKind::CheckFailed, 4));
    let board = all(&events);
    assert_eq!(board.gate.badge, GateBadge::CiRed);
    assert_eq!(board.cards[0].lane, *template.home(Role::Worker));
    assert_eq!(board.cards[0].cycle, 1);

    // a second red while the first is still being fixed is the same cycle
    events.push(worker(EventKind::TaskStarted, "w2", 5, "m1"));
    events.push(gh(EventKind::CheckFailed, 6));
    let board = all(&events);
    assert_eq!(board.cards[0].lane, *template.home(Role::Worker));
    assert_eq!(board.cards[0].cycle, 1);

    events.push(gh(EventKind::CheckPassed, 7));
    let board = all(&events);
    assert_eq!(board.gate.badge, GateBadge::CiGreen);
    assert_eq!(board.cards[0].lane, *template.final_lane());
    assert_eq!(board.cards[0].cycle, 1);

    events.push(gh(EventKind::CheckFailed, 8));
    let board = all(&events);
    assert_eq!(board.gate.badge, GateBadge::CiRed);
    assert_eq!(board.cards[0].lane, *template.home(Role::Worker));
    assert_eq!(board.cards[0].cycle, 2);
}
