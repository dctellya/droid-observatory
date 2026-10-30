use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::IngestConfig;
use crate::ingest::{JsonlSource, RawRecord, Source};
use crate::model::{SessionId, Timestamp};

/// Droid writes a `todo_state` record into the session file when the plan changes and prints
/// no notification for it (0.247.0), so a hosted run learns its plan revisions from the file.
const PLAN_RECORD: &str = "todo_state";

/// The `todo_state` records of the hosted child's session file, read as the file grows. The
/// records keep their `Jsonl` pointers, so their events have the ids and pointers the attach
/// path gives the same lines.
pub(super) struct SessionFile {
    sessions_dir: PathBuf,
    cfg: IngestConfig,
    tail: Option<JsonlSource>,
    /// Records read and not yet handed out, in file order.
    ready: VecDeque<(Option<Timestamp>, RawRecord)>,
    /// `todo_state` records read from the start of the file, and handed out. A file read again
    /// from its start (Droid rewrites its first line in place) gives the first `handed` back
    /// with new offsets; they are skipped, not mapped twice.
    read: usize,
    handed: usize,
}

impl SessionFile {
    pub(super) fn new(cfg: &IngestConfig) -> Self {
        Self {
            sessions_dir: cfg.resolved_sessions_dir(),
            cfg: cfg.clone(),
            tail: None,
            ready: VecDeque::new(),
            read: 0,
            handed: 0,
        }
    }

    /// The records written up to `until`, in file order. Later ones wait for a later call.
    pub(super) fn due(&mut self, session: &SessionId, until: Timestamp) -> Vec<RawRecord> {
        self.poll(session);
        let mut due = Vec::new();
        while self
            .ready
            .front()
            .is_some_and(|(at, _)| at.is_none_or(|at| at <= until))
        {
            due.extend(self.ready.pop_front().map(|(_, record)| record));
        }
        self.handed += due.len();
        due
    }

    /// Every record not handed out yet: for a process that has exited.
    pub(super) fn rest(&mut self, session: &SessionId) -> Vec<RawRecord> {
        self.poll(session);
        self.handed += self.ready.len();
        self.ready.drain(..).map(|(_, record)| record).collect()
    }

    /// Reads what the file has gained. A file that is not there yet, or cannot be read now, is
    /// tried again at the next call.
    fn poll(&mut self, session: &SessionId) {
        if self.tail.is_none() {
            let Some(path) = locate(&self.sessions_dir, session) else {
                return;
            };
            self.tail = JsonlSource::open(&path, &self.cfg).ok();
        }
        while let Some(tail) = self.tail.as_mut() {
            let batch = match tail.next_batch() {
                Ok(batch) => batch,
                Err(_) => {
                    self.restart();
                    return;
                }
            };
            if tail.take_rewritten().is_some() {
                self.ready.clear();
                self.read = 0;
            }
            if batch.is_empty() {
                return;
            }
            for record in batch {
                let Some(at) = todo_state(&record.bytes) else {
                    continue;
                };
                self.read += 1;
                if self.read > self.handed {
                    self.ready.push_back((at, record));
                }
            }
        }
    }

    fn restart(&mut self) {
        self.tail = None;
        self.ready.clear();
        self.read = 0;
    }
}

/// `<sessions>/<cwd-slug>/<session>.jsonl`: the folder is named after the working directory
/// of the session, which the caller does not know, so every folder is looked in.
fn locate(sessions_dir: &Path, session: &SessionId) -> Option<PathBuf> {
    if session.0.is_empty() || session.0.contains(['/', '\\']) {
        return None;
    }
    let name = format!("{}.jsonl", session.0);
    std::fs::read_dir(sessions_dir)
        .ok()?
        .flatten()
        .map(|folder| folder.path().join(&name))
        .find(|path| path.is_file())
}

/// `Some(time)` when the line is a `todo_state` record; the time is its own, if it has one.
fn todo_state(bytes: &[u8]) -> Option<Option<Timestamp>> {
    if !bytes
        .windows(PLAN_RECORD.len())
        .any(|window| window == PLAN_RECORD.as_bytes())
    {
        return None;
    }
    let line = serde_json::from_slice::<Value>(bytes).ok()?;
    (line.get("type").and_then(Value::as_str) == Some(PLAN_RECORD)).then(|| {
        line.get("timestamp")
            .and_then(Value::as_str)
            .and_then(|text| Timestamp::parse(text).ok())
    })
}
