//! The cycle is data, not code (F-041, D-035): a second template with four lanes and other names
//! renders the board, the digest, the Panel and the post-mortem input through `core::view` with
//! no code change, and the default template reproduces the M2 behaviour byte for byte.
//!
//! The `four_step` template below is the only place its lane names exist.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use observatory_core::config::{Config, CycleConfig, MissionConfig};
use observatory_core::ingest::{JsonlSource, SessionTree, Source, mapping_version_of};
use observatory_core::mapping::TreeMapper;
use observatory_core::model::{CycleTemplate, Event, EventKind, LaneGrouping, LaneId, Role};
use observatory_core::view::{
    BoardView, DeclaredModels, ViewConfig, card_lane, lane_counts, post_mortem_input,
};

const SLUG: &str = "2026-10-22-observatory-v1-m0";

const FOUR_STEP: &str = r#"
name = "four_step"
final_event = "integration.merged"

[[lanes]]
id = "spec"
name = "Spec"
owner_role = "orchestrator"
home_for = ["orchestrator", "human"]
kinds = [
  "intent.created", "intent.amended", "question.asked", "intent.answered", "plan.created",
  "feature.defined", "milestone.defined", "plan.revised", "escalation.raised", "spec.versioned",
  "approval.requested", "approval.given", "question.answered", "mission.paused",
  "mission.resumed", "mission.completed", "milestone.completed",
]

[[lanes]]
id = "code"
name = "Code"
owner_role = "worker"
home_for = ["worker", "unresolved"]
grouping = "sessions"

[[lanes]]
id = "check"
name = "Check"
owner_role = "scrutiny_validator"
home_for = ["scrutiny_validator", "user_testing_validator"]
kinds = [
  "finding.blocking", "finding.non_blocking", "rework.requested", "rework.resolved",
  "scrutiny.passed", "qa.scenario.passed", "qa.scenario.failed", "qa.evidence",
]

[[lanes]]
id = "ship"
name = "Ship"
owner_role = "external"
home_for = ["external"]
grouping = "milestones"
clocks = ["spec_to_ship"]
kinds = [
  "integration.merged", "integration.conflict", "integration.conflict_resolved",
  "gate_command.run", "gate_command.passed", "gate_command.failed", "pr.opened",
  "pr.review_requested", "check.started", "check.passed", "check.failed",
  "check.required_missing", "ruleset.blocked", "attestation.recorded", "attestation.verified",
  "review.outcome", "handed_to_cd",
]

[[clocks]]
id = "spec_to_ship"
label = "spec to ship"
start = "feature.defined"
start_fallback = "task.started"
end = "integration.merged"

[rules]
integration_session = "ship"
contract_session = "spec"
orchestrator_commit = "ship"
kind_follows_emitter = ["feature.defined"]
"#;

fn four_step() -> CycleTemplate {
    CycleTemplate::from_toml(FOUR_STEP).expect("the four_step template parses")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root exists")
}

fn ids(template: &CycleTemplate) -> Vec<&str> {
    template.lanes.iter().map(|lane| lane.id.as_str()).collect()
}

/// The M0 capture through the tree mapper the CLI uses, under `template`.
fn map_m0(template: &CycleTemplate) -> Vec<Event> {
    let path = repo_root().join(format!("fixtures/streams/{SLUG}.jsonl"));
    let cfg = observatory_core::config::IngestConfig::default();
    let tree = SessionTree::discover(&path, &cfg).expect("the M0 capture opens");
    let mut source = JsonlSource::open_tree(&tree, &cfg).expect("every session file opens");
    let version = mapping_version_of(&path).expect("capture header is readable");
    let mut mapper = TreeMapper::with_template(&tree, version, Arc::new(template.clone()));
    let mut events = Vec::new();
    loop {
        let mut batch = source.next_batch().expect("batch reads");
        let at_eof = source.at_eof();
        if at_eof {
            batch.extend(source.drain_partial());
        }
        for record in &batch {
            events.extend(mapper.map(record));
        }
        if at_eof {
            break;
        }
    }
    events.extend(mapper.finish());
    events.sort_by(|a, b| (a.at, &a.id.0).cmp(&(b.at, &b.id.0)));
    events
}

