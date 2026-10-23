//! SQLite index of events by pointer (architecture §6). The store keeps `path`, `byte_offset`
//! and `byte_len` of every raw line and never the line itself, tool output, prompts or the
//! `systemPrompt` (invariant I3): the line is re-read from its file on demand.

mod error;
mod record;
mod rows;
mod schema;

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

pub use error::StoreError;
pub use record::CaptureInfo;

use rows::{PointerColumns, raw_type, sub_of, wire_text};

use crate::model::{Event, EventId, EventKind, MissionId, SourcePtr, Timestamp};

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens `path`, creating the file and its parent directories, and applies the migrations
    /// not yet applied. Foreign keys are enforced and the journal is write-ahead.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|source| StoreError::Io {
                path: parent.to_owned(),
                source,
            })?;
        }
        let open = |source| StoreError::Open {
            path: path.to_owned(),
            source,
        };
        let mut conn = Connection::open(path).map_err(open)?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(open)?;
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))
            .map_err(open)?;
        schema::migrate(&mut conn).map_err(open)?;
        schema::seed_mapping_versions(&conn).map_err(open)?;
        Ok(Self { conn })
    }

    /// Opens a store that must already exist, read-only: nothing is created and nothing
    /// written. For readers such as `inspect`.
    pub fn open_existing(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(
            |source| StoreError::Open {
                path: path.to_owned(),
                source,
            },
        )?;
        Ok(Self { conn })
    }

    /// Stores the pointer and the flat columns of each event. An id already stored is left
    /// as it is, so replaying a capture twice adds nothing. Returns the number of new rows.
    pub fn insert_events(&mut self, events: &[Event]) -> Result<usize, StoreError> {
        let tx = self.conn.transaction()?;
        let mut added = 0;
        {
            let mut insert = tx.prepare_cached(
                "INSERT INTO events (id, mission_id, session_id, milestone_id, feature_id, lane, kind, \
                 raw_type, mapping_version, role, model, at, duration_ms, source, path, byte_offset, \
                 byte_len, sub, repo, sha, url, action_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, \
                 ?18, ?19, ?20, ?21, ?22) ON CONFLICT (id) DO NOTHING",
            )?;
            let mut insert_usage = tx.prepare_cached(
                "INSERT INTO usage (event_id, input, output, cache_read, cache_write, credits) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT (event_id) DO NOTHING",
            )?;
            for event in events {
                let id = event.id.to_string();
                let ptr = PointerColumns::of(&event.source, &id)?;
                let duration_ms = event
                    .duration
                    .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX));
                let new = insert.execute(params![
                    id,
                    event.mission_id.to_string(),
                    event.session_id.as_ref().map(ToString::to_string),
                    event.milestone_id.as_ref().map(ToString::to_string),
                    event.feature_id.as_ref().map(ToString::to_string),
                    event.lane.number(),
                    event.kind.prd_name(),
                    raw_type(&event.kind),
                    event.mapping_version.to_string(),
                    wire_text(&event.role),
                    event.model.as_ref().map(ToString::to_string),
                    event.at.to_rfc3339_millis(),
                    duration_ms,
                    ptr.source,
                    ptr.path,
                    ptr.byte_offset,
                    ptr.byte_len,
                    sub_of(event),
                    ptr.repo,
                    ptr.sha,
                    ptr.url,
                    ptr.action_id,
                ])?;
                if let (1, Some(usage)) = (new, event.usage) {
                    insert_usage.execute(params![
                        id,
                        to_i64(usage.input),
                        to_i64(usage.output),
                        to_i64(usage.cache_read),
                        to_i64(usage.cache_write),
                        usage.credits,
                    ])?;
                }
                added += new;
            }
        }
        tx.commit()?;
        Ok(added)
    }

    /// Where the raw material of event `id` lives.
    pub fn pointer(&self, id: &EventId) -> Result<SourcePtr, StoreError> {
        let columns = self
            .conn
            .query_row(
                "SELECT source, path, byte_offset, byte_len, repo, sha, url, action_id \
                 FROM events WHERE id = ?1",
                params![id.to_string()],
                |row| {
                    Ok(PointerColumns {
                        source: row.get(0)?,
                        path: row.get(1)?,
                        byte_offset: row.get(2)?,
                        byte_len: row.get(3)?,
                        repo: row.get(4)?,
                        sha: row.get(5)?,
                        url: row.get(6)?,
                        action_id: row.get(7)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| StoreError::UnknownEvent { id: id.to_string() })?;
        columns.into_ptr(&id.to_string())
    }

    /// Events of `mission` that no mapping rule claimed (rule 18: counted and shown).
    pub fn unknown_count(&self, mission: &MissionId) -> Result<u64, StoreError> {
        let unknown = EventKind::Unknown {
            raw_type: String::new(),
        };
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM events WHERE mission_id = ?1 AND kind = ?2",
            params![mission.to_string(), unknown.prd_name()],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(count).unwrap_or(0))
    }
}

fn to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn now_text() -> String {
    Timestamp(time::OffsetDateTime::now_utc()).to_rfc3339_millis()
}
