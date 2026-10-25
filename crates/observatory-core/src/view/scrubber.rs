use serde::{Deserialize, Serialize};

use crate::model::{
    CycleTemplate, Event, EventId, EventKind, LaneId, Overlay, Role, SessionId, Timestamp,
};

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
pub fn lane_counts(events: &[Event], template: &CycleTemplate) -> Vec<u64> {
    let mut counts = vec![0; template.lanes.len()];
    for event in events.iter().filter(|event| !event.kind.is_panel_only()) {
        if let Some(index) = template.index(&event.lane) {
            counts[index] += 1;
        }
    }
    counts
}

/// The events a person (or the officer) caused, for the overlay filter.
pub fn overlay_markers(events: &[Event], overlay: Overlay) -> Vec<&Event> {
    events
        .iter()
        .filter(|event| event.overlay == Some(overlay))
        .collect()
}

/// One marker of the overlay filter: an event a person (or the officer) caused, with the
/// lane it sits on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayMarker {
    pub event_id: EventId,
    /// The dotted PRD name.
    pub kind: String,
    pub lane: LaneId,
    pub role: Role,
    pub session_id: Option<SessionId>,
    pub at: Timestamp,
}

/// The overlay filter: the markers of `overlay`, in time order.
pub fn overlay_marker_rows(events: &[Event], overlay: Overlay) -> Vec<OverlayMarker> {
    overlay_markers(events, overlay)
        .into_iter()
        .map(OverlayMarker::of)
        .collect()
}

impl OverlayMarker {
    pub(super) fn of(event: &Event) -> Self {
        Self {
            event_id: event.id.clone(),
            kind: event.kind.prd_name().into_owned(),
            lane: event.lane.clone(),
            role: event.role,
            session_id: event.session_id.clone(),
            at: event.at,
        }
    }
}

/// The positions of the replay slider: one per distinct event time. The last position is the
/// live end (`at` is `None`), so the board there is the board of every event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scrubber {
    stops: Vec<Timestamp>,
}

impl Scrubber {
    /// `events` must be in time order.
    pub fn new(events: &[Event]) -> Self {
        let mut stops: Vec<Timestamp> = events
            .iter()
            .filter(|event| !event.kind.is_panel_only())
            .map(|event| event.at)
            .collect();
        stops.dedup();
        Self { stops }
    }

    pub fn stops(&self) -> usize {
        self.stops.len()
    }

    pub fn last_index(&self) -> usize {
        self.stops.len().saturating_sub(1)
    }

    /// The scrub time of a position; `None` at (or past) the end.
    pub fn at(&self, index: usize) -> Option<Timestamp> {
        if index >= self.last_index() {
            return None;
        }
        self.stops.get(index).copied()
    }

    /// The position of a scrub time: the last stop at or before it.
    pub fn index_of(&self, at: Option<Timestamp>) -> usize {
        match at {
            None => self.last_index(),
            Some(at) => self
                .stops
                .partition_point(|stop| *stop <= at)
                .saturating_sub(1),
        }
    }

    /// The timestamp shown next to the slider.
    pub fn label(&self, at: Option<Timestamp>) -> String {
        at.or_else(|| self.stops.last().copied())
            .map(|at| at.to_rfc3339_millis())
            .unwrap_or_default()
    }
}
