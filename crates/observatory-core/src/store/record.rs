//! What a replay records besides the events: the capture, the mission and one row per
//! session file.

use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use super::rows::utf8;
use super::{Store, StoreError, now_text};
use crate::ingest::{IngestError, SessionTree, capture_path, read_capture};
use crate::model::{MissionId, SessionId};

/// The `captures` row: where a set of session files came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureInfo {
    pub source_dir: String,
    pub droid_version: Option<String>,
    /// The `.capture.json` beside the file; `None` for a live directory, which has no header.
    pub header_path: Option<PathBuf>,
    pub started_at: String,
    pub mission_name: Option<String>,
}

impl CaptureInfo {
    /// Reads the header beside `jsonl_path`. Without one, the file's directory is the source
    /// and the time is now.
    pub fn from_session_file(jsonl_path: &Path) -> Result<Self, IngestError> {
        let header = read_capture(jsonl_path)?;
        let absolute = |path: &Path| std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
        let source_dir = header
            .as_ref()
            .and_then(|header| header.source_dir.clone())
            .unwrap_or_else(|| {
                absolute(jsonl_path)
                    .parent()
                    .map_or_else(String::new, |dir| dir.to_string_lossy().into_owned())
            });
        Ok(Self {
            source_dir,
            droid_version: header
                .as_ref()
                .and_then(|header| header.droid_version.clone()),
            header_path: header
                .is_some()
                .then(|| absolute(&capture_path(jsonl_path))),
            started_at: header
                .as_ref()
                .and_then(|header| header.captured_at.clone())
                .unwrap_or_else(now_text),
            mission_name: header.and_then(|header| header.mission_name),
        })
    }
}

impl Store {
    /// Records the capture, the tree's mission and its session files (only the root's with
    /// `root_only`), parents before children. Safe to repeat: rows are matched by identity
    /// and updated. Session paths are stored as the tree holds them, so pass absolute paths
    /// for pointers that outlive the working directory.
    pub fn record_tree(
        &mut self,
        tree: &SessionTree,
        capture: &CaptureInfo,
        root_only: bool,
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        let header_path = capture.header_path.as_deref().map(utf8).transpose()?;
        let known: Option<i64> = tx
            .query_row(
                "SELECT id FROM captures WHERE header_path IS ?1 AND source_dir = ?2",
                params![header_path, capture.source_dir],
                |row| row.get(0),
            )
            .optional()?;
        let capture_id = match known {
            Some(id) => id,
            None => {
                tx.execute(
                    "INSERT INTO captures (source_dir, droid_version, header_path, started_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        capture.source_dir,
                        capture.droid_version,
                        header_path,
                        capture.started_at
                    ],
                )?;
                tx.last_insert_rowid()
            }
        };

        let mission = tree.mission_id().to_string();
        let name = capture
            .mission_name
            .clone()
            .or_else(|| tree.root().user_tag.clone())
            .unwrap_or_else(|| mission.clone());
        tx.execute(
            "INSERT INTO missions (id, name, capture_id, state, started_at) \
             VALUES (?1, ?2, ?3, 'building', ?4) \
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, capture_id = excluded.capture_id",
            params![mission, name, capture_id, capture.started_at],
        )?;

        let count = if root_only { 1 } else { tree.sessions().len() };
        for session in &tree.sessions()[..count] {
            let size = std::fs::metadata(&session.path)
                .map_err(|source| StoreError::Io {
                    path: session.path.clone(),
                    source,
                })?
                .len();
            tx.execute(
                "INSERT INTO sessions (id, path, mission_id, parent_id, role, tag, model, droid_version, \
                 size_bytes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                 ON CONFLICT (id) DO UPDATE SET path = excluded.path, mission_id = excluded.mission_id, \
                 parent_id = excluded.parent_id, role = excluded.role, tag = excluded.tag, \
                 model = excluded.model, droid_version = excluded.droid_version, \
                 size_bytes = excluded.size_bytes",
                params![
                    session.id.to_string(),
                    utf8(&session.path)?,
                    mission,
                    session.parent.as_ref().map(ToString::to_string),
                    super::rows::wire_text(&session.role),
                    session.display_name(),
                    session.model(),
                    capture.droid_version,
                    i64::try_from(size).unwrap_or(i64::MAX),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Sets the display name (`tag`) of a session already recorded. False when the store has no
    /// row for it.
    pub fn set_session_name(&mut self, id: &SessionId, name: &str) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE sessions SET tag = ?2 WHERE id = ?1",
            params![id.to_string(), name],
        )?;
        Ok(changed > 0)
    }

    /// Records the branch a mission integrates on (`missions.branch`). False when the store has
    /// no row for the mission.
    pub fn set_mission_branch(
        &mut self,
        mission: &MissionId,
        branch: &str,
    ) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE missions SET branch = ?2 WHERE id = ?1",
            params![mission.to_string(), branch],
        )?;
        Ok(changed > 0)
    }

    /// The branch recorded for a mission, if any.
    pub fn mission_branch(&self, mission: &MissionId) -> Result<Option<String>, StoreError> {
        let branch: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT branch FROM missions WHERE id = ?1",
                params![mission.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        Ok(branch.flatten())
    }

    /// Marks every session file of `mission` as read to the size it had when it was recorded.
    pub fn mark_tailed(&mut self, mission: &MissionId) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE sessions SET tailed_to = size_bytes WHERE mission_id = ?1",
            params![mission.to_string()],
        )?;
        Ok(())
    }
}
