//! The ingest loop of the shell without the shell: attach, store and batcher driven by `step`.
//! The Tauri thread calls `step` in a loop and emits what it returns, so the part that decides
//! when events leave and what is stored is tested here.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use super::Attach;
use crate::config::Config;
use crate::ingest::{AttachStatus, EventBatch, EventBatcher, IngestError, SessionRow};
use crate::model::{Event, MissionId, SessionId};
use crate::store::{CaptureInfo, Store, StoreError};

/// What one `LiveIngest::step` has for the shell to emit.
#[derive(Default)]
pub struct LiveStep {
    /// Batches whose oldest event waited `[ui].event_batch_ms`.
    pub batches: Vec<EventBatch>,
    /// Sessions seen for the first time, and sessions whose name changed (a tag written after
    /// the file appeared).
    pub discovered: Vec<SessionRow>,
    /// Raised by attach in this step; each is raised once.
    pub warnings: Vec<IngestError>,
    /// The store could not take something; the events still go to the UI.
    pub errors: Vec<StoreError>,
    /// Set when the watched list, the unwatched count or the warnings changed.
    pub status: Option<AttachStatus>,
}

pub struct LiveIngest {
    attach: Attach,
    store: Store,
    batcher: EventBatcher,
    poll: Duration,
    /// The poll while a watched file is growing: the notifier does not report every write of a
    /// handle the writer keeps open, so for one poll interval after a line was read the files are read
    /// as often as batches leave.
    hot_poll: Duration,
    hot_until: Instant,
    announced: HashMap<SessionId, String>,
    warnings: Vec<String>,
    unwatched: usize,
    last_status: Option<AttachStatus>,
    names_checked: Instant,
}

impl LiveIngest {
    /// Starts watching `[ingest].factory_sessions_dir`; sessions already growing are attached
    /// by the first step. Fails when the directory cannot be read.
    pub fn start(cfg: &Config, store: Store) -> Result<Self, IngestError> {
        Ok(Self {
            attach: Attach::start(&cfg.ingest)?,
            store,
            batcher: EventBatcher::new(Duration::from_millis(cfg.ui.event_batch_ms)),
            poll: Duration::from_millis(cfg.ingest.jsonl_poll_ms.max(1)),
            hot_poll: Duration::from_millis(cfg.ui.event_batch_ms.max(1)),
            hot_until: Instant::now(),
            announced: HashMap::new(),
            warnings: Vec::new(),
            unwatched: 0,
            last_status: None,
            names_checked: Instant::now(),
        })
    }

    /// Skips the session file of `id`: its events come from another source (a hosted spool).
    pub fn exclude_session(&mut self, id: &SessionId) {
        self.attach.exclude_session(id);
    }

    /// Waits for a file change, a poll interval or the next batch deadline, whichever is
    /// first, then reads what the watched files gained.
    pub fn step(&mut self) -> LiveStep {
        let idle_wait = if Instant::now() < self.hot_until {
            self.hot_poll
        } else {
            self.poll
        };
        let wait = self
            .batcher
            .due_in(Instant::now())
            .map_or(idle_wait, |due| due.min(idle_wait));
        self.attach.wait_up_to(wait);
        let batch = self.attach.poll();
        if batch.records > 0 {
            self.hot_until = Instant::now() + self.poll;
        }

        let mut step = LiveStep::default();
        for tree in &batch.trees {
            let recorded = CaptureInfo::from_session_file(&tree.root().path)
                .map_err(StoreError::from)
                .and_then(|capture| self.store.record_tree(tree, &capture, false));
            if let Err(err) = recorded {
                step.errors.push(err);
            }
        }
        for (mission, retraction) in batch.retractions {
            if let Err(err) = self
                .store
                .retract_events(&retraction.session_id, retraction.below_offset)
            {
                step.errors.push(err);
            }
            let unknown = self.store.unknown_count(&mission).unwrap_or(0);
            self.batcher
                .retract(&mission, retraction, unknown, Instant::now());
        }
        self.store_and_queue(batch.events, &mut step);
        for warning in &batch.warnings {
            self.warnings.push(match warning {
                IngestError::TooLarge { .. } => format!("TooLarge: {warning}"),
                other => other.to_string(),
            });
        }
        step.warnings = batch.warnings;
        self.unwatched = batch.unwatched;

        let names_due = self.names_checked.elapsed() >= self.poll;
        if !batch.trees.is_empty() || names_due {
            self.names_checked = Instant::now();
            step.discovered = self.announce_changes();
        }
        step.batches = self.batcher.flush_due(Instant::now());
        let status = self.status();
        if self.last_status.as_ref() != Some(&status) {
            self.last_status = Some(status.clone());
            step.status = Some(status);
        }
        step
    }

    /// Ends every session (usage and end events only a finished file can give), stores those
    /// events and returns every batch still pending.
    pub fn finish(&mut self) -> Vec<EventBatch> {
        let events = self.attach.finish();
        let mut step = LiveStep::default();
        self.store_and_queue(events, &mut step);
        self.batcher.flush_all()
    }

    pub fn status(&self) -> AttachStatus {
        AttachStatus {
            watched: self.rows(),
            unwatched: self.unwatched,
            warnings: self.warnings.clone(),
            error: None,
        }
    }

    fn rows(&self) -> Vec<SessionRow> {
        self.attach
            .watched()
            .into_iter()
            .map(|session| SessionRow {
                session_id: session.id,
                name: session.name,
                path: session.path,
                parent: session.parent,
            })
            .collect()
    }

    fn announce_changes(&mut self) -> Vec<SessionRow> {
        let mut changed = Vec::new();
        for row in self.rows() {
            if self.announced.get(&row.session_id) != Some(&row.name) {
                self.announced
                    .insert(row.session_id.clone(), row.name.clone());
                changed.push(row);
            }
        }
        changed
    }

    fn store_and_queue(&mut self, events: Vec<Event>, step: &mut LiveStep) {
        let mut by_mission: BTreeMap<MissionId, Vec<Event>> = BTreeMap::new();
        for event in events {
            by_mission
                .entry(event.mission_id.clone())
                .or_default()
                .push(event);
        }
        for (mission, events) in by_mission {
            if let Err(err) = self.store.insert_events(&events) {
                step.errors.push(err);
            }
            let unknown = self.store.unknown_count(&mission).unwrap_or(0);
            self.batcher.push(&mission, events, unknown, Instant::now());
        }
    }
}
