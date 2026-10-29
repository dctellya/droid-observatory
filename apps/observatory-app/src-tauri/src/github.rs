//! The GitHub poll: every `[github].poll_interval_s` each live mission's pull request and its
//! checks are read through core's `GitHubSource`, recorded in the store, and the new events
//! reach the window as `events_appended`. Requests are logged by the client as
//! `github.request{method, url, status}`; this module logs what a poll saw, never a header.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use observatory_core::config::Config;
use observatory_core::ingest::EventBatch;
use observatory_core::ingest::github::{GitHubClient, GitHubSource, UreqTransport};
use observatory_core::model::{CycleTemplate, MissionId, Timestamp};
use observatory_core::secrets::{self, Secret, TokenSource};
use observatory_core::store::Store;
use observatory_core::view::mission_branch;
use tauri::{AppHandle, Emitter, Runtime};

type Source = GitHubSource<GitHubClient<UreqTransport>>;

/// The running poll thread.
pub struct Github {
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Github {
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

pub fn spawn<R: Runtime>(
    app: AppHandle<R>,
    cfg: Config,
    template: Arc<CycleTemplate>,
    store_path: PathBuf,
) -> Github {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let started = std::thread::Builder::new()
        .name("github".to_owned())
        .spawn(move || run(&app, &cfg, &template, &store_path, &flag));
    if let Err(err) = &started {
        tracing::error!(%err, "github thread");
    }
    Github {
        stop,
        thread: Mutex::new(started.ok()),
    }
}

fn run<R: Runtime>(
    app: &AppHandle<R>,
    cfg: &Config,
    template: &CycleTemplate,
    store_path: &std::path::Path,
    stop: &AtomicBool,
) {
    let mut store = match Store::open(store_path) {
        Ok(store) => store,
        Err(error) => {
            tracing::error!(%error, "github poll not started");
            return;
        }
    };
    tracing::info!(
        repo = %cfg.github.repo,
        target = %cfg.github.target_branch,
        interval_s = cfg.github.poll_interval_s,
        "github_started"
    );
    // One secrets call per app process: a keychain read per poll or per mission would prompt
    // again, so the value is kept here and cloned into each client.
    let token = read_token();
    let slice = Duration::from_millis(cfg.ingest.jsonl_poll_ms.max(1));
    let interval = Duration::from_secs(cfg.github.poll_interval_s.max(1));
    let mut sources: HashMap<MissionId, Source> = HashMap::new();
    while !stop.load(Ordering::Relaxed) {
        for mission in crate::stall::live_missions(app) {
            poll_mission(
                app,
                cfg,
                template,
                &mut store,
                &mut sources,
                token.as_ref(),
                mission,
            );
        }
        let mut waited = Duration::ZERO;
        while waited < interval && !stop.load(Ordering::Relaxed) {
            std::thread::sleep(slice);
            waited += slice;
        }
    }
}

/// The token, read once at start. A refusing keychain leaves the poll unauthenticated rather than
/// asking again.
fn read_token() -> Option<Secret> {
    let (token, source) = match secrets::github_token_with_source() {
        Ok(resolved) => resolved,
        Err(error) => {
            tracing::warn!(%error, "github token lookup");
            (None, TokenSource::None)
        }
    };
    tracing::info!(source = %source.as_str(), "github.token");
    token
}

fn poll_mission<R: Runtime>(
    app: &AppHandle<R>,
    cfg: &Config,
    template: &CycleTemplate,
    store: &mut Store,
    sources: &mut HashMap<MissionId, Source>,
    token: Option<&Secret>,
    mission: MissionId,
) {
    if !sources.contains_key(&mission) {
        let Some(name) = store
            .missions()
            .ok()
            .and_then(|rows| rows.into_iter().find(|row| row.id == mission))
            .map(|row| row.name)
        else {
            return;
        };
        let branch = mission_branch(&cfg.mission.branch_pattern, &name);
        let client = GitHubClient::with_token(
            &cfg.github,
            token.map(|secret| Secret::new(secret.expose().to_owned())),
        );
        let _ = store.set_mission_branch(&mission, &branch);
        sources.insert(
            mission.clone(),
            GitHubSource::new(
                client,
                mission.clone(),
                &branch,
                &cfg.github,
                template.clone(),
            ),
        );
    }
    let Some(source) = sources.get_mut(&mission) else {
        return;
    };
    match source.poll(Timestamp::now()) {
        Ok(poll) => {
            tracing::info!(
                mission_id = %mission,
                pr = poll.pr.as_ref().map(|pr| pr.number),
                checks = poll.checks.len(),
                events = poll.events.len(),
                "github_poll"
            );
            if let Err(error) = store.record_poll(&mission, &poll) {
                tracing::error!(%error, "store");
                return;
            }
            if !poll.events.is_empty() {
                let batch = EventBatch {
                    unknown_total: store.unknown_count(&mission).unwrap_or(0),
                    mission_id: mission,
                    events: poll.events,
                    retracted: Vec::new(),
                    retracted_events: Vec::new(),
                };
                crate::ingest::log_batch(&batch);
                let _ = app.emit("events_appended", &batch);
            }
        }
        Err(error) => tracing::warn!(mission_id = %mission, %error, "github_poll_failed"),
    }
}
