use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::{IngestError, RawRecord, SessionTree, Source, SourceKind};
use crate::config::IngestConfig;
use crate::model::{SessionId, SourcePtr};

const KB: u64 = 1024;

/// Reads session JSONL files by byte offset. Files are opened read-only and never written.
pub struct JsonlSource {
    files: Vec<TailState>,
    chunk_bytes: usize,
}

struct TailState {
    path: PathBuf,
    session_id: SessionId,
    /// Offset of the first byte not yet consumed into a line or `partial`.
    offset: u64,
    /// Bytes after the last newline: a line the writer has not finished.
    partial: Vec<u8>,
}

impl TailState {
    fn io_error(&self, source: std::io::Error) -> IngestError {
        io_error(&self.path, source)
    }

    /// Reads at most `chunk_bytes` past `offset` and returns the lines that ended in it.
    fn read_chunk(&mut self, chunk_bytes: usize) -> Result<Vec<RawRecord>, IngestError> {
        let mut handle = File::open(&self.path).map_err(|source| self.io_error(source))?;
        let length = handle
            .metadata()
            .map_err(|source| self.io_error(source))?
            .len();
        if length < self.offset {
            return Err(IngestError::FileMoved {
                path: self.path.clone(),
            });
        }
        handle
            .seek(SeekFrom::Start(self.offset))
            .map_err(|source| self.io_error(source))?;
        let mut buffer = std::mem::take(&mut self.partial);
        let carried = buffer.len();
        let read = handle
            .take(chunk_bytes as u64)
            .read_to_end(&mut buffer)
            .map_err(|source| self.io_error(source));
        let read = match read {
            Ok(read) => read,
            Err(err) => {
                buffer.truncate(carried);
                self.partial = buffer;
                return Err(err);
            }
        };

        let mut records = Vec::new();
        let mut line_start = self.offset - carried as u64;
        let mut consumed = 0;
        while let Some(newline) = buffer[consumed..].iter().position(|byte| *byte == b'\n') {
            let line = buffer[consumed..consumed + newline].to_vec();
            let next_start = line_start + newline as u64 + 1;
            records.push(self.record(line_start, line));
            line_start = next_start;
            consumed += newline + 1;
        }
        self.offset += read as u64;
        self.partial = buffer.split_off(consumed);
        Ok(records)
    }

    fn record(&self, byte_offset: u64, bytes: Vec<u8>) -> RawRecord {
        RawRecord {
            ptr: SourcePtr::Jsonl {
                path: self.path.clone(),
                byte_offset,
                byte_len: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
            },
            session_id: self.session_id.clone(),
            bytes,
        }
    }
}

fn io_error(path: &Path, source: std::io::Error) -> IngestError {
    IngestError::Io {
        path: path.to_owned(),
        source,
    }
}

impl JsonlSource {
    /// `root` is the orchestrator's session file. Fails when it cannot be read.
    pub fn open(root: &Path, cfg: &IngestConfig) -> Result<Self, IngestError> {
        let chunk_bytes =
            usize::try_from(cfg.tail_chunk_kb.saturating_mul(KB)).unwrap_or(usize::MAX);
        let mut source = Self {
            files: Vec::new(),
            chunk_bytes: chunk_bytes.max(1),
        };
        source.add_file(root)?;
        Ok(source)
    }

    /// Opens the root of a discovered tree, then every child in it.
    pub fn open_tree(tree: &SessionTree, cfg: &IngestConfig) -> Result<Self, IngestError> {
        let mut source = Self::open(&tree.root().path, cfg)?;
        for child in &tree.sessions()[1..] {
            source.add_file(&child.path)?;
        }
        Ok(source)
    }

    /// Starts reading another file (a child or a newly discovered session) from its start.
    pub fn add_file(&mut self, path: &Path) -> Result<SessionId, IngestError> {
        let session_id = session_id_of(path, self.chunk_bytes)?;
        self.files.push(TailState {
            path: path.to_owned(),
            session_id: session_id.clone(),
            offset: 0,
            partial: Vec::new(),
        });
        Ok(session_id)
    }

    /// The session of the file `open` was given.
    pub fn root_session_id(&self) -> &SessionId {
        &self.files[0].session_id
    }

    /// Re-reads the line a pointer names. A moved, shortened or unreadable file is
    /// `PointerLost`, which names the pointer.
    pub fn read_at(path: &Path, byte_offset: u64, byte_len: u32) -> Result<Vec<u8>, IngestError> {
        let lost = |source: std::io::Error| IngestError::PointerLost {
            path: path.to_owned(),
            byte_offset,
            byte_len,
            source,
        };
        let mut file = File::open(path).map_err(lost)?;
        file.seek(SeekFrom::Start(byte_offset)).map_err(lost)?;
        let mut line = vec![0; byte_len as usize];
        file.read_exact(&mut line).map_err(lost)?;
        Ok(line)
    }

    /// Bytes read so far, over all files: how far a capped index has got.
    pub fn consumed(&self) -> u64 {
        self.files.iter().map(|file| file.offset).sum()
    }

    /// Returns each unfinished last line as a record. For a file that will not grow again
    /// (replay): `next_batch` keeps such a line back because a writer may be mid-line.
    pub fn drain_partial(&mut self) -> Vec<RawRecord> {
        self.files
            .iter_mut()
            .filter(|file| !file.partial.is_empty())
            .map(|file| {
                let bytes = std::mem::take(&mut file.partial);
                file.record(file.offset - bytes.len() as u64, bytes)
            })
            .collect()
    }
}

impl Source for JsonlSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Jsonl
    }

    fn next_batch(&mut self) -> Result<Vec<RawRecord>, IngestError> {
        let mut batch = Vec::new();
        for file in &mut self.files {
            batch.extend(file.read_chunk(self.chunk_bytes)?);
        }
        Ok(batch)
    }

    fn at_eof(&self) -> bool {
        self.files
            .iter()
            .all(|file| std::fs::metadata(&file.path).is_ok_and(|meta| meta.len() == file.offset))
    }
}

/// The `id` of a leading `session_start` line, else the file stem.
fn session_id_of(path: &Path, chunk_bytes: usize) -> Result<SessionId, IngestError> {
    let mut head = Vec::new();
    File::open(path)
        .and_then(|file| file.take(chunk_bytes as u64).read_to_end(&mut head))
        .map_err(|source| io_error(path, source))?;
    let first_line = head.split(|byte| *byte == b'\n').next().unwrap_or_default();
    let started = serde_json::from_slice::<serde_json::Value>(first_line)
        .ok()
        .filter(|line| line["type"] == "session_start")
        .and_then(|line| line["id"].as_str().map(str::to_owned));
    let id = started.unwrap_or_else(|| {
        path.file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
    });
    Ok(SessionId(id))
}