fn declared() -> DeclaredModels {
    DeclaredModels::from_config(&MissionConfig::default())
}

fn board(events: &[Event], template: &CycleTemplate) -> BoardView {
    BoardView::build(
        events,
        &ViewConfig::default(),
        &declared(),
        "mission/x",
        template,
    )
}

fn per_lane(events: &[Event]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for event in events {
        *counts.entry(event.lane.to_string()).or_insert(0) += 1;
    }
    counts
}

#[test]
fn the_default_template_is_the_engineering_cycle_of_d024() {
    let template = CycleTemplate::engineering();

    assert_eq!(template.name, "engineering");
    assert_eq!(
        ids(&template),
        [
            "plan",
            "contract",
            "build",
            "scrutiny",
            "user_testing",
            "integrate",
            "gate"
        ]
    );
    let names: Vec<&str> = template.lanes.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Plan",
            "Contract",
            "Build",
            "Scrutiny",
            "User testing",
            "Integrate",
            "Gate"
        ]
    );
    assert_eq!(template.final_event, "handed_to_cd");
    assert_eq!(template.final_lane(), &LaneId::from("gate"));
    assert_eq!(template.first(), &LaneId::from("plan"));
    let grouping: Vec<LaneGrouping> = template.lanes.iter().map(|l| l.grouping).collect();
    assert_eq!(grouping[2], LaneGrouping::Sessions);
    assert_eq!(grouping[5], LaneGrouping::Milestones);
    assert_eq!(
        grouping
            .iter()
            .filter(|g| **g != LaneGrouping::None)
            .count(),
        2
    );

    let homes = [
        (Role::Orchestrator, "plan"),
        (Role::Human, "plan"),
        (Role::Worker, "build"),
        (Role::Unresolved, "build"),
        (Role::ScrutinyValidator, "scrutiny"),
        (Role::UserTestingValidator, "user_testing"),
        (Role::External, "gate"),
    ];
    for (role, lane) in homes {
        assert_eq!(template.home(role), &LaneId::from(lane), "{role:?}");
    }

    let labels: Vec<&str> = template.clocks.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "engineering cycle time",
            "lead time to production: delivery platform's clock"
        ]
    );
    let cycle = &template.clocks[0];
    assert_eq!(cycle.start.as_deref(), Some("feature.defined"));
    assert_eq!(cycle.start_fallback.as_deref(), Some("task.started"));
    assert_eq!(cycle.end.as_deref(), Some("handed_to_cd"));
    assert_eq!(template.clocks[1].end, None);

    assert_eq!(
        template.rules.integration_session,
        LaneId::from("integrate")
    );
    assert_eq!(template.rules.contract_session, LaneId::from("contract"));
    assert_eq!(
        template.rules.orchestrator_commit,
        LaneId::from("integrate")
    );
    assert_eq!(template.rules.kind_follows_emitter, ["feature.defined"]);
}

