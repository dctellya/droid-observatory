//! What the shell hands the UI while a session is attached (architecture §7): event batches
//! flushed every `[ui].event_batch_ms`, the sessions being watched and the limits that apply.
//! Plain data and a clock-injected batcher, so the UI crate can read the types in wasm and the
//! timing is testable without sleeping.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{SessionInfo, order_key};
use crate::model::{Event, MissionId, SessionId};
use crate::view::MissionSummary;

/// The `events_appended` payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventBatch {
    pub mission_id: MissionId,
    pub events: Vec<Event>,
    /// Unknown events of the mission in the store after this batch (rule 18: the tray count).
    pub unknown_total: u64,
}

/// One session file: the `session_discovered` payload and a row of the watched list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRow {
    pub session_id: SessionId,
    /// The first tag, else the session id (FR-L1).
    pub name: String,
    pub path: PathBuf,
    pub parent: Option<SessionId>,
}

/// What attach is doing now: the `list_sessions` answer and the `attach_status` payload.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachStatus {
    pub watched: Vec<SessionRow>,
    /// Sessions found and not watched because of `max_watched_sessions`.
    pub unwatched: usize,
    /// Warnings raised so far, such as a `TooLarge` file; each is raised once.
    pub warnings: Vec<String>,
    /// Why nothing is watched, when attach could not start.
    pub error: Option<String>,
}

/// A mission handed to the UI: the answer of `open_replay`, `attach` and `open_mission`. The
/// UI folds later `events_appended` batches into `events` with `merge_ordered`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpenedMission {
    pub summary: MissionSummary,
    pub events: Vec<Event>,
    pub sessions: Vec<SessionInfo>,
    /// True while one of the mission's session files is being tailed.
    pub live: bool,
}

/// Adds the events of `incoming` whose id `held` does not hold and puts `held` back in replay
/// order, so a board fed by batches lists what a replay of the same files lists. Returns the
/// number of events added.
pub fn merge_ordered(held: &mut Vec<Event>, incoming: Vec<Event>) -> usize {
    let mut known: HashSet<String> = held.iter().map(|event| event.id.0.clone()).collect();
    let before = held.len();
    for event in incoming {
        if known.insert(event.id.0.clone()) {
            held.push(event);
        }
    }
    let added = held.len() - before;
    if added > 0 {
        held.sort_by_cached_key(order_key);
    }
    added
}

struct Pending {
    mission: MissionId,
    events: Vec<Event>,
    unknown_total: u64,
    /// When the oldest event of this batch arrived.
    since: Instant,
}

/// Collects events per mission and releases them once the oldest has waited `interval`, so the
/// UI renders one batch per interval instead of one per line (`[ui].event_batch_ms`). The caller
/// passes the time; nothing here reads a clock.
pub struct EventBatcher {
    interval: Duration,
    pending: Vec<Pending>,
}

impl EventBatcher {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            pending: Vec::new(),
        }
    }

    /// Adds `events` of `mission`, in arrival order. `unknown_total` is the mission's count
    /// after them; the latest one pushed before a flush is the one reported.
    pub fn push(
        &mut self,
        mission: &MissionId,
        events: Vec<Event>,
        unknown_total: u64,
        now: Instant,
    ) {
        if events.is_empty() {
            return;
        }
        match self.pending.iter_mut().find(|p| p.mission == *mission) {
            Some(pending) => {
                pending.events.extend(events);
                pending.unknown_total = unknown_total;
            }
            None => self.pending.push(Pending {
                mission: mission.clone(),
                events,
                unknown_total,
                since: now,
            }),
        }
    }

    /// How long until the next batch is due; `None` when nothing is pending.
    pub fn due_in(&self, now: Instant) -> Option<Duration> {
        self.pending
            .iter()
            .map(|p| {
                self.interval
                    .saturating_sub(now.saturating_duration_since(p.since))
            })
            .min()
    }

    /// The batches whose oldest event has waited the whole interval.
    pub fn flush_due(&mut self, now: Instant) -> Vec<EventBatch> {
        let interval = self.interval;
        let (due, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|p| now.saturating_duration_since(p.since) >= interval);
        self.pending = rest;
        due.into_iter().map(Pending::into_batch).collect()
    }

    /// Every pending batch, due or not: at shutdown and when a session ends.
    pub fn flush_all(&mut self) -> Vec<EventBatch> {
        std::mem::take(&mut self.pending)
            .into_iter()
            .map(Pending::into_batch)
            .collect()
    }
}

impl Pending {
    fn into_batch(self) -> EventBatch {
        EventBatch {
            mission_id: self.mission,
            events: self.events,
            unknown_total: self.unknown_total,
        }
    }
}
