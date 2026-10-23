//! Attach: the directory watcher, the tail by byte offset and the per-session mappers
//! (architecture §4.3, FR-L1, FR-L2, D-008), over a temp directory that stands in for
//! `[ingest].factory_sessions_dir`.
#![cfg(feature = "watch")]

mod common;

use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use common::{Collected, append, byte_offset, config, m0_lines, pump, session_start};
use observatory_core::config::IngestConfig;
use observatory_core::ingest::watch::{Attach, SessionWatcher};
use observatory_core::ingest::{IngestError, JsonlSource, Source};
use observatory_core::mapping::{Mapper, MappingVersion, SessionMeta};
use observatory_core::model::{EventKind, SessionId};
use tempfile::TempDir;

const POLL_MS: u64 = 200;
const LIMIT: Duration = Duration::from_secs(10);

fn id(text: &str) -> SessionId {
    SessionId(text.to_owned())
}

fn session_path(dir: &Path, session: &str) -> std::path::PathBuf {
    dir.join("-home-user-project")
        .join(format!("{session}.jsonl"))
}

/// A session file: its `session_start` and the first `lines` message lines of the M0 root.
fn write_session(dir: &Path, session: &str, lines: usize) -> std::path::PathBuf {
    let path = session_path(dir, session);
    append(&path, &session_start(session, None));
    for line in m0_lines(lines + 1).into_iter().skip(1) {
        append(&path, &line);
    }
    path
}

fn set_age(path: &Path, age: Duration) {
    OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - age)
        .unwrap();
}

/// What a replay of the finished file maps to: the oracle for what attach must produce.
fn replayed_ids(path: &Path, session: &str) -> Vec<String> {
    let cfg = IngestConfig::default();
    let mut source = JsonlSource::open(path, &cfg).unwrap();
    let mut mapper = Mapper::new(SessionMeta::root(id(session)), MappingVersion::LATEST);
    let mut ids = Vec::new();
    while !source.at_eof() {
        for record in source.next_batch().unwrap() {
            ids.extend(mapper.map(&record).into_iter().map(|e| e.id.to_string()));
        }
    }
    ids.extend(mapper.finish().into_iter().map(|e| e.id.to_string()));
    ids
}

fn event_ids(collected: &Collected) -> Vec<String> {
    collected.events.iter().map(|e| e.id.to_string()).collect()
}

#[test]
fn watcher_sees_a_new_session_file_within_poll_interval() {
    let dir = TempDir::new().unwrap();
    let mut watcher = SessionWatcher::start(&config(dir.path(), POLL_MS)).unwrap();
    assert!(watcher.next_discovered().is_empty());

    let path = session_path(dir.path(), "new-session");
    let created = Instant::now();
    append(&path, &session_start("new-session", None));
    let found = loop {
        watcher.wait();
        let found = watcher.next_discovered();
        if !found.is_empty() {
            break found;
        }
        assert!(created.elapsed() < LIMIT, "the file was never discovered");
    };
    let seen_after = created.elapsed();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].session_id, id("new-session"));
    assert_eq!(found[0].path, std::fs::canonicalize(&path).unwrap());
    assert_eq!(found[0].parent, None);
    // One tick to reach the next scan, one for the scheduler.
    assert!(
        seen_after <= Duration::from_millis(2 * POLL_MS),
        "discovered after {seen_after:?}, poll interval is {POLL_MS} ms"
    );
    assert!(watcher.next_discovered().is_empty(), "reported only once");
}

#[test]
fn startup_scan_attaches_files_growing_within_the_window_only() {
    let dir = TempDir::new().unwrap();
    let recent = write_session(dir.path(), "recent", 1);
    let old = write_session(dir.path(), "old", 1);
    set_age(&old, Duration::from_secs(48 * 3600));
    let _ = recent;

    let mut watcher = SessionWatcher::start(&config(dir.path(), POLL_MS)).unwrap();
    let first: Vec<_> = watcher
        .next_discovered()
        .into_iter()
        .map(|found| found.session_id)
        .collect();
    assert_eq!(first, vec![id("recent")]);

    append(&old, &m0_lines(3)[2]);
    let mut later = Vec::new();
    let started = Instant::now();
    while later.is_empty() && started.elapsed() < LIMIT {
        watcher.wait();
        later = watcher.next_discovered();
    }
    assert_eq!(
        later.into_iter().map(|f| f.session_id).collect::<Vec<_>>(),
        vec![id("old")],
        "an old session that grows again is attached"
    );
}

