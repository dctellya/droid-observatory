//! Runs `git` in a repository and parses what it prints. Read-only: no command here writes to
//! the repository, and optional locks are off so a poll never blocks the owner's own git.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ingest::IngestError;

/// Separates the fields of a `git log` record; neither occurs in a hash, a date or a subject.
const FIELD: char = '\u{1f}';
const RECORD: char = '\u{1e}';

fn command(repo: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repo)
        .args(["-c", "core.quotepath=off"])
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C");
    command
}

fn failure(repo: &Path, args: &[&str], detail: &str) -> IngestError {
    IngestError::Git {
        repo: repo.to_owned(),
        message: format!("git {}: {}", args.join(" "), detail.trim()),
    }
}

fn run(repo: &Path, args: &[&str]) -> Result<std::process::Output, IngestError> {
    command(repo, args)
        .output()
        .map_err(|source| IngestError::Io {
            path: PathBuf::from("git"),
            source,
        })
}

/// Stdout of a command that must succeed.
pub(super) fn git(repo: &Path, args: &[&str]) -> Result<String, IngestError> {
    let output = run(repo, args)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(failure(
            repo,
            args,
            &String::from_utf8_lossy(&output.stderr),
        ))
    }
}

/// Stdout of a command whose exit code 1 means "nothing there" (`rev-parse --verify -q`).
pub(super) fn git_if_found(repo: &Path, args: &[&str]) -> Result<Option<String>, IngestError> {
    let output = run(repo, args)?;
    match output.status.code() {
        Some(0) => Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )),
        Some(1) => Ok(None),
        _ => Err(failure(
            repo,
            args,
            &String::from_utf8_lossy(&output.stderr),
        )),
    }
}

/// Stdout of a command that fails when it names an object this clone does not hold (a run's head
/// that was never fetched); that is "nothing there", not an error. Spawn failures still are.
pub(super) fn git_or_none(repo: &Path, args: &[&str]) -> Result<Option<String>, IngestError> {
    let output = run(repo, args)?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned()))
}

/// One entry of `git worktree list --porcelain`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RawWorktree {
    pub path: PathBuf,
    pub head: String,
    pub branch: Option<String>,
}

pub(super) fn parse_worktrees(porcelain: &str) -> Vec<RawWorktree> {
    let mut worktrees = Vec::new();
    for block in porcelain.split("\n\n") {
        let mut path = None;
        let mut head = String::new();
        let mut branch = None;
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                path = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("HEAD ") {
                value.clone_into(&mut head);
            } else if let Some(value) = line.strip_prefix("branch ") {
                branch = Some(
                    value
                        .strip_prefix("refs/heads/")
                        .unwrap_or(value)
                        .to_owned(),
                );
            }
        }
        if let Some(path) = path {
            worktrees.push(RawWorktree { path, head, branch });
        }
    }
    worktrees
}

/// One first-parent merge as `git log` prints it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RawMerge {
    pub sha: String,
    pub parents: Vec<String>,
    pub committed_at: String,
    pub subject: String,
}

/// The `--format` that [`parse_merges`] reads.
pub(super) const MERGE_FORMAT: &str = "--format=%H%x1f%P%x1f%cI%x1f%s%x1e";

pub(super) fn parse_merges(log: &str) -> Vec<RawMerge> {
    log.split(RECORD)
        .filter_map(|record| {
            let mut fields = record.trim_start_matches('\n').split(FIELD);
            let sha = fields.next().filter(|sha| !sha.is_empty())?;
            let parents = fields.next()?;
            let committed_at = fields.next()?;
            let subject = fields.next()?;
            Some(RawMerge {
                sha: sha.to_owned(),
                parents: parents.split_whitespace().map(str::to_owned).collect(),
                committed_at: committed_at.to_owned(),
                subject: subject.to_owned(),
            })
        })
        .collect()
}

/// The branch a merge subject names: `Integrate m1: wt/m1` and git's own
/// `Merge branch 'wt/m1' into ...` both carry `wt/m1`. A prefix inside a longer path
/// (`o/wt/m1`) is not a mention.
pub(super) fn named_in_subject<'a>(subject: &'a str, prefix: &str) -> Option<&'a str> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    let mut from = 0;
    while let Some(found) = subject[from..].find(prefix) {
        let start = from + found;
        let after = start + prefix.len();
        let standalone = subject[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !(allowed(c) || c == '/'));
        let name_len = subject[after..]
            .find(|c: char| !allowed(c))
            .unwrap_or(subject.len() - after);
        let name = subject[after..after + name_len].trim_end_matches('.');
        if standalone && !name.is_empty() {
            return Some(name);
        }
        from = after;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktrees_are_read_from_the_porcelain_blocks() {
        let text = "worktree /r\nHEAD aaa\nbranch refs/heads/mission/x\n\n\
                    worktree /w\nHEAD bbb\ndetached\n\nworktree /b\nHEAD ccc\nbare\n";
        let parsed = parse_worktrees(text);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].branch.as_deref(), Some("mission/x"));
        assert_eq!(parsed[1].branch, None);
        assert_eq!(parsed[1].head, "bbb");
        assert_eq!(parsed[2].path, PathBuf::from("/b"));
    }

    #[test]
    fn merge_records_split_on_the_separators_and_skip_fragments() {
        let log = "a1\u{1f}p1 p2\u{1f}2026-10-25T19:20:47+01:00\u{1f}Integrate m3: wt/m3\u{1e}\n\
                   b2\u{1f}p3 p4\u{1f}2026-10-24T20:29:01+02:00\u{1f}two words\u{1e}\n\u{1e}broken";
        let merges = parse_merges(log);
        assert_eq!(merges.len(), 2);
        assert_eq!(merges[0].parents, vec!["p1", "p2"]);
        assert_eq!(merges[1].sha, "b2");
        assert_eq!(merges[1].subject, "two words");
    }

    #[test]
    fn a_subject_names_a_worktree_branch_only_as_a_standalone_word() {
        let named = |s| named_in_subject(s, "wt/");
        assert_eq!(named("Integrate m3: wt/m3"), Some("m3"));
        assert_eq!(named("Merge branch 'wt/m4' into mission/x"), Some("m4"));
        assert_eq!(named("Merge wt/m1.5."), Some("m1.5"));
        assert_eq!(named("Merge pull request #1 from o/wt/m1"), None);
        assert_eq!(named("Integrate m1: board"), None);
        assert_eq!(named("see wt/ later"), None);
        assert_eq!(named("not wt/ then wt/m2"), Some("m2"));
    }
}