#[test]
fn a_template_lists_every_fixed_kind_at_most_once_and_knows_the_final_event() {
    let text = |from: &str, to: &str| {
        assert!(FOUR_STEP.contains(from), "{from} is in the template");
        FOUR_STEP.replacen(from, to, 1)
    };

    let twice = text(
        r#"home_for = ["worker", "unresolved"]"#,
        r#"home_for = ["worker", "unresolved"]
kinds = ["intent.created"]"#,
    );
    let error = CycleTemplate::from_toml(&twice).expect_err("a kind in two lanes is refused");
    assert!(error.to_string().contains("intent.created"), "{error}");

    let unknown = text(r#""qa.evidence","#, r#""qa.evidence", "no.such_kind","#);
    let error = CycleTemplate::from_toml(&unknown).expect_err("an unknown kind is refused");
    assert!(error.to_string().contains("no.such_kind"), "{error}");

    let homeless = text(r#"home_for = ["external"]"#, "home_for = []");
    let error = CycleTemplate::from_toml(&homeless).expect_err("a role without a home is refused");
    assert!(error.to_string().contains("external"), "{error}");

    let outside = text(
        r#"final_event = "integration.merged""#,
        r#"final_event = "context.compacted""#,
    );
    let error =
        CycleTemplate::from_toml(&outside).expect_err("a final event in no lane is refused");
    assert!(error.to_string().contains("context.compacted"), "{error}");

    let missing_rule = text(
        r#"orchestrator_commit = "ship""#,
        r#"orchestrator_commit = "gone""#,
    );
    let error = CycleTemplate::from_toml(&missing_rule).expect_err("a rule needs a real lane");
    assert!(error.to_string().contains("gone"), "{error}");

    let no_lanes = "name = \"empty\"\nfinal_event = \"handed_to_cd\"\nclocks = []\nlanes = []\n\
                    [rules]\nintegration_session = \"a\"\ncontract_session = \"a\"\n\
                    orchestrator_commit = \"a\"\n";
    assert!(CycleTemplate::from_toml(no_lanes).is_err());

    assert!(CycleTemplate::from_toml(FOUR_STEP).is_ok());
}

#[test]
fn a_second_template_with_four_lanes_maps_every_event_into_one_of_them() {
    let template = four_step();
    let default_events = map_m0(&CycleTemplate::engineering());
    let events = map_m0(&template);

    assert_eq!(events.len(), default_events.len(), "nothing dropped");
    let counts = per_lane(&events);
    let wanted: BTreeSet<&str> = ["spec", "code", "check", "ship"].into_iter().collect();
    assert!(
        counts.keys().all(|id| wanted.contains(id.as_str())),
        "{counts:?}"
    );
    assert_eq!(counts.values().sum::<u64>(), events.len() as u64);
    assert!(counts.len() == 4, "every lane has events: {counts:?}");
    for event in &events {
        assert!(template.index(&event.lane).is_some(), "{}", event.lane);
    }
}

#[test]
fn the_board_iterates_the_template_lanes_and_places_the_placeholder_on_the_final_lane() {
    let template = four_step();
    let events = map_m0(&template);
    let board = board(&events, &template);

    assert_eq!(board.lanes.len(), 4);
    let names: Vec<&str> = board.lanes.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Spec", "Code", "Check", "Ship"]);
    let labels: Vec<&str> = board.lanes.iter().map(|l| l.aria_label.as_str()).collect();
    assert_eq!(labels, ["lane-1", "lane-2", "lane-3", "lane-4"]);
    let grouped: Vec<bool> = board.lanes.iter().map(|l| l.grouped).collect();
    assert_eq!(grouped, [false, true, false, true]);

    let with_placeholder: Vec<&str> = board
        .lanes
        .iter()
        .filter(|lane| lane.placeholder.is_some())
        .map(|lane| lane.lane.as_str())
        .collect();
    assert_eq!(with_placeholder, ["ship"]);
    let placeholder = board.lanes[3].placeholder.as_ref().unwrap();
    let clocks: Vec<&str> = placeholder
        .clocks
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(clocks, ["spec to ship"]);

    let digest = board.digest();
    assert_eq!(digest.lane_events.len(), 4);
    assert_eq!(digest.lane_cards.len(), 4);
    assert_eq!(
        digest.lanes,
        ["spec", "code", "check", "ship"].map(LaneId::from)
    );
    assert_eq!(
        digest.lane_events.iter().sum::<u64>(),
        events.len() as u64,
        "the digest counts every event"
    );
}

#[test]
fn the_panel_does_not_change_when_the_template_changes() {
    let default = CycleTemplate::engineering();
    let other = four_step();
    let under_default = board(&map_m0(&default), &default);
    let under_other = board(&map_m0(&other), &other);

    assert_eq!(under_default.panel, under_other.panel);
    assert!(!under_default.panel.roles.is_empty());
    assert_eq!(under_default.event_count, under_other.event_count);
}

#[test]
fn the_post_mortem_input_lists_one_summary_per_template_lane_and_one_reading_per_clock() {
    let template = four_step();
    let events = map_m0(&template);
    let board = board(&events, &template);
    let input = post_mortem_input(&events, &template);

    let lanes: Vec<&str> = input.lanes.iter().map(|l| l.lane.as_str()).collect();
    assert_eq!(lanes, ["spec", "code", "check", "ship"]);
    let names: Vec<&str> = input.lanes.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Spec", "Code", "Check", "Ship"]);
    let counts: Vec<u64> = input.lanes.iter().map(|l| l.event_count).collect();
    let board_counts: Vec<u64> = board.lanes.iter().map(|l| l.event_count).collect();
    assert_eq!(counts, board_counts);
    for summary in &input.lanes {
        assert_eq!(summary.first_at.is_some(), summary.event_count > 0);
        assert!(summary.first_at <= summary.last_at);
    }

    assert_eq!(input.clocks.len(), 1);
    let reading = &input.clocks[0];
    assert_eq!(reading.id, "spec_to_ship");
    assert_eq!(reading.label, "spec to ship");
    assert!(reading.start_at.is_some());
    assert_eq!(reading.end_at, None, "no integration.merged on the capture");
    assert_eq!(input.final_event_at, None);

    let started: BTreeSet<_> = events
        .iter()
        .filter(|e| e.kind.prd_name() == "task.started")
        .filter_map(|e| e.feature_id.clone())
        .collect();
    let defined: BTreeSet<_> = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::FeatureDefined))
        .filter_map(|e| e.feature_id.clone())
        .collect();
    let expected = started.difference(&defined).count() as u64;
    assert!(
        expected > 0,
        "the capture has features that never got a feature.defined"
    );
    assert_eq!(reading.fallback_used, expected);
}