#[test]
fn tail_renders_appended_lines_in_order() {
    let dir = TempDir::new().unwrap();
    let mut attach = Attach::start(&config(dir.path(), POLL_MS)).unwrap();
    let mut collected = Collected::default();
    let path = session_path(dir.path(), "tail");
    let lines = m0_lines(8);

    append(&path, &session_start("tail", None));
    for line in &lines[1..4] {
        append(&path, line);
    }
    pump(&mut attach, &mut collected, LIMIT, |_, c| {
        c.events.iter().any(|e| byte_offset(e) > 0)
    });
    assert!(!collected.events.is_empty());

    for line in &lines[4..8] {
        let (head, rest) = line.split_at(line.len() / 2);
        let before = collected.events.len();
        append(&path, head);
        collected.absorb(attach.poll());
        assert_eq!(
            collected.events.len(),
            before,
            "half a line is held back until its newline arrives"
        );
        append(&path, rest);
        let want = before + 1;
        pump(&mut attach, &mut collected, LIMIT, |_, c| {
            c.events.len() >= want
        });
    }

    let offsets: Vec<u64> = collected.events.iter().map(byte_offset).collect();
    assert!(
        offsets.windows(2).all(|pair| pair[0] <= pair[1]),
        "events keep the order of the file: {offsets:?}"
    );
    assert_eq!(event_ids(&collected), replayed_ids(&path, "tail"));
}

#[test]
fn name_prefers_tag_over_session_id() {
    let dir = TempDir::new().unwrap();
    let tagged = write_session(dir.path(), "tagged-id", 1);
    std::fs::write(
        tagged.with_extension("settings.json"),
        r#"{"tags":[{"name":"release-hardening"},{"name":"read-only"}]}"#,
    )
    .unwrap();
    write_session(dir.path(), "plain-id", 1);

    let mut watcher = SessionWatcher::start(&config(dir.path(), POLL_MS)).unwrap();
    let mut names: Vec<_> = watcher
        .next_discovered()
        .into_iter()
        .map(|found| (found.session_id.0, found.name))
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            ("plain-id".to_owned(), "plain-id".to_owned()),
            ("tagged-id".to_owned(), "release-hardening".to_owned()),
        ]
    );

    let mut attach = Attach::start(&config(dir.path(), POLL_MS)).unwrap();
    attach.poll();
    let shown: BTreeSet<_> = attach.watched().into_iter().map(|s| s.name).collect();
    assert_eq!(
        shown,
        BTreeSet::from(["plain-id".to_owned(), "release-hardening".to_owned()])
    );
}

#[test]
fn max_watched_sessions_keeps_the_newest_and_reports_the_rest() {
    let dir = TempDir::new().unwrap();
    for (name, minutes) in [("s1", 50), ("s2", 40), ("s3", 30), ("s4", 20), ("s5", 10)] {
        let path = write_session(dir.path(), name, 2);
        set_age(&path, Duration::from_secs(minutes * 60));
    }
    let cfg = IngestConfig {
        max_watched_sessions: 3,
        ..config(dir.path(), POLL_MS)
    };
    let mut attach = Attach::start(&cfg).unwrap();
    let mut collected = Collected::default();
    collected.absorb(attach.poll());

    let watched = |attach: &Attach| -> BTreeSet<String> {
        attach.watched().into_iter().map(|s| s.id.0).collect()
    };
    let names = |names: &[&str]| -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    };
    assert_eq!(watched(&attach), names(&["s3", "s4", "s5"]));
    assert_eq!(attach.unwatched(), 2);
    assert_eq!(collected.unwatched, 2);
    assert!(
        collected.events.iter().all(|e| matches!(
            e.session_id.as_ref().map(|s| s.0.as_str()),
            Some("s3" | "s4" | "s5")
        )),
        "no event comes from a session that is not watched"
    );

    write_session(dir.path(), "s6", 2);
    pump(&mut attach, &mut collected, LIMIT, |attach, _| {
        watched(attach).contains("s6")
    });
    assert_eq!(watched(&attach), names(&["s4", "s5", "s6"]));
    assert_eq!(attach.unwatched(), 3);
}

