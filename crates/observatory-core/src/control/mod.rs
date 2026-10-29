//! Controls (architecture §8, F-028): every human action is one event on a phase lane with
//! `overlay = Human`, `role = Human` and `source = App`, plus one `human_actions` row. Lanes
//! are addressed by `LaneId` through the cycle template, never by name (F-041, D-035).

mod action;
pub mod merge;
pub mod stall;

use std::collections::HashSet;

pub use action::HumanAction;
pub use stall::RunbookPrompt;

use crate::mapping::MappingVersion;
use crate::model::{
    CycleTemplate, Event, EventId, EventKind, LaneId, MissionId, Overlay, Role, SessionId,
    SourcePtr, Timestamp,
};

/// The `sub` of an event made from a host acknowledgement. Its pointer is the ack line of the
/// spool; a `sub` no mapped line yields keeps the ids apart from the stream's own events.
const ACK_SUB: u32 = 1 << 30;

/// Whether `id` names an event [`ack_event`] made.
pub fn is_ack(id: &EventId) -> bool {
    id.0.rsplit(':').next() == Some(ACK_SUB.to_string().as_str())
}

/// The `sub` of the `permission.requested` event the host makes from Droid's
/// `droid.request_permission` request: the stream maps no event for it.
const PERMISSION_SUB: u32 = ACK_SUB + 1;

/// The lane of the event an action produces: the first lane of the template for a
/// mission-level control, the lane of the request for approve and deny, the final lane
/// (Gate) for a merge.
fn lane_of(action: &HumanAction, template: &CycleTemplate, about: Option<&Event>) -> LaneId {
    match (action, about) {
        (HumanAction::Approve { .. } | HumanAction::Deny { .. }, Some(request)) => {
            request.lane.clone()
        }
        (HumanAction::Merge { .. }, _) => template.final_lane().clone(),
        _ => template.first().clone(),
    }
}

/// An event with the fixed human attribution; the caller sets `id`, `kind`, `at`, `source`.
fn human_event(
    action: &HumanAction,
    mission: &MissionId,
    template: &CycleTemplate,
    about: Option<&Event>,
    parts: (EventId, EventKind, Timestamp, SourcePtr),
) -> Event {
    let (id, kind, at, source) = parts;
    Event {
        id,
        mission_id: mission.clone(),
        milestone_id: about.and_then(|event| event.milestone_id.clone()),
        feature_id: action.target().cloned(),
        lane: lane_of(action, template, about),
        overlay: Some(Overlay::Human),
        kind,
        role: Role::Human,
        session_id: None,
        model: None,
        at,
        duration: None,
        usage: None,
        source,
        mapping_version: MappingVersion::LATEST,
    }
}

/// The event of one control. `about` is the event the action concerns: the open request of an
/// approve or deny (its lane is used), the question of an answer, or any event of the feature
/// a mark names; its milestone becomes the milestone of the new event.
pub fn record(
    action: &HumanAction,
    mission: &MissionId,
    at: Timestamp,
    action_id: u64,
    template: &CycleTemplate,
    about: Option<&Event>,
) -> Event {
    human_event(
        action,
        mission,
        template,
        about,
        (
            EventId(format!("app:{action_id}:0")),
            action.event_kind(),
            at,
            SourcePtr::App { action_id },
        ),
    )
}

/// The `permission.requested` event of a `droid.request_permission` request, which the mapped
/// stream does not carry. `ptr` is the request's line of the spool, `lane` the lane of the
/// session that asks; an approve or deny points at this event (`human_actions.payload_ptr`).
pub fn permission_requested_event(
    mission: &MissionId,
    session: &SessionId,
    at: Timestamp,
    ptr: SourcePtr,
    lane: LaneId,
) -> Event {
    let offset = match &ptr {
        SourcePtr::Rpc { byte_offset, .. } | SourcePtr::Jsonl { byte_offset, .. } => *byte_offset,
        _ => 0,
    };
    Event {
        id: EventId(format!("{session}:{offset}:{PERMISSION_SUB}")),
        mission_id: mission.clone(),
        milestone_id: None,
        feature_id: None,
        lane,
        overlay: Some(Overlay::Human),
        kind: EventKind::PermissionRequested,
        role: Role::Orchestrator,
        session_id: Some(session.clone()),
        model: None,
        at,
        duration: None,
        usage: None,
        source: ptr,
        mapping_version: MappingVersion::LATEST,
    }
}

