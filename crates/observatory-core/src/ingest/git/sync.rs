//! One git poll of a mission: what the repository says goes into the store.

use super::{FastCheckRun, GitSnapshot, GitSource};
use crate::ingest::IngestError;
use crate::model::Event;
use crate::store::{Store, StoreError};

#[derive(Debug, thiserror::Error)]
pub enum GitSyncError {
    #[error(transparent)]
    Git(#[from] IngestError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// What one poll read and what of it was new.
#[derive(Clone, Debug, PartialEq)]
pub struct GitSync {
    pub snapshot: GitSnapshot,
    /// First-parent merges of the mission branch.
    pub merges: usize,
    /// The fast-check runs the caller passed in.
    pub runs: usize,
    /// Events the poll computed, stored before or not.
    pub events: usize,
    /// The events the store did not hold before this poll.
    pub new_events: Vec<Event>,
}

/// Records the mission branch (`missions.branch`), one `integration.merged` per first-parent
/// merge and the `gate_command` events of `runs`. Reading the repository comes first, so a
/// branch that does not exist writes nothing. Repeating it with the same facts stores nothing.
pub fn git_sync(
    store: &mut Store,
    source: &GitSource,
    runs: &[FastCheckRun],
) -> Result<GitSync, GitSyncError> {
    let snapshot = source.snapshot()?;
    let merges = source.merges()?;
    let merge_count = merges.len();
    let gate_events = source.gate_command_events_of(&merges, runs)?;
    let mut events = source.integration_events_of(merges);
    events.extend(gate_events);
    store.set_mission_branch(&source.mission, source.branch())?;
    let new_events = store.insert_new_events(&events)?;
    Ok(GitSync {
        snapshot,
        merges: merge_count,
        runs: runs.len(),
        events: events.len(),
        new_events,
    })
}
