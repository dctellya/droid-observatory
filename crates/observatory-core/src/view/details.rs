use serde::{Deserialize, Serialize};

use crate::model::{
    Event, EventId, EventKind, FeatureId, Lane, MilestoneId, ModelId, Overlay, Role, SessionId,
    SourcePtr, Timestamp,
};

use super::scrub::{ScrubbedLine, scrub};

/// One row of the Details Timeline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineRow {
    pub event_id: EventId,
    /// The dotted PRD name.
    pub kind: String,
    /// The raw `type` of an event the mapping did not know.
    pub raw_type: Option<String>,
    pub unmapped: bool,
    pub lane: Lane,
    pub overlay: Option<Overlay>,
    pub role: Role,
    pub session_id: Option<SessionId>,
    /// The milestone whose Details this row belongs to.
    pub milestone: Option<MilestoneId>,
    pub feature: Option<FeatureId>,
    pub model: Option<ModelId>,
    pub at: Timestamp,
    pub duration_ms: Option<u64>,
    /// Opens the raw line.
    pub pointer: SourcePtr,
}

/// Details of one milestone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Details {
    pub milestone: MilestoneId,
    /// Time order.
    pub timeline: Vec<TimelineRow>,
}

impl TimelineRow {
    pub fn of(event: &Event) -> Self {
        let raw_type = match &event.kind {
            EventKind::Unknown { raw_type } => Some(raw_type.clone()),
            _ => None,
        };
        Self {
            event_id: event.id.clone(),
            kind: event.kind.prd_name().into_owned(),
            unmapped: raw_type.is_some(),
            raw_type,
            lane: event.lane,
            overlay: event.overlay,
            role: event.role,
            session_id: event.session_id.clone(),
            milestone: event.milestone_id.clone(),
            feature: event.feature_id.clone(),
            model: event.model.clone(),
            at: event.at,
            duration_ms: event
                .duration
                .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX)),
            pointer: event.source.clone(),
        }
    }
}

/// The Details view-model of `milestone`: every event of it, oldest first.
pub fn details(events: &[Event], milestone: &MilestoneId) -> Details {
    let mut timeline: Vec<TimelineRow> = events
        .iter()
        .filter(|event| event.milestone_id.as_ref() == Some(milestone))
        .map(TimelineRow::of)
        .collect();
    timeline.sort_by_key(|row| row.at);
    Details {
        milestone: milestone.clone(),
        timeline,
    }
}

/// A raw line beside its pointer, scrubbed for display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawLineView {
    pub event_id: EventId,
    pub kind: String,
    pub pointer: SourcePtr,
    pub line: ScrubbedLine,
}

/// The raw-line view-model: `raw` is what the pointer reads; only the shown text is masked.
pub fn raw_line_view(event: &Event, raw: &[u8]) -> RawLineView {
    RawLineView {
        event_id: event.id.clone(),
        kind: event.kind.prd_name().into_owned(),
        pointer: event.source.clone(),
        line: scrub(raw),
    }
}
