//! The store as the source of what the UI opens: a capture replayed into it, a mission
//! reopened from its rows, a raw line re-read by pointer (architecture §7 `open_replay`,
//! `list_missions`, `event_raw`).

use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use super::read::from_wire;
use super::{EventScope, Store, StoreError, TimeRange};
use crate::config::IngestConfig;
use crate::ingest::{
    JsonlSource, OpenedMission, SessionInfo, SessionTree, Source, UsageLogSource,
    missing_usage_log_warning, read_capture, read_sidecar, select_mapping_of,
};
use crate::mapping::{MappingVersion, TreeMapper};
use std::sync::Arc;

use crate::model::{
    CycleTemplate, EventId, EventKind, MissionId, ModelId, Role, SessionId, SourcePtr,
};
use crate::store::CaptureInfo;
use crate::view::{MissionSummary, RawLineView, raw_line_view};

const CAPTURE_SUFFIX: &str = ".capture.json";

/// The session file a path names: a `.capture.json` stands for the `.jsonl` beside it.
fn root_of(file: &Path) -> PathBuf {
    let text = file.to_string_lossy();
    match text.strip_suffix(CAPTURE_SUFFIX) {
        Some(stem) => PathBuf::from(format!("{stem}.jsonl")),
        None => file.to_owned(),
    }
}

impl Store {
    /// Replays the capture at `file` (the root `.jsonl` or its `.capture.json`) with its child
    /// sessions into the store and returns the mission as the store now holds it. Replaying a
    /// capture twice adds nothing.
    pub fn replay_file(
        &mut self,
        file: &Path,
        cfg: &IngestConfig,
    ) -> Result<OpenedMission, StoreError> {
        self.replay_file_with(file, cfg, CycleTemplate::engineering())
    }

    /// `replay_file`, assigning the events to the lanes of `template` (F-041).
    pub fn replay_file_with(
        &mut self,
        file: &Path,
        cfg: &IngestConfig,
        template: impl Into<Arc<CycleTemplate>>,
    ) -> Result<OpenedMission, StoreError> {
        let root = std::path::absolute(root_of(file)).unwrap_or_else(|_| root_of(file));
        let (version, warning) = select_mapping_of(&root)?;
        let tree = SessionTree::discover(&root, cfg)?;
        self.record_tree(&tree, &CaptureInfo::from_session_file(&root)?, false)?;

        let mut source = JsonlSource::open_tree(&tree, cfg)?;
        let mut mapper = TreeMapper::with_template(&tree, version, template);
        let mut records = 0_u64;
        loop {
            let mut batch = source.next_batch()?;
            let at_eof = source.at_eof();
            if at_eof {
                batch.extend(source.drain_partial());
            }
            records += batch.len() as u64;
            let mut events: Vec<_> = batch.iter().flat_map(|record| mapper.map(record)).collect();
            if at_eof {
                events.extend(mapper.finish());
            }
            self.insert_events(&events)?;
            if at_eof {
                break;
            }
        }
        let mut warnings: Vec<String> = warning.iter().map(ToString::to_string).collect();
        if let Some(info) = read_capture(&root)?.and_then(|header| header.usage_log) {
            let path = root.with_file_name(&info.file);
            if path.is_file() {
                let mut log = UsageLogSource::open_files(&[path], cfg)?;
                let turns = log.read_all()?;
                let events: Vec<_> = turns
                    .iter()
                    .filter_map(|turn| mapper.map_usage(turn))
                    .collect();
                self.insert_events(&events)?;
            } else {
                warnings.push(missing_usage_log_warning(&info.file));
            }
        }
        self.mark_tailed(tree.mission_id())?;

        let mut opened = self.open_mission(tree.mission_id())?;
        opened.summary.record_count = records;
        opened.summary.warning = (!warnings.is_empty()).then(|| warnings.join("; "));
        Ok(opened)
    }

    /// The mission `mission` as its rows describe it: events in replay order, sessions, counts.
    /// `record_count` is 0 (the store keeps pointers, not how many lines were read) and `live`
    /// is false; the shell sets it when a session of the mission is being tailed.
    pub fn open_mission(&self, mission: &MissionId) -> Result<OpenedMission, StoreError> {
        let (name, started_at): (String, String) = self
            .conn
            .query_row(
                "SELECT name, started_at FROM missions WHERE id = ?1",
                params![mission.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::UnknownMission {
                id: mission.to_string(),
            })?;
        let events = self.events(&EventScope::Mission(mission.clone()), TimeRange::ALL)?;
        let sessions = self.sessions_of(mission)?;
        let summary = MissionSummary {
            mission_id: mission.clone(),
            name,
            started_at,
            event_count: events
                .iter()
                .filter(|event| !event.kind.is_panel_only())
                .count() as u64,
            unknown_count: events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
                .count() as u64,
            record_count: 0,
            session_count: sessions.len() as u64,
            mapping_version: events
                .first()
                .map_or(MappingVersion::LATEST, |event| event.mapping_version)
                .to_string(),
            warning: None,
        };
        Ok(OpenedMission {
            summary,
            events,
            sessions,
            live: false,
            branch: None,
        })
    }

    /// The mission a session file belongs to.
    pub fn mission_of_session(&self, session: &SessionId) -> Result<Option<MissionId>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT mission_id FROM sessions WHERE id = ?1",
                params![session.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(MissionId))
    }

    /// The raw line of event `id`, re-read from its file by pointer and masked for display
    /// (B-006). The store holds no text.
    pub fn event_raw(&self, id: &EventId) -> Result<RawLineView, StoreError> {
        let event = self.event(id)?;
        let SourcePtr::Jsonl {
            path,
            byte_offset,
            byte_len,
        } = &event.source
        else {
            return Err(StoreError::BadRow {
                id: id.to_string(),
                message: "the event does not point into a session file".to_owned(),
            });
        };
        let raw = JsonlSource::read_at(path, *byte_offset, *byte_len)?;
        Ok(raw_line_view(&event, &raw))
    }

    /// The session files of `mission` in the order the store learned them.
    pub fn sessions_of(&self, mission: &MissionId) -> Result<Vec<SessionInfo>, StoreError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT id, parent_id, role, tag, model, path FROM sessions \
             WHERE mission_id = ?1 ORDER BY rowid",
        )?;
        let rows = statement.query_map(params![mission.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            let (id, parent, role, tag, model, path) = row?;
            let effort = read_sidecar(Path::new(&path))
                .ok()
                .flatten()
                .and_then(|sidecar| sidecar.reasoning_effort);
            sessions.push(SessionInfo {
                role: from_wire::<Role>(&role).unwrap_or(Role::Unresolved),
                id: SessionId(id),
                parent: parent.map(SessionId),
                model: model.map(ModelId),
                tag,
                effort,
            });
        }
        Ok(sessions)
    }
}
