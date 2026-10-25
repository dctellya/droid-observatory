//! A session is named by its feature id when its files carry one, else by its user tag, else by
//! its id (F-038, owner decision 28): one rule, three call sites (watcher, store recorder and
//! memory replay), over small synthetic sessions and the M0 capture.
#![cfg(all(feature = "watch", feature = "store"))]

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common::{config, session_start};
use observatory_core::config::IngestConfig;
use observatory_core::ingest::watch::{Attach, SessionWatcher};
use observatory_core::ingest::{SessionTree, read_sidecar, replay_files, user_tag};
use observatory_core::store::{CaptureInfo, Store};
use rusqlite::Connection;
use serde_json::json;

const M0_SLUG: &str = "2026-10-22-observatory-v1-m0";
const WORKER_TAGS: &str = r#"[{"name":"exec"},{"name":"mission-worker"},
    {"name":"mission-session","metadata":{"role":"worker","missionId":"m-1"}}]"#;

/// The `(session id, feature id)` pairs of the M0 capture, read with
/// `grep -o 'assigned to execute feature \\"[^\\]*\\"'` over the root and every child file.
const M0_FEATURE_IDS: [(&str, &str); 15] = [
    ("2c04bd42-00fc-4c8b-a8bc-00ff515bac46", "m0-integrate-s"),
    ("36d06ed0-bebe-4318-a481-3876dbc9543b", "m0-integrate-s"),
    ("4dd52b9e-97b7-4e01-9020-a956fd5d1949", "m0-fixture-freeze"),
    ("57905379-0ef4-4ec7-aabc-48c7082dcff4", "s-ci-wiring"),
    (
        "6eb88099-278d-4ddb-9f5d-23f1255cdbc5",
        "scrutiny-validator-s",
    ),
    ("72e5184a-e450-432c-b01c-9c3323bc7617", "s-cli-replay"),
    ("73cd8242-3b2c-46ef-b15c-3d2cb7088061", "s-fixture-snapshot"),
    ("82a61cf6-5508-4b01-921a-c275a720e0c8", "m0-integrate-s"),
    (
        "94334a8c-8a4d-4784-99a5-3253d732f9bb",
        "user-testing-validator-s",
    ),
    ("b8c3d386-030e-4602-9781-586b1e928883", "s-core-event-model"),
    ("bfc3f3f7-9075-4ae5-ac74-1bd0a840baf9", "s-app-empty-lanes"),
    (
        "ca58108e-3da3-4ed6-bf3e-0a7073d5833b",
        "s-workspace-scaffold",
    ),
    (
        "cb3408d4-9cad-434f-b3dd-d24ef1e5d1d6",
        "m0-ci-rust-job-linux",
    ),
    (
        "d0304e2e-652e-443e-bd19-4afd804a90ed",
        "user-testing-validator-s",
    ),
    (
        "fef523bb-b669-4ebc-aca2-b6b115cf4b9a",
        "m0-fixture-redaction",
    ),
];

fn streams_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/streams")
}

fn m0_root() -> PathBuf {
    streams_dir().join(format!("{M0_SLUG}.jsonl"))
}

fn message(role: &str, text: &str) -> Vec<u8> {
    let line = json!({
        "type": "message",
        "message": { "role": role, "content": [{ "type": "text", "text": text }] },
    });
    let mut bytes = serde_json::to_vec(&line).unwrap();
    bytes.push(b'\n');
    bytes
}

fn assignment(feature: &str) -> String {
    format!(
        "<system-reminder>\nYou are a worker assigned to execute feature \"{feature}\".\n\n\
         ## Worker Session\n</system-reminder>"
    )
}

