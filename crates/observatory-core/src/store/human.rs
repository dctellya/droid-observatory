//! The human side of the store: one `human_actions` row and one event per control, written
//! together. A row holds the kind, the state, the target feature and the id of the event the
//! action points at; it has no column for text (PRD §11.2).

use rusqlite::params;

use super::{Store, StoreError, insert_events_in};
use crate::control::{self, HumanAction};
use crate::model::{CycleTemplate, Event, MissionId, Timestamp};

/// What `Store::record_human_action` wrote.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedAction {
    pub action_id: u64,
    pub event: Event,
}

/// A row of `human_actions`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HumanActionRow {
    pub id: u64,
    pub mission_id: String,
    pub kind: String,
    pub target: Option<String>,
    pub at: String,
    pub state: String,
    pub payload_ptr: Option<String>,
}

impl Store {
    /// Records `action`: the row and the human-overlay event (`control::record`) in one
    /// transaction, the event pointing at the row. `about` is as for `control::record`.
    pub fn record_human_action(
        &mut self,
        action: &HumanAction,
        mission: &MissionId,
        at: Timestamp,
        template: &CycleTemplate,
        about: Option<&Event>,
    ) -> Result<RecordedAction, StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO human_actions (mission_id, kind, target, at, state, payload_ptr) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                mission.to_string(),
                action.kind(),
                action.target().map(ToString::to_string),
                at.to_rfc3339_millis(),
                action.state(),
                action.payload_ptr().map(ToString::to_string),
            ],
        )?;
        let action_id = u64::try_from(tx.last_insert_rowid()).unwrap_or_default();
        let event = control::record(action, mission, at, action_id, template, about);
        insert_events_in(&tx, std::slice::from_ref(&event))?;
        tx.commit()?;
        Ok(RecordedAction { action_id, event })
    }

    /// The controls of `mission`, oldest first.
    pub fn human_actions(&self, mission: &MissionId) -> Result<Vec<HumanActionRow>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id, mission_id, kind, target, at, state, payload_ptr FROM human_actions \
             WHERE mission_id = ?1 ORDER BY id",
        )?;
        let rows = statement
            .query_map(params![mission.to_string()], |row| {
                Ok(HumanActionRow {
                    id: u64::try_from(row.get::<_, i64>(0)?).unwrap_or_default(),
                    mission_id: row.get(1)?,
                    kind: row.get(2)?,
                    target: row.get(3)?,
                    at: row.get(4)?,
                    state: row.get(5)?,
                    payload_ptr: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
