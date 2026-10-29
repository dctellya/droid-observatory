//! Time states (PRD §10.1, architecture §9): every instant of a feature is in exactly one state.

mod metrics_support;

use std::collections::BTreeMap;

use metrics_support::{EventExt, ev, m0_events, secs, t};
use observatory_core::metrics::{
    Interval, PartitionContext, Span, TimeState, WaitKind, WasteCause, WasteGroup, partition,
    partition_in, waste_by_cause,
};
use observatory_core::model::{Event, EventKind, FeatureId, Role};

fn feature_events() -> Vec<Event> {
    vec![
        ev("f:0", EventKind::TaskStarted, 0).feature("f"),
        ev("f:1", EventKind::FileTouched, 100)
            .feature("f")
            .lasting(60),
        ev("f:2", EventKind::ToolError, 130)
            .feature("f")
            .lasting(10),
        ev("f:3", EventKind::QuestionAsked, 150).feature("f"),
        ev("f:4", EventKind::QuestionAnswered, 400).feature("f"),
        ev("f:5", EventKind::TaskDone, 500).feature("f"),
        ev("f:6", EventKind::ScrutinyPassed, 700)
            .feature("f")
            .by(Role::ScrutinyValidator),
    ]
}

fn states(intervals: &[Interval]) -> Vec<(i64, i64, TimeState)> {
    intervals
        .iter()
        .map(|i| (secs(t(0), i.from), secs(t(0), i.to), i.state))
        .collect()
}

fn assert_contiguous(intervals: &[Interval], first: i64, end: i64) {
    assert_eq!(
        secs(t(0), intervals.first().expect("an interval").from),
        first
    );
    assert_eq!(secs(t(0), intervals.last().expect("an interval").to), end);
    for pair in intervals.windows(2) {
        assert_eq!(pair[0].to, pair[1].from, "no gap and no overlap");
    }
    for interval in intervals {
        assert!(interval.from < interval.to, "no empty interval");
    }
}

#[test]
fn time_states_partition_every_instant_exactly_once() {
    let intervals = partition(&feature_events(), t(800));

    assert_contiguous(&intervals, 0, 800);
    let total: i64 = intervals.iter().map(|i| secs(i.from, i.to)).sum();
    assert_eq!(total, 800, "the interval lengths sum to the elapsed time");
    assert_eq!(
        states(&intervals),
        vec![
            (0, 40, TimeState::IdleUnscheduled),
            (40, 100, TimeState::Active),
            (100, 120, TimeState::IdleUnscheduled),
            (120, 130, TimeState::RetryFailure),
            (130, 150, TimeState::IdleUnscheduled),
            (150, 400, TimeState::WaitingHuman(WaitKind::Question)),
            (400, 500, TimeState::IdleUnscheduled),
            (500, 700, TimeState::WaitingValidation),
            (700, 800, TimeState::IdleUnscheduled),
        ]
    );

    for feature in distinct_m0_features() {
        let events: Vec<Event> = m0_events()
            .into_iter()
            .filter(|e| e.feature_id.as_ref() == Some(&feature) && !e.kind.is_panel_only())
            .collect();
        let first = events.iter().map(|e| e.at).min().expect("an event");
        let end = events.iter().map(|e| e.at).max().expect("an event");
        let intervals = partition(&events, end);
        let sum: std::time::Duration = intervals.iter().map(Interval::length).sum();
        let elapsed = (end.0 - intervals[0].from.0).unsigned_abs();
        assert!(
            intervals[0].from <= first,
            "{feature}: starts at the first span"
        );
        assert_eq!(sum, elapsed, "{feature}: lengths sum to the elapsed time");
        for pair in intervals.windows(2) {
            assert_eq!(pair[0].to, pair[1].from, "{feature}");
        }
        assert_eq!(intervals.last().map(|i| i.to), Some(end), "{feature}");
    }
}

fn distinct_m0_features() -> Vec<FeatureId> {
    let mut seen: BTreeMap<FeatureId, ()> = BTreeMap::new();
    for event in m0_events() {
        if let Some(feature) = event.feature_id {
            seen.insert(feature, ());
        }
    }
    assert!(!seen.is_empty(), "the M0 capture names features");
    seen.into_keys().collect()
}

#[test]
fn a_gap_is_queued_while_another_feature_is_active_and_a_pass_waits_for_the_merge() {
    let others = [Span {
        from: t(0),
        to: t(30),
    }];
    let ctx = PartitionContext {
        others_active: &others,
        throttled: &[],
    };
    let intervals = partition_in(&feature_events(), t(800), &ctx);
    assert_eq!(
        states(&intervals)[0],
        (0, 30, TimeState::IdleQueued),
        "busy elsewhere"
    );
    assert_eq!(
        states(&intervals)[1],
        (30, 40, TimeState::IdleUnscheduled),
        "nobody busy"
    );

    let merge = [
        ev("g:0", EventKind::TaskStarted, 0).feature("g"),
        ev("g:1", EventKind::CheckPassed, 50)
            .feature("g")
            .by(Role::External),
        ev(
            "g:2",
            EventKind::HandedToCd {
                merge_sha: "abc".into(),
                pr_number: 1,
                attestation_ids: vec![],
            },
            200,
        )
        .feature("g")
        .by(Role::External),
    ];
    let intervals = partition(&merge, t(200));
    assert!(
        states(&intervals).contains(&(50, 200, TimeState::WaitingHuman(WaitKind::Merge))),
        "green and unmerged waits on a human: {:?}",
        states(&intervals)
    );
}

