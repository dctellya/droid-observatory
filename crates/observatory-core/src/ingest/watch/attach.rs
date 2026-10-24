//! Attach: the watcher, one tail and one `Mapper` per session, driven from a single thread
//! (architecture §4.3). Replay is the same code stopped when `at_eof` holds (D-008).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::watcher::{DiscoveredSession, SessionWatcher, session_name};
use crate::config::IngestConfig;
use crate::ingest::{
    IngestError, JsonlSource, SessionTree, Source, UsageLogSource, mapping_version_of, read_sidecar,
};
use crate::mapping::{Mapper, SessionMeta};
use crate::model::{Event, Role, SessionId};

const MB: u64 = 1024 * 1024;
/// Parent links followed while looking for a session's root, so a cycle in the files ends.
const MAX_ANCESTORS: usize = 32;

/// What one `Attach::poll` produced.
#[derive(Default)]
pub struct AttachBatch {
    pub events: Vec<Event>,
    /// Lines read from the watched files in this poll, whether or not they map to events: a
    /// file that is growing.
    pub records: usize,
    /// Trees that gained a session in this poll. Record them in the store before `events`:
    /// an event names its session and mission.
    pub trees: Vec<SessionTree>,
    pub warnings: Vec<IngestError>,
    /// Sessions found but not watched because of `max_watched_sessions`.
    pub unwatched: usize,
}

/// A session that is being tailed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchedSession {
    pub id: SessionId,
    pub path: PathBuf,
    pub parent: Option<SessionId>,
    /// The first tag, else the id; read from the sidecar when asked, so a tag Droid writes
    /// after the session started shows up.
    pub name: String,
}

struct Tail {
    found: DiscoveredSession,
    source: JsonlSource,
    mapper: Mapper,
    /// Modification time at the last growth seen: what "newest" means for the watch limit.
    modified: SystemTime,
    /// A file above `max_session_file_mb` is indexed up to this many bytes and not followed.
    index_to: Option<u64>,
    finished: bool,
    failed: bool,
    /// The mapper has yielded an event from the file as it was read so far.
    emitted: bool,
}

impl Tail {
    fn active(&self) -> bool {
        !self.finished
            && !self.failed
            && self.index_to.is_none_or(|end| self.source.consumed() < end)
    }

    fn step(&mut self, batch: &mut AttachBatch) {
        if !self.active() {
            return;
        }
        match self.source.next_batch() {
            Ok(records) => {
                if self.source.take_rewritten() {
                    if self.emitted {
                        // Events with offsets of the old bytes are out; this rule has no
                        // way to replace them, so the file is reported rather than doubled.
                        self.failed = true;
                        batch.warnings.push(IngestError::FileMoved {
                            path: self.found.path.clone(),
                        });
                        return;
                    }
                    self.mapper.restart();
                }
                if !records.is_empty() {
                    self.modified = std::fs::metadata(&self.found.path)
                        .and_then(|meta| meta.modified())
                        .unwrap_or(self.modified);
                }
                batch.records += records.len();
                for record in &records {
                    let events = self.mapper.map(record);
                    self.emitted |= !events.is_empty();
                    batch.events.extend(events);
                }
            }
            Err(err) => {
                self.failed = true;
                batch.warnings.push(err);
            }
        }
    }

    fn at_eof(&self) -> bool {
        !self.active() || self.source.at_eof()
    }

    /// The end of the session: a last unfinished line, the usage the sidecar holds now and the
    /// child's end.
    fn finish(&mut self) -> Vec<Event> {
        if self.finished {
            return Vec::new();
        }
        let mut events = Vec::new();
        for record in self.source.drain_partial() {
            events.extend(self.mapper.map(&record));
        }
        if let Ok(Some(sidecar)) = read_sidecar(&self.found.path)
            && let Some(usage) = sidecar.token_usage
        {
            self.mapper.set_usage(Some(usage.to_token_usage()));
        }
        events.extend(self.mapper.finish());
        self.finished = true;
        events
    }
}

