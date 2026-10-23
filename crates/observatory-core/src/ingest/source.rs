use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::{SessionId, SourcePtr};

/// One line of a source, as read: not parsed, not copied anywhere else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawRecord {
    pub ptr: SourcePtr,
    pub session_id: SessionId,
    /// The line without its newline.
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Jsonl,
    Rpc,
    Git,
    GitHub,
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("cannot read {}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
    /// The line a stored pointer names cannot be read back: the file moved, was rewritten
    /// or is shorter than the pointer says. The error carries the whole pointer.
    #[error(
        "cannot read the raw line at path={} byte_offset={byte_offset} byte_len={byte_len}: {source}",
        path.display()
    )]
    PointerLost {
        path: PathBuf,
        byte_offset: u64,
        byte_len: u32,
        source: io::Error,
    },
    #[error("{} is shorter than what was already read (moved, rotated or rewritten)", path.display())]
    FileMoved { path: PathBuf },
    #[error("{} is not a valid capture header: {message}", path.display())]
    BadCapture { path: PathBuf, message: String },
    #[error("{} is not a valid session settings file: {message}", path.display())]
    BadSidecar { path: PathBuf, message: String },
}

pub trait Source {
    fn kind(&self) -> SourceKind;

    /// Pulls the next records. An empty `Vec` means nothing new is available right now;
    /// "no data" is never an error.
    fn next_batch(&mut self) -> Result<Vec<RawRecord>, IngestError>;

    /// True when every known file has been read to its current end. Replay is attach
    /// that stopped here (D-008).
    fn at_eof(&self) -> bool;
}
