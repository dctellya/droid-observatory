//! The five Details tabs beside the Timeline (FR-D2 to FR-D6): the figures the UI draws, as
//! `core::view` functions over the events held.

use std::path::{Path, PathBuf};

use observatory_core::mapping::MappingVersion;
use observatory_core::metrics::{
    CostInput, Notes, PriceTable, ReworkKind, WasteCause, cost_report,
};
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, MilestoneId, MissionId, ModelId, Role,
    SessionId, SourcePtr, Timestamp,
};
use observatory_core::view::{
    GateGroup, commits_tab, cost_tab, criteria_tab, details, gate_tab, review_tab,
};

const PRICES: &str = include_str!("../../../config/prices/2026-10-22.toml");
const BRANCH: &str = "mission/observatory-v1";

fn template() -> CycleTemplate {
    CycleTemplate::engineering()
}

fn prices() -> PriceTable {
    PriceTable::parse(PRICES).expect("the committed price table parses")
}

fn golden() -> Vec<Event> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.events.json");
    let text = std::fs::read_to_string(path).expect("the golden file is committed");
    serde_json::from_str(&text).expect("the golden file is a list of events")
}

fn milestone(name: &str) -> MilestoneId {
    MilestoneId(name.to_owned())
}

fn count(events: &[Event], milestone: &MilestoneId, kinds: &[EventKind]) -> usize {
    events
        .iter()
        .filter(|event| event.milestone_id.as_ref() == Some(milestone))
        .filter(|event| kinds.contains(&event.kind))
        .count()
}

fn at(second: u32) -> Timestamp {
    Timestamp::parse(&format!(
        "2026-10-22T10:{:02}:{:02}.000Z",
        second / 60,
        second % 60
    ))
    .expect("valid time")
}

fn event(kind: EventKind, role: Role, second: u32, feature: Option<&str>) -> Event {
    Event {
        id: EventId(format!("session:{second}:0")),
        mission_id: MissionId("mission".to_owned()),
        milestone_id: Some(milestone("m9")),
        feature_id: feature.map(|f| FeatureId(f.to_owned())),
        lane: template().lane_of(&kind, role),
        overlay: kind.overlay(),
        kind,
        role,
        session_id: Some(SessionId("session".to_owned())),
        model: Some(ModelId("model-a".to_owned())),
        at: at(second),
        duration: None,
        usage: None,
        source: SourcePtr::Jsonl {
            path: PathBuf::from("fixtures/session.jsonl"),
            byte_offset: u64::from(second) * 100,
            byte_len: 90,
        },
        mapping_version: MappingVersion::V1,
    }
}

#[test]
fn cost_tab_totals_equal_the_milestone_row_of_the_cost_report() {
    let events = golden();
    let prices = prices();
    let end = events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .map(|event| event.at)
        .max()
        .expect("the golden holds events");
    let report = cost_report(&CostInput {
        events: &events,
        template: &template(),
        notes: &Notes::default(),
        prices: &prices,
        end,
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: observatory_core::config::OverheadApportionment::Separate,
    });
    let mut compared = 0;
    for row in report
        .tokens_by_milestone
        .iter()
        .filter(|row| row.milestone.is_some())
    {
        let milestone = row.milestone.clone().expect("filtered");
        let tab = cost_tab(&events, &milestone, &template(), &prices, &Notes::default());
        assert_eq!(tab.totals.tokens, row.tokens, "{milestone:?}");
        assert!((tab.totals.cost.amount - row.cost.amount).abs() < 0.005);
        assert_eq!(tab.price_table, "2026-10-22");
        assert_eq!(tab.totals.cost.price_table, "2026-10-22");
        let by_feature: u64 = tab.features.iter().map(|f| f.tokens.total()).sum();
        let by_role: u64 = tab.roles.iter().map(|r| r.tokens.total()).sum();
        assert_eq!(by_feature, row.tokens.total());
        assert_eq!(by_role, row.tokens.total());
        compared += 1;
    }
    assert!(compared >= 2, "the golden has turns in several milestones");
}