/// What the usage-log tail has seen that is not a `turn.usage` event of a watched session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UsageCounters {
    /// Streaming result lines that joined no session (rule 18).
    pub unknown_turns: u64,
    /// Turns whose joined id differs from the line's own `tags.sessionId`.
    pub join_mismatches: u64,
    /// Turns of sessions this attach does not watch (other missions, sessions pushed out by
    /// `max_watched_sessions`).
    pub turns_elsewhere: u64,
}

pub struct Attach {
    watcher: SessionWatcher,
    usage_log: Option<UsageLogSource>,
    turns_elsewhere: u64,
    cfg: IngestConfig,
    tails: Vec<Tail>,
    unwatched: Vec<DiscoveredSession>,
    warned: HashSet<PathBuf>,
    polled: bool,
}

impl Attach {
    /// Starts the watcher. Sessions already growing are attached by the first `poll`.
    pub fn start(cfg: &IngestConfig) -> Result<Self, IngestError> {
        Ok(Self {
            watcher: SessionWatcher::start(cfg)?,
            usage_log: UsageLogSource::follow(cfg).ok(),
            turns_elsewhere: 0,
            cfg: cfg.clone(),
            tails: Vec::new(),
            unwatched: Vec::new(),
            warned: HashSet::new(),
            polled: false,
        })
    }

    /// Blocks until a session file changes or one poll interval passes.
    pub fn wait(&mut self) {
        self.watcher.wait();
    }

    /// `wait`, returning after `limit` when that is sooner than a poll interval.
    pub fn wait_up_to(&mut self, limit: std::time::Duration) {
        self.watcher.wait_up_to(limit);
    }

    /// One step: attaches what the watcher found, reads at most one chunk of every watched
    /// session and maps the lines that ended in it.
    pub fn poll(&mut self) -> AttachBatch {
        self.polled = true;
        let mut batch = AttachBatch::default();
        let found = self.watcher.next_discovered();
        self.admit(found, &mut batch);
        for tail in &mut self.tails {
            tail.step(&mut batch);
        }
        self.poll_usage_log(&mut batch);
        batch.unwatched = self.unwatched.len();
        batch
    }

    /// What the usage-log tail counts besides the events it yielded.
    pub fn usage_counters(&self) -> UsageCounters {
        UsageCounters {
            unknown_turns: self
                .usage_log
                .as_ref()
                .map_or(0, UsageLogSource::unknown_turns),
            join_mismatches: self
                .usage_log
                .as_ref()
                .map_or(0, UsageLogSource::join_mismatches),
            turns_elsewhere: self.turns_elsewhere,
        }
    }

    /// The turns appended to the log since the last poll, each given to the mapper of its
    /// session. A turn is only ever read once, so one of a session that is not watched yet is
    /// counted, not kept.
    fn poll_usage_log(&mut self, batch: &mut AttachBatch) {
        let Some(log) = self.usage_log.as_mut() else {
            return;
        };
        match log.next_batch() {
            Ok(turns) => {
                for turn in &turns {
                    let tail = turn.session_id.as_ref().and_then(|session| {
                        self.tails
                            .iter_mut()
                            .find(|tail| tail.found.session_id == *session)
                    });
                    match tail {
                        Some(tail) => batch.events.push(tail.mapper.map_usage(turn)),
                        None => self.turns_elsewhere += 1,
                    }
                }
            }
            Err(err) => {
                batch.warnings.push(err);
                self.usage_log = None;
            }
        }
    }

    /// The sessions being tailed, in the order they were attached.
    pub fn watched(&self) -> Vec<WatchedSession> {
        self.tails
            .iter()
            .map(|tail| WatchedSession {
                id: tail.found.session_id.clone(),
                path: tail.found.path.clone(),
                parent: tail.found.parent.clone(),
                name: session_name(
                    &tail.found.path,
                    &tail.found.session_id,
                    &self.cfg.runner_tags,
                ),
            })
            .collect()
    }

