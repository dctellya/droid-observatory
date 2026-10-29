//! Rework windows (PRD §10.2, architecture §9): open and close pairs, merge, unresolved at the
//! mission's end, and the first pass without them.

mod metrics_support;

use metrics_support::{EventExt, ev, secs, t};
use observatory_core::metrics::{
    Notes, ReworkKind, ReworkWindow, first_pass_and_rework, fix_loops, merge_windows, partition,
    rework_windows,
};
use observatory_core::model::{CycleTemplate, Event, EventKind, Role};

fn windows(events: &[Event], notes: &Notes, end: i64) -> Vec<ReworkWindow> {
    rework_windows(events, &CycleTemplate::engineering(), notes, t(end))
}

fn closed_by(window: &ReworkWindow) -> Option<&str> {
    window.closed_by.as_ref().map(|id| id.0.as_str())
}

#[test]
fn rework_window_closes_on_resolution() {
    let events = vec![
        ev("a:finding", EventKind::FindingBlocking, 10)
            .feature("f")
            .by(Role::ScrutinyValidator),
        ev("a:resolved", EventKind::ReworkResolved, 60)
            .feature("f")
            .by(Role::ScrutinyValidator),
        ev("b:failed", EventKind::QaScenarioFailed, 100)
            .feature("f")
            .by(Role::UserTestingValidator),
        ev("b:other", EventKind::QaScenarioPassed, 120)
            .feature("f")
            .by(Role::UserTestingValidator),
        ev("b:passed", EventKind::QaScenarioPassed, 160)
            .feature("f")
            .by(Role::UserTestingValidator),
        ev("c:conflict", EventKind::IntegrationConflict, 200)
            .feature("f")
            .by(Role::Orchestrator),
        ev("c:resolved", EventKind::IntegrationConflictResolved, 260)
            .feature("f")
            .by(Role::Orchestrator),
        ev("d:red", EventKind::CheckFailed, 300)
            .feature("f")
            .by(Role::External),
        ev("d:other", EventKind::CheckPassed, 330)
            .feature("f")
            .by(Role::External),
        ev("d:green", EventKind::CheckPassed, 380)
            .feature("f")
            .by(Role::External),
    ];
    let notes = Notes::default()
        .with_key("b:failed", "VAL-A-001")
        .with_key("b:other", "VAL-B-002")
        .with_key("b:passed", "VAL-A-001")
        .with_key("d:red", "demo-gate")
        .with_key("d:other", "lint")
        .with_key("d:green", "demo-gate");

    let found = windows(&events, &notes, 400);

    let shape: Vec<(ReworkKind, &str, &str, Option<&str>)> = found
        .iter()
        .map(|w| {
            (
                w.kind,
                w.cause.as_str(),
                w.opened_by.0.as_str(),
                closed_by(w),
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            (ReworkKind::Finding, "scrutiny", "a:finding", Some("a:resolved")),
            (ReworkKind::Scenario, "user_testing", "b:failed", Some("b:passed")),
            (ReworkKind::Conflict, "integrate", "c:conflict", Some("c:resolved")),
            (ReworkKind::Check, "gate", "d:red", Some("d:green")),
        ],
        "each pair closes, and a pass for another scenario or check does not"
    );
    assert_eq!(secs(t(0), found[1].to), 160, "closed at the matching pass");
    assert_eq!(secs(t(0), found[3].to), 380, "closed at the matching check");
}

#[test]
fn overlapping_windows_merge_on_one_feature() {
    let on = |feature: &str, tag: &str| -> Vec<Event> {
        vec![
            ev(&format!("{tag}:finding"), EventKind::FindingBlocking, 10)
                .feature(feature)
                .by(Role::ScrutinyValidator),
            ev(&format!("{tag}:scenario"), EventKind::QaScenarioFailed, 50)
                .feature(feature)
                .by(Role::UserTestingValidator),
            ev(&format!("{tag}:resolved"), EventKind::ReworkResolved, 100)
                .feature(feature)
                .by(Role::ScrutinyValidator),
            ev(&format!("{tag}:passed"), EventKind::QaScenarioPassed, 150)
                .feature(feature)
                .by(Role::UserTestingValidator),
        ]
    };
    let notes = Notes::default();

    let one = windows(&on("f", "f"), &notes, 200);
    let spans = merge_windows(&one);
    assert_eq!(one.len(), 2);
    assert_eq!(spans.len(), 1, "one feature, overlapping: one span");
    assert_eq!(
        (secs(t(0), spans[0].from), secs(t(0), spans[0].to)),
        (10, 150)
    );
    assert_eq!(spans[0].windows, 2);

    let mut both = on("f", "f");
    both.extend(on("g", "g"));
    let all = windows(&both, &notes, 200);
    let count: usize = ["f", "g"]
        .iter()
        .map(|feature| {
            let mine: Vec<ReworkWindow> = all
                .iter()
                .filter(|w| w.feature.as_ref().is_some_and(|f| f.0 == *feature))
                .cloned()
                .collect();
            merge_windows(&mine).len()
        })
        .sum();
    assert_eq!(count, 2, "two features: each counts its own");
}

#[test]
fn unresolved_window_is_reported_at_mission_end() {
    let events = vec![
        ev("u:start", EventKind::TaskStarted, 0).feature("f"),
        ev("u:finding", EventKind::FindingBlocking, 100)
            .feature("f")
            .by(Role::ScrutinyValidator),
        ev("u:work", EventKind::FileTouched, 300)
            .feature("f")
            .lasting(100),
        ev("u:last", EventKind::CommitCreated, 500).feature("f"),
    ];

    let found = windows(&events, &Notes::default(), 500);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].closed_by, None, "no closing event: unresolved");
    assert_eq!(secs(t(0), found[0].to), 500, "kept open to the last event");
    let spans = merge_windows(&found);
    assert!(spans[0].unresolved);
    let (_, rework) = first_pass_and_rework(&partition(&events, t(500)), &spans);
    assert_eq!(rework.as_secs(), 100, "the Active time inside it is rework");
}

