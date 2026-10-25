//! `replay` on the spool of a hosted mission maps it through the RPC source and counts
//! unknown events the way it does for a session file.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::Source;
use observatory_core::ingest::rpc::{RpcMapper, RpcSource, Spool, spool_path};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::Event;

const LINES: [&str; 5] = [
    r#"{"jsonrpc":"2.0","type":"notification","method":"droid.session_notification","params":{"notification":{"type":"mcp_status_changed"}}}"#,
    r#"{"jsonrpc":"2.0","type":"response","id":"1","result":{"sessionId":"s-1","settings":{}}}"#,
    r#"{"jsonrpc":"2.0","type":"notification","method":"droid.session_notification","params":{"sessionId":"s-1","notification":{"type":"create_message","message":{"id":"m1","role":"user","content":[{"type":"text","text":"héllo"}],"createdAt":1792820344488}}}}"#,
    r#"{"jsonrpc":"2.0","type":"notification","method":"droid.session_notification","params":{"sessionId":"s-1","notification":{"type":"brand_new_thing"}}}"#,
    r#"{"jsonrpc":"2.0","type":"notification","method":"droid.session_notification","params":{"sessionId":"s-1","notification":{"type":"agent_turn_completed","reason":"cancelled","turnId":"t1"}}}"#,
];

fn spool_in(base: &Path) -> PathBuf {
    let path = spool_path(base, "launch-1");
    let mut spool = Spool::create(&path).unwrap();
    for line in LINES {
        spool.append_line(line.as_bytes()).unwrap();
    }
    path
}

fn expected(path: &Path) -> Vec<Event> {
    let mut source = RpcSource::open(path, &IngestConfig::default()).unwrap();
    let mut mapper = RpcMapper::new("launch-1", MappingVersion::LATEST);
    let mut events: Vec<Event> = source
        .next_batch()
        .unwrap()
        .iter()
        .flat_map(|record| mapper.map(record))
        .collect();
    events.extend(mapper.finish());
    events
}

fn replay(home: &Path, args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_observatory-cli"))
        .env("HOME", home)
        .env_remove("OBSERVATORY_CONFIG")
        .arg("replay")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn replay_of_a_hosted_spool_maps_it_and_counts_unknown_events() {
    let home = tempfile::tempdir().unwrap();
    let spool = spool_in(home.path());
    let events = expected(&spool);
    let unknown = events
        .iter()
        .filter(|event| event.kind.prd_name() == "unknown.event")
        .count();
    assert_eq!(unknown, 1, "the fixture holds one unknown notification");

    let output = replay(home.path(), &[&spool]);

    assert!(output.status.success(), "{:?}", output.status);
    let text = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), events.len() + 1, "{text}");
    assert_eq!(lines.last(), Some(&"unknown events: 1"));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("unknown.event(rpc/brand_new_thing)")),
        "{text}"
    );
}

#[test]
fn replay_of_a_hosted_spool_writes_pointers_that_inspect_reads_back() {
    let home = tempfile::tempdir().unwrap();
    let spool = spool_in(home.path());
    let store = home.path().join("observatory.sqlite");
    let events = expected(&spool);
    let first = events.first().expect("the fixture maps to events");

    let output = Command::new(env!("CARGO_BIN_EXE_observatory-cli"))
        .env("HOME", home.path())
        .env_remove("OBSERVATORY_CONFIG")
        .arg("replay")
        .arg(&spool)
        .arg("--store")
        .arg(&store)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.status);

    let inspected = Command::new(env!("CARGO_BIN_EXE_observatory-cli"))
        .env("HOME", home.path())
        .env_remove("OBSERVATORY_CONFIG")
        .args(["inspect", "--store"])
        .arg(&store)
        .arg(first.id.to_string())
        .output()
        .unwrap();
    assert!(inspected.status.success(), "{:?}", inspected.status);
    assert!(
        std::str::from_utf8(&inspected.stdout)
            .unwrap()
            .starts_with(r#"{"jsonrpc":"2.0""#)
    );
}

#[test]
fn replay_of_a_hosted_spool_reads_plan_revised_from_the_session_file() {
    let home = tempfile::tempdir().unwrap();
    let spool = spool_in(home.path());
    let folder = home.path().join(".factory/sessions/-private-tmp-scratch");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("s-1.jsonl"),
        concat!(
            r#"{"type":"session_start","id":"s-1"}"#,
            "\n",
            r#"{"type":"todo_state","id":"t1","timestamp":"2026-10-24T05:39:04.100Z","todos":{"todos":[{"id":"1","status":"pending","content":"write the plan","priority":"high"}]}}"#,
            "\n",
        ),
    )
    .unwrap();

    let output = replay(home.path(), &[&spool]);

    assert!(output.status.success(), "{:?}", output.status);
    let text = String::from_utf8(output.stdout).unwrap();
    let plans: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("plan.revised"))
        .collect();
    assert_eq!(plans.len(), 1, "{text}");
    assert_eq!(text.lines().last(), Some("unknown events: 1"));
}
