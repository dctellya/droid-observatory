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

fn root_only_replay(home: &Path, file: &Path) -> Output {
    cli(home, &[Path::new("replay"), Path::new("--root-only"), file])
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

    let output = replay(home.path(), &s_fixture());

    assert!(output.status.success(), "{:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let printed: Vec<&str> = stdout.lines().collect();
    let (summary, events) = printed.split_last().unwrap();
    assert!(!events.is_empty());
    let unknown = events
        .iter()
        .filter(|line| line.contains("\tunknown.event"))
        .count();
    assert_eq!(*summary, format!("unknown events: {unknown}"));
    for event in events {
        let fields: Vec<&str> = event.split('\t').collect();
        assert_eq!(fields.len(), 4, "{event}");
        assert!(fields[0].ends_with('Z'), "{event}");
        assert!(
            fields[1].starts_with(|c: char| c.is_ascii_digit()),
            "{event}"
        );
        assert!(!fields[2].is_empty(), "{event}");
        assert!(!fields[3].is_empty(), "{event}");
    }
}

#[test]
fn replay_includes_the_child_sessions_under_their_own_lanes() {
    let home = tempfile::tempdir().unwrap();

    let output = replay(home.path(), &s_fixture());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let sessions: std::collections::BTreeSet<&str> = stdout
        .lines()
        .filter_map(|line| line.split('\t').nth(3))
        .collect();
    assert!(sessions.len() > 1, "{sessions:?}");
    assert!(sessions.contains(SESSION));
    let lanes: std::collections::BTreeSet<&str> = stdout
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .collect();
    assert!(lanes.contains("2 Orchestrator"), "{lanes:?}");
    assert!(lanes.contains("3 Workers"), "{lanes:?}");
}

#[test]
fn replay_root_only_prints_only_events_of_the_root_session() {
    let home = tempfile::tempdir().unwrap();

    let all = replay(home.path(), &s_fixture());
    let root = root_only_replay(home.path(), &s_fixture());

    assert!(root.status.success(), "{:?}", root.status);
    let (all, root) = (
        String::from_utf8(all.stdout).unwrap(),
        String::from_utf8(root.stdout).unwrap(),
    );
    let printed: Vec<&str> = root.lines().collect();
    let (summary, events) = printed.split_last().unwrap();
    assert!(summary.starts_with("unknown events: "), "{summary}");
    assert!(!events.is_empty());
    assert!(events.iter().all(|line| line.ends_with(SESSION)));
    assert!(
        events.len() < all.lines().count() - 1,
        "the children add events to the root's"
    );
}

#[test]
fn replay_names_the_kind_of_each_event_by_its_prd_name() {
    let home = tempfile::tempdir().unwrap();

    let output = replay(home.path(), &s_fixture());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let first = stdout.lines().next().unwrap();
    assert!(first.contains("\t1 Intent\tintent.created\t"), "{first}");
    assert!(stdout.contains("\tsubagent.spawned\t"));
    assert!(stdout.contains("\tfile.touched\t"));
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

const M0_SLUG: &str = "2026-10-22-observatory-v1-m0";

fn m0_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams")
        .join(format!("{M0_SLUG}.jsonl"))
}

/// A scratch copy of the M0 capture: the sidecar and the children are linked, the header is
/// rewritten with `droid_version`, and the root file gets `appended` after its last line.
fn m0_copy(dir: &Path, droid_version: &str, appended: &str) -> PathBuf {
    let original = m0_fixture();
    let streams = original.parent().unwrap();
    let root = dir.join(format!("{M0_SLUG}.jsonl"));

    let mut bytes = std::fs::read(&original).unwrap();
    bytes.extend_from_slice(appended.as_bytes());
    std::fs::write(&root, bytes).unwrap();
    for extension in ["settings.json", "children"] {
        std::os::unix::fs::symlink(
            streams.join(format!("{M0_SLUG}.{extension}")),
            dir.join(format!("{M0_SLUG}.{extension}")),
        )
        .unwrap();
    }
    let header = std::fs::read_to_string(streams.join(format!("{M0_SLUG}.capture.json"))).unwrap();
    assert!(
        header.contains("0.247.0"),
        "the header names the Droid version"
    );
    let edited = header.replace("0.247.0", droid_version);
    std::fs::write(dir.join(format!("{M0_SLUG}.capture.json")), edited).unwrap();
    root
}

fn last_line(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout)
        .lines()
        .last()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn replay_of_the_m0_capture_ends_with_zero_unknown_events() {
    let home = tempfile::tempdir().unwrap();

    let output = replay(home.path(), &m0_fixture());

    assert!(output.status.success(), "{:?}", output.status);
    assert_eq!(last_line(&output.stdout), "unknown events: 0");
    assert!(output.stderr.is_empty(), "no warning for a 0.247.0 capture");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\tsubagent.spawned\t"));
    assert!(stdout.contains("\tcommit.created\t"));
}

#[test]
fn an_unknown_droid_version_prints_one_warning_line_and_replays_on() {
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let copy = m0_copy(dir.path(), "0.300.0", "");

    let output = replay(home.path(), &copy);

    assert!(output.status.success(), "{:?}", output.status);
    let stderr = String::from_utf8(output.stderr).unwrap();
    let warnings = stderr
        .lines()
        .filter(|line| line.contains("0.300.0") && line.contains("v1"))
        .count();
    assert_eq!(warnings, 1, "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert_eq!(last_line(&output.stdout), "unknown events: 0");
}

#[test]
fn unparseable_and_unseen_lines_are_counted_and_listed_not_dropped() {
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let copy = m0_copy(
        dir.path(),
        "0.247.0",
        "not json\n{\"type\":\"future_kind\"}\n",
    );

    let output = replay(home.path(), &copy);

    assert!(output.status.success(), "{:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let unknown: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("unknown.event"))
        .collect();
    assert_eq!(unknown.len(), 2, "{unknown:?}");
    assert!(
        unknown[0].contains("unknown.event(<unparseable>)"),
        "{unknown:?}"
    );
    assert!(
        unknown[1].contains("unknown.event(future_kind)"),
        "{unknown:?}"
    );
    assert_eq!(last_line(stdout.as_bytes()), "unknown events: 2");
}

#[test]
fn root_only_replay_of_the_m0_capture_also_ends_with_zero_unknown_events() {
    let home = tempfile::tempdir().unwrap();

    let output = root_only_replay(home.path(), &m0_fixture());

    assert!(output.status.success(), "{:?}", output.status);
    assert_eq!(last_line(&output.stdout), "unknown events: 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout
            .lines()
            .filter_map(|line| line.split('\t').nth(3))
            .all(|session| session == SESSION)
    );
}