#[test]
fn cost_tab_lists_the_eight_causes_and_every_feature_cost_names_the_price_table() {
    let events = golden();
    let tab = cost_tab(
        &events,
        &milestone("s"),
        &template(),
        &prices(),
        &Notes::default(),
    );
    let causes: Vec<WasteCause> = tab.waste.iter().map(|row| row.cause).collect();
    assert_eq!(causes, WasteCause::ALL);
    assert!(!tab.features.is_empty());
    assert!(
        tab.features
            .iter()
            .all(|row| row.cost.price_table == "2026-10-22")
    );
}

#[test]
fn cost_tab_lists_an_unresolved_window_with_its_cause_lane() {
    let events = vec![
        event(EventKind::TaskStarted, Role::Worker, 1, Some("f1")),
        event(
            EventKind::FindingBlocking,
            Role::ScrutinyValidator,
            5,
            Some("f1"),
        ),
        event(EventKind::TaskDone, Role::Worker, 9, Some("f1")),
    ];
    let template = template();
    let tab = cost_tab(
        &events,
        &milestone("m9"),
        &template,
        &prices(),
        &Notes::default(),
    );
    assert_eq!(tab.rework.len(), 1);
    let window = &tab.rework[0];
    assert!(window.unresolved);
    assert!(window.closed.is_none());
    assert_eq!(window.kind, ReworkKind::Finding);
    assert_eq!(
        window.cause,
        template.name_of(&template.lane_of(&EventKind::FindingBlocking, Role::ScrutinyValidator))
    );
}

#[test]
fn review_tab_counts_equal_the_golden_events_of_the_milestone() {
    let events = golden();
    for name in ["s", "m0"] {
        let milestone = milestone(name);
        let tab = review_tab(&events, &milestone, &template(), &Notes::default());
        assert_eq!(
            tab.findings.len(),
            count(
                &events,
                &milestone,
                &[EventKind::FindingBlocking, EventKind::FindingNonBlocking]
            )
        );
        assert_eq!(
            tab.scenarios.len(),
            count(
                &events,
                &milestone,
                &[EventKind::QaScenarioPassed, EventKind::QaScenarioFailed]
            )
        );
    }
    let s = review_tab(&events, &milestone("s"), &template(), &Notes::default());
    assert_eq!(s.findings.len(), 24);
    assert!(
        s.findings
            .iter()
            .all(|f| f.severity == "non-blocking" && f.status == "open")
    );
}

#[test]
fn review_tab_marks_a_blocking_finding_resolved_once_rework_resolves_it() {
    let events = vec![
        event(
            EventKind::FindingBlocking,
            Role::ScrutinyValidator,
            5,
            Some("f1"),
        ),
        event(
            EventKind::FindingBlocking,
            Role::ScrutinyValidator,
            6,
            Some("f2"),
        ),
        event(EventKind::ReworkResolved, Role::Worker, 8, Some("f1")),
        event(
            EventKind::QaScenarioFailed,
            Role::UserTestingValidator,
            9,
            Some("f1"),
        ),
        event(
            EventKind::QaEvidence,
            Role::UserTestingValidator,
            10,
            Some("f1"),
        ),
    ];
    let tab = review_tab(&events, &milestone("m9"), &template(), &Notes::default());
    let status = |feature: &str| {
        tab.findings
            .iter()
            .find(|f| f.feature.as_ref().is_some_and(|x| x.0 == feature))
            .map(|f| (f.severity.as_str(), f.status.as_str()))
    };
    assert_eq!(status("f1"), Some(("blocking", "resolved")));
    assert_eq!(status("f2"), Some(("blocking", "open")));
    assert_eq!(tab.scenarios.len(), 1);
    assert_eq!(tab.scenarios[0].result, "failed");
    assert_eq!(
        tab.scenarios[0].evidence,
        vec![EventId("session:10:0".to_owned())]
    );
}

