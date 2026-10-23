use serde::{Deserialize, Serialize};

use crate::ingest::SessionInfo;
use crate::model::{Event, FeatureId, ModelId, Role, SessionId, Timestamp, TokenUsage};

use super::agents::{AgentRow, AgentStatus, Marker, agent_markers};
use super::board::DeclaredModels;
use super::details::TimelineRow;

/// One session of the agent panel, with what the UI shows of it (FR-A2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentEntry {
    pub session_id: SessionId,
    /// The first sidecar tag, else the session id (FR-L1).
    pub name: String,
    pub role: Role,
    pub status: AgentStatus,
    pub feature: Option<FeatureId>,
    /// The latest model seen on the session's events, else the sidecar's.
    pub model: Option<ModelId>,
    /// The model the mission command declares for the role.
    pub declared: Option<ModelId>,
    /// `model` differs from `declared`.
    pub drifted: bool,
    /// Sidecar `reasoningEffort`.
    pub effort: Option<String>,
    pub tokens: Option<TokenUsage>,
    /// `model.changed` and `context.compacted` events of the session (FR-A4).
    pub markers: usize,
    pub event_count: u64,
}

/// The name a session is shown by: its first tag, else its id.
pub fn session_name(sessions: &[SessionInfo], id: &SessionId) -> String {
    sessions
        .iter()
        .find(|session| &session.id == id)
        .and_then(|session| session.tag.clone())
        .unwrap_or_else(|| id.0.clone())
}

/// The agent panel: one entry per session, in order of the session's first event. At the end of
/// a replay (`ended`) nothing runs any more, so every session reads `finished`.
pub fn agent_entries(
    agents: &[AgentRow],
    sessions: &[SessionInfo],
    declared: &DeclaredModels,
    ended: bool,
) -> Vec<AgentEntry> {
    agents
        .iter()
        .map(|agent| {
            let info = sessions.iter().find(|s| s.id == agent.session_id);
            let model = agent
                .model
                .clone()
                .or_else(|| info.and_then(|s| s.model.clone()));
            let declared = declared.of(agent.role).cloned();
            AgentEntry {
                session_id: agent.session_id.clone(),
                name: session_name(sessions, &agent.session_id),
                role: agent.role,
                status: if ended {
                    AgentStatus::Finished
                } else {
                    agent.status
                },
                feature: agent.feature.clone(),
                drifted: matches!((&model, &declared), (Some(m), Some(d)) if m != d),
                model,
                declared,
                effort: info.and_then(|s| s.effort.clone()),
                tokens: agent.tokens,
                markers: agent.markers.len(),
                event_count: agent.event_count,
            }
        })
        .collect()
}

/// The output stream of one session (FR-A3): its events in time order, and the markers among
/// them (FR-A4).
#[derive(Clone, Debug, PartialEq)]
pub struct AgentStream {
    pub session_id: SessionId,
    pub rows: Vec<TimelineRow>,
    pub markers: Vec<Marker>,
}

/// The stream of `session` up to `at` (none: to the end), from `events` in time order.
pub fn agent_stream(events: &[Event], session: &SessionId, at: Option<Timestamp>) -> AgentStream {
    let own: Vec<Event> = events
        .iter()
        .filter(|event| event.session_id.as_ref() == Some(session))
        .filter(|event| at.is_none_or(|at| event.at <= at))
        .cloned()
        .collect();
    AgentStream {
        session_id: session.clone(),
        rows: own.iter().map(TimelineRow::of).collect(),
        markers: agent_markers(&own),
    }
}