/// A root session file in `dir` made of `session_start` and the given message lines, with a
/// sidecar carrying `tags_json`.
fn session(dir: &Path, id: &str, tags_json: &str, messages: &[(&str, String)]) -> PathBuf {
    let path = dir.join(format!("{id}.jsonl"));
    let mut bytes = session_start(id, None);
    for (role, text) in messages {
        bytes.extend(message(role, text));
    }
    std::fs::write(&path, bytes).unwrap();
    std::fs::write(
        path.with_extension("settings.json"),
        format!(r#"{{"tags":{tags_json}}}"#),
    )
    .unwrap();
    path
}

/// What the watcher, `Attach`, the store recorder and the memory replay call the session.
struct Names {
    discovered: String,
    attached: String,
    stored: String,
    replayed: String,
}

fn names_of(path: &Path) -> Names {
    let dir = path.parent().unwrap();
    let cfg = config(dir, 200);
    let discovered = SessionWatcher::start(&cfg)
        .unwrap()
        .next_discovered()
        .remove(0)
        .name;
    let mut attach = Attach::start(&cfg).unwrap();
    attach.poll();
    let attached = attach.watched().remove(0).name;

    let db = dir.join("names.sqlite");
    let mut store = Store::open(&db).unwrap();
    let tree = SessionTree::discover(path, &IngestConfig::default()).unwrap();
    let capture = CaptureInfo::from_session_file(path).unwrap();
    store.record_tree(&tree, &capture, false).unwrap();
    let stored = Connection::open(&db)
        .unwrap()
        .query_row("SELECT tag FROM sessions", [], |row| row.get(0))
        .unwrap();

    let files = BTreeMap::from([
        (path.to_owned(), std::fs::read(path).unwrap()),
        (
            path.with_extension("settings.json"),
            std::fs::read(path.with_extension("settings.json")).unwrap(),
        ),
    ]);
    let replayed = replay_files(files).unwrap().sessions.remove(0).tag.unwrap();
    Names {
        discovered,
        attached,
        stored,
        replayed,
    }
}

fn assert_all(names: &Names, expected: &str) {
    assert_eq!(names.discovered, expected, "watcher discovery");
    assert_eq!(names.attached, expected, "attach watched list");
    assert_eq!(names.stored, expected, "store recorder");
    assert_eq!(names.replayed, expected, "memory replay");
}

#[test]
fn a_worker_session_is_named_by_its_feature_id_when_present() {
    let dir = tempfile::tempdir().unwrap();
    let path = session(
        dir.path(),
        "worker-id",
        WORKER_TAGS,
        &[
            (
                "user",
                "<system-reminder>\nUnified tool catalog\n</system-reminder>".into(),
            ),
            ("user", assignment("m3-session-name-feature-id")),
            ("assistant", "Reading the settings first.".into()),
        ],
    );
    assert_all(&names_of(&path), "m3-session-name-feature-id");

    let with_user_tag = session(
        dir.path(),
        "tagged-worker-id",
        r#"[{"name":"exec"},{"name":"release-hardening"}]"#,
        &[("user", assignment("s-cli-replay"))],
    );
    let tree = SessionTree::discover(&with_user_tag, &IngestConfig::default()).unwrap();
    assert_eq!(
        tree.root().display_name(),
        "s-cli-replay",
        "the feature id wins over a user tag"
    );
}

#[test]
fn a_session_without_a_feature_id_keeps_its_user_tag_name() {
    let dir = tempfile::tempdir().unwrap();
    let tagged = session(
        dir.path(),
        "tagged-id",
        r#"[{"name":"exec"},{"name":"probe-m3"}]"#,
        &[("user", "Say hello.".into())],
    );
    assert_all(&names_of(&tagged), "probe-m3");

    let other = tempfile::tempdir().unwrap();
    let untagged = session(
        other.path(),
        "plain-id",
        WORKER_TAGS,
        &[("user", "Say hello.".into())],
    );
    assert_all(&names_of(&untagged), "plain-id");

    // The phrase in an assistant message, or after the first assistant message, is not an
    // assignment: only the user messages that open the session count.
    let third = tempfile::tempdir().unwrap();
    let quoted = session(
        third.path(),
        "quoting-id",
        r#"[{"name":"exec"},{"name":"probe-m3"}]"#,
        &[
            ("user", "Summarise the mission.".into()),
            ("assistant", assignment("not-a-feature")),
            ("user", assignment("also-not-a-feature")),
        ],
    );
    assert_all(&names_of(&quoted), "probe-m3");
}

#[test]
fn replay_store_and_watcher_agree_on_the_feature_id_name() {
    let tree = SessionTree::discover(&m0_root(), &IngestConfig::default()).unwrap();
    let runner_tags = IngestConfig::default().runner_tags;
    let oracle: BTreeMap<&str, &str> = M0_FEATURE_IDS.into_iter().collect();
    let expected: BTreeMap<String, String> = tree
        .sessions()
        .iter()
        .map(|session| {
            let name = oracle.get(session.id.0.as_str()).map_or_else(
                || {
                    let sidecar = read_sidecar(&session.path).unwrap();
                    let tags = sidecar.as_ref().and_then(|s| s.tags.as_deref());
                    user_tag(tags.unwrap_or_default(), &runner_tags)
                        .map_or_else(|| session.id.0.clone(), str::to_owned)
                },
                |feature| (*feature).to_owned(),
            );
            (session.id.0.clone(), name)
        })
        .collect();
    assert_eq!(expected.len(), 56);
    assert_eq!(
        expected
            .iter()
            .filter(|(id, name)| oracle.contains_key(id.as_str()) && name != id)
            .count(),
        15
    );

    let tree_names: BTreeMap<String, String> = tree
        .sessions()
        .iter()
        .map(|s| (s.id.0.clone(), s.display_name()))
        .collect();
    assert_eq!(tree_names, expected, "session tree");

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("m0.sqlite");
    let mut store = Store::open(&db).unwrap();
    let capture = CaptureInfo::from_session_file(&m0_root()).unwrap();
    store.record_tree(&tree, &capture, false).unwrap();
    let conn = Connection::open(&db).unwrap();
    let mut statement = conn.prepare("SELECT id, tag FROM sessions").unwrap();
    let stored: BTreeMap<String, String> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(stored, expected, "store recorder");

    let mut files = BTreeMap::new();
    let children = streams_dir().join(format!("{M0_SLUG}.children"));
    for entry in std::fs::read_dir(&children).unwrap() {
        let path = entry.unwrap().path();
        files.insert(path.clone(), std::fs::read(&path).unwrap());
    }
    for suffix in ["jsonl", "settings.json", "capture.json"] {
        let path = streams_dir().join(format!("{M0_SLUG}.{suffix}"));
        if path.exists() {
            files.insert(path.clone(), std::fs::read(&path).unwrap());
        }
    }
    let replayed: BTreeMap<String, String> = replay_files(files)
        .unwrap()
        .sessions
        .into_iter()
        .map(|info| (info.id.0, info.tag.unwrap()))
        .collect();
    assert_eq!(replayed, expected, "memory replay");

    let cfg = IngestConfig {
        startup_scan_hours: u64::from(u32::MAX),
        ..config(&streams_dir(), 200)
    };
    let discovered: BTreeMap<String, String> = SessionWatcher::start(&cfg)
        .unwrap()
        .next_discovered()
        .into_iter()
        .filter(|found| expected.contains_key(&found.session_id.0))
        .map(|found| (found.session_id.0, found.name))
        .collect();
    assert_eq!(discovered, expected, "watcher discovery");
}