    /// How many session files were found and are not watched (`max_watched_sessions`).
    pub fn unwatched(&self) -> usize {
        self.unwatched.len()
    }

    /// True once a poll has run and every watched file was read to its current end. Replay
    /// stops here; attach keeps polling.
    pub fn at_eof(&self) -> bool {
        self.polled && self.tails.iter().all(Tail::at_eof)
    }

    /// Ends one session: the events only the end of a file can produce (`usage_reported`,
    /// `subagent_finished`). Call it when the file will not grow again. A session ends once.
    pub fn finish_session(&mut self, id: &SessionId) -> Vec<Event> {
        self.tails
            .iter_mut()
            .find(|tail| tail.found.session_id == *id)
            .map(Tail::finish)
            .unwrap_or_default()
    }

    /// Ends every session not ended yet, in the order they were attached.
    pub fn finish(&mut self) -> Vec<Event> {
        self.tails.iter_mut().flat_map(Tail::finish).collect()
    }

    /// Keeps the newest `max_watched_sessions` of the watched and the newly found sessions;
    /// the rest are counted as unwatched. A watched session pushed out stays out.
    fn admit(&mut self, found: Vec<DiscoveredSession>, batch: &mut AttachBatch) {
        let found: Vec<_> = found
            .into_iter()
            .filter(|new| {
                self.tails
                    .iter()
                    .all(|tail| tail.found.session_id != new.session_id)
            })
            .collect();
        if found.is_empty() {
            return;
        }
        let limit = self.cfg.max_watched_sessions as usize;
        let mut ranked: Vec<(SystemTime, Option<DiscoveredSession>, usize)> = self
            .tails
            .iter()
            .enumerate()
            .map(|(index, tail)| (tail.modified, None, index))
            .chain(found.into_iter().map(|new| (new.modified, Some(new), 0)))
            .collect();
        // Newest first; on a tie the session already watched stays.
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.is_some().cmp(&b.1.is_some())));
        let rest = ranked.split_off(limit.min(ranked.len()));
        let pushed_out: HashSet<usize> = rest
            .iter()
            .filter(|(_, new, _)| new.is_none())
            .map(|(_, _, index)| *index)
            .collect();
        let mut index = 0;
        self.tails.retain(|tail| {
            let keep = !pushed_out.contains(&index);
            index += 1;
            if !keep {
                self.unwatched.push(tail.found.clone());
            }
            keep
        });
        self.unwatched
            .extend(rest.into_iter().filter_map(|(_, new, _)| new));
        let mut admitted: Vec<_> = ranked.into_iter().filter_map(|(_, new, _)| new).collect();
        admitted.sort_by(|a, b| a.path.cmp(&b.path));
        self.attach(admitted, batch);
    }

    fn attach(&mut self, admitted: Vec<DiscoveredSession>, batch: &mut AttachBatch) {
        let mut trees: Vec<(PathBuf, SessionTree)> = Vec::new();
        for found in admitted {
            let root = root_of(&found);
            let index = match trees.iter().position(|(path, _)| *path == root) {
                Some(index) => index,
                None => match SessionTree::discover(&root, &self.cfg) {
                    Ok(tree) => {
                        trees.push((root, tree));
                        trees.len() - 1
                    }
                    Err(err) => {
                        batch.warnings.push(err);
                        continue;
                    }
                },
            };
            let tree = &trees[index].1;
            let meta = tree.get(&found.session_id).map_or_else(
                || SessionMeta {
                    role: Role::Unresolved,
                    ..SessionMeta::root(found.session_id.clone())
                },
                |session| tree.meta(session),
            );
            match self.tail_of(found, meta, batch) {
                Ok(tail) => self.tails.push(tail),
                Err(err) => batch.warnings.push(err),
            }
        }
        batch.trees.extend(trees.into_iter().map(|(_, tree)| tree));
    }

    fn tail_of(
        &mut self,
        found: DiscoveredSession,
        meta: SessionMeta,
        batch: &mut AttachBatch,
    ) -> Result<Tail, IngestError> {
        let source = JsonlSource::open(&found.path, &self.cfg)?;
        let size = std::fs::metadata(&found.path)
            .map_err(|source| IngestError::Io {
                path: found.path.clone(),
                source,
            })?
            .len();
        let index_to = (size > self.cfg.max_session_file_mb.saturating_mul(MB)).then_some(size);
        if index_to.is_some() && self.warned.insert(found.path.clone()) {
            batch.warnings.push(IngestError::TooLarge {
                path: found.path.clone(),
                mb: size / MB,
            });
        }
        let version = mapping_version_of(&found.path)?;
        Ok(Tail {
            modified: found.modified,
            mapper: Mapper::new(meta, version),
            source,
            index_to,
            finished: false,
            failed: false,
            emitted: false,
            found,
        })
    }
}

