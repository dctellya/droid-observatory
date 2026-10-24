use serde::{Deserialize, Serialize};

use crate::model::{
    Event, EventId, EventKind, FeatureId, ModelId, Role, SessionId, Timestamp, TokenUsage,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    /// Its task is done and the session is still open.
    Idle,
    /// A question, permission or approval is open.
    Waiting,
    Finished,
}

/// FR-A4: the two events that change what an agent is running on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkerKind {
    ModelChanged,
    ContextCompacted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub kind: MarkerKind,
    pub at: Timestamp,
    pub event_id: EventId,
}

/// One session in the agent panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentRow {
    pub session_id: SessionId,
    pub role: Role,
    pub status: AgentStatus,
    pub feature: Option<FeatureId>,
    pub model: Option<ModelId>,
    /// The latest `usage.reported` of the session (the sidecar totals).
    pub tokens: Option<TokenUsage>,
    pub markers: Vec<Marker>,
    pub event_count: u64,
    pub last_at: Timestamp,
}

fn marker_of(event: &Event) -> Option<Marker> {
    let kind = match event.kind {
        EventKind::ModelChanged => MarkerKind::ModelChanged,
        EventKind::ContextCompacted => MarkerKind::ContextCompacted,
        _ => return None,
    };
    Some(Marker {
        kind,
        at: event.at,
        event_id: event.id.clone(),
    })
}

/// The model-change and compaction markers of one session's events, in order.
pub fn agent_markers(events_of_session: &[Event]) -> Vec<Marker> {
    events_of_session.iter().filter_map(marker_of).collect()
}

impl AgentRow {
    pub(super) fn start(session_id: SessionId, first: &Event) -> Self {
        Self {
            session_id,
            role: first.role,
            status: AgentStatus::Active,
            feature: None,
            model: None,
            tokens: None,
            markers: Vec::new(),
            event_count: 0,
            last_at: first.at,
        }
    }

    pub(super) fn apply(&mut self, event: &Event) {
        self.event_count += 1;
        self.last_at = self.last_at.max(event.at);
        if matches!(self.role, Role::Human | Role::External | Role::Unresolved)
            && !matches!(event.role, Role::Human | Role::External | Role::Unresolved)
        {
            self.role = event.role;
        }
        if event.feature_id.is_some() {
            self.feature.clone_from(&event.feature_id);
        }
        if event.model.is_some() {
            self.model.clone_from(&event.model);
        }
        if let Some(marker) = marker_of(event) {
            self.markers.push(marker);
        }
        if event.kind == EventKind::UsageReported && event.usage.is_some() {
            self.tokens = event.usage;
        }
        self.status = next_status(self.status, &event.kind);
    }
}

/// Usage, model and compaction reports are bookkeeping: they never revive an idle, waiting
/// or finished session.
fn next_status(current: AgentStatus, kind: &EventKind) -> AgentStatus {
    match kind {
        EventKind::SubagentFinished => AgentStatus::Finished,
        _ if current == AgentStatus::Finished => current,
        EventKind::QuestionAsked
        | EventKind::PermissionRequested
        | EventKind::ApprovalRequested => AgentStatus::Waiting,
        EventKind::QuestionAnswered
        | EventKind::IntentAnswered
        | EventKind::PermissionGranted
        | EventKind::PermissionDenied
        | EventKind::ApprovalGiven => AgentStatus::Active,
        EventKind::TaskDone => AgentStatus::Idle,
        EventKind::UsageReported
        | EventKind::TurnUsage { .. }
        | EventKind::ModelChanged
        | EventKind::ContextCompacted => current,
        _ if current == AgentStatus::Idle => AgentStatus::Active,
        _ => current,
    }
}
