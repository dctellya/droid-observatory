//! The spool: the hosted process's stdout, appended as it was printed. A hosted stream exists
//! nowhere else, so this file is the raw source `SourcePtr::Rpc` points into.

use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

const SPOOL_DIR: &str = "hosted";
const EXTENSION: &str = "jsonl";

/// `<base>/hosted/<launch-id>.jsonl`; `base` is `~/.droid-observatory`.
pub fn spool_path(base: &Path, launch_id: &str) -> PathBuf {
    base.join(SPOOL_DIR)
        .join(format!("{launch_id}.{EXTENSION}"))
}

pub struct Spool {
    file: File,
    path: PathBuf,
    written: u64,
}

impl Spool {
    /// Creates the file and its directory. A launch id is used once: an existing spool is an
    /// error, never overwritten, because pointers into it would then name other bytes.
    pub fn create(path: &Path) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file,
            path: path.to_owned(),
            written: 0,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes_written(&self) -> u64 {
        self.written
    }

    /// Appends one stdout line (without its newline) and the newline. Written at once, not
    /// buffered, so the tail sees the line within one poll.
    pub fn append_line(&mut self, line: &[u8]) -> io::Result<()> {
        let mut bytes = Vec::with_capacity(line.len() + 1);
        bytes.extend_from_slice(line);
        bytes.push(b'\n');
        self.file.write_all(&bytes)?;
        self.written += bytes.len() as u64;
        Ok(())
    }

    /// Appends everything `reader` yields, line by line and byte for byte, until it ends.
    /// Returns the bytes copied.
    pub fn copy_from(&mut self, mut reader: impl BufRead) -> io::Result<u64> {
        let before = self.written;
        let mut line = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                return Ok(self.written - before);
            }
            self.file.write_all(&line)?;
            self.written += line.len() as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_keeps_a_last_line_without_a_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = spool_path(dir.path(), "l1");
        let mut spool = Spool::create(&path).unwrap();

        let copied = spool.copy_from(&b"{\"a\":1}\r\n{\"b\":2}"[..]).unwrap();

        assert_eq!(copied, 16);
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"a\":1}\r\n{\"b\":2}");
        assert_eq!(spool.path(), path);
    }
}
