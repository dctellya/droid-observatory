//! `inspect` on an event of a hosted mission prints the spool line its pointer names.

use std::path::Path;
use std::process::Command;

use observatory_core::config::IngestConfig;
use observatory_core::ingest::Source;
use observatory_core::ingest::rpc::{RpcMapper, RpcSource, Spool, spool_path};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{Event, SourcePtr};
use observatory_core::store::{HostedRun, Store};

const LINES: [&str; 4] = [
    r#"{"jsonrpc":"2.0","type":"notification","method":"droid.session_notification","params":{"notification":{"type":"mcp_status_changed"}}}"#,
    r#"{"jsonrpc":"2.0","type":"response","id":"1","result":{"sessionId":"s-1","settings":{}}}"#,
    r#"{"jsonrpc":"2.0","type":"notification","method":"droid.session_notification","params":{"sessionId":"s-1","notification":{"type":"create_message","message":{"id":"m1","role":"user","content":[{"type":"text","text":"héllo"}],"createdAt":1792820344488}}}}"#,
    r#"{"jsonrpc":"2.0","type":"notification","method":"droid.session_notification","params":{"sessionId":"s-1","notification":{"type":"agent_turn_completed","reason":"cancelled","turnId":"t1"}}}"#,
];

fn host_into(store: &Path, base: &Path) -> (Vec<Event>, std::path::PathBuf) {
    let path = spool_path(base, "launch-1");
    let mut spool = Spool::create(&path).unwrap();
    for line in LINES {
        spool.append_line(line.as_bytes()).unwrap();
    }
    let mut source = RpcSource::open(&path, &IngestConfig::default()).unwrap();
    let mut mapper = RpcMapper::new("launch-1", MappingVersion::LATEST);
    let events: Vec<Event> = source
        .next_batch()
        .unwrap()
        .iter()
        .flat_map(|record| mapper.map(record))
        .collect();
    let mut store = Store::open(store).unwrap();
    store
        .record_hosted(&HostedRun {
            mission_name: "hosted".into(),
            session: mapper.session_id().cloned().unwrap(),
            spool: path.clone(),
            droid_version: None,
            started_at: "2026-10-24T05:39:04.000Z".into(),
        })
        .unwrap();
    store.insert_events(&events).unwrap();
    (events, path)
}

#[test]
fn inspect_prints_the_exact_spool_line_of_a_hosted_event() {
    let home = tempfile::tempdir().unwrap();
    let store = home.path().join("observatory.sqlite");
    let (events, spool) = host_into(&store, home.path());
    let bytes = std::fs::read(&spool).unwrap();
    assert!(!events.is_empty());

    for event in &events {
        let SourcePtr::Rpc {
            byte_offset,
            byte_len,
            ..
        } = &event.source
        else {
            panic!("a hosted event points into the spool: {:?}", event.source);
        };
        let output = Command::new(env!("CARGO_BIN_EXE_observatory-cli"))
            .env("HOME", home.path())
            .env_remove("OBSERVATORY_CONFIG")
            .args(["inspect", "--store"])
            .arg(&store)
            .arg(event.id.to_string())
            .output()
            .unwrap();

        let start = usize::try_from(*byte_offset).unwrap();
        let end = start + *byte_len as usize;
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(output.stdout, bytes[start..end], "{}", event.id);
        assert_eq!(output.stdout.len(), *byte_len as usize);
    }
}
