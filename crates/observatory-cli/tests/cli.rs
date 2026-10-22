//! `observatory-cli` as a user runs it: arguments in, stdout, stderr and exit code out.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SESSION: &str = "e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c";

fn s_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-s.jsonl")
}

fn cli(home: &Path, args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_observatory-cli"))
        .env("HOME", home)
        .env_remove("OBSERVATORY_CONFIG")
        .args(args)
        .output()
        .expect("the binary starts")
}

fn replay(home: &Path, file: &Path) -> Output {
    cli(home, &[Path::new("replay"), file])
}

fn line_count(file: &Path) -> usize {
    std::fs::read(file)
        .unwrap()
        .split_inclusive(|byte| *byte == b'\n')
        .count()
}

#[test]
fn cli_exits_nonzero_on_missing_file() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("does-not-exist.jsonl");

    let output = replay(home.path(), &missing);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does-not-exist.jsonl"), "{stderr}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("unknown events:"), "{stdout}");
}

#[test]
fn replay_without_a_file_exits_nonzero() {
    let home = tempfile::tempdir().unwrap();

    let output = cli(home.path(), &[Path::new("replay")]);

    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn replay_prints_one_line_per_event_and_ends_with_the_unknown_count() {
    let home = tempfile::tempdir().unwrap();
    let lines = line_count(&s_fixture());

    let output = replay(home.path(), &s_fixture());

    assert!(output.status.success(), "{:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let printed: Vec<&str> = stdout.lines().collect();
    assert_eq!(printed.len(), lines + 1);
    assert_eq!(
        printed.last().copied(),
        Some(format!("unknown events: {lines}").as_str())
    );
    for event in &printed[..lines] {
        let fields: Vec<&str> = event.split('\t').collect();
        assert_eq!(fields.len(), 4, "{event}");
        assert!(fields[0].ends_with('Z'), "{event}");
        assert_eq!(fields[1], "2 Orchestrator", "{event}");
        assert!(fields[2].starts_with("unknown.event("), "{event}");
        assert_eq!(fields[3], SESSION, "{event}");
    }
}

#[test]
fn replay_names_the_raw_type_of_each_unknown_event() {
    let home = tempfile::tempdir().unwrap();

    let output = replay(home.path(), &s_fixture());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let first = stdout.lines().next().unwrap();
    assert!(first.contains("unknown.event(session_start)"), "{first}");
    assert!(stdout.contains("unknown.event(message)"));
}

#[test]
fn replay_writes_nothing_and_leaves_the_fixture_unchanged() {
    let home = tempfile::tempdir().unwrap();
    let before = std::fs::read(s_fixture()).unwrap();

    let output = replay(home.path(), &s_fixture());

    assert!(String::from_utf8_lossy(&output.stdout).contains("unknown events:"));
    assert!(output.status.success());
    assert_eq!(std::fs::read(s_fixture()).unwrap(), before);
    let left_behind: Vec<_> = std::fs::read_dir(home.path()).unwrap().collect();
    assert!(left_behind.is_empty(), "{left_behind:?}");
}
