//! A gate rework window (a red then green PR check of a milestone, no feature) reaches the card
//! that owns it: the milestone's row, and the Active time of the milestone's features inside it
//! (PRD §10.2: rework time is Active time inside a rework window; red-to-green wall time is not).

mod metrics_support;

use std::path::{Path, PathBuf};

use metrics_support::{EventExt, ev, m0_events, prices};
use observatory_core::config::{IngestConfig, OverheadApportionment};
use observatory_core::ingest::{JsonlSource, Source, mapping_version_of};
use observatory_core::mapping::{Mapper, SessionMeta};
use observatory_core::metrics::{
    MetricsReport, MetricsRow, Notes, ReportSetup, RowScope, WasteCause, report_at_last_event,
};
use observatory_core::model::{CycleTemplate, Event, EventKind, Role};

fn notes() -> Notes {
    Notes::default()
        .with_key("pr:red", "demo-gate")
        .with_key("pr:green", "demo-gate")
        .with_key("pr6:red", "demo-gate")
        .with_key("pr6:green", "demo-gate")
}

fn report_of(events: &[Event]) -> MetricsReport {
    let template = CycleTemplate::engineering();
    report_at_last_event(
        events,
        &ReportSetup {
            template: &template,
            prices: &prices(),
            notes: &notes(),
            budget: 0.0,
            launch_budget_usd: None,
            apportionment: OverheadApportionment::Separate,
        },
    )
    .expect("the events hold a last event")
}

fn row<'a>(report: &'a MetricsReport, scope: RowScope, id: &str) -> &'a MetricsRow {
    report
        .rows
        .iter()
        .find(|row| row.scope == scope && row.id == id)
        .unwrap_or_else(|| panic!("no {scope:?} row {id}"))
}

fn rework_s(row: &MetricsRow) -> u64 {
    row.waste_s[&WasteCause::Rework]
}

/// A red `demo-gate` check at 100 s and its green at 400 s, both naming `milestone` and no feature.
fn gate_window(milestone: &str) -> Vec<Event> {
    vec![
        ev("pr:red", EventKind::CheckFailed, 100)
            .milestone(milestone)
            .by(Role::External),
        ev("pr:green", EventKind::CheckPassed, 400)
            .milestone(milestone)
            .by(Role::External),
    ]
}

/// A feature of `milestone` with Active work at 10..60 s (before the window) and the given spans.
fn feature_work(feature: &str, milestone: &str, work: &[(i64, u64)]) -> Vec<Event> {
    let mut events = vec![
        ev(&format!("{feature}:fd"), EventKind::FeatureDefined, 0)
            .feature(feature)
            .milestone(milestone)
            .by(Role::Orchestrator),
        ev(&format!("{feature}:early"), EventKind::FileTouched, 60)
            .feature(feature)
            .milestone(milestone)
            .lasting(50),
    ];
    for (index, (at, length)) in work.iter().enumerate() {
        events.push(
            ev(&format!("{feature}:w{index}"), EventKind::FileTouched, *at)
                .feature(feature)
                .milestone(milestone)
                .lasting(*length),
        );
    }
    events
}

#[test]
fn a_milestone_with_only_a_gate_window_gets_a_row_with_one_window_and_one_fix_loop() {
    let report = report_of(&gate_window("m5"));

    let milestone = row(&report, RowScope::Milestone, "m5");
    assert_eq!(milestone.rework_windows, 1);
    assert_eq!(milestone.fix_loops, 1);
    assert_eq!(
        milestone.rework_s, 0,
        "no Active time in the window: the wall time of the red check is not rework"
    );
    assert_eq!(rework_s(milestone), 0);
    assert_eq!(report.mission.fix_loops, 1);
    assert_eq!(report.mission.milestones, 1);
}

#[test]
fn active_time_of_a_milestones_features_inside_its_gate_window_counts_as_rework() {
    let mut events = gate_window("m5");
    events.extend(feature_work("m5-a", "m5", &[(300, 120)]));
    events.extend(feature_work("m5-b", "m5", &[(350, 20)]));

    let report = report_of(&events);

    let a = row(&report, RowScope::Feature, "m5-a");
    assert_eq!(a.rework_s, 120, "Active 180..300 lies inside 100..400");
    assert_eq!(a.first_pass_active_s, 50, "Active 10..60 lies before it");
    assert_eq!(rework_s(a), 120);
    let b = row(&report, RowScope::Feature, "m5-b");
    assert_eq!(b.rework_s, 20);
    assert_eq!(rework_s(b), 20);
    let milestone = row(&report, RowScope::Milestone, "m5");
    assert_eq!(milestone.rework_s, 140);
    assert_eq!(rework_s(milestone), 140);
    assert_eq!(milestone.first_pass_active_s, 100);
    assert_eq!(report.mission.rework_s, 140);
    assert_eq!(report.mission.waste_s[&WasteCause::Rework], 140);
    assert!(report.mission.fix_loops >= 1);
}

