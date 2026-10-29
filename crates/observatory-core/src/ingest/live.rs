//! What the shell hands the UI while a session is attached (architecture §7): event batches
//! flushed every `[ui].event_batch_ms`, the sessions being watched and the limits that apply.
//! Plain data and a clock-injected batcher, so the UI crate can read the types in wasm and the
//! timing is testable without sleeping.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{SessionInfo, order_key};
use crate::model::{Event, EventId, MissionId, SessionId, SourcePtr};
use crate::view::MissionSummary;

/// The `events_appended` payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventBatch {
    pub mission_id: MissionId,
    pub events: Vec<Event>,
    /// Unknown events of the mission in the store after this batch (rule 18: the tray count).
    pub unknown_total: u64,
    /// Events the UI holds that a rewrite of their session file made stale; drop them before
    /// adding `events`.
    #[serde(default)]
    pub retracted: Vec<Retraction>,
    /// Events the store deleted after the UI may have received them (the ack-derived
    /// `mission.paused` that the stream's own line replaced); drop them before adding `events`.
    #[serde(default)]
    pub retracted_events: Vec<EventId>,
}

/// The lines of a session file that Droid rewrote in place moved, so the events read from them
/// have pointers into bytes that are gone. They are replaced by the events of the same lines
/// read again, under their new offsets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retraction {
    pub session_id: SessionId,
    /// Everything the tail had read: the lines that started before this offset.
    pub below_offset: u64,
}

impl Retraction {
    /// A line of the session file after the first. The first line is the one rewritten in
    /// place, its events keep their ids and are not emitted again.
    pub fn covers(&self, event: &Event) -> bool {
        event.session_id.as_ref() == Some(&self.session_id)
            && matches!(
                &event.source,
                SourcePtr::Jsonl { byte_offset, .. }
                    if *byte_offset > 0 && *byte_offset < self.below_offset
            )
    }
}

/// Drops from `held` the events a retraction covers. Returns how many were dropped.
pub fn retract_stale(held: &mut Vec<Event>, retracted: &[Retraction]) -> usize {
    let before = held.len();
    held.retain(|event| !retracted.iter().any(|r| r.covers(event)));
    before - held.len()
}

/// Drops from `held` the events named in `ids`. Returns how many were dropped.
pub fn drop_events(held: &mut Vec<Event>, ids: &[EventId]) -> usize {
    let before = held.len();
    held.retain(|event| !ids.contains(&event.id));
    before - held.len()
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
    /// The branch the Gate placeholder names; the shell fills it from `missions.branch` or the
    /// configured pattern, so a store opened without a config leaves it unset.
    #[serde(default)]
    pub branch: Option<String>,
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
    retracted: Vec<Retraction>,
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
                retracted: Vec::new(),
                since: now,
            }),
        }
    }

    /// Takes back what a rewrite made stale: the pending events it covers never leave, and the
    /// next batch tells the UI to drop the ones it already holds. Events pushed after this call
    /// are the lines read again.
    pub fn retract(
        &mut self,
        mission: &MissionId,
        retraction: Retraction,
        unknown_total: u64,
        now: Instant,
    ) {
        let index = self
            .pending
            .iter()
            .position(|p| p.mission == *mission)
            .unwrap_or_else(|| {
                self.pending.push(Pending {
                    mission: mission.clone(),
                    events: Vec::new(),
                    unknown_total,
                    retracted: Vec::new(),
                    since: now,
                });
                self.pending.len() - 1
            });
        let pending = &mut self.pending[index];
        pending.events.retain(|event| !retraction.covers(event));
        pending.unknown_total = unknown_total;
        pending.retracted.push(retraction);
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
            retracted: self.retracted,
            retracted_events: Vec::new(),
        }
    }
}
