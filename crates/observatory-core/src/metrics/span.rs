//! Spans of time and the event timeline the engine reads.

use std::time::Duration;

use serde::Serialize;

use crate::model::{Event, Timestamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Span {
    pub from: Timestamp,
    pub to: Timestamp,
}

impl Span {
    pub fn length(&self) -> Duration {
        gap(self.from, self.to)
    }

    pub(super) fn covers(&self, from: Timestamp, to: Timestamp) -> bool {
        self.from <= from && to <= self.to
    }
}

/// `to - from`, zero when `to` is not later.
pub(super) fn gap(from: Timestamp, to: Timestamp) -> Duration {
    Duration::try_from(to.0 - from.0).unwrap_or(Duration::ZERO)
}

/// The span an event's `duration` measured. The mapper stamps the end of what it measured
/// (`tool_result.at`, the outcome line, the subagent's last line) and sets `duration` to the
/// time since its start, so the span ends at `at`.
pub fn event_span(event: &Event) -> Option<Span> {
    let length = time::Duration::try_from(event.duration?).ok()?;
    (!length.is_zero()).then(|| Span {
        from: Timestamp(event.at.0 - length),
        to: event.at,
    })
}

/// The events the engine reads, oldest first (ties keep their order): `turn.usage` is Panel
/// data and takes no part in time (rule 24).
pub(super) fn timeline(events: &[Event]) -> Vec<&Event> {
    let mut line: Vec<&Event> = events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .collect();
    line.sort_by_key(|event| event.at);
    line
}

/// Disjoint spans covering the same time as `spans`, oldest first.
pub(super) fn union(mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_by_key(|span| (span.from, span.to));
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans {
        match merged.last_mut() {
            Some(last) if span.from <= last.to => last.to = last.to.max(span.to),
            _ => merged.push(span),
        }
    }
    merged
}
