//! The read path of the store: events and missions, rebuilt from the flat columns. A row holds
//! a pointer, never text, so what comes back is what `insert_events` was given, except the
//! payload of `handed_to_cd` (M4 keeps it in `prs` and `attestations`).

use rusqlite::{Row, params};
use serde::de::DeserializeOwned;

use super::rows::PointerColumns;
use super::{Store, StoreError};
use crate::mapping::MappingVersion;
use crate::model::{
    Event, EventId, EventKind, FeatureId, LaneId, MilestoneId, MissionId, ModelId, Overlay, Role,
    SessionId, Timestamp, TokenUsage,
};
use crate::view::MissionRow;

/// Which events `Store::events` returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventScope {
    Mission(MissionId),
    Milestone(MissionId, MilestoneId),
    Feature(FeatureId),
    Session(SessionId),
}

/// A closed time interval; an open end is `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimeRange {
    pub from: Option<Timestamp>,
    pub to: Option<Timestamp>,
}

impl TimeRange {
    pub const ALL: TimeRange = TimeRange {
        from: None,
        to: None,
    };

    /// Everything up to and including `at`: the prefix a scrubber shows.
    pub fn until(at: Timestamp) -> Self {
        Self {
            from: None,
            to: Some(at),
        }
    }
}

const SELECT_EVENTS: &str = "SELECT e.id, e.mission_id, e.session_id, e.milestone_id, e.feature_id, \
    e.lane, e.overlay, e.kind, e.raw_type, e.mapping_version, e.role, e.model, e.at, e.duration_ms, \
    e.source, e.path, e.byte_offset, e.byte_len, e.repo, e.sha, e.url, e.action_id, \
    u.input, u.output, u.cache_read, u.cache_write, u.credits \
    FROM events e LEFT JOIN usage u ON u.event_id = e.id \
    WHERE (?1 IS NULL OR e.mission_id = ?1) AND (?2 IS NULL OR e.milestone_id = ?2) \
    AND (?3 IS NULL OR e.feature_id = ?3) AND (?4 IS NULL OR e.session_id = ?4) \
    AND (?5 IS NULL OR e.at >= ?5) AND (?6 IS NULL OR e.at <= ?6) AND (?7 IS NULL OR e.id = ?7) \
    ORDER BY e.at, e.session_id, e.byte_offset, e.sub";

