//! The rows a hosted mission needs before its events can be stored: a capture, the mission
//! and the one session whose lines are in the spool.

use std::path::{Path, PathBuf};

use rusqlite::params;

use super::rows::utf8;
use super::{Store, StoreError};
use crate::model::SessionId;

/// A mission the app launched. Its orchestrator's session id is the mission id, as for an
/// attached mission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedRun {
    pub mission_name: String,
    pub session: SessionId,
    /// The spool the events point into.
    pub spool: PathBuf,
    pub droid_version: Option<String>,
    pub started_at: String,
}

impl Store {
    /// Records the capture, the mission and the orchestrator session of a hosted run. Safe to
    /// repeat: rows are matched by identity and updated.
    pub fn record_hosted(&mut self, run: &HostedRun) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        let spool = utf8(&run.spool)?;
        let source_dir = utf8(run.spool.parent().unwrap_or_else(|| Path::new("")))?;
        let known: Option<i64> = tx
            .query_row(
                "SELECT id FROM captures WHERE header_path IS NULL AND source_dir = ?1",
                params![source_dir],
                |row| row.get(0),
            )
            .ok();
        let capture_id = match known {
            Some(id) => id,
            None => {
                tx.execute(
                    "INSERT INTO captures (source_dir, droid_version, header_path, started_at) \
                     VALUES (?1, ?2, NULL, ?3)",
                    params![source_dir, run.droid_version, run.started_at],
                )?;
                tx.last_insert_rowid()
            }
        };
        let mission = run.session.to_string();
        tx.execute(
            "INSERT INTO missions (id, name, capture_id, state, started_at) \
             VALUES (?1, ?2, ?3, 'building', ?4) \
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, capture_id = excluded.capture_id",
            params![mission, run.mission_name, capture_id, run.started_at],
        )?;
        let size = std::fs::metadata(&run.spool).map_or(0, |meta| meta.len());
        tx.execute(
            "INSERT INTO sessions (id, path, mission_id, role, tag, droid_version, size_bytes) \
             VALUES (?1, ?2, ?3, 'orchestrator', ?4, ?5, ?6) \
             ON CONFLICT (id) DO UPDATE SET path = excluded.path, tag = excluded.tag, \
             droid_version = excluded.droid_version, size_bytes = excluded.size_bytes",
            params![
                mission,
                spool,
                mission,
                run.mission_name,
                run.droid_version,
                i64::try_from(size).unwrap_or(i64::MAX),
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
}
