//! Engine defects the reference CSV exposed (reports/readiness/2026-10-30-oracle-deviations.md,
//! E-1 to E-4): each test builds the smallest event stream that shows one.

mod metrics_support;

use metrics_support::{EventExt, ev, prices, t};
use observatory_core::config::OverheadApportionment;
use observatory_core::metrics::{CostInput, MetricsReport, Notes, WasteCause, metrics_report};
use observatory_core::model::{CycleTemplate, Event, EventKind, Role};

fn report(events: &[Event], end: i64) -> MetricsReport {
    metrics_report(&CostInput {
        events,
        template: &CycleTemplate::engineering(),
        notes: &Notes::default(),
        prices: &prices(),
        end: t(end),
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: OverheadApportionment::Separate,
    })
}

fn row<'a>(report: &'a MetricsReport, id: &str) -> &'a observatory_core::metrics::MetricsRow {
    report
        .rows
        .iter()
        .find(|row| row.id == id)
        .unwrap_or_else(|| panic!("no row {id}"))
}

#[test]
fn a_milestone_clock_starts_at_its_first_feature_defined_not_at_a_fallback_start() {
    let events = vec![
        ev("b:0", EventKind::TaskStarted, 100)
            .feature("b")
            .milestone("m1"),
        ev("a:0", EventKind::FeatureDefined, 1000)
            .feature("a")
            .milestone("m1")
            .by(Role::ScrutinyValidator),
        ev("a:1", EventKind::TaskStarted, 1010)
            .feature("a")
            .milestone("m1"),
    ];

    let report = report(&events, 2000);

    assert_eq!(
        row(&report, "b").cycle_s,
        1900,
        "the fallback feature keeps its own start"
    );
    assert_eq!(
        row(&report, "m1").cycle_s,
        1000,
        "the milestone starts at its first feature.defined"
    );
}

#[test]
fn a_worker_turn_the_runner_cancelled_is_work_done_and_waits_for_validation() {
    let events = vec![
        ev("f:0", EventKind::TaskStarted, 0).feature("f"),
        ev("f:1", EventKind::TaskSkipped, 300)
            .feature("f")
            .lasting(300),
    ];

    let report = report(&events, 1000);
    let f = row(&report, "f");

    assert_eq!(f.first_pass_active_s, 300);
    assert_eq!(f.waste_s[&WasteCause::WaitingValidation], 700);
    assert_eq!(f.waste_s[&WasteCause::IdleUnscheduled], 0);
}

#[test]
fn a_skipped_validator_turn_is_not_work_done() {
    let events = vec![
        ev("f:0", EventKind::TaskStarted, 0).feature("f"),
        ev("f:1", EventKind::TaskSkipped, 300)
            .feature("f")
            .by(Role::ScrutinyValidator)
            .lasting(300),
    ];

    let report = report(&events, 1000);

    assert_eq!(row(&report, "f").waste_s[&WasteCause::WaitingValidation], 0);
}

#[test]
fn a_question_no_feature_owns_is_a_wait_of_every_feature_that_is_not_active() {
    let orchestrator = |id: &str, kind, at| ev(id, kind, at).by(Role::Orchestrator);
    let events = vec![
        ev("f:0", EventKind::FileTouched, 100)
            .feature("f")
            .lasting(100),
        orchestrator("q:a", EventKind::QuestionAsked, 200),
        orchestrator("q:b", EventKind::QuestionAsked, 550),
        orchestrator("q:b-err", EventKind::ToolError, 551).lasting(1),
        orchestrator("q:c", EventKind::QuestionAsked, 600),
        orchestrator("a:a", EventKind::IntentAnswered, 500),
        orchestrator("a:c", EventKind::IntentAnswered, 650),
        ev("f:1", EventKind::FileTouched, 800)
            .feature("f")
            .lasting(100),
    ];

    let report = report(&events, 800);
    let f = row(&report, "f");

    // 200 to 500 and 600 to 650 are answered; 550 to 551 ends in the AskUser error.
    assert_eq!(f.waste_s[&WasteCause::WaitingHumanQuestion], 351);
    assert_eq!(f.waste_s[&WasteCause::IdleUnscheduled], 249);
    assert_eq!(f.first_pass_active_s, 200);
}

#[test]
fn a_subagents_whole_run_span_is_not_active_time() {
    let events = vec![
        ev("f:0", EventKind::TaskSkipped, 100)
            .feature("f")
            .lasting(100),
        ev("f:1", EventKind::TaskSkipped, 4000)
            .feature("f")
            .lasting(100),
        ev("f:2", EventKind::SubagentFinished, 4000)
            .feature("f")
            .lasting(4000),
    ];

    let report = report(&events, 4000);

    assert_eq!(row(&report, "f").first_pass_active_s, 200);
}

#[test]
fn a_turn_still_open_at_the_end_counts_by_its_subagent_span() {
    let events = vec![
        ev("f:0", EventKind::SubagentFinished, 238)
            .feature("f")
            .lasting(238),
    ];

    let report = report(&events, 238);

    assert_eq!(row(&report, "f").first_pass_active_s, 238);
}
