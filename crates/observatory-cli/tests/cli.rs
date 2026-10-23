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
    assert!(lanes.contains("1 Plan"), "{lanes:?}");
    assert!(lanes.contains("3 Build"), "{lanes:?}");
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
    assert!(first.contains("\t1 Plan\tintent.created\t"), "{first}");
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

/// The golden lane events of the M0 capture: ids and the pointers they were mapped from.
fn golden_all_events() -> Vec<serde_json::Value> {
    let path = m0_fixture().with_extension("events.json");
    serde_json::from_slice(&std::fs::read(path).expect("the golden file exists"))
        .expect("the golden file is a JSON array")
}

fn is_turn_usage(event: &serde_json::Value) -> bool {
    event["kind"].get("turn_usage").is_some()
}

/// The session events only: plain replay never reads the usage log.
fn golden_events() -> Vec<serde_json::Value> {
    let mut events = golden_all_events();
    events.retain(|event| !is_turn_usage(event));
    events
}

fn golden_turns() -> Vec<serde_json::Value> {
    let mut events = golden_all_events();
    events.retain(is_turn_usage);
    events
}

fn usage_log_extract() -> PathBuf {
    m0_fixture().with_file_name("2026-10-22-observatory-v1-m0.usage-log.jsonl")
}

/// The line the golden event points at, cut from the fixture file by hand.
fn golden_line(event: &serde_json::Value) -> Vec<u8> {
    let source = &event["source"];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join(source["path"].as_str().unwrap())).unwrap();
    let offset = usize::try_from(source["byte_offset"].as_u64().unwrap()).unwrap();
    let len = usize::try_from(source["byte_len"].as_u64().unwrap()).unwrap();
    bytes[offset..offset + len].to_vec()
}

fn event_id(event: &serde_json::Value) -> &str {
    event["id"].as_str().unwrap()
}

fn run(home: &Path, config: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_observatory-cli"));
    command.env("HOME", home).env_remove("OBSERVATORY_CONFIG");
    if let Some(config) = config {
        command.env("OBSERVATORY_CONFIG", config);
    }
    command.args(args).output().expect("the binary starts")
}