#[test]
fn a_gate_window_of_another_milestone_leaves_a_feature_alone() {
    let mut events = gate_window("m5");
    events.extend(feature_work("m4-a", "m4", &[(300, 120)]));

    let report = report_of(&events);

    let feature = row(&report, RowScope::Feature, "m4-a");
    assert_eq!(feature.rework_s, 0);
    assert_eq!(feature.first_pass_active_s, 170);
    assert_eq!(rework_s(feature), 0);
    let other = row(&report, RowScope::Milestone, "m4");
    assert_eq!(other.rework_s, 0);
    assert_eq!(other.rework_windows, 0);
    assert_eq!(other.fix_loops, 0);
    let owner = row(&report, RowScope::Milestone, "m5");
    assert_eq!(owner.rework_windows, 1);
    assert_eq!(owner.rework_s, 0, "m5 has no feature with Active time");
}

#[test]
fn a_gate_window_is_counted_once_in_the_milestone_row_and_the_mission_summary() {
    let mut events = gate_window("m5");
    events.push(
        ev("pr6:red", EventKind::CheckFailed, 500)
            .milestone("m6")
            .by(Role::External),
    );
    events.push(
        ev("pr6:green", EventKind::CheckPassed, 600)
            .milestone("m6")
            .by(Role::External),
    );
    events.extend(feature_work("m5-a", "m5", &[(300, 120)]));
    events.extend(feature_work("m5-b", "m5", &[(350, 20)]));

    let report = report_of(&events);

    let milestone = row(&report, RowScope::Milestone, "m5");
    assert_eq!(milestone.rework_windows, 1, "two features, one window");
    assert_eq!(milestone.fix_loops, 1);
    for feature in ["m5-a", "m5-b"] {
        let own = row(&report, RowScope::Feature, feature);
        assert_eq!(
            own.rework_windows, 0,
            "{feature}: the window is not its own"
        );
        assert_eq!(own.fix_loops, 0, "{feature}");
    }
    assert_eq!(row(&report, RowScope::Milestone, "m6").fix_loops, 1);
    assert_eq!(
        report.mission.fix_loops, 2,
        "one per milestone, not per feature"
    );
}

#[test]
fn a_gate_window_without_a_milestone_lands_on_the_mission_level_card() {
    let mut events = vec![
        ev("pr:red", EventKind::CheckFailed, 100).by(Role::External),
        ev("pr:green", EventKind::CheckPassed, 400).by(Role::External),
    ];
    events.push(ev("orch:edit", EventKind::FileTouched, 300).lasting(120));

    let report = report_of(&events);

    assert_eq!(
        report.cost.overhead.waste_s[&WasteCause::Rework],
        120,
        "the mission-level card holds the Active time inside the window"
    );
    assert!(
        report.rows.is_empty(),
        "a window of no milestone makes no milestone row"
    );
}

const S_SLUG: &str = "2026-10-22-observatory-v1-s";

fn s_events() -> Vec<Event> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams")
        .join(format!("{S_SLUG}.jsonl"));
    let mut source = JsonlSource::open(&path, &IngestConfig::default()).expect("fixture opens");
    let version = mapping_version_of(&path).expect("capture header is readable");
    let mut mapper = Mapper::new(SessionMeta::root(source.root_session_id().clone()), version);
    let mut events = Vec::new();
    while !source.at_eof() {
        for record in source.next_batch().expect("batch reads") {
            events.extend(mapper.map(&record));
        }
    }
    for record in source.drain_partial() {
        events.extend(mapper.map(&record));
    }
    events.extend(mapper.finish());
    events
}

fn golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

/// `OBSERVATORY_UPDATE_GOLDEN=1` rewrites the golden files; the diff is reviewed in the commit.
fn pin(name: &str, csv: &str) {
    let path = golden(name);
    if std::env::var_os("OBSERVATORY_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, csv).unwrap();
    }
    let expected = std::fs::read_to_string(&path).expect("the golden CSV exists");
    assert_eq!(csv, expected, "{name} changed");
}

#[test]
fn the_m0_and_s_metrics_csv_are_unchanged_by_gate_window_attribution() {
    pin("metrics_m0.csv", &report_of(&m0_events()).to_csv());
    pin("metrics_s_root.csv", &report_of(&s_events()).to_csv());
}
