use crate::model::{Event, EventKind, Lane, Overlay, Timestamp};

/// FR-B6: the events up to and including `at`. `events` must be in time order, as the store and
/// the mapping deliver them.
pub fn scrub_to(events: &[Event], at: Timestamp) -> &[Event] {
    &events[..events.partition_point(|event| event.at <= at)]
}

/// FR-B7: how many events the mapping did not know. Equals the CLI's `unknown events: N`.
pub fn tray_count(events: &[Event]) -> u64 {
    events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
        .count() as u64
}

/// Events per phase, in lane order. Must equal the per-phase counts of the CLI replay.
pub fn lane_counts(events: &[Event]) -> [u64; Lane::ALL.len()] {
    let mut counts = [0; Lane::ALL.len()];
    for event in events {
        counts[lane_index(event.lane)] += 1;
    }
    counts
}

pub(super) fn lane_index(lane: Lane) -> usize {
    usize::from(lane.number() - 1)
}

/// The events a person (or the commander) caused, for the overlay filter.
pub fn overlay_markers(events: &[Event], overlay: Overlay) -> Vec<&Event> {
    events
        .iter()
        .filter(|event| event.overlay == Some(overlay))
        .collect()
}
