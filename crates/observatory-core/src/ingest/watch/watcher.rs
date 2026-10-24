//! Discovery of session files below `[ingest].factory_sessions_dir` (architecture §4.3).
//!
//! `notify` says which files changed as soon as the file system does. FSEvents coalesces and a
//! file created before the watcher started produces no event, so a full listing every
//! `jsonl_poll_ms` backs it: the listing is what guarantees a file is found, the notifier only
//! makes it quicker.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use super::scan::{Found, is_session_file, scan, stat};
use crate::config::IngestConfig;
use crate::ingest::{IngestError, SidecarTag, read_sidecar};
use crate::model::SessionId;

const KB: u64 = 1024;
const SECONDS_PER_HOUR: u64 = 3600;

/// A session file the watcher found, ready to be attached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredSession {
    pub path: PathBuf,
    pub session_id: SessionId,
    /// `callingSessionId` of the first line: set for a child session.
    pub parent: Option<SessionId>,
    /// The first sidecar tag a user wrote, else the session id (FR-L1).
    pub name: String,
    pub modified: SystemTime,
    pub size: u64,
}

/// Metadata keys only the runner sets on the tags it creates.
const RUNNER_METADATA_KEYS: [&str; 3] = ["role", "missionId", "callingSessionId"];

/// What the UI calls a session: the first sidecar tag that is not runner-generated (named in
/// `runner_tags` or carrying runner metadata), else its id.
pub fn session_name(jsonl_path: &Path, id: &SessionId, runner_tags: &[String]) -> String {
    let user_tag = |tag: &SidecarTag| {
        !runner_tags.contains(&tag.name)
            && !RUNNER_METADATA_KEYS.iter().any(|key| {
                tag.metadata
                    .as_ref()
                    .is_some_and(|meta| meta.get(key).is_some())
            })
    };
    read_sidecar(jsonl_path)
        .ok()
        .flatten()
        .and_then(|sidecar| sidecar.tags?.into_iter().find(user_tag))
        .map_or_else(|| id.0.clone(), |tag| tag.name)
}

pub struct SessionWatcher {
    dir: PathBuf,
    poll: Duration,
    window: Duration,
    head_bytes: usize,
    runner_tags: Vec<String>,
    changed: Receiver<PathBuf>,
    /// Paths `wait` took off the channel and `next_discovered` has not looked at yet.
    hints: HashSet<PathBuf>,
    /// Held so the notifier lives as long as the watcher; `None` when it could not start and
    /// the listing alone drives discovery.
    _notifier: Option<RecommendedWatcher>,
    last_scan: Option<Instant>,
    /// Files already returned.
    reported: HashSet<PathBuf>,
    /// Files left out by the startup window, with the length they had: they are returned when
    /// they grow.
    idle: HashMap<PathBuf, u64>,
    scan_failed: bool,
}

impl SessionWatcher {
    /// Starts watching `cfg.factory_sessions_dir` recursively. Fails when the directory
    /// cannot be read.
    pub fn start(cfg: &IngestConfig) -> Result<Self, IngestError> {
        let configured = cfg.resolved_sessions_dir();
        let dir = std::fs::canonicalize(&configured).map_err(|source| IngestError::Io {
            path: configured.clone(),
            source,
        })?;
        let (sender, changed) = mpsc::channel();
        let notifier = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else {
                return;
            };
            if event.kind.is_access() {
                return;
            }
            for path in event.paths.into_iter().filter(|path| is_session_file(path)) {
                let _ = sender.send(path);
            }
        })
        .and_then(|mut notifier| {
            notifier.watch(&dir, RecursiveMode::Recursive)?;
            Ok(notifier)
        })
        .map_err(|err| {
            tracing::warn!(dir = %dir.display(), "file notifications unavailable, polling only: {err}");
        })
        .ok();
        Ok(Self {
            dir,
            poll: Duration::from_millis(cfg.jsonl_poll_ms.max(1)),
            window: Duration::from_secs(cfg.startup_scan_hours.saturating_mul(SECONDS_PER_HOUR)),
            head_bytes: usize::try_from(cfg.tail_chunk_kb.saturating_mul(KB)).unwrap_or(usize::MAX),
            runner_tags: cfg.runner_tags.clone(),
            changed,
            hints: HashSet::new(),
            _notifier: notifier,
            last_scan: None,
            reported: HashSet::new(),
            idle: HashMap::new(),
            scan_failed: false,
        })
    }

    /// Blocks until a session file changes or one poll interval passes, whichever is first.
    pub fn wait(&mut self) {
        self.wait_up_to(self.poll);
    }

    /// `wait`, but for at most `limit` when that is shorter than a poll interval: the caller
    /// has something else due sooner.
    pub fn wait_up_to(&mut self, limit: Duration) {
        if let Ok(path) = self.changed.recv_timeout(limit.min(self.poll)) {
            self.hints.insert(path);
        }
    }

    /// Session files that are new since the last call, or that grew after the startup window
    /// left them out. The first call applies `startup_scan_hours`; later calls list the
    /// directory at most once per poll interval and otherwise look only at the files the
    /// notifier named.
    pub fn next_discovered(&mut self) -> Vec<DiscoveredSession> {
        self.hints.extend(self.changed.try_iter());
        let first = self.last_scan.is_none();
        let due = self.last_scan.is_none_or(|at| at.elapsed() >= self.poll);
        let candidates = if due {
            self.last_scan = Some(Instant::now());
            match scan(&self.dir) {
                Ok(files) => {
                    self.scan_failed = false;
                    self.hints.clear();
                    files
                }
                Err(err) => {
                    if !std::mem::replace(&mut self.scan_failed, true) {
                        tracing::warn!(dir = %self.dir.display(), "cannot list the sessions directory: {err}");
                    }
                    Vec::new()
                }
            }
        } else {
            let mut hinted: Vec<Found> =
                self.hints.drain().filter_map(|path| stat(&path)).collect();
            hinted.sort_by(|a, b| a.path.cmp(&b.path));
            hinted
        };

        let oldest_attached = SystemTime::now()
            .checked_sub(self.window)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut discovered = Vec::new();
        for file in candidates {
            if self.reported.contains(&file.path) {
                continue;
            }
            let grew = self
                .idle
                .get(&file.path)
                .is_some_and(|seen| file.len > *seen);
            if first && file.modified < oldest_attached {
                self.idle.insert(file.path, file.len);
                continue;
            }
            if self.idle.contains_key(&file.path) && !grew {
                continue;
            }
            if let Some(session) = self.describe(&file) {
                self.idle.remove(&file.path);
                self.reported.insert(file.path);
                discovered.push(session);
            }
        }
        discovered
    }

    /// `None` until the first line is complete: a file Droid has just created is empty, and
    /// its id and parent are in that line.
    fn describe(&self, file: &Found) -> Option<DiscoveredSession> {
        let line = first_line(&file.path, self.head_bytes)?;
        let start = serde_json::from_slice::<serde_json::Value>(&line)
            .ok()
            .filter(|start| start["type"] == "session_start");
        let text = |key: &str| {
            start
                .as_ref()
                .and_then(|start| start[key].as_str())
                .map(str::to_owned)
        };
        let session_id = SessionId(text("id").unwrap_or_else(|| {
            file.path
                .file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        }));
        let name = session_name(&file.path, &session_id, &self.runner_tags);
        Some(DiscoveredSession {
            path: file.path.clone(),
            parent: text("callingSessionId").map(SessionId),
            session_id,
            name,
            modified: file.modified,
            size: file.len,
        })
    }
}

