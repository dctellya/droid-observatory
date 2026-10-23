//! Raw record to canonical event mapping. Compiled to wasm for the UI (architecture I9).

mod v1;
mod version;

pub use v1::RoleEvidence;
pub use version::MappingVersion;

use std::collections::HashMap;

use crate::ingest::{RawRecord, SessionTree};
use crate::model::{Event, MissionId, ModelId, Role, SessionId, Timestamp};

/// What the mapping needs to know about the session a record came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionMeta {
    pub id: SessionId,
    pub mission_id: MissionId,
    pub role: Role,
    pub model: Option<ModelId>,
}

impl SessionMeta {
    /// The session with no `callingSessionId`: it is the mission and its orchestrator.
    pub fn root(id: SessionId) -> Self {
        Self {
            mission_id: MissionId(id.0.clone()),
            id,
            role: Role::Orchestrator,
            model: None,
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

/// Total: every record yields at least one event and the call never panics (invariant I5).
pub fn map_record(record: &RawRecord, ctx: &MapContext<'_>) -> Vec<Event> {
    match ctx.version {
        MappingVersion::V1 => v1::map_record(record, ctx),
    }
}

/// Maps the records of one session in file order. A record without a timestamp takes the
/// time of the record before it.
pub struct Mapper {
    session: SessionMeta,
    version: MappingVersion,
    last_at: Timestamp,
}

impl Mapper {
    pub fn new(session: SessionMeta, version: MappingVersion) -> Self {
        Self {
            session,
            version,
            // Lines before the first timestamped one (`session_start` has none) sort first;
            // the epoch says "no time known" without inventing one.
            last_at: Timestamp(time::OffsetDateTime::UNIX_EPOCH),
        }
    }

    pub fn map(&mut self, record: &RawRecord) -> Vec<Event> {
        let ctx = MapContext {
            session: &self.session,
            version: self.version,
            fallback_at: self.last_at,
        };
        let events = map_record(record, &ctx);
        if let Some(last) = events.last() {
            self.last_at = last.at;
        }
        events
    }
}

/// One `Mapper` per session of a tree, picked by the record's session id.
pub struct TreeMapper {
    mappers: HashMap<SessionId, Mapper>,
    mission_id: MissionId,
    version: MappingVersion,
}

impl TreeMapper {
    pub fn new(tree: &SessionTree, version: MappingVersion) -> Self {
        let mappers = tree
            .sessions()
            .iter()
            .map(|session| (session.id.clone(), Mapper::new(tree.meta(session), version)))
            .collect();
        Self {
            mappers,
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
                    model: None,
                };
                Mapper::new(session, version)
            })
            .map(record)
    }
}
