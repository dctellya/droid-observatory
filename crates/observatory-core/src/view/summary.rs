use serde::{Deserialize, Serialize};

use crate::model::MissionId;

use super::library::MissionRow;

/// What opening a replay tells the UI before the board is drawn (architecture §7 `open_replay`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionSummary {
    pub mission_id: MissionId,
    pub name: String,
    /// The capture header's `captured_at`, else the time of the first event.
    pub started_at: String,
    pub event_count: u64,
    /// Equals the `unknown events: N` line of the CLI replay.
    pub unknown_count: u64,
    /// Lines read from every session file of the tree.
    pub record_count: u64,
    pub session_count: u64,
    pub mapping_version: String,
    pub warning: Option<String>,
}

impl MissionSummary {
    /// The library line of this replay.
    pub fn library_row(&self) -> MissionRow {
        MissionRow {
            id: self.mission_id.clone(),
            name: self.name.clone(),
            state: "replay".to_owned(),
            started_at: self.started_at.clone(),
            completed_at: None,
            event_count: self.event_count,
            headline: None,
        }
    }
}