impl Store {
    /// The events of `scope` inside `range`, in the order of the golden file:
    /// `(at, session_id, byte_offset, sub)`.
    pub fn events(&self, scope: &EventScope, range: TimeRange) -> Result<Vec<Event>, StoreError> {
        let (mission, milestone, feature, session) = match scope {
            EventScope::Mission(mission) => (Some(mission.to_string()), None, None, None),
            EventScope::Milestone(mission, milestone) => (
                Some(mission.to_string()),
                Some(milestone.to_string()),
                None,
                None,
            ),
            EventScope::Feature(feature) => (None, None, Some(feature.to_string()), None),
            EventScope::Session(session) => (None, None, None, Some(session.to_string())),
        };
        let mut statement = self.conn.prepare_cached(SELECT_EVENTS)?;
        let rows = statement.query_map(
            params![
                mission,
                milestone,
                feature,
                session,
                range.from.map(|at| at.to_rfc3339_millis()),
                range.to.map(|at| at.to_rfc3339_millis()),
                None::<String>,
            ],
            |row| Ok(event_of(row)),
        )?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row??);
        }
        Ok(events)
    }

    /// The event with `id`.
    pub fn event(&self, id: &EventId) -> Result<Event, StoreError> {
        let mut statement = self.conn.prepare_cached(SELECT_EVENTS)?;
        let mut rows = statement.query_map(
            params![
                None::<String>,
                None::<String>,
                None::<String>,
                None::<String>,
                None::<String>,
                None::<String>,
                Some(id.to_string()),
            ],
            |row| Ok(event_of(row)),
        )?;
        match rows.next() {
            Some(row) => row?,
            None => Err(StoreError::UnknownEvent { id: id.to_string() }),
        }
    }

    /// Every mission with the number of events indexed for it, newest first.
    pub fn missions(&self) -> Result<Vec<MissionRow>, StoreError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT m.id, m.name, m.state, m.started_at, m.completed_at, \
             (SELECT count(*) FROM events e WHERE e.mission_id = m.id AND e.kind <> 'turn.usage') \
             FROM missions m ORDER BY m.started_at DESC, m.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(MissionRow {
                id: MissionId(row.get(0)?),
                name: row.get(1)?,
                state: row.get(2)?,
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                event_count: u64::try_from(row.get::<_, i64>(5)?).unwrap_or(0),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

fn event_of(row: &Row<'_>) -> Result<Event, StoreError> {
    let id: String = row.get(0)?;
    let bad = |message: String| StoreError::BadRow {
        id: id.clone(),
        message,
    };
    let lane = LaneId(row.get(5)?);
    let overlay = row
        .get::<_, Option<String>>(6)?
        .map(|text| Overlay::parse(&text).ok_or_else(|| bad(format!("overlay {text:?}"))))
        .transpose()?;
    let pointer = PointerColumns {
        source: row.get(14)?,
        path: row.get(15)?,
        byte_offset: row.get(16)?,
        byte_len: row.get(17)?,
        repo: row.get(18)?,
        sha: row.get(19)?,
        url: row.get(20)?,
        action_id: row.get(21)?,
    };
    let kind_name: String = row.get(7)?;
    let kind = kind_of(&kind_name, row.get(8)?, &pointer)
        .ok_or_else(|| bad(format!("kind {kind_name:?}")))?;
    let version: String = row.get(9)?;
    let role: String = row.get(10)?;
    let at: String = row.get(12)?;
    let usage = match row.get::<_, Option<i64>>(22)? {
        Some(input) => Some(TokenUsage {
            input: unsigned(input),
            output: unsigned(row.get(23)?),
            cache_read: unsigned(row.get(24)?),
            cache_write: unsigned(row.get(25)?),
            credits: row.get(26)?,
        }),
        None => None,
    };
    let model = row.get::<_, Option<String>>(11)?.map(ModelId);
    let at = Timestamp::parse(&at).map_err(|error| bad(format!("at {at:?}: {error}")))?;
    // A turn.usage keeps input, output and cache_read in the usage table; `reasoning` and
    // `total_input` have no column and read back as 0 (the pointer still names the line).
    let (kind, usage) = match (kind, usage) {
        (EventKind::TurnUsage { .. }, Some(counts)) => (
            EventKind::TurnUsage {
                input: counts.input,
                output: counts.output,
                cache_read: counts.cache_read,
                reasoning: 0,
                total_input: 0,
                model: model.clone(),
                log_ts: at,
            },
            None,
        ),
        other => other,
    };
    Ok(Event {
        mission_id: MissionId(row.get(1)?),
        session_id: row.get::<_, Option<String>>(2)?.map(SessionId),
        milestone_id: row.get::<_, Option<String>>(3)?.map(MilestoneId),
        feature_id: row.get::<_, Option<String>>(4)?.map(FeatureId),
        lane,
        overlay,
        kind,
        mapping_version: from_wire::<MappingVersion>(&version)
            .ok_or_else(|| bad(format!("mapping version {version:?}")))?,
        role: from_wire::<Role>(&role).ok_or_else(|| bad(format!("role {role:?}")))?,
        model,
        at,
        duration: row
            .get::<_, Option<i64>>(13)?
            .map(|ms| std::time::Duration::from_millis(unsigned(ms))),
        usage,
        source: pointer.into_ptr(&id)?,
        id: EventId(id.clone()),
    })
}

fn unsigned(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

/// A unit-variant enum from its wire text (`"scrutiny_validator"`).
pub(super) fn from_wire<T: DeserializeOwned>(text: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(text.to_owned())).ok()
}

fn kind_of(name: &str, raw_type: Option<String>, pointer: &PointerColumns) -> Option<EventKind> {
    match name {
        "unknown.event" => Some(EventKind::Unknown {
            raw_type: raw_type.unwrap_or_default(),
        }),
        "handed_to_cd" => Some(EventKind::HandedToCd {
            merge_sha: pointer.sha.clone().unwrap_or_default(),
            pr_number: pointer
                .url
                .as_deref()
                .and_then(|url| url.rsplit('/').next())
                .and_then(|last| last.parse().ok())
                .unwrap_or(0),
            attestation_ids: Vec::new(),
        }),
        _ => EventKind::KNOWN
            .iter()
            .find(|kind| kind.prd_name() == name)
            .cloned(),
    }
}