#[test]
fn cards_and_lane_counts_follow_the_template() {
    let template = four_step();
    let events = map_m0(&template);
    let board = board(&events, &template);

    let counts = lane_counts(&events, &template);
    let board_counts: Vec<u64> = board.lanes.iter().map(|l| l.event_count).collect();
    assert_eq!(counts, board_counts);
    assert_eq!(counts.len(), 4);

    let mut by_milestone: BTreeMap<String, Vec<Event>> = BTreeMap::new();
    for event in &events {
        if let Some(milestone) = &event.milestone_id {
            by_milestone
                .entry(milestone.to_string())
                .or_default()
                .push(event.clone());
        }
    }
    assert!(!by_milestone.is_empty());
    for (milestone, own) in &by_milestone {
        let lane = card_lane(own, &template);
        assert!(template.index(&lane).is_some(), "{milestone}: {lane}");
        let card = board
            .lanes
            .iter()
            .flat_map(|view| &view.cards)
            .find(|card| card.milestone.to_string() == *milestone)
            .expect("every milestone has a card");
        assert_eq!(card.lane, lane, "{milestone}");
    }
    assert_eq!(card_lane(&[], &template), LaneId::from("spec"));
}

#[test]
fn default_template_renders_the_same_board_as_the_m2_golden() {
    let template = CycleTemplate::engineering();
    let events = map_m0(&template);
    let board = board(&events, &template);

    let names: Vec<&str> = board.lanes.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Plan",
            "Contract",
            "Build",
            "Scrutiny",
            "User testing",
            "Integrate",
            "Gate"
        ]
    );
    let labels: Vec<&str> = board.lanes.iter().map(|l| l.aria_label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "lane-1", "lane-2", "lane-3", "lane-4", "lane-5", "lane-6", "lane-7"
        ]
    );
    let grouped: Vec<bool> = board.lanes.iter().map(|l| l.grouped).collect();
    assert_eq!(grouped, [false, false, true, false, false, true, false]);
    let counts: Vec<u64> = board.lanes.iter().map(|l| l.event_count).collect();
    assert_eq!(counts, [437, 69, 424, 93, 122, 134, 0]);

    let gate = &board.lanes[6];
    let placeholder = gate.placeholder.as_ref().expect("no PR on the capture");
    assert_eq!(placeholder.text, "gate: pending (no PR)");
    let clocks: Vec<&str> = placeholder
        .clocks
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(
        clocks,
        [
            "engineering cycle time",
            "lead time to production: delivery platform's clock"
        ]
    );
    assert!(board.lanes[..6].iter().all(|l| l.placeholder.is_none()));
}

