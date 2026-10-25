use serde::{Deserialize, Serialize};

use crate::model::{EventId, EventKind, FeatureId};

/// What the person did. Free text never appears here: only its length (PRD §11.2); the text
/// goes to the host over `HostControl` and nowhere else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum HumanAction {
    Pause,
    Resume,
    Message {
        text_len: u32,
    },
    /// `question_event` is the `QuestionAsked` event being answered.
    Answer {
        question_event: EventId,
        text_len: u32,
    },
    /// `request_event` is the `PermissionRequested` event being granted.
    Approve {
        request_event: EventId,
    },
    Deny {
        request_event: EventId,
    },
    MarkComplete {
        feature_id: FeatureId,
    },
    MarkSkipped {
        feature_id: FeatureId,
        reason_len: u32,
    },
}

fn length(text: &str) -> u32 {
    u32::try_from(text.chars().count()).unwrap_or(u32::MAX)
}

impl HumanAction {
    pub fn message(text: &str) -> Self {
        Self::Message {
            text_len: length(text),
        }
    }

    pub fn answer(question_event: EventId, text: &str) -> Self {
        Self::Answer {
            question_event,
            text_len: length(text),
        }
    }

    pub fn mark_skipped(feature_id: FeatureId, reason: &str) -> Self {
        Self::MarkSkipped {
            feature_id,
            reason_len: length(reason),
        }
    }

    /// The `human_actions.kind` text.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Message { .. } => "message",
            Self::Answer { .. } => "answer",
            Self::Approve { .. } => "approve",
            Self::Deny { .. } => "deny",
            Self::MarkComplete { .. } => "mark_complete",
            Self::MarkSkipped { .. } => "mark_skipped",
        }
    }

    /// The `human_actions.state` text a control starts in.
    pub fn state(&self) -> &'static str {
        match self {
            Self::Approve { .. } => "granted",
            Self::Deny { .. } => "denied",
            _ => "sent",
        }
    }

    /// The feature a mark names (`human_actions.target`).
    pub fn target(&self) -> Option<&FeatureId> {
        match self {
            Self::MarkComplete { feature_id } | Self::MarkSkipped { feature_id, .. } => {
                Some(feature_id)
            }
            _ => None,
        }
    }

    /// The event the action answers (`human_actions.payload_ptr`).
    pub fn payload_ptr(&self) -> Option<&EventId> {
        match self {
            Self::Answer { question_event, .. } => Some(question_event),
            Self::Approve { request_event } | Self::Deny { request_event } => Some(request_event),
            _ => None,
        }
    }

    /// The kind of the event the action itself produces (architecture §8 table): no new kind
    /// is invented, the control name lives in `human_actions.kind`.
    pub fn event_kind(&self) -> EventKind {
        match self {
            Self::Answer { .. } => EventKind::QuestionAnswered,
            _ => EventKind::ApprovalGiven,
        }
    }

    /// The kind of the event the host's acknowledgement stands for. The hosted stream carries
    /// none of these but `mission.paused`; a mark complete reaches no host.
    pub fn ack_kind(&self) -> Option<EventKind> {
        match self {
            Self::Pause => Some(EventKind::MissionPaused),
            Self::Resume => Some(EventKind::MissionResumed),
            Self::Message { .. } | Self::MarkSkipped { .. } => Some(EventKind::IntentAmended),
            Self::Answer { .. } => Some(EventKind::IntentAnswered),
            Self::Approve { .. } => Some(EventKind::PermissionGranted),
            Self::Deny { .. } => Some(EventKind::PermissionDenied),
            Self::MarkComplete { .. } => None,
        }
    }
}