#[test]
fn first_pass_excludes_rework_windows() {
    let events = vec![
        ev("w:0", EventKind::FileTouched, 100).feature("f").lasting(100),
        ev("w:finding", EventKind::FindingBlocking, 50)
            .feature("f")
            .by(Role::ScrutinyValidator),
        ev("w:resolved", EventKind::ReworkResolved, 150)
            .feature("f")
            .by(Role::ScrutinyValidator),
        ev("w:1", EventKind::FileTouched, 200).feature("f").lasting(80),
    ];
    let mut sorted = events.clone();
    sorted.sort_by_key(|e| e.at);
    let intervals = partition(&sorted, t(300));
    let spans = merge_windows(&windows(&sorted, &Notes::default(), 300));

    let (first_pass, rework) = first_pass_and_rework(&intervals, &spans);

    // Active [0,100] and [120,200]; the window is [50,150].
    assert_eq!(rework.as_secs(), 50 + 30);
    assert_eq!(first_pass.as_secs(), 50 + 50);
    assert_eq!((first_pass + rework).as_secs(), 180, "the sum is the Active time");
}

#[test]
fn red_check_opens_gate_rework_window() {
    let events = vec![
        ev("pr:red", EventKind::CheckFailed, 100)
            .milestone("m1")
            .by(Role::External),
        ev("pr:red-again", EventKind::CheckFailed, 150)
            .milestone("m1")
            .by(Role::External),
        ev("fix:edit", EventKind::FileTouched, 300)
            .milestone("m1")
            .lasting(120),
        ev("pr:green", EventKind::CheckPassed, 400)
            .milestone("m1")
            .by(Role::External),
    ];
    let notes = Notes::default()
        .with_key("pr:red", "demo-gate")
        .with_key("pr:red-again", "demo-gate")
        .with_key("pr:green", "demo-gate");

    let found = windows(&events, &notes, 400);

    assert_eq!(found.len(), 1, "a second red of the same check opens no second window");
    assert_eq!(found[0].cause.as_str(), "gate");
    assert_eq!(found[0].opened_by.0, "pr:red");
    assert_eq!(closed_by(&found[0]), Some("pr:green"));
    assert_eq!(fix_loops(&found), 1);
    let spans = merge_windows(&found);
    let (_, rework) = first_pass_and_rework(&partition(&events, t(400)), &spans);
    assert_eq!(rework.as_secs(), 120, "waste_rework_s is above 0");
}