#[test]
fn a_lane_follows_the_home_of_its_emitting_role_when_the_template_says_so() {
    let template = four_step();
    let events = map_m0(&template);
    let scrutiny_session = "6eb88099-278d-4ddb-9f5d-23f1255cdbc5";
    let defined: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::FeatureDefined))
        .filter(|e| {
            e.session_id
                .as_ref()
                .is_some_and(|s| s.0 == scrutiny_session)
        })
        .collect();
    assert!(!defined.is_empty());
    assert!(
        defined.iter().all(|e| e.lane == LaneId::from("check")),
        "the validator's feature.defined lands in the home of its role"
    );
    let orchestrator = events
        .iter()
        .find(|e| matches!(e.kind, EventKind::FeatureDefined) && e.role == Role::Orchestrator);
    assert_eq!(orchestrator.map(|e| &e.lane), Some(&LaneId::from("spec")));

    let without_rule = FOUR_STEP.replace(
        r#"kind_follows_emitter = ["feature.defined"]"#,
        "kind_follows_emitter = []",
    );
    let template = CycleTemplate::from_toml(&without_rule).unwrap();
    let events = map_m0(&template);
    assert!(
        events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::FeatureDefined))
            .all(|e| e.lane == LaneId::from("spec")),
        "without the rule the kind keeps its fixed lane"
    );
}

#[test]
fn the_template_comes_from_the_config_and_falls_back_to_the_default_with_a_warning() {
    let config = Config::default();
    assert_eq!(config.cycle.template, "engineering");
    let loaded = config.cycle.load_template();
    assert_eq!(loaded.template, CycleTemplate::engineering());
    assert_eq!(loaded.warning, None);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("four_step.toml");
    std::fs::write(&path, FOUR_STEP).unwrap();
    let chosen = CycleConfig {
        template: path.to_string_lossy().into_owned(),
    };
    let loaded = chosen.load_template();
    assert_eq!(loaded.template, four_step());
    assert_eq!(loaded.warning, None);

    let missing = CycleConfig {
        template: dir.path().join("gone.toml").to_string_lossy().into_owned(),
    };
    let loaded = missing.load_template();
    assert_eq!(loaded.template, CycleTemplate::engineering());
    assert!(
        loaded
            .warning
            .as_deref()
            .is_some_and(|w| w.contains("gone.toml"))
    );

    let broken = dir.path().join("broken.toml");
    std::fs::write(&broken, "name = ").unwrap();
    let loaded = CycleConfig {
        template: broken.to_string_lossy().into_owned(),
    }
    .load_template();
    assert_eq!(loaded.template, CycleTemplate::engineering());
    assert!(loaded.warning.is_some());

    let from_file = Config::from_toml_str("[cycle]\ntemplate = \"engineering\"\n").unwrap();
    assert_eq!(from_file.cycle, config.cycle);
}

#[cfg(feature = "store")]
mod stored {
    use super::*;
    use observatory_core::ingest::SessionTree;
    use observatory_core::store::{CaptureInfo, EventScope, Store, TimeRange};

    #[test]
    fn the_store_keeps_the_lane_id_as_text_and_reads_an_unknown_id_back_as_itself() {
        let template = four_step();
        let mut events = map_m0(&template);
        let path = repo_root().join(format!("fixtures/streams/{SLUG}.jsonl"));
        let cfg = observatory_core::config::IngestConfig::default();
        let tree = SessionTree::discover(&path, &cfg).unwrap();
        let capture = CaptureInfo::from_session_file(&path).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("observatory.sqlite");
        let mut store = Store::open(&db).unwrap();
        store.record_tree(&tree, &capture, false).unwrap();
        let mission = events[0].mission_id.clone();

        // One row whose lane is in no template: stored and read back as itself (rule 18).
        events[0].lane = LaneId::from("retired_lane");
        let odd = events[0].id.clone();
        store.insert_events(&events).unwrap();

        let back = store
            .events(&EventScope::Mission(mission), TimeRange::ALL)
            .unwrap();
        assert_eq!(back.len(), events.len());
        let lanes: BTreeSet<String> = back.iter().map(|e| e.lane.to_string()).collect();
        let wanted: BTreeSet<String> = ["spec", "code", "check", "ship", "retired_lane"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(lanes, wanted);
        assert_eq!(
            store.event(&odd).unwrap().lane,
            LaneId::from("retired_lane")
        );

        let conn = rusqlite::Connection::open(&db).unwrap();
        let types: Vec<String> = conn
            .prepare("SELECT DISTINCT typeof(lane) FROM events")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(types, ["text"]);
    }
}