/// The event the host's acknowledgement of a control stands for: the hosted stream carries no
/// `permission.granted`, `permission.denied` or `mission.resumed`, so the ack is their source.
/// `at` is the arrival time of the ack and `ptr` its line of the spool. `None` for a control
/// that reaches no host (mark complete). A `mission.paused` that the stream carries too is
/// removed by [`dedupe_paused`].
pub fn ack_event(
    action: &HumanAction,
    mission: &MissionId,
    session: &SessionId,
    at: Timestamp,
    ptr: SourcePtr,
    template: &CycleTemplate,
    about: Option<&Event>,
) -> Option<Event> {
    let kind = action.ack_kind()?;
    let offset = match &ptr {
        SourcePtr::Rpc { byte_offset, .. } | SourcePtr::Jsonl { byte_offset, .. } => *byte_offset,
        _ => 0,
    };
    let id = EventId(format!("{session}:{offset}:{ACK_SUB}"));
    let mut event = human_event(action, mission, template, about, (id, kind, at, ptr));
    event.session_id = Some(session.clone());
    Some(event)
}

/// Drops an ack-derived `mission.paused` when the stream reports the same pause: a
/// `mission.paused` of the stream follows it before any `mission.resumed`. `events` is in time
/// order and `ack_ids` holds the ids [`ack_event`] made.
pub fn dedupe_paused(events: Vec<Event>, ack_ids: &HashSet<EventId>) -> Vec<Event> {
    let echoed = |index: usize| -> bool {
        events[index + 1..]
            .iter()
            .find_map(|later| match later.kind {
                EventKind::MissionPaused if !ack_ids.contains(&later.id) => Some(true),
                EventKind::MissionResumed => Some(false),
                _ => None,
            })
            .unwrap_or(false)
    };
    let drop: Vec<bool> = events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            event.kind == EventKind::MissionPaused && ack_ids.contains(&event.id) && echoed(index)
        })
        .collect();
    events
        .into_iter()
        .zip(drop)
        .filter_map(|(event, dropped)| (!dropped).then_some(event))
        .collect()
}

/// The instruction the orchestrator receives when a feature is skipped (the orchestrator
/// owns the plan, so the app asks it rather than editing the feature list).
#[cfg(feature = "rpc")]
fn skip_instruction(feature: &crate::model::FeatureId, reason: Option<&str>) -> String {
    let mut text = format!(
        "Skip feature {feature}: mark it skipped and do not assign it again in this mission."
    );
    if let Some(reason) = reason.filter(|reason| !reason.is_empty()) {
        text.push_str(" Reason: ");
        text.push_str(reason);
    }
    text
}

/// The host control that carries the action to the hosted mission, for the controls that need
/// no lookup of an open request. `text` is the free text of a message or the reason of a
/// skip; it is relayed and never stored. Answer, approve and deny are built by the host from
/// the open request, and a mark complete reaches no host.
#[cfg(feature = "rpc")]
pub fn relay(action: &HumanAction, text: Option<&str>) -> Option<crate::ingest::rpc::HostControl> {
    use crate::ingest::rpc::HostControl;
    match action {
        HumanAction::Pause => Some(HostControl::Pause),
        HumanAction::Resume => Some(HostControl::Resume),
        HumanAction::Message { .. } => text.map(|text| HostControl::Message {
            text: text.to_owned(),
        }),
        HumanAction::MarkSkipped { feature_id, .. } => Some(HostControl::Message {
            text: skip_instruction(feature_id, text),
        }),
        HumanAction::Answer { .. }
        | HumanAction::Approve { .. }
        | HumanAction::Deny { .. }
        | HumanAction::MarkComplete { .. }
        | HumanAction::Merge { .. } => None,
    }
}
