//! Directory listing of session files for the watcher.

use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const JSONL_EXTENSION: &str = "jsonl";

/// A session file as the file system shows it right now.
#[derive(Clone, Debug)]
pub(super) struct Found {
    pub path: PathBuf,
    pub len: u64,
    pub modified: SystemTime,
}

pub(super) fn is_session_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == JSONL_EXTENSION)
}

/// `None` when the path is gone, is not a file or is not a `.jsonl`.
pub(super) fn stat(path: &Path) -> Option<Found> {
    if !is_session_file(path) {
        return None;
    }
    let meta = std::fs::metadata(path)
        .ok()
        .filter(std::fs::Metadata::is_file)?;
    Some(Found {
        path: path.to_owned(),
        len: meta.len(),
        modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
    })
}

/// Every `.jsonl` below `dir`, whatever the depth, sorted by path. Only an unreadable `dir`
/// itself is an error: a subdirectory that vanishes during the walk is skipped. Symlinked
/// directories are not followed.
pub(super) fn scan(dir: &Path) -> io::Result<Vec<Found>> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_owned()];
    let mut first = true;
    while let Some(current) = pending.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(err) if first => return Err(err),
            Err(_) => continue,
        };
        first = false;
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
            } else if let Some(file) = stat(&path) {
                found.push(file);
            }
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_lists_session_files_at_any_depth_in_path_order() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["b/deep/two.jsonl", "a.jsonl", "b/one.jsonl", "b/notes.txt"] {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x\n").unwrap();
        }
        let found: Vec<_> = scan(dir.path())
            .unwrap()
            .into_iter()
            .map(|file| file.path.strip_prefix(dir.path()).unwrap().to_owned())
            .collect();
        assert_eq!(
            found,
            [
                PathBuf::from("a.jsonl"),
                PathBuf::from("b/deep/two.jsonl"),
                PathBuf::from("b/one.jsonl"),
            ]
        );
    }

    #[test]
    fn scan_fails_for_a_missing_root_and_stat_ignores_other_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan(&dir.path().join("absent")).is_err());
        assert!(stat(&dir.path().join("absent.jsonl")).is_none());
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();
        assert!(stat(&dir.path().join("a.txt")).is_none());
        assert!(stat(dir.path()).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn scan_does_not_follow_symlinked_directories() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("other.jsonl"), "x\n").unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();
        assert!(scan(dir.path()).unwrap().is_empty());
    }
}