/// The orchestrator file of the session's tree. Droid names a file after its session id and
/// keeps a mission's sessions in one directory, so a parent is found by name; a session whose
/// parent file is not there is the root of its own tree.
fn root_of(found: &DiscoveredSession) -> PathBuf {
    let dir = found.path.parent().unwrap_or(Path::new("."));
    let mut root = found.path.clone();
    let mut parent = found.parent.clone();
    for _ in 0..MAX_ANCESTORS {
        let Some(id) = parent.take() else { break };
        let candidate = dir.join(format!("{id}.jsonl"));
        if !candidate.is_file() || candidate == root {
            break;
        }
        parent = parent_of(&candidate);
        root = candidate;
    }
    root
}

fn parent_of(path: &Path) -> Option<SessionId> {
    use std::io::{BufRead, BufReader};
    let mut line = String::new();
    BufReader::new(std::fs::File::open(path).ok()?)
        .read_line(&mut line)
        .ok()?;
    let start: serde_json::Value = serde_json::from_str(&line).ok()?;
    start["callingSessionId"]
        .as_str()
        .map(|id| SessionId(id.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_file(dir: &Path, id: &str, parent: Option<&str>) -> DiscoveredSession {
        let start = match parent {
            Some(parent) => {
                format!(r#"{{"type":"session_start","id":"{id}","callingSessionId":"{parent}"}}"#)
            }
            None => format!(r#"{{"type":"session_start","id":"{id}"}}"#),
        };
        let path = dir.join(format!("{id}.jsonl"));
        std::fs::write(&path, format!("{start}\n")).unwrap();
        DiscoveredSession {
            path,
            session_id: SessionId(id.into()),
            parent: parent.map(|parent| SessionId(parent.into())),
            name: id.into(),
            modified: SystemTime::now(),
            size: 0,
        }
    }

    #[test]
    fn the_root_is_found_by_following_parents_through_their_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = session_file(dir.path(), "root", None);
        session_file(dir.path(), "middle", Some("root"));
        let leaf = session_file(dir.path(), "leaf", Some("middle"));
        assert_eq!(root_of(&leaf), root.path);
        assert_eq!(root_of(&root), root.path);
    }

    #[test]
    fn a_session_whose_parent_file_is_missing_is_its_own_root() {
        let dir = tempfile::tempdir().unwrap();
        let orphan = session_file(dir.path(), "orphan", Some("gone"));
        assert_eq!(root_of(&orphan), orphan.path);
    }

    #[test]
    fn a_cycle_of_parents_ends() {
        let dir = tempfile::tempdir().unwrap();
        session_file(dir.path(), "a", Some("b"));
        let b = session_file(dir.path(), "b", Some("a"));
        assert!(root_of(&b).starts_with(dir.path()));
    }
}
