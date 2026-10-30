//! The engine on the dogfooded M0 capture (its golden events): the partition of every feature
//! and the overnight stop, measured. Characterization: the figures are the engine's output.

mod metrics_support;

use std::collections::BTreeSet;

use metrics_support::m0_events;
use observatory_core::metrics::{
    Interval, Notes, PartitionContext, TimeState, WasteCause, active_spans, events_of_feature,
    first_pass_and_rework, merge_windows, partition_in, rework_windows, throttled, waste_by_cause,
};
use observatory_core::model::{CycleTemplate, FeatureId};

#[test]
fn m0_features_partition_and_the_overnight_stop_is_throttled_apart() {
    let events = m0_events();
    let notes = Notes::default();
    let end = events
        .iter()
        .filter(|e| !e.kind.is_panel_only())
        .map(|e| e.at)
        .max()
        .expect("events");
    let stop = throttled(&events, &notes, end);
    println!("throttled: {stop:#?}");
    let spans = stop.spans();

    let features: BTreeSet<FeatureId> =
        events.iter().filter_map(|e| e.feature_id.clone()).collect();
    let template = CycleTemplate::engineering();
    let all_windows = rework_windows(&events, &template, &notes, end);
    println!("rework windows on M0: {}", all_windows.len());
    let mut rows = 0;
    for feature in &features {
        let mine = events_of_feature(&events, feature, &notes);
        let others: Vec<_> = features
            .iter()
            .filter(|other| *other != feature)
            .flat_map(|other| active_spans(&events_of_feature(&events, other, &notes)))
            .collect();
        let context = PartitionContext {
            others_active: &others,
            throttled: &spans,
        };
        let intervals = partition_in(&mine, end, &context);
        let windows = rework_windows(&mine, &template, &notes, end);
        let merged = merge_windows(&windows);
        let (first_pass, rework) = first_pass_and_rework(&intervals, &merged);
        let waste = waste_by_cause(&intervals, &merged);
        let active: u64 = intervals
            .iter()
            .filter(|i| i.state == TimeState::Active)
            .map(|i| i.length().as_secs())
            .sum();
        let causes: Vec<String> = WasteCause::ALL
            .iter()
            .map(|c| format!("{c:?}={}", waste.seconds(*c)))
            .collect();
        println!(
            "{feature}: intervals={} active_s={active} first_pass_s={} rework_s={} windows={} {}",
            intervals.len(),
            first_pass.as_secs(),
            rework.as_secs(),
            merged.len(),
            causes.join(" ")
        );
        for pair in intervals.windows(2) {
            assert!(pair[0].to <= pair[1].from, "{feature}: no overlap");
        }
        for interval in &intervals {
            for span in &spans {
                assert!(
                    interval.to <= span.from || interval.from >= span.to,
                    "{feature}: {interval:?} inside the stop"
                );
            }
        }
        let _: &[Interval] = &intervals;
        assert_eq!(
            first_pass + rework,
            intervals
                .iter()
                .filter(|i| i.state == TimeState::Active)
                .map(Interval::length)
                .sum::<std::time::Duration>()
        );
        rows += 1;
    }
    assert_eq!(rows, features.len());
    assert_eq!(stop.intervals.len(), 1, "one overnight stop");
    assert_eq!(stop.intervals[0].reason, "unknown");
}
