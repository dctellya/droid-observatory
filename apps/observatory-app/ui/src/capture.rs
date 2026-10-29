//! A capture on the board: its events, which grow when the shell sends batches, and where its
//! raw lines come from. A capture opened in the browser host holds the picked files; one opened
//! in the Tauri window asks the shell, which re-reads the line by pointer.

use std::sync::Arc;

use leptos::prelude::*;
use observatory_core::ingest::{
    EventBatch, MemoryReplay, OpenedMission, SessionInfo, drop_events, merge_ordered, retract_stale,
};
use observatory_core::model::{Event, EventId, MissionId};
use observatory_core::view::{GateGroup, MissionSummary, RawLineView, raw_line_view};
use serde_json::json;

use crate::shell;

#[derive(Clone)]
enum RawSource {
    Memory(Arc<MemoryReplay>),
    Shell,
}

#[derive(Clone)]
pub struct Capture {
    /// As the capture was opened; the live counts are derived from `events`.
    pub summary: MissionSummary,
    pub events: ArcRwSignal<Vec<Event>>,
    pub sessions: ArcRwSignal<Vec<SessionInfo>>,
    /// True while a session of the mission is being tailed.
    pub live: ArcRwSignal<bool>,
    /// The Gate header as the last poll read it; none before the first poll or in the browser host.
    pub gate: ArcRwSignal<Option<GateGroup>>,
    raw: RawSource,
}

impl Capture {
    pub fn of_memory(replay: Arc<MemoryReplay>) -> Self {
        Self {
            summary: replay.summary.clone(),
            events: ArcRwSignal::new(replay.events.clone()),
            sessions: ArcRwSignal::new(replay.sessions.clone()),
            live: ArcRwSignal::new(false),
            gate: ArcRwSignal::new(None),
            raw: RawSource::Memory(replay),
        }
    }

    pub fn of_shell(opened: OpenedMission) -> Self {
        Self {
            summary: opened.summary,
            events: ArcRwSignal::new(opened.events),
            sessions: ArcRwSignal::new(opened.sessions),
            live: ArcRwSignal::new(opened.live),
            gate: ArcRwSignal::new(None),
            raw: RawSource::Shell,
        }
    }

    pub fn mission_id(&self) -> &MissionId {
        &self.summary.mission_id
    }

    /// Folds a batch of the shell into the events. True when the batch names a session this
    /// capture has no row for, so the caller can ask the shell for the sessions again.
    pub fn merge(&self, batch: EventBatch) -> bool {
        let unknown_session = self.sessions.with_untracked(|sessions| {
            batch.events.iter().any(|event| {
                event
                    .session_id
                    .as_ref()
                    .is_some_and(|id| sessions.iter().all(|session| session.id != *id))
            })
        });
        let mut changed = 0;
        self.events.update(|held| {
            changed = retract_stale(held, &batch.retracted);
            changed += drop_events(held, &batch.retracted_events);
            changed += merge_ordered(held, batch.events);
        });
        if changed > 0 {
            self.live.set(true);
        }
        unknown_session
    }

    /// Asks the shell for the sessions of the mission and replaces the held rows.
    pub async fn refresh_sessions(&self) {
        let args = json!({ "mission_id": self.mission_id() });
        if let Ok(sessions) = shell::invoke::<_, Vec<SessionInfo>>("mission_sessions", &args).await
        {
            self.sessions.set(sessions);
        }
    }

    /// Asks the shell for the Gate header the last poll read.
    pub async fn refresh_gate(&self) {
        let args = json!({ "mission_id": self.mission_id() });
        if let Ok(Some(gate)) = shell::invoke::<_, Option<GateGroup>>("gate_group", &args).await {
            self.gate.set(Some(gate));
        }
    }

    /// The raw line of an event, masked for display and read-only.
    pub async fn raw_line(&self, id: &EventId) -> Result<RawLineView, String> {
        match &self.raw {
            RawSource::Memory(replay) => self
                .events
                .with_untracked(|events| {
                    let event = events.iter().find(|event| event.id == *id)?;
                    replay
                        .raw_line(&event.source)
                        .map(|raw| raw_line_view(event, raw))
                })
                .ok_or_else(|| {
                    "The raw line is not available: its file is not in the selection.".to_owned()
                }),
            RawSource::Shell => shell::invoke("event_raw", &json!({ "event_id": id })).await,
        }
    }
}