/// The first line without its newline; `None` while the line is unfinished and the read did
/// not hit `limit` (a first line longer than a chunk counts as complete).
fn first_line(path: &Path, limit: usize) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut head = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(limit as u64)
        .read_to_end(&mut head)
        .ok()?;
    match head.iter().position(|byte| *byte == b'\n') {
        Some(end) => {
            head.truncate(end);
            Some(head)
        }
        None if head.len() >= limit && limit > 0 => Some(head),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn first_line_waits_for_the_newline_unless_the_chunk_is_full() {
        let dir = tempfile::tempdir().unwrap();
        let unfinished = write(dir.path(), "a.jsonl", r#"{"type":"session_st"#);
        assert_eq!(first_line(&unfinished, 1024), None);
        assert_eq!(first_line(&unfinished, 4).as_deref(), Some(&b"{\"ty"[..]));
        let finished = write(dir.path(), "b.jsonl", "{\"a\":1}\nrest");
        assert_eq!(
            first_line(&finished, 1024).as_deref(),
            Some(&b"{\"a\":1}"[..])
        );
        assert_eq!(first_line(&dir.path().join("none.jsonl"), 1024), None);
    }

    #[test]
    fn a_file_without_session_start_is_named_after_its_stem() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "stray.jsonl", "{\"type\":\"message\"}\n");
        let mut watcher = SessionWatcher::start(&IngestConfig {
            factory_sessions_dir: dir.path().to_string_lossy().into_owned(),
            ..IngestConfig::default()
        })
        .unwrap();
        let found = watcher.next_discovered();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].session_id, SessionId("stray".into()));
        assert_eq!(found[0].name, "stray");
        assert_eq!(found[0].parent, None);
        assert_eq!(found[0].size, std::fs::metadata(path).unwrap().len());
    }

    #[test]
    fn a_session_whose_first_line_is_unfinished_is_found_once_it_is_complete() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "late.jsonl",
            r#"{"type":"session_start","id":"late""#,
        );
        let mut watcher = SessionWatcher::start(&IngestConfig {
            factory_sessions_dir: dir.path().to_string_lossy().into_owned(),
            jsonl_poll_ms: 1,
            ..IngestConfig::default()
        })
        .unwrap();
        assert!(watcher.next_discovered().is_empty());
        std::fs::write(&path, "{\"type\":\"session_start\",\"id\":\"late\"}\n").unwrap();
        std::thread::sleep(Duration::from_millis(5));
        let found = watcher.next_discovered();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].session_id, SessionId("late".into()));
    }

    #[test]
    fn starting_on_a_missing_directory_is_an_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = SessionWatcher::start(&IngestConfig {
            factory_sessions_dir: dir.path().join("absent").to_string_lossy().into_owned(),
            ..IngestConfig::default()
        })
        .err()
        .expect("a missing directory cannot be watched");
        assert!(matches!(err, IngestError::Io { .. }), "{err}");
    }
}
