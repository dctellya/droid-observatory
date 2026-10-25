//! The store row of a live-attached session (VAL-BOARD-040): written at discovery with the
//! display name in `tag`, renamed once when the feature id becomes readable, and equal to what
//! a replay of the same file records.
#![cfg(feature = "watch")]

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::{append, session_start};
use observatory_core::config::Config;
use observatory_core::ingest::watch::LiveIngest;
use observatory_core::model::SessionId;
use observatory_core::store::Store;
use tempfile::TempDir;

const POLL_MS: u64 = 50;
const LIMIT: Duration = Duration::from_secs(10);

fn live_config(dir: &Path) -> Config {
    let mut cfg = Config::default();
    cfg.ingest.factory_sessions_dir = dir.to_string_lossy().into_owned();
    cfg.ingest.factory_logs_dir = dir.join("no-factory-logs").to_string_lossy().into_owned();
    cfg.ingest.jsonl_poll_ms = POLL_MS;
    cfg.ui.event_batch_ms = POLL_MS;
    cfg
}

fn session_path(dir: &Path, id: &str) -> PathBuf {
    dir.join("-home-user-project").join(format!("{id}.jsonl"))
}

fn write_sidecar(dir: &Path, id: &str, tags: &[&str]) {
    let tags: Vec<_> = tags
        .iter()
        .map(|name| serde_json::json!({ "name": name }))
        .collect();
    let text = serde_json::json!({ "model": "claude-sonnet-5-5", "tags": tags }).to_string();
    let path = session_path(dir, id).with_extension("settings.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// The runner's first user message of a worker session (D-036).
fn runner_message(feature: &str) -> Vec<u8> {
    let text =
        format!("<system-reminder>\nYou are a worker assigned to execute feature \"{feature}\".\n");
    let mut line = serde_json::json!({
        "type": "message",
        "id": "runner-1",
        "timestamp": "2026-10-25T10:00:00.000Z",
        "message": { "role": "user", "content": [{ "type": "text", "text": text }] },
    })
    .to_string()
    .into_bytes();
    line.push(b'\n');
    line
}

/// `SELECT tag FROM sessions WHERE id = ?` through a second, read-only connection.
fn stored_name(store_path: &Path, id: &str) -> Option<String> {
    let reader = Store::open_existing(store_path).ok()?;
    let id = SessionId(id.to_owned());
    let mission = reader.mission_of_session(&id).ok()??;
    let sessions = reader.sessions_of(&mission).ok()?;
    sessions.into_iter().find(|s| s.id == id)?.tag
}

/// Steps `live` until the stored name of `id` satisfies `done`; returns it.
fn step_until(
    live: &mut LiveIngest,
    store_path: &Path,
    id: &str,
    done: impl Fn(Option<&str>) -> bool,
) -> Option<String> {
    let started = Instant::now();
    loop {
        let step = live.step();
        assert!(step.errors.is_empty(), "{:?}", step.errors);
        let name = stored_name(store_path, id);
        if done(name.as_deref()) || started.elapsed() >= LIMIT {
            return name;
        }
    }
}

#[test]
fn a_tagged_exec_session_has_a_row_named_by_its_tag_at_discovery() {
    let dir = TempDir::new().unwrap();
    let store_dir = TempDir::new().unwrap();
    let store_path = store_dir.path().join("rows.sqlite");
    write_sidecar(dir.path(), "tagged", &["exec", "scratch-run"]);
    append(
        &session_path(dir.path(), "tagged"),
        &session_start("tagged", None),
    );

    let mut live =
        LiveIngest::start(&live_config(dir.path()), Store::open(&store_path).unwrap()).unwrap();
    let name = step_until(&mut live, &store_path, "tagged", |name| name.is_some());

    assert_eq!(name.as_deref(), Some("scratch-run"));
}

#[test]
fn a_worker_row_goes_from_its_id_to_its_feature_id_once_and_replay_agrees() {
    let dir = TempDir::new().unwrap();
    let store_dir = TempDir::new().unwrap();
    let store_path = store_dir.path().join("rows.sqlite");
    let path = session_path(dir.path(), "worker-1");
    write_sidecar(
        dir.path(),
        "worker-1",
        &["exec", "mission-worker", "mission-session"],
    );
    append(&path, &session_start("worker-1", None));

    let mut live =
        LiveIngest::start(&live_config(dir.path()), Store::open(&store_path).unwrap()).unwrap();
    let first = step_until(&mut live, &store_path, "worker-1", |name| name.is_some());
    assert_eq!(
        first.as_deref(),
        Some("worker-1"),
        "no feature id yet: the id names it"
    );

    append(&path, &runner_message("m3-feature-x"));
    let second = step_until(&mut live, &store_path, "worker-1", |name| {
        name == Some("m3-feature-x")
    });
    assert_eq!(second.as_deref(), Some("m3-feature-x"));

    let replay_dir = TempDir::new().unwrap();
    let replay_path = replay_dir.path().join("replay.sqlite");
    let mut replay = Store::open(&replay_path).unwrap();
    replay
        .replay_file(&path, &live_config(dir.path()).ingest)
        .unwrap();
    assert_eq!(stored_name(&replay_path, "worker-1"), second);
}
