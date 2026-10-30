//! `throttled` (F-033, D-028): the platform stopped the work. Measured from the pause to the
//! resume, apart from the eight waste causes and the five groups.

mod metrics_support;

use metrics_support::{EventExt, ev, secs, t};
use observatory_core::metrics::{
    Notes, PartitionContext, WasteCause, partition, partition_in, throttled, waste_by_cause,
};
use observatory_core::model::{Event, EventKind, MilestoneId};

fn stream() -> Vec<Event> {
    vec![
        ev("e0", EventKind::TaskStarted, 0)
            .feature("f")
            .milestone("m1"),
        ev("p1", EventKind::MissionPaused, 100)
            .feature("f")
            .milestone("m1"),
        ev("r1", EventKind::MissionResumed, 400)
            .feature("f")
            .milestone("m1"),
        ev("w1", EventKind::FileTouched, 990)
            .feature("f")
            .milestone("m2"),
        ev("x1", EventKind::EscalationRaised, 1000)
            .feature("f")
            .milestone("m2"),
        ev("x2", EventKind::EscalationRaised, 1000)
            .feature("f")
            .milestone("m2"),
        ev("x3", EventKind::EscalationRaised, 1000)
            .feature("f")
            .milestone("m2"),
        ev("n1", EventKind::PlanRevised, 4600)
            .feature("f")
            .milestone("m2"),
        ev("p2", EventKind::MissionPaused, 5000)
            .feature("f")
            .milestone("m1"),
        ev("r2", EventKind::MissionResumed, 5120)
            .feature("f")
            .milestone("m1"),
    ]
}

#[test]
fn throttled_is_measured_from_pause_to_resume_and_reported_apart() {
    let events = stream();
    let notes = Notes::default().with_pause_reason("p1", "unrecoverable_usage_402");

    let found = throttled(&events, &notes, t(5200));

    let shape: Vec<(i64, i64, &str, u64)> = found
        .intervals
        .iter()
        .map(|i| {
            (
                secs(t(0), i.from),
                secs(t(0), i.to),
                i.reason.as_str(),
                i.minutes,
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            (100, 400, "unrecoverable_usage_402", 5),
            // No resume marker: from the last event before the escalations to the first after.
            (990, 4600, "unknown", 60),
            (5000, 5120, "unknown", 2),
        ]
    );
    assert_eq!(found.mission_minutes, 67, "4030 s of throttling");
    assert_eq!(found.per_milestone.get(&MilestoneId("m1".into())), Some(&7));
    assert_eq!(
        found.per_milestone.get(&MilestoneId("m2".into())),
        Some(&60)
    );

    // Apart from the eight causes: no ninth variant, and the time is in no waste total.
    assert_eq!(WasteCause::ALL.len(), 8);
    let spans = found.spans();
    let ctx = PartitionContext {
        others_active: &[],
        throttled: &spans,
    };
    let intervals = partition_in(&events, t(5200), &ctx);
    for interval in &intervals {
        for span in &spans {
            assert!(
                interval.to <= span.from || interval.from >= span.to,
                "{interval:?} overlaps the throttled span {span:?}"
            );
        }
    }
    let waste = waste_by_cause(&intervals, &[]);
    let elapsed = secs(t(0), t(5200));
    let waste_s = i64::try_from(waste.total_seconds()).unwrap();
    assert_eq!(
        waste_s,
        elapsed - 4030,
        "the rest of the elapsed time is idle"
    );
    assert!(waste_s + 60 * i64::try_from(found.mission_minutes).unwrap() <= elapsed + 30);

    // Without the carve-out the same events count it as idle.
    let plain = waste_by_cause(&partition(&events, t(5200)), &[]);
    assert_eq!(i64::try_from(plain.total_seconds()).unwrap(), elapsed);
}

#[test]
fn a_pause_that_never_resumes_stays_open_to_the_end() {
    let events = [
        ev("e0", EventKind::TaskStarted, 0).feature("f"),
        ev("p1", EventKind::MissionPaused, 600).feature("f"),
    ];
    let found = throttled(&events, &Notes::default(), t(1200));
    assert_eq!(found.intervals.len(), 1);
    assert_eq!(
        secs(t(0), found.intervals[0].from),
        0,
        "the last event before it"
    );
    assert_eq!(secs(t(0), found.intervals[0].to), 1200);
    assert_eq!(found.intervals[0].minutes, 20);
}

#[test]
fn an_active_span_across_the_stop_does_not_count_the_stop_as_active() {
    // A turn is one span; the platform stopped it for 400 s inside the run.
    let events = [
        ev("e0", EventKind::TaskStarted, 0).feature("f"),
        ev("done", EventKind::TaskSkipped, 1000)
            .feature("f")
            .lasting(1000),
    ];
    let spans = [observatory_core::metrics::Span {
        from: t(300),
        to: t(700),
    }];
    let ctx = PartitionContext {
        others_active: &[],
        throttled: &spans,
    };

    let intervals = partition_in(&events, t(1000), &ctx);

    let active: i64 = intervals
        .iter()
        .filter(|i| i.state == observatory_core::metrics::TimeState::Active)
        .map(|i| secs(i.from, i.to))
        .sum();
    assert_eq!(active, 600, "1000 s of run, 400 s of them throttled");
}
