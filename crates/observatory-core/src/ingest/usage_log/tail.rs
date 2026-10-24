//! One log file read by byte offset, with the fragment rule of the session tailer: bytes after
//! the last newline wait until the writer ends the line.

use std::fs::{File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::ingest::IngestError;

pub(super) const CURRENT: &str = "droid-log-single.log";

/// A complete line and the offset of its first byte.
pub(super) struct Line {
    pub offset: u64,
    pub bytes: Vec<u8>,
}

pub(super) struct Tail {
    pub path: PathBuf,
    /// First byte not yet consumed into a line or `partial`.
    pub offset: u64,
    /// Bytes after the last newline.
    pub partial: Vec<u8>,
    /// Identity of the file the offset belongs to, so a renamed or replaced file is told from
    /// one that merely grew.
    pub id: Option<u64>,
}

#[cfg(unix)]
pub(super) fn file_id(meta: &Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(meta.ino())
}

#[cfg(not(unix))]
pub(super) fn file_id(_: &Metadata) -> Option<u64> {
    None
}

impl Tail {
    /// A file read from its start. A file that is not there yet is waited for.
    pub fn from_start(path: PathBuf) -> Self {
        let id = std::fs::metadata(&path)
            .ok()
            .and_then(|meta| file_id(&meta));
        Self {
            path,
            offset: 0,
            partial: Vec::new(),
            id,
        }
    }

    /// A file read from where it ends now.
    pub fn from_end(path: PathBuf) -> Self {
        let meta = std::fs::metadata(&path).ok();
        Self {
            offset: meta.as_ref().map_or(0, Metadata::len),
            id: meta.as_ref().and_then(file_id),
            path,
            partial: Vec::new(),
        }
    }

    /// The tail of a rotated file the reader was in the middle of: it goes on where this one
    /// stopped, and this one starts over on the new file under the same name.
    pub fn take_over(&mut self, rotated: PathBuf) -> Self {
        let taken = Self {
            path: rotated,
            offset: self.offset,
            partial: std::mem::take(&mut self.partial),
            id: self.id,
        };
        self.offset = 0;
        self.id = std::fs::metadata(&self.path)
            .ok()
            .and_then(|meta| file_id(&meta));
        taken
    }

    pub fn at_eof(&self) -> bool {
        std::fs::metadata(&self.path)
            .ok()
            .is_none_or(|meta| meta.len() == self.offset)
    }

    /// Reads at most `chunk_bytes` past the offset and returns the lines that ended in it. A
    /// file that is shorter than the offset, or is another file under the same name, is read
    /// again from 0. A missing file yields nothing.
    pub fn read_chunk(&mut self, chunk_bytes: usize) -> Result<Vec<Line>, IngestError> {
        let io = |path: &Path, source| IngestError::Io {
            path: path.to_owned(),
            source,
        };
        let mut handle = match File::open(&self.path) {
            Ok(handle) => handle,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(io(&self.path, source)),
        };
        let meta = handle.metadata().map_err(|source| io(&self.path, source))?;
        let id = file_id(&meta);
        let replaced = self.id.is_some() && id != self.id;
        if replaced || meta.len() < self.offset {
            self.offset = 0;
            self.partial.clear();
        }
        self.id = id;
        handle
            .seek(SeekFrom::Start(self.offset))
            .map_err(|source| io(&self.path, source))?;
        let mut buffer = std::mem::take(&mut self.partial);
        let carried = buffer.len();
        let read = handle.take(chunk_bytes as u64).read_to_end(&mut buffer);
        let read = match read {
            Ok(read) => read,
            Err(source) => {
                buffer.truncate(carried);
                self.partial = buffer;
                return Err(io(&self.path, source));
            }
        };

        let mut lines = Vec::new();
        let mut line_start = self.offset - carried as u64;
        let mut consumed = 0;
        while let Some(newline) = buffer[consumed..].iter().position(|byte| *byte == b'\n') {
            lines.push(Line {
                offset: line_start,
                bytes: buffer[consumed..consumed + newline].to_vec(),
            });
            line_start += newline as u64 + 1;
            consumed += newline + 1;
        }
        self.offset += read as u64;
        self.partial = buffer.split_off(consumed);
        Ok(lines)
    }

    /// The unfinished last line, for a file that will not grow again (replay).
    pub fn drain_partial(&mut self) -> Option<Line> {
        if self.partial.is_empty() {
            return None;
        }
        let bytes = std::mem::take(&mut self.partial);
        Some(Line {
            offset: self.offset - bytes.len() as u64,
            bytes,
        })
    }
}

/// `(date, n)` of `droid-log-single.log.<YYYY-MM-DD>[.<n>]`: Droid numbers a second rotation of
/// the same day `.1`. `None` for any other name, the current file included.
pub(super) fn dated_key(name: &str) -> Option<(String, u32)> {
    let suffix = name.strip_prefix(CURRENT)?.strip_prefix('.')?;
    let (date, n) = match suffix.split_once('.') {
        Some((date, n)) => (date, n.parse().ok()?),
        None => (suffix, 0),
    };
    let shape = date.len() == 10
        && date.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        });
    shape.then(|| (date.to_owned(), n))
}
