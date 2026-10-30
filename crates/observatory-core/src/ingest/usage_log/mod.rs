//! The usage-log source (architecture §4.7, F-034, D-029): Droid's CLI log
//! `droid-log-single.log` and its dated rotations carry the token counts of every model turn,
//! which the session files and their sidecars do not.
//!
//! v1 heuristics, for Droid 0.247. The Droid log is not a documented format:
//!
//! - A turn is a line `[<RFC 3339>] INFO: [Agent] Streaming result | Context: {json}`. The
//!   Context holds `inputTokens`, `cacheReadInputTokens`, `totalInputTokens`,
//!   `reasoningTokens` and `outputTokens`.
//! - The session and model of a turn come from the nearest preceding line whose Context JSON
//!   has a top-level `sessionId` (for example `metrics_log_droid_chat_client_success_count`)
//!   when that line carries the same second as the turn. A neighbour one second older does not
//!   join.
//! - The turn line's own `tags.sessionId` is a cross-check only. When it differs from the
//!   joined id, `join_mismatches` counts it and the join stands.
//! - A turn with no such neighbour has no session. It is counted (`unknown_turns`) and still
//!   returned, never dropped (rule 18).
//! - Files are read in date order (`droid-log-single.log.<YYYY-MM-DD>`, a second rotation of
//!   the day `.<YYYY-MM-DD>.1` after it), then the current file. A new dated file or a current
//!   file that shrank means a rotation: the rotated file is finished first and the current
//!   file goes on from 0.
//!
//! The source is read-only. Of a line only the parsed fields leave it, and the pointer back to
//! the line; no line is stored or printed.

mod parse;
mod tail;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tail::{CURRENT, Line, Tail, dated_key};

pub use parse::{LogLine, SessionLine, StreamingResult, parse_line};

use super::IngestError;
use crate::config::IngestConfig;
use crate::model::{ModelId, SessionId, SourcePtr, Timestamp};

const KB: u64 = 1024;

/// One model turn of the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageTurn {
    /// `None` for an unknown turn.
    pub session_id: Option<SessionId>,
    pub model: Option<ModelId>,
    pub log_ts: Timestamp,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub reasoning: u64,
    pub total_input: u64,
    /// The Streaming result line.
    pub ptr: SourcePtr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Neighbour {
    second: i64,
    session: SessionId,
    model: Option<ModelId>,
}

pub struct UsageLogSource {
    files: Vec<Tail>,
    /// The log directory, re-listed on every batch; `None` for an explicit list of files.
    dir: Option<PathBuf>,
    /// Dated files already known, whether they are read or were there before `follow`.
    seen: HashSet<PathBuf>,
    chunk_bytes: usize,
    neighbour: Option<Neighbour>,
    unknown_turns: u64,
    join_mismatches: u64,
}

impl UsageLogSource {
    /// Reads `[ingest].factory_logs_dir`: every dated file in date order from its start, then
    /// the current file, then whatever is appended. A directory that does not exist yet is
    /// waited for.
    pub fn open(cfg: &IngestConfig) -> Result<Self, IngestError> {
        let dir = cfg.resolved_logs_dir();
        let mut source = Self::new(cfg, Some(dir.clone()));
        for path in dated_files(&dir) {
            source.seen.insert(path.clone());
            source.files.push(Tail::from_start(path));
        }
        source.files.push(Tail::from_start(dir.join(CURRENT)));
        Ok(source)
    }

    /// Like `open`, for attach: the log is already long when the app starts, so only what is
    /// appended to the current file from now on is read.
    pub fn follow(cfg: &IngestConfig) -> Result<Self, IngestError> {
        let dir = cfg.resolved_logs_dir();
        let mut source = Self::new(cfg, Some(dir.clone()));
        source.seen.extend(dated_files(&dir));
        source.files.push(Tail::from_end(dir.join(CURRENT)));
        Ok(source)
    }

    /// Reads the given files in the given order (replay: `--usage-log <file>...`). Each must
    /// be readable now.
    pub fn open_files(paths: &[PathBuf], cfg: &IngestConfig) -> Result<Self, IngestError> {
        let mut source = Self::new(cfg, None);
        for path in paths {
            std::fs::metadata(path).map_err(|source| IngestError::Io {
                path: path.clone(),
                source,
            })?;
            source.files.push(Tail::from_start(path.clone()));
        }
        Ok(source)
    }

    /// A source with no file, for a log the host already holds in memory (the browser).
    pub fn in_memory(cfg: &IngestConfig) -> Self {
        Self::new(cfg, None)
    }

    /// The turns of a whole log held in memory, its unfinished last line included (a replay's
    /// log will not grow). Call once per file, oldest first: the join state carries over.
    pub fn read_bytes(&mut self, path: &Path, bytes: &[u8]) -> Vec<UsageTurn> {
        let mut turns = Vec::new();
        let mut offset = 0_u64;
        let mut lines = bytes.split(|byte| *byte == b'\n').peekable();
        while let Some(line) = lines.next() {
            let last = lines.peek().is_none();
            if !(last && line.is_empty()) {
                let line_at = Line {
                    offset,
                    bytes: line.to_vec(),
                };
                turns.extend(self.consume(path, &line_at));
            }
            offset += line.len() as u64 + 1;
        }
        turns
    }

