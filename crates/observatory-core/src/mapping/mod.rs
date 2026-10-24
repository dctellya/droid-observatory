//! Raw record to canonical event mapping. Compiled to wasm for the UI (architecture I9).

mod v1;
mod version;

pub use v1::RoleEvidence;
pub use version::{MappingVersion, Warning, select_version};

use std::collections::HashMap;

use crate::ingest::{RawRecord, SessionTree, UsageTurn};
use crate::model::{
    Event, MilestoneId, MissionId, ModelId, Role, SessionId, Timestamp, TokenUsage,
};

/// What the mapping needs to know about the session a record came from.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionMeta {
    pub id: SessionId,
    pub mission_id: MissionId,
    pub role: Role,
    pub model: Option<ModelId>,
    /// The sidecar `tokenUsage`, reported once when the session ends.
    pub usage: Option<TokenUsage>,
    /// Milestone ids of the mission's plan, to tell a feature id from any other word.
    pub milestones: Vec<MilestoneId>,
    /// The session was spawned by a `Task` whose description names the validation contract
    /// (`is_contract_task`); its role-neutral events land in the Contract phase.
    pub contract: bool,
}

impl SessionMeta {
    /// The session with no `callingSessionId`: it is the mission and its orchestrator.
    pub fn root(id: SessionId) -> Self {
        Self {
            mission_id: MissionId(id.0.clone()),
            id,
            role: Role::Orchestrator,
            model: None,
            usage: None,
            milestones: Vec::new(),
            contract: false,
        }
    }
}

pub struct MapContext<'a> {
    pub session: &'a SessionMeta,
    pub version: MappingVersion,
    /// Time given to a record that carries none of its own.
    pub fallback_at: Timestamp,
}

/// Which role a session has, by the rules of the mapping version.
pub fn resolve_role(version: MappingVersion, evidence: &RoleEvidence<'_>) -> Role {
    match version {
        MappingVersion::V1 => v1::resolve_role(evidence),
    }
}

/// Whether the description of the `Task` that spawned a session says it writes the validation
/// contract. A heuristic of the mapping version, so the version decides.
pub fn is_contract_task(version: MappingVersion, description: &str) -> bool {
    match version {
        MappingVersion::V1 => v1::is_contract_task(description),
    }
}

/// Milestone ids named by a `ProposeMission` proposal, in the order of its plan table.
pub(crate) fn proposal_milestones(version: MappingVersion, proposal: &str) -> Vec<MilestoneId> {
    match version {
        MappingVersion::V1 => v1::proposal_milestones(proposal),
    }
    .into_iter()
    .map(MilestoneId)
    .collect()
}

/// One record on its own, as the first line of a session: nothing is remembered between calls,
/// so rules that compare lines (a changed model, the first prompt) see no earlier line. Total:
/// the call never panics, and a line the rules do not know yields an `Unknown` event.
pub fn map_record(record: &RawRecord, ctx: &MapContext<'_>) -> Vec<Event> {
    match ctx.version {
        MappingVersion::V1 => {
            let mut state = v1::State::new(ctx.session);
            state.set_last_at(ctx.fallback_at);
            let mut events = v1::map_record(record, ctx.session, ctx.version, &mut state);
            events.extend(v1::flush(&mut state, ctx.session));
            events
        }
    }
}

enum State {
    V1(v1::State),
}

/// Maps the records of one session in file order. Rules that need earlier lines (the first
/// prompt, a changed model, the answer to a question) keep their memory here.
pub struct Mapper {
    session: SessionMeta,
    version: MappingVersion,
    state: State,
}

impl Mapper {
    pub fn new(session: SessionMeta, version: MappingVersion) -> Self {
        let state = match version {
            MappingVersion::V1 => State::V1(v1::State::new(&session)),
        };
        Self {
            session,
            version,
            state,
        }
    }

    /// A record without a time takes the time of the record before it. `session_start`, which
    /// has none and comes first, yields no events yet: they come out, timed by the next
    /// timestamped record, ahead of that record's own events.
    pub fn map(&mut self, record: &RawRecord) -> Vec<Event> {
        match &mut self.state {
            State::V1(state) => v1::map_record(record, &self.session, self.version, state),
        }
    }

    /// Forgets every record seen, as when the file is read again from its first line.
    pub fn restart(&mut self) {
        self.state = match self.version {
            MappingVersion::V1 => State::V1(v1::State::new(&self.session)),
        };
    }

    /// The `turn.usage` event of a turn of this session, in the phase the session is in now.
    pub fn map_usage(&mut self, turn: &UsageTurn) -> Event {
        match &self.state {
            State::V1(state) => v1::turn_usage(turn, &self.session, self.version, state),
        }
    }

    /// Replaces the usage reported by `finish`: a live session's sidecar is only final once the
    /// session has ended, long after the mapper was created.
    pub fn set_usage(&mut self, usage: Option<TokenUsage>) {
        self.session.usage = usage;
    }

    /// The events of the end of the session file: lines still waiting for a time, the child's
    /// `SubagentFinished` and its sidecar usage. Call it once, when the file will not grow.
    pub fn finish(&mut self) -> Vec<Event> {
        match &mut self.state {
            State::V1(state) => v1::finish(&self.session, self.version, state),
        }
    }
}

/// One `Mapper` per session of a tree, picked by the record's session id.
pub struct TreeMapper {
    mappers: HashMap<SessionId, Mapper>,
    order: Vec<SessionId>,
    mission_id: MissionId,
    version: MappingVersion,
}

impl TreeMapper {
    pub fn new(tree: &SessionTree, version: MappingVersion) -> Self {
        let order = tree
            .sessions()
            .iter()
            .map(|session| session.id.clone())
            .collect();
        let mappers = tree
            .sessions()
            .iter()
            .map(|session| (session.id.clone(), Mapper::new(tree.meta(session), version)))
            .collect();
        Self {
            mappers,
            order,
            mission_id: tree.mission_id().clone(),
            version,
        }
    }

    /// A record from a session the tree does not know is mapped as `Unresolved`, not dropped.
    pub fn map(&mut self, record: &RawRecord) -> Vec<Event> {
        let (mission_id, version) = (&self.mission_id, self.version);
        self.mappers
            .entry(record.session_id.clone())
            .or_insert_with(|| {
                let session = SessionMeta {
                    id: record.session_id.clone(),
                    mission_id: mission_id.clone(),
                    role: Role::Unresolved,
                    ..SessionMeta::root(record.session_id.clone())
                };
                Mapper::new(session, version)
            })
            .map(record)
    }

    /// The `turn.usage` event of a turn of one of the tree's sessions. `None` for a turn with
    /// no session or of a session this tree does not hold: the caller counts those.
    pub fn map_usage(&mut self, turn: &UsageTurn) -> Option<Event> {
        let session = turn.session_id.as_ref()?;
        self.mappers
            .get_mut(session)
            .filter(|_| self.order.contains(session))
            .map(|mapper| mapper.map_usage(turn))
    }

    /// `Mapper::finish` for every session: the tree's own first, in tree order, then any
    /// session met only in the records, by id.
    pub fn finish(&mut self) -> Vec<Event> {
        let mut extra: Vec<&SessionId> = self
            .mappers
            .keys()
            .filter(|id| !self.order.contains(id))
            .collect();
        extra.sort();
        let ids: Vec<SessionId> = self
            .order
            .iter()
            .cloned()
            .chain(extra.into_iter().cloned())
            .collect();
        ids.iter()
            .flat_map(|id| {
                self.mappers
                    .get_mut(id)
                    .map(Mapper::finish)
                    .unwrap_or_default()
            })
            .collect()
    }
}