#[test]
fn a_permission_request_waits_until_it_is_granted() {
    let events = [
        ev("p:0", EventKind::TaskStarted, 0).feature("p"),
        ev("p:1", EventKind::PermissionRequested, 20).feature("p"),
        ev("p:2", EventKind::PermissionGranted, 80).feature("p"),
    ];
    let intervals = partition(&events, t(100));
    assert!(states(&intervals).contains(&(20, 80, TimeState::WaitingHuman(WaitKind::Permission))));
}

#[test]
fn waste_and_rework_causes_are_closed_sets() {
    use observatory_core::metrics::{ReworkKind, rework_causes};
    use observatory_core::model::CycleTemplate;

    let names = |causes: &[WasteCause]| -> Vec<String> {
        causes
            .iter()
            .map(|c| {
                serde_json::to_value(c)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect()
    };
    assert_eq!(
        names(&WasteCause::ALL),
        [
            "waiting_human_question",
            "waiting_human_permission",
            "waiting_merge",
            "waiting_validation",
            "rework",
            "retry_failure",
            "idle_queued",
            "idle_unscheduled",
        ]
    );
    // An exhaustive match: a ninth cause (for example `throttled`) does not compile here.
    for cause in WasteCause::ALL {
        let _ = match cause {
            WasteCause::WaitingHumanQuestion
            | WasteCause::WaitingHumanPermission
            | WasteCause::WaitingMerge
            | WasteCause::WaitingValidation
            | WasteCause::Rework
            | WasteCause::RetryFailure
            | WasteCause::IdleQueued
            | WasteCause::IdleUnscheduled => cause.group(),
        };
    }
    let groups: Vec<String> = WasteGroup::ALL
        .iter()
        .map(|g| {
            serde_json::to_value(g)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        groups,
        [
            "waiting_on_human",
            "waiting_on_validation",
            "rework",
            "retry",
            "idle"
        ]
    );
    assert!(!names(&WasteCause::ALL).contains(&"throttled".to_owned()));

    let kinds: Vec<String> = ReworkKind::ALL
        .iter()
        .map(|k| {
            serde_json::to_value(k)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(kinds, ["finding", "scenario", "conflict", "check"]);
    let causes: Vec<String> = rework_causes(&CycleTemplate::engineering())
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(causes, ["scrutiny", "user_testing", "integrate", "gate"]);
}

#[test]
fn waste_groups_sum_the_eight_measured_causes() {
    let events = feature_events();
    let intervals = partition(&events, t(800));
    let totals = waste_by_cause(&intervals, &[]);

    let by_cause: u64 = WasteCause::ALL.iter().map(|c| totals.seconds(*c)).sum();
    let by_group: u64 = WasteGroup::ALL
        .iter()
        .map(|g| totals.group_seconds(*g))
        .sum();
    assert_eq!(by_cause, by_group);
    assert_eq!(by_cause, totals.total_seconds());
    assert_eq!(totals.seconds(WasteCause::WaitingHumanQuestion), 250);
    assert_eq!(totals.seconds(WasteCause::WaitingValidation), 200);
    assert_eq!(totals.seconds(WasteCause::RetryFailure), 10);
    assert_eq!(
        totals.seconds(WasteCause::IdleUnscheduled),
        40 + 20 + 20 + 100 + 100
    );
    assert_eq!(totals.group_seconds(WasteGroup::Idle), 280);
    assert_eq!(totals.group_seconds(WasteGroup::WaitingOnHuman), 250);
    // Active time is not waste.
    assert_eq!(by_cause, 800 - 60);
}

#[test]
fn lane7_answer_closes_waiting_human_interval() {
    use metrics_support::MISSION;
    use observatory_core::control::{self, HumanAction};
    use observatory_core::metrics::{Notes, events_of_feature};
    use observatory_core::model::{CycleTemplate, MissionId, Overlay, SourcePtr};

    let template = CycleTemplate::engineering();
    let asked = ev("q:ask", EventKind::QuestionAsked, 100)
        .feature("f")
        .by(Role::Orchestrator);
    let answer = control::record(
        &HumanAction::answer(asked.id.clone(), "yes"),
        &MissionId(MISSION.to_owned()),
        t(400),
        7,
        &template,
        Some(&asked),
    );
    assert!(matches!(answer.source, SourcePtr::App { action_id: 7 }));
    assert_eq!(answer.overlay, Some(Overlay::Human));
    let all = vec![
        ev("q:start", EventKind::TaskStarted, 0).feature("f"),
        asked.clone(),
        answer.clone(),
        ev("q:next", EventKind::FileTouched, 450)
            .feature("f")
            .lasting(30),
    ];
    let notes = Notes::default().with_answer(&answer.id.0, &asked.id.0);

    let mine = events_of_feature(&all, &FeatureId("f".to_owned()), &notes);
    let intervals = partition(&mine, t(500));

    let waiting: Vec<&Interval> = intervals
        .iter()
        .filter(|i| i.state == TimeState::WaitingHuman(WaitKind::Question))
        .collect();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].from, asked.at);
    assert_eq!(waiting[0].to, answer.at, "closed by the app's answer event");
    let totals = waste_by_cause(&intervals, &[]);
    assert_eq!(totals.seconds(WasteCause::WaitingHumanQuestion), 300);
}
