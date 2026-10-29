//! The git poll: every `[github].poll_interval_s` each live mission's repository is read through
//! core's `GitSource`, its merges and, when `GITHUB_TOKEN` is set, the fast-check runs of its
//! branch are recorded in the store, and the new events reach the window as `events_appended`.
//! The local part needs no token and runs without the forge.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use observatory_core::config::Config;
use observatory_core::ingest::EventBatch;
use observatory_core::ingest::git::{self, FastCheckRun, GitOptions, GitSource, git_sync};
use observatory_core::ingest::github::{GitHubClient, UreqTransport};
use observatory_core::model::{CycleTemplate, MissionId};
use observatory_core::secrets::{self, Secret};
use observatory_core::store::Store;
use observatory_core::view::{MilestoneWorktree, milestone_worktrees, mission_branch};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// The running poll thread.
pub struct Git {
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Git {
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

/// The worktrees of each mission's milestones as the last poll listed them, for a board opened
/// after the poll.
#[derive(Default)]
pub struct GitCache(Mutex<HashMap<MissionId, Vec<MilestoneWorktree>>>);

impl GitCache {
    pub fn get(&self, mission: &MissionId) -> Vec<MilestoneWorktree> {
        self.lock().get(mission).cloned().unwrap_or_default()
    }

    fn put(&self, mission: &MissionId, worktrees: Vec<MilestoneWorktree>) {
        self.lock().insert(mission.clone(), worktrees);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<MissionId, Vec<MilestoneWorktree>>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub fn spawn<R: Runtime>(
    app: AppHandle<R>,
    cfg: Config,
    template: Arc<CycleTemplate>,
    store_path: PathBuf,
) -> Git {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let started = std::thread::Builder::new()
        .name("git".to_owned())
        .spawn(move || run(&app, &cfg, &template, &store_path, &flag));
    if let Err(err) = &started {
        tracing::error!(%err, "git thread");
    }
    Git {
        stop,
        thread: Mutex::new(started.ok()),
    }
}

/// What a poll shares across missions.
struct Poller<'a> {
    cfg: &'a Config,
    template: &'a CycleTemplate,
    options: GitOptions,
    /// The repository of an attached mission: the working tree the app runs in.
    fallback: Option<PathBuf>,
    /// Present when `GITHUB_TOKEN` is set: the client of the runs part.
    client: Option<GitHubClient<UreqTransport>>,
    skipped: HashSet<MissionId>,
}

fn run<R: Runtime>(
    app: &AppHandle<R>,
    cfg: &Config,
    template: &CycleTemplate,
    store_path: &Path,
    stop: &AtomicBool,
) {
    let mut store = match Store::open(store_path) {
        Ok(store) => store,
        Err(error) => {
            tracing::error!(%error, "git poll not started");
            return;
        }
    };
    let client = secrets::forge_source_enabled_in_env().then(|| client_of(cfg));
    tracing::info!(
        interval_s = cfg.github.poll_interval_s,
        pattern = %cfg.mission.branch_pattern,
        runs = client.is_some(),
        "git_started"
    );
    let mut poller = Poller {
        cfg,
        template,
        options: GitOptions::from_branch_pattern(&cfg.mission.branch_pattern),
        fallback: std::env::current_dir()
            .ok()
            .and_then(|dir| git::toplevel(&dir).ok()),
        client,
        skipped: HashSet::new(),
    };
    let slice = Duration::from_millis(cfg.ingest.jsonl_poll_ms.max(1));
    let interval = Duration::from_secs(cfg.github.poll_interval_s.max(1));
    while !stop.load(Ordering::Relaxed) {
        for mission in crate::stall::live_missions(app) {
            poller.poll_mission(app, &mut store, mission);
        }
        let mut waited = Duration::ZERO;
        while waited < interval && !stop.load(Ordering::Relaxed) {
            std::thread::sleep(slice);
            waited += slice;
        }
    }
}

/// The token comes from `GITHUB_TOKEN`, read once; a missing or refusing lookup leaves the
/// requests unauthenticated.
fn client_of(cfg: &Config) -> GitHubClient<UreqTransport> {
    let token: Option<Secret> = secrets::github_token_with_source()
        .map(|(token, _)| token)
        .unwrap_or_else(|error| {
            tracing::warn!(%error, "git token lookup");
            None
        });
    GitHubClient::with_token(&cfg.github, token)
}

impl Poller<'_> {
    /// The repository of a mission: where a hosted one was launched, else the working tree the
    /// app runs in.
    fn repository<R: Runtime>(&self, app: &AppHandle<R>, mission: &MissionId) -> Option<PathBuf> {
        app.state::<crate::host::Hosts>()
            .repo_of(&mission.0)
            .or_else(|| self.fallback.clone())
    }

    /// The source of the mission's branch in its repository; none, with the reason logged, when
    /// the mission has no row yet, no repository or no such branch.
    fn open_source<R: Runtime>(
        &mut self,
        app: &AppHandle<R>,
        store: &Store,
        mission: &MissionId,
    ) -> Option<GitSource> {
        let name = store
            .missions()
            .ok()?
            .into_iter()
            .find(|row| row.id == *mission)?
            .name;
        let Some(repo) = self.repository(app, mission) else {
            if self.skipped.insert(mission.clone()) {
                tracing::info!(mission_id = %mission, reason = "no repository", "git_skipped");
            }
            return None;
        };
        let branch = mission_branch(&self.cfg.mission.branch_pattern, &name);
        GitSource::open(
            &repo,
            mission.clone(),
            &branch,
            self.template.clone(),
            self.options.clone(),
        )
        .inspect_err(|error| {
            tracing::warn!(mission_id = %mission, %branch, %error, "git_poll_failed");
        })
        .ok()
    }

    fn poll_mission<R: Runtime>(
        &mut self,
        app: &AppHandle<R>,
        store: &mut Store,
        mission: MissionId,
    ) {
        let Some(source) = self.open_source(app, store, &mission) else {
            return;
        };
        let branch = source.branch().to_owned();
        let runs = self.runs(&mission, &branch);
        match git_sync(store, &source, &runs) {
            Ok(sync) => {
                tracing::info!(
                    mission_id = %mission,
                    %branch,
                    merges = sync.merges,
                    runs = sync.runs,
                    events = sync.events,
                    new = sync.new_events.len(),
                    "git_poll"
                );
                if let Some(cache) = app.try_state::<GitCache>() {
                    cache.put(&mission, milestone_worktrees(&sync.snapshot));
                }
                announce(app, store, &mission, sync.new_events);
            }
            Err(error) => {
                tracing::warn!(mission_id = %mission, %branch, %error, "git_poll_failed");
            }
        }
    }

    /// The fast-check runs of the branch; none without a token, and none when the forge fails:
    /// the local part of the poll does not wait on it.
    fn runs(&self, mission: &MissionId, branch: &str) -> Vec<FastCheckRun> {
        let Some(client) = &self.client else {
            return Vec::new();
        };
        client.mission_branch_runs(branch).unwrap_or_else(|error| {
            tracing::warn!(mission_id = %mission, %branch, %error, "git_poll_failed");
            Vec::new()
        })
    }
}

/// Tells the window about the events a poll added.
fn announce<R: Runtime>(
    app: &AppHandle<R>,
    store: &Store,
    mission: &MissionId,
    events: Vec<observatory_core::model::Event>,
) {
    if events.is_empty() {
        return;
    }
    let batch = EventBatch {
        unknown_total: store.unknown_count(mission).unwrap_or(0),
        mission_id: mission.clone(),
        events,
        retracted: Vec::new(),
        retracted_events: Vec::new(),
    };
    crate::ingest::log_batch(&batch);
    let _ = app.emit("events_appended", &batch);
}