#[test]
fn criteria_tab_maps_each_criterion_to_events_or_the_unsatisfied_marker() {
    let events = vec![
        event(EventKind::FeatureDefined, Role::Orchestrator, 0, Some("f1")),
        event(EventKind::TestsRun, Role::Worker, 2, Some("f1")),
        event(EventKind::TestsRun, Role::Worker, 3, Some("f1")),
        event(EventKind::CommitCreated, Role::Worker, 4, Some("f1")),
    ];
    let tab = criteria_tab(&events, &milestone("m9"), &Notes::default());
    assert_eq!(tab.features.len(), 1);
    let feature = &tab.features[0];
    assert_eq!(feature.defined_by, Some(EventId("session:0:0".to_owned())));
    assert!(!feature.criteria.is_empty());
    for criterion in &feature.criteria {
        assert!(
            criterion.satisfied_by.is_empty() == (criterion.status == "unsatisfied"),
            "{}",
            criterion.label
        );
        assert!(criterion.status == "satisfied" || criterion.status == "unsatisfied");
    }
    let tests = feature
        .criteria
        .iter()
        .find(|c| c.label == "Tests run")
        .expect("tests criterion");
    assert_eq!(tests.satisfied_by.len(), 2);
    assert!(feature.criteria.iter().any(|c| c.status == "unsatisfied"));
}

#[test]
fn criteria_tab_lists_features_of_the_golden_milestone() {
    let events = golden();
    let tab = criteria_tab(&events, &milestone("s"), &Notes::default());
    assert!(!tab.features.is_empty());
    assert!(
        tab.features
            .iter()
            .any(|f| f.criteria.iter().any(|c| !c.satisfied_by.is_empty()))
    );
}

#[test]
fn commits_tab_counts_equal_the_commit_created_events_of_the_milestone() {
    let events = golden();
    for (name, expected) in [("s", 28), ("m0", 14)] {
        let milestone = milestone(name);
        let tab = commits_tab(&events, &milestone, &template(), BRANCH, None);
        assert_eq!(tab.commits.len(), expected, "{name}");
        assert_eq!(
            tab.commits.len(),
            count(&events, &milestone, &[EventKind::CommitCreated])
        );
        assert_eq!(
            tab.checks.len(),
            count(
                &events,
                &milestone,
                &[
                    EventKind::GateCommandRun,
                    EventKind::GateCommandPassed,
                    EventKind::GateCommandFailed
                ]
            )
        );
    }
}

#[test]
fn commits_tab_says_integrated_in_mission_and_never_merged() {
    let events = vec![
        event(EventKind::CommitCreated, Role::Worker, 1, Some("f1")),
        event(
            EventKind::IntegrationConflict,
            Role::Orchestrator,
            2,
            Some("f1"),
        ),
        event(
            EventKind::IntegrationConflictResolved,
            Role::Orchestrator,
            3,
            Some("f1"),
        ),
        event(
            EventKind::IntegrationMerged,
            Role::Orchestrator,
            4,
            Some("f1"),
        ),
        event(
            EventKind::GateCommandPassed,
            Role::Orchestrator,
            5,
            Some("f1"),
        ),
    ];
    let tab = commits_tab(&events, &milestone("m9"), &template(), BRANCH, None);
    assert_eq!(tab.integration.len(), 1);
    assert_eq!(
        tab.integration[0].text,
        format!("integrated in mission {BRANCH}")
    );
    assert_eq!(tab.conflicts.len(), 2);
    let all = tab
        .integration
        .iter()
        .chain(&tab.conflicts)
        .chain(&tab.checks)
        .map(|row| row.text.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(!all.contains("merged"), "{all}");
}

#[test]
fn gate_tab_is_pending_with_empty_sections_and_two_unvalued_clocks_without_a_pr() {
    let events = golden();
    let template = template();
    let view = GateGroup::of(&events).view(&template);
    let tab = gate_tab(&view, &events);
    assert_eq!(tab.state, "Pending");
    assert!(tab.pr.is_none());
    assert!(tab.checks.is_empty());
    assert!(tab.attestations.is_empty());
    assert_eq!(tab.reviews, 0);
    assert!(tab.history.is_empty());
    assert_eq!(tab.clocks.len(), 2);
    assert!(tab.clocks.iter().all(|clock| clock.value.is_none()));
    let labels: Vec<&str> = tab.clocks.iter().map(|c| c.label.as_str()).collect();
    assert!(
        labels
            .iter()
            .any(|l| l.to_lowercase().contains("cycle time"))
    );
    assert!(
        labels
            .iter()
            .any(|l| l.to_lowercase().contains("lead time"))
    );
}

#[test]
fn details_stay_the_timeline_of_the_milestone() {
    let events = golden();
    assert!(!details(&events, &milestone("s")).timeline.is_empty());
}