    fn new(cfg: &IngestConfig, dir: Option<PathBuf>) -> Self {
        let chunk_bytes =
            usize::try_from(cfg.tail_chunk_kb.saturating_mul(KB)).unwrap_or(usize::MAX);
        Self {
            files: Vec::new(),
            dir,
            seen: HashSet::new(),
            chunk_bytes: chunk_bytes.max(1),
            neighbour: None,
            unknown_turns: 0,
            join_mismatches: 0,
        }
    }

    /// Streaming result lines that joined no session.
    pub fn unknown_turns(&self) -> u64 {
        self.unknown_turns
    }

    /// Turns whose joined id differs from the line's own `tags.sessionId`.
    pub fn join_mismatches(&self) -> u64 {
        self.join_mismatches
    }

    /// The turns that ended in the next chunk. Files are read in order and a later file waits
    /// until an earlier one has been read to its end, so turns come out in log order. An empty
    /// `Vec` means nothing new.
    pub fn next_batch(&mut self) -> Result<Vec<UsageTurn>, IngestError> {
        self.rotate();
        let mut turns = Vec::new();
        for index in 0..self.files.len() {
            let lines = self.files[index].read_chunk(self.chunk_bytes)?;
            let path = self.files[index].path.clone();
            turns.extend(lines.iter().filter_map(|line| self.consume(&path, line)));
            if !self.files[index].at_eof() {
                break;
            }
        }
        Ok(turns)
    }

    /// True when every file was read to its current end.
    pub fn at_eof(&self) -> bool {
        self.files.iter().all(Tail::at_eof)
    }

    /// The unfinished last line of each file, for logs that will not grow again.
    pub fn drain_partial(&mut self) -> Vec<UsageTurn> {
        let mut turns = Vec::new();
        for index in 0..self.files.len() {
            if let Some(line) = self.files[index].drain_partial() {
                let path = self.files[index].path.clone();
                turns.extend(self.consume(&path, &line));
            }
        }
        turns
    }

    /// Everything to the end of the files (replay), the unfinished last lines included.
    pub fn read_all(&mut self) -> Result<Vec<UsageTurn>, IngestError> {
        let mut turns = Vec::new();
        loop {
            turns.extend(self.next_batch()?);
            if self.at_eof() {
                break;
            }
        }
        turns.extend(self.drain_partial());
        Ok(turns)
    }

    /// A dated file that appeared is the current file renamed when it is the file the current
    /// tail has been reading: it is finished before the new current file is read from 0.
    fn rotate(&mut self) {
        let Some(dir) = self.dir.clone() else {
            return;
        };
        let fresh: Vec<PathBuf> = dated_files(&dir)
            .into_iter()
            .filter(|path| self.seen.insert(path.clone()))
            .collect();
        for path in fresh {
            let id = std::fs::metadata(&path)
                .ok()
                .and_then(|meta| tail::file_id(&meta));
            let current = self
                .files
                .last_mut()
                .filter(|tail| tail.path == dir.join(CURRENT));
            let rotated = match current {
                Some(current) if id.is_some() && current.id == id => current.take_over(path),
                _ => Tail::from_start(path),
            };
            let at = self.files.len().saturating_sub(1);
            self.files.insert(at, rotated);
        }
    }

    fn consume(&mut self, path: &Path, line: &Line) -> Option<UsageTurn> {
        match parse_line(&line.bytes) {
            LogLine::Session(found) => {
                self.neighbour = Some(Neighbour {
                    second: found.log_ts.0.unix_timestamp(),
                    session: found.session_id,
                    model: found.model,
                });
                None
            }
            LogLine::Turn(turn) => {
                let joined = self
                    .neighbour
                    .as_ref()
                    .filter(|near| near.second == turn.log_ts.0.unix_timestamp());
                match (joined, &turn.own_session) {
                    (Some(near), Some(own)) if near.session != *own => self.join_mismatches += 1,
                    (None, _) => self.unknown_turns += 1,
                    _ => {}
                }
                Some(UsageTurn {
                    session_id: joined.map(|near| near.session.clone()),
                    model: joined.and_then(|near| near.model.clone()),
                    log_ts: turn.log_ts,
                    input: turn.input,
                    output: turn.output,
                    cache_read: turn.cache_read,
                    reasoning: turn.reasoning,
                    total_input: turn.total_input,
                    ptr: SourcePtr::UsageLog {
                        path: path.to_owned(),
                        byte_offset: line.offset,
                        byte_len: u32::try_from(line.bytes.len()).unwrap_or(u32::MAX),
                    },
                })
            }
            LogLine::Other => None,
        }
    }
}

/// The dated files of `dir`, oldest first.
fn dated_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dated: Vec<((String, u32), PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let key = dated_key(&entry.file_name().to_string_lossy())?;
            Some((key, entry.path()))
        })
        .collect();
    dated.sort();
    dated.into_iter().map(|(_, path)| path).collect()
}
