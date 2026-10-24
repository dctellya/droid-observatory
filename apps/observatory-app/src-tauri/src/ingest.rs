//! The ingest thread: core's `LiveIngest` driven in a loop, its steps turned into UI events and
//! the INFO lines of architecture §12.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use observatory_core::config::Config;
use observatory_core::ingest::watch::{LiveIngest, LiveStep};
use observatory_core::ingest::{AttachStatus, EventBatch, IngestError};
use observatory_core::model::SessionId;
use observatory_core::store::Store;
use tauri::{AppHandle, Emitter};

/// The running ingest thread. Stopping it ends every watched session (their final usage and end
/// events only exist once the file is complete) before the thread returns.
pub struct Ingest {
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Ingest {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let thread = self
            .thread
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(thread) = thread {
            let _ = thread.join();
        }
    }
}

pub fn spawn(
    app: AppHandle,
    cfg: Config,
    store_path: PathBuf,
    status: Arc<Mutex<AttachStatus>>,
    excluded: Receiver<SessionId>,
) -> Ingest {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let started = std::thread::Builder::new()
        .name("ingest".to_owned())
        .spawn(move || run(&app, &cfg, &store_path, &status, &flag, &excluded));
    if let Err(err) = &started {
        tracing::error!(%err, "ingest thread");
    }
    Ingest {
        stop,
        thread: Mutex::new(started.ok()),
    }
}

fn run(
    app: &AppHandle,
    cfg: &Config,
    store_path: &Path,
    status: &Mutex<AttachStatus>,
    stop: &AtomicBool,
    excluded: &Receiver<SessionId>,
) {
    let started = Store::open(store_path)
        .map_err(|err| err.to_string())
        .and_then(|store| LiveIngest::start(cfg, store).map_err(|err| err.to_string()));
    let mut live = match started {
        Ok(live) => live,
        Err(error) => {
            tracing::error!(%error, "attach not started");
            publish(
                app,
                status,
                AttachStatus {
                    error: Some(error),
                    ..AttachStatus::default()
                },
            );
            return;
        }
    };
    tracing::info!(dir = %cfg.ingest.factory_sessions_dir, "ingest_started");
    while !stop.load(Ordering::Relaxed) {
        while let Ok(id) = excluded.try_recv() {
            live.exclude_session(&id);
        }
        report(app, status, live.step());
    }
    for batch in live.finish() {
        log_batch(&batch);
        let _ = app.emit("events_appended", &batch);
    }
}

fn report(app: &AppHandle, status: &Mutex<AttachStatus>, step: LiveStep) {
    for row in &step.discovered {
        tracing::info!(
            session_id = %row.session_id,
            path = %row.path.display(),
            name = %row.name,
            "session_discovered"
        );
        let _ = app.emit("session_discovered", row);
    }
    for warning in &step.warnings {
        match warning {
            IngestError::TooLarge { path, mb } => {
                tracing::warn!(path = %path.display(), mb, "TooLarge");
            }
            other => tracing::warn!(%other, "attach"),
        }
    }
    for error in &step.errors {
        tracing::error!(%error, "store");
    }
    if let Some(next) = step.status {
        tracing::info!(
            watched = next.watched.len(),
            unwatched = next.unwatched,
            "attach_status"
        );
        publish(app, status, next);
    }
    for batch in &step.batches {
        log_batch(batch);
        let _ = app.emit("events_appended", batch);
    }
}

pub(crate) fn log_batch(batch: &EventBatch) {
    tracing::info!(
        mission_id = %batch.mission_id,
        count = batch.events.len(),
        unknown_total = batch.unknown_total,
        "events_appended"
    );
}

fn publish(app: &AppHandle, shared: &Mutex<AttachStatus>, next: AttachStatus) {
    *shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = next.clone();
    let _ = app.emit("attach_status", next);
}
