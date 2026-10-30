//! Helpers shared by the attach tests: a sessions directory stand-in, session files built from
//! the first lines of the M0 capture, and an `Attach` pump.
#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::watch::{Attach, AttachBatch};
use observatory_core::ingest::{IngestError, JsonlSource, Source};
use observatory_core::ingest::{Retraction, SessionTree};
use observatory_core::mapping::{Mapper, MappingVersion, SessionMeta};
use observatory_core::model::{Event, MissionId, SessionId, SourcePtr};

const M0_ROOT: &str = "2026-10-22-observatory-v1-m0.jsonl";

/// Instrumented binaries run the watcher and the other live tests several times slower; the slowest
/// line of a coverage run measured 3.198 s against the 1 s plain bound, so 4 keeps 1 s above it.
pub const INSTRUMENTED_LATENCY_FACTOR: u32 = 4;

/// `cargo llvm-cov` exports `CARGO_LLVM_COV=1` to everything it runs.
pub fn instrumented_from(var: Option<std::ffi::OsString>) -> bool {
    var.is_some_and(|value| !value.is_empty())
}

pub fn scaled_latency_bound(plain: Duration, instrumented: bool) -> Duration {
    if instrumented {
        plain * INSTRUMENTED_LATENCY_FACTOR
    } else {
        plain
    }
}

/// A wall-time bound that holds in a plain run and, widened, under `cargo llvm-cov`.
pub fn latency_bound(plain: Duration) -> Duration {
    scaled_latency_bound(plain, instrumented_from(std::env::var_os("CARGO_LLVM_COV")))
}

pub fn m0_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams")
        .join(M0_ROOT)
}

/// The first `count` lines of the M0 root, each with its newline, as Droid wrote them.
pub fn m0_lines(count: usize) -> Vec<Vec<u8>> {
    let mut reader = BufReader::new(File::open(m0_root()).expect("the M0 capture exists"));
    let mut lines = Vec::new();
    while lines.len() < count {
        let mut line = Vec::new();
        if reader.read_until(b'\n', &mut line).unwrap() == 0 {
            break;
        }
        lines.push(line);
    }
    lines
}

/// Defaults, with the sessions directory and the poll interval of the test. The usage-log
/// directory is one that does not exist, so no test reads the owner's `~/.factory/logs`.
pub fn config(dir: &Path, poll_ms: u64) -> IngestConfig {
    IngestConfig {
        factory_sessions_dir: dir.to_string_lossy().into_owned(),
        factory_logs_dir: dir.join("no-factory-logs").to_string_lossy().into_owned(),
        jsonl_poll_ms: poll_ms,
        ..IngestConfig::default()
    }
}

/// The `session_start` line of a session. A child names the session and the tool call that
/// spawned it.
pub fn session_start(id: &str, caller: Option<(&str, &str)>) -> Vec<u8> {
    let mut line = serde_json::json!({ "type": "session_start", "id": id, "version": 2 });
    if let Some((session, tool_use)) = caller {
        line["callingSessionId"] = session.into();
        line["callingToolUseId"] = tool_use.into();
    }
    let mut bytes = line.to_string().into_bytes();
    bytes.push(b'\n');
    bytes
}

pub fn append(path: &Path, bytes: &[u8]) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}

pub fn byte_offset(event: &Event) -> u64 {
    match &event.source {
        SourcePtr::Jsonl { byte_offset, .. } => *byte_offset,
        other => panic!("not a session line: {other:?}"),
    }
}

/// Everything `Attach::poll` returned so far.
#[derive(Default)]
pub struct Collected {
    pub events: Vec<Event>,
    pub trees: Vec<SessionTree>,
    pub warnings: Vec<IngestError>,
    pub retractions: Vec<(MissionId, Retraction)>,
    pub unwatched: usize,
}

impl Collected {
    pub fn absorb(&mut self, batch: AttachBatch) {
        self.events.extend(batch.events);
        self.trees.extend(batch.trees);
        self.warnings.extend(batch.warnings);
        self.retractions.extend(batch.retractions);
        self.unwatched = batch.unwatched;
    }
}

/// Polls until `done` holds or `limit` passes; the last state is returned either way.
pub fn pump(
    attach: &mut Attach,
    collected: &mut Collected,
    limit: Duration,
    mut done: impl FnMut(&Attach, &Collected) -> bool,
) {
    let deadline = Instant::now() + limit;
    loop {
        attach.wait();
        collected.absorb(attach.poll());
        if done(attach, collected) || Instant::now() >= deadline {
            return;
        }
    }
}

/// What a replay of the finished file maps to: the oracle for what attach must produce.
pub fn replayed_ids(path: &Path, session: &str) -> Vec<String> {
    let cfg = IngestConfig::default();
    let mut source = JsonlSource::open(path, &cfg).unwrap();
    let mut mapper = Mapper::new(
        SessionMeta::root(SessionId(session.to_owned())),
        MappingVersion::LATEST,
    );
    let mut ids = Vec::new();
    while !source.at_eof() {
        for record in source.next_batch().unwrap() {
            ids.extend(mapper.map(&record).into_iter().map(|e| e.id.to_string()));
        }
    }
    ids.extend(mapper.finish().into_iter().map(|e| e.id.to_string()));
    ids
}