fn utf8(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn inspect_prints_the_raw_line_byte_identically() {
    let home = tempfile::tempdir().unwrap();
    let store = home.path().join("observatory.sqlite");
    let replayed = run(
        home.path(),
        None,
        &["replay", "--store", utf8(&store), utf8(&m0_fixture())],
    );
    assert!(replayed.status.success(), "{:?}", replayed.status);
    assert_eq!(last_line(&replayed.stdout), "unknown events: 0");

    let events = golden_events();
    let child = events
        .iter()
        .find(|event| event["session_id"] != SESSION)
        .expect("the capture has a child session");
    for event in [&events[0], &events[100], child, events.last().unwrap()] {
        let output = run(
            home.path(),
            None,
            &["inspect", "--store", utf8(&store), event_id(event)],
        );

        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(output.stdout, golden_line(event), "{}", event_id(event));
        assert!(output.stderr.is_empty(), "{:?}", output.stderr);
    }
}

#[test]
fn inspect_names_the_pointer_when_the_file_is_gone() {
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = home.path().join("observatory.sqlite");
    let copy = m0_copy(dir.path(), "0.247.0", "");
    let replayed = run(
        home.path(),
        None,
        &["replay", "--store", utf8(&store), utf8(&copy)],
    );
    assert!(replayed.status.success(), "{:?}", replayed.status);
    let events = golden_events();
    let event = events
        .iter()
        .find(|event| event["session_id"] == SESSION)
        .unwrap();
    let moved = dir.path().join("moved.jsonl");
    std::fs::rename(&copy, &moved).unwrap();

    let output = run(
        home.path(),
        None,
        &["inspect", "--store", utf8(&store), event_id(event)],
    );

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1), "an error, not a panic");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(stderr.contains(utf8(&copy)), "{stderr}");
    let offset = event["source"]["byte_offset"].as_u64().unwrap();
    let len = event["source"]["byte_len"].as_u64().unwrap();
    assert!(
        stderr.contains(&format!("byte_offset={offset}")),
        "{stderr}"
    );
    assert!(stderr.contains(&format!("byte_len={len}")), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn inspect_of_an_unknown_event_id_exits_nonzero_naming_the_id() {
    let home = tempfile::tempdir().unwrap();
    let store = home.path().join("observatory.sqlite");
    run(
        home.path(),
        None,
        &["replay", "--store", utf8(&store), utf8(&m0_fixture())],
    );

    let output = run(
        home.path(),
        None,
        &["inspect", "--store", utf8(&store), "no-such-session:0:0"],
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no-such-session:0:0"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}

#[test]
fn inspect_without_a_store_fails_and_does_not_create_one() {
    let home = tempfile::tempdir().unwrap();

    let output = run(home.path(), None, &["inspect", "any-session:0:0"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("observatory.sqlite"), "{stderr}");
    let left_behind: Vec<_> = std::fs::read_dir(home.path()).unwrap().collect();
    assert!(left_behind.is_empty(), "{left_behind:?}");
}

#[test]
fn replay_and_inspect_use_the_store_path_of_the_config_file_in_effect() {
    let home = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let store = scratch.path().join("configured.sqlite");
    let config = scratch.path().join("config.toml");
    std::fs::write(
        &config,
        format!("[store]\npath = \"{}\"\n", store.display()),
    )
    .unwrap();

    let replayed = run(home.path(), Some(&config), &["replay", utf8(&m0_fixture())]);
    assert!(replayed.status.success(), "{:?}", replayed.status);
    assert!(store.exists(), "the configured store was written");

    let events = golden_events();
    let output = run(
        home.path(),
        Some(&config),
        &["inspect", event_id(&events[0])],
    );
    assert!(output.status.success(), "{:?}", output.status);
    assert_eq!(output.stdout, golden_line(&events[0]));
    let left_behind: Vec<_> = std::fs::read_dir(home.path()).unwrap().collect();
    assert!(left_behind.is_empty(), "{left_behind:?}");
}

/// The lane as `inspect` and the event lines write it: number and name.
fn lane_text(wire: &str) -> &'static str {
    match wire {
        "plan" => "1 Plan",
        "contract" => "2 Contract",
        "build" => "3 Build",
        "scrutiny" => "4 Scrutiny",
        "user_testing" => "5 User testing",
        "integrate" => "6 Integrate",
        "gate" => "7 Gate",
        other => panic!("not a lane: {other}"),
    }
}

const ROLES: [&str; 4] = [
    "orchestrator",
    "worker",
    "scrutiny_validator",
    "user_testing_validator",
];

fn usage_replay(home: &Path, log: &Path, root: &Path) -> Output {
    run(
        home,
        None,
        &["replay", "--usage-log", utf8(log), utf8(root)],
    )
}

#[test]
fn replay_with_a_usage_log_prints_turn_lines_role_totals_and_the_two_closing_lines() {
    let home = tempfile::tempdir().unwrap();

    let output = usage_replay(home.path(), &usage_log_extract(), &m0_fixture());

    assert!(output.status.success(), "{:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    let turns = golden_turns();
    let turn_lines = lines
        .iter()
        .filter(|line| line.split('\t').nth(2) == Some("turn.usage"))
        .count();
    assert_eq!(turn_lines, turns.len());
    assert_eq!(lines[lines.len() - 1], "unknown events: 0");
    assert_eq!(lines[lines.len() - 2], "unknown turns: 0");
    for role in ROLES {
        let of_role: Vec<&serde_json::Value> =
            turns.iter().filter(|event| event["role"] == role).collect();
        let sum = |field: &str| -> u64 {
            of_role
                .iter()
                .map(|event| event["kind"]["turn_usage"][field].as_u64().unwrap())
                .sum()
        };
        let expected = format!(
            "totals\t{role}\t{}\t{}\t{}\t{}",
            of_role.len(),
            sum("input"),
            sum("output"),
            sum("cache_read")
        );
        assert!(lines.contains(&expected.as_str()), "{expected}");
    }
    let last_event = lines
        .iter()
        .rposition(|line| line.split('\t').nth(2) == Some("turn.usage"))
        .unwrap();
    let first_total = lines
        .iter()
        .position(|line| line.starts_with("totals\t"))
        .unwrap();
    assert!(last_event < first_total, "totals follow the event lines");
}

#[test]
fn a_usage_log_with_one_orphan_turn_prints_unknown_turns_one_and_exits_zero() {
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("orphan.log");
    std::fs::write(
        &log,
        "[2026-10-22T10:20:00.000Z] INFO: [Agent] Streaming result | Context: {\"inputTokens\":1,\"cacheReadInputTokens\":2,\"totalInputTokens\":3,\"reasoningTokens\":4,\"outputTokens\":5}\n",
    )
    .unwrap();

    let output = usage_replay(home.path(), &log, &m0_fixture());

    assert!(output.status.success(), "{:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[lines.len() - 2], "unknown turns: 1");
    assert_eq!(lines[lines.len() - 1], "unknown events: 0");
    assert!(!stdout.contains("turn.usage"), "an orphan has no event");
}

#[test]
fn plain_replay_prints_no_turn_usage_line_and_no_totals() {
    let home = tempfile::tempdir().unwrap();

    let output = replay(home.path(), &m0_fixture());

    assert!(output.status.success(), "{:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("turn.usage"));
    assert!(!stdout.contains("totals\t"));
    assert!(!stdout.contains("unknown turns"));
    assert_eq!(last_line(stdout.as_bytes()), "unknown events: 0");
    assert_eq!(stdout.lines().count(), golden_events().len() + 1);
}

#[test]
fn a_missing_usage_log_file_fails_naming_the_path() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("no-such.log");

    let output = usage_replay(home.path(), &missing, &m0_fixture());

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no-such.log"), "{stderr}");
}

#[test]
fn inspect_of_a_turn_prints_the_parsed_row_and_the_store_holds_no_log_text() {
    let home = tempfile::tempdir().unwrap();
    let store = home.path().join("observatory.sqlite");
    let replayed = run(
        home.path(),
        None,
        &[
            "replay",
            "--store",
            utf8(&store),
            "--usage-log",
            utf8(&usage_log_extract()),
            utf8(&m0_fixture()),
        ],
    );
    assert!(replayed.status.success(), "{:?}", replayed.status);

    let turns = golden_turns();
    for turn in [&turns[0], &turns[turns.len() / 2], turns.last().unwrap()] {
        let output = run(
            home.path(),
            None,
            &["inspect", "--store", utf8(&store), event_id(turn)],
        );

        assert!(output.status.success(), "{:?}", output.status);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let usage = &turn["kind"]["turn_usage"];
        let mut expected = vec![
            format!("session: {}", turn["session_id"].as_str().unwrap()),
            format!("role: {}", turn["role"].as_str().unwrap()),
            format!("lane: {}", lane_text(turn["lane"].as_str().unwrap())),
            format!("log_ts: {}", usage["log_ts"].as_str().unwrap()),
            format!("model: {}", usage["model"].as_str().unwrap()),
        ];
        for field in ["input", "output", "cache_read", "reasoning", "total_input"] {
            expected.push(format!("{field}: {}", usage[field]));
        }
        for line in &expected {
            assert!(stdout.lines().any(|l| l == line), "{line} in {stdout}");
        }
        assert_eq!(stdout.lines().count(), expected.len(), "{stdout}");
        assert!(!stdout.contains("Streaming result"), "{stdout}");
        assert!(!stdout.contains("Context"), "{stdout}");
    }

    let bytes = std::fs::read(&store).unwrap();
    assert!(
        !bytes
            .windows(b"Streaming result".len())
            .any(|window| window == b"Streaming result"),
        "the store holds no log line"
    );
}

#[test]
fn inspect_of_a_turn_names_the_pointer_when_the_log_is_gone() {
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = home.path().join("observatory.sqlite");
    let log = dir.path().join("usage.log");
    std::fs::copy(usage_log_extract(), &log).unwrap();
    let replayed = run(
        home.path(),
        None,
        &[
            "replay",
            "--store",
            utf8(&store),
            "--usage-log",
            utf8(&log),
            utf8(&m0_fixture()),
        ],
    );
    assert!(replayed.status.success(), "{:?}", replayed.status);
    std::fs::remove_file(&log).unwrap();

    let turns = golden_turns();
    let output = run(
        home.path(),
        None,
        &["inspect", "--store", utf8(&store), event_id(&turns[0])],
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("usage.log"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}