#[test]
fn too_large_file_warns_once_and_is_still_indexed() {
    let dir = TempDir::new().unwrap();
    let cfg = IngestConfig {
        max_session_file_mb: 1,
        tail_chunk_kb: 256,
        ..config(dir.path(), POLL_MS)
    };
    let path = session_path(dir.path(), "big");
    append(&path, &session_start("big", None));
    let mut size = std::fs::metadata(&path).unwrap().len();
    for line in m0_lines(1000).into_iter().skip(1) {
        if size > 1024 * 1024 {
            break;
        }
        size += line.len() as u64;
        append(&path, &line);
    }
    assert!(size > 1024 * 1024, "the file is above the limit");

    let mut attach = Attach::start(&cfg).unwrap();
    let mut collected = Collected::default();
    pump(&mut attach, &mut collected, LIMIT, |attach, _| {
        attach.at_eof()
    });
    collected.absorb(attach.poll());

    let too_large: Vec<_> = collected
        .warnings
        .iter()
        .filter(|warning| matches!(warning, IngestError::TooLarge { .. }))
        .collect();
    assert_eq!(too_large.len(), 1, "one warning: {:?}", collected.warnings);
    assert!(matches!(
        too_large[0],
        IngestError::TooLarge { mb, .. } if *mb >= 1
    ));
    assert_eq!(
        event_ids(&collected),
        replayed_ids(&path, "big"),
        "every line is indexed"
    );

    let indexed = collected.events.len();
    append(&path, &m0_lines(2)[1]);
    for _ in 0..3 {
        attach.wait();
        collected.absorb(attach.poll());
    }
    assert_eq!(
        collected.events.len(),
        indexed,
        "a file over the limit is not followed"
    );
    let warned = collected
        .warnings
        .iter()
        .filter(|warning| matches!(warning, IngestError::TooLarge { .. }))
        .count();
    assert_eq!(warned, 1, "the warning is not repeated");
}

#[test]
fn a_child_session_joins_its_parents_tree_and_finish_reports_usage_and_end() {
    let dir = TempDir::new().unwrap();
    let mut attach = Attach::start(&config(dir.path(), POLL_MS)).unwrap();
    let mut collected = Collected::default();
    write_session(dir.path(), "parent", 3);
    pump(&mut attach, &mut collected, LIMIT, |_, c| {
        !c.trees.is_empty()
    });
    assert_eq!(collected.trees[0].sessions().len(), 1);

    let child = session_path(dir.path(), "child");
    append(&child, &session_start("child", Some(("parent", "toolu-1"))));
    for line in m0_lines(3).into_iter().skip(1) {
        append(&child, &line);
    }
    std::fs::write(
        child.with_extension("settings.json"),
        r#"{"tokenUsage":{"inputTokens":10,"outputTokens":5}}"#,
    )
    .unwrap();
    let trees_before = collected.trees.len();
    pump(&mut attach, &mut collected, LIMIT, |_, c| {
        c.trees.len() > trees_before
    });

    let tree = collected.trees.last().unwrap();
    let joined = tree.get(&id("child")).expect("the child is in the tree");
    assert_eq!(joined.parent, Some(id("parent")));
    assert_eq!(attach.watched().len(), 2);

    let ended = attach.finish_session(&id("child"));
    let kinds: Vec<_> = ended.iter().map(|e| &e.kind).collect();
    assert!(kinds.contains(&&EventKind::UsageReported), "{kinds:?}");
    assert!(kinds.contains(&&EventKind::SubagentFinished), "{kinds:?}");
    let usage = ended
        .iter()
        .find(|e| e.kind == EventKind::UsageReported)
        .and_then(|e| e.usage)
        .expect("usage from the sidecar as it is at the end");
    assert_eq!((usage.input, usage.output), (10, 5));

    collected.absorb(attach.poll());
    assert!(
        attach.finish_session(&id("child")).is_empty(),
        "a session ends once"
    );
}

#[test]
fn replay_is_attach_at_end_of_file() {
    let dir = TempDir::new().unwrap();
    let path = write_session(dir.path(), "done", 40);
    let mut attach = Attach::start(&config(dir.path(), POLL_MS)).unwrap();
    let mut collected = Collected::default();
    assert!(!attach.at_eof(), "nothing has been read yet");
    pump(&mut attach, &mut collected, LIMIT, |attach, _| {
        attach.at_eof()
    });
    assert!(attach.at_eof());
    collected.events.extend(attach.finish());

    assert_eq!(event_ids(&collected), replayed_ids(&path, "done"));
}
