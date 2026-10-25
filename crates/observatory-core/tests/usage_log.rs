//! The usage-log source (architecture §4.7, F-034, D-029): Droid's CLI log as a second input
//! beside the session files. Lines are synthetic; the owner's log is never opened. The M0
//! extract is the committed fixture `<capture>.usage-log.jsonl`.
#![cfg(feature = "watch")]

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::{append, m0_root, session_start};
use observatory_core::config::IngestConfig;
use observatory_core::ingest::usage_log::{LogLine, UsageLogSource, UsageTurn, parse_line};
use observatory_core::ingest::watch::Attach;
use observatory_core::ingest::{
    JsonlSource, SessionTree, Source, mapping_version_of, read_capture,
};
use observatory_core::mapping::{Mapper, MappingVersion, SessionMeta, TreeMapper};
use observatory_core::model::{
    CycleTemplate, Event, EventKind, LaneId, ModelId, Role, SessionId, SourcePtr,
};
use tempfile::TempDir;

fn lane(id: &str) -> LaneId {
    LaneId::from(id)
}

fn engineering() -> CycleTemplate {
    CycleTemplate::engineering()
}

const CURRENT: &str = "droid-log-single.log";
const SESSION_A: &str = "00000000-0000-4000-8000-000000000001";
const SESSION_B: &str = "00000000-0000-4000-8000-000000000002";

/// A `sessionId` line, as `metrics_log_droid_chat_client_success_count` writes it.
fn session_line(ts: &str, session: &str, model: &str) -> String {
    format!(
        "[{ts}] INFO: [metrics_log_droid_chat_client_success_count] | Context: \
         {{\"modelId\":\"{model}\",\"sessionId\":\"{session}\",\"metric\":\"droid_chat_client_success_count\",\"value\":1}}\n"
    )
}

/// A Streaming result line; `own` is the `tags.sessionId` Droid 0.247 writes into it.
fn turn_line(ts: &str, input: u64, output: u64, own: Option<&str>) -> String {
    let tags = own.map_or_else(String::new, |own| {
        format!(",\"tags\":{{\"sessionId\":\"{own}\",\"modelId\":\"tag-model\"}}")
    });
    format!(
        "[{ts}] INFO: [Agent] Streaming result | Context: {{\"count\":1,\"inputTokens\":{input},\
         \"cacheReadInputTokens\":3400,\"totalInputTokens\":{},\"reasoningTokens\":20,\
         \"outputTokens\":{output},\"reason\":\"tool-calls\"{tags}}}\n",
        input + 3400
    )
}

fn cfg(dir: &Path, poll_ms: u64) -> IngestConfig {
    IngestConfig {
        factory_logs_dir: dir.to_string_lossy().into_owned(),
        jsonl_poll_ms: poll_ms,
        ..IngestConfig::default()
    }
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
}

fn read_turns(path: &Path) -> (Vec<UsageTurn>, UsageLogSource) {
    let mut source = UsageLogSource::open_files(&[path.to_owned()], &IngestConfig::default())
        .expect("the log opens");
    let turns = source.read_all().expect("the log reads");
    (turns, source)
}

#[test]
fn usage_log_line_parses_token_counts() {
    let line = turn_line("2026-10-23T12:00:00.001Z", 12, 57, None);
    let LogLine::Turn(turn) = parse_line(line.trim_end().as_bytes()) else {
        panic!("a Streaming result line is a turn");
    };
    assert_eq!(turn.input, 12);
    assert_eq!(turn.cache_read, 3400);
    assert_eq!(turn.total_input, 3412);
    assert_eq!(turn.reasoning, 20);
    assert_eq!(turn.output, 57);
    assert_eq!(turn.log_ts.to_rfc3339_millis(), "2026-10-23T12:00:00.001Z");
    assert_eq!(turn.own_session, None);

    let tagged = turn_line("2026-10-23T12:00:00.001Z", 1, 2, Some(SESSION_A));
    let LogLine::Turn(turn) = parse_line(tagged.trim_end().as_bytes()) else {
        panic!("a tagged Streaming result line is a turn");
    };
    assert_eq!(turn.own_session, Some(SessionId(SESSION_A.to_owned())));

    let neighbour = session_line("2026-10-23T12:00:00.000Z", SESSION_A, "claude-sonnet-5-5");
    let LogLine::Session(found) = parse_line(neighbour.trim_end().as_bytes()) else {
        panic!("a line with a sessionId in its Context is a session line");
    };
    assert_eq!(found.session_id, SessionId(SESSION_A.to_owned()));
    assert_eq!(found.model, Some(ModelId("claude-sonnet-5-5".to_owned())));

    for other in [
        "[2026-10-23T12:00:00.000Z] INFO: [Agent] something else | Context: {\"x\":1}",
        "[2026-10-23T12:00:00.000Z] INFO: no context at all",
        "stack trace continuation",
        "",
    ] {
        assert_eq!(parse_line(other.as_bytes()), LogLine::Other, "{other}");
    }
}

#[test]
fn turn_joins_to_the_nearest_preceding_session_id_line() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(CURRENT);
    write(
        &path,
        &[
            // Two ids in the same second: the later one is the nearer.
            session_line("2026-10-23T12:00:00.100Z", SESSION_A, "model-a"),
            session_line("2026-10-23T12:00:00.200Z", SESSION_B, "model-b"),
            turn_line("2026-10-23T12:00:00.300Z", 10, 11, Some(SESSION_B)),
            // One second earlier than the turn: no join, whatever came before.
            session_line("2026-10-23T12:00:01.900Z", SESSION_A, "model-a"),
            turn_line("2026-10-23T12:00:02.000Z", 20, 21, None),
            // The line's own tag disagrees with the join: counted and shown, not dropped.
            session_line("2026-10-23T12:00:03.100Z", SESSION_A, "model-a"),
            turn_line("2026-10-23T12:00:03.200Z", 30, 31, Some(SESSION_B)),
        ]
        .concat(),
    );

    let (turns, source) = read_turns(&path);

    assert_eq!(turns.len(), 3, "every Streaming result line yields a turn");
    assert_eq!(turns[0].session_id, Some(SessionId(SESSION_B.to_owned())));
    assert_eq!(turns[0].model, Some(ModelId("model-b".to_owned())));
    assert_eq!((turns[0].input, turns[0].output), (10, 11));
    assert_eq!(turns[1].session_id, None, "the neighbour is a second old");
    assert_eq!(turns[1].model, None);
    assert_eq!(turns[2].session_id, Some(SessionId(SESSION_A.to_owned())));
    assert_eq!(source.join_mismatches(), 1);
    assert_eq!(source.unknown_turns(), 1);
}

#[test]
fn turn_without_session_id_is_counted_as_unknown() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(CURRENT);
    let orphan = turn_line("2026-10-23T12:00:00.001Z", 5, 6, None);
    write(&path, &orphan);

    let (turns, source) = read_turns(&path);

    assert_eq!(turns.len(), 1, "the orphan is in the batch, not dropped");
    assert_eq!(turns[0].session_id, None);
    assert_eq!((turns[0].input, turns[0].output), (5, 6));
    assert_eq!(source.unknown_turns(), 1);
    assert_eq!(source.join_mismatches(), 0);
    let SourcePtr::UsageLog {
        byte_offset,
        byte_len,
        ..
    } = &turns[0].ptr
    else {
        panic!("a turn points into the log: {:?}", turns[0].ptr);
    };
    assert_eq!((*byte_offset, *byte_len as usize), (0, orphan.len() - 1));
}

fn pair(day: &str, session: &str, input: u64) -> String {
    format!(
        "{}{}",
        session_line(&format!("{day}T12:00:00.000Z"), session, "model-a"),
        turn_line(&format!("{day}T12:00:00.500Z"), input, 1, None)
    )
}

#[test]
fn rotated_files_are_read_in_date_order_then_current() {
    let dir = TempDir::new().unwrap();
    // Created out of order on purpose; a same-day second rotation and a foreign file are there.
    write(&dir.path().join(CURRENT), &pair("2026-10-24", SESSION_A, 4));
    write(
        &dir.path().join(format!("{CURRENT}.2026-10-22")),
        &pair("2026-10-22", SESSION_A, 2),
    );
    write(
        &dir.path().join(format!("{CURRENT}.2026-10-23.1")),
        &pair("2026-10-23", SESSION_A, 3),
    );
    write(
        &dir.path().join(format!("{CURRENT}.2026-10-21")),
        &pair("2026-10-21", SESSION_A, 1),
    );
    write(&dir.path().join("console.log"), "not a usage log\n");

    let mut source = UsageLogSource::open(&cfg(dir.path(), 500)).expect("the directory opens");
    let turns = source.read_all().expect("the files read");

    let inputs: Vec<u64> = turns.iter().map(|turn| turn.input).collect();
    assert_eq!(
        inputs,
        [1, 2, 3, 4],
        "dated files by date, then the current one"
    );
    assert!(
        source.next_batch().unwrap().is_empty(),
        "each line exactly once"
    );
    assert_eq!(source.unknown_turns(), 0);
}

fn inputs(turns: &[UsageTurn]) -> Vec<u64> {
    turns.iter().map(|turn| turn.input).collect()
}

fn wait_for_turns(
    source: &mut UsageLogSource,
    poll_ms: u64,
    want: usize,
) -> (Vec<UsageTurn>, Duration) {
    let started = Instant::now();
    let mut got = Vec::new();
    while got.len() < want && started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(poll_ms));
        got.extend(source.next_batch().unwrap());
    }
    (got, started.elapsed())
}

#[test]
fn tail_picks_up_appended_turn_within_poll_interval() {
    let dir = TempDir::new().unwrap();
    let current = dir.path().join(CURRENT);
    write(&current, "");
    let config = cfg(dir.path(), 500);
    let mut source = UsageLogSource::open(&config).expect("the directory opens");
    assert!(source.next_batch().unwrap().is_empty());

    // One synthetic pair for a session of the tree under test.
    let session = SessionId(SESSION_A.to_owned());
    let mut mapper = Mapper::new(SessionMeta::root(session.clone()), MappingVersion::LATEST);
    append(&current, pair("2026-10-23", SESSION_A, 12).as_bytes());
    let appended = Instant::now();
    let (turns, _) = wait_for_turns(&mut source, config.jsonl_poll_ms, 1);
    let elapsed = appended.elapsed();
    println!(
        "turn.usage came out of next_batch {} ms after the append",
        elapsed.as_millis()
    );
    assert_eq!(turns.len(), 1);
    let event = mapper.map_usage(&turns[0]);
    match &event.kind {
        EventKind::TurnUsage {
            input,
            output,
            cache_read,
            ..
        } => assert_eq!((*input, *output, *cache_read), (12, 1, 3400)),
        other => panic!("not a turn.usage event: {other:?}"),
    }
    assert_eq!(event.session_id, Some(session));
    assert!(
        elapsed < Duration::from_millis(config.jsonl_poll_ms) + Duration::from_millis(500),
        "picked up after {elapsed:?}, poll interval is {} ms",
        config.jsonl_poll_ms
    );

    // A line without its newline is a fragment: nothing until the newline arrives.
    let next = pair("2026-10-23", SESSION_A, 13);
    let (head, tail) = next.split_at(next.len() - 1);
    append(&current, head.as_bytes());
    assert!(source.next_batch().unwrap().is_empty());
    append(&current, tail.as_bytes());
    assert_eq!(inputs(&source.next_batch().unwrap()), [13]);

    // The current file is renamed to a dated one and a new current starts: the unread end of
    // the dated file comes first, then the current one from 0.
    append(&current, pair("2026-10-23", SESSION_A, 14).as_bytes());
    std::fs::rename(&current, dir.path().join(format!("{CURRENT}.2026-10-23"))).unwrap();
    write(&current, &pair("2026-10-24", SESSION_A, 15));
    assert_eq!(inputs(&source.next_batch().unwrap()), [14, 15]);

    // The current file shrinks with no dated file: it is read again from 0.
    write(&current, &pair("2026-10-24", SESSION_A, 6));
    assert_eq!(inputs(&source.next_batch().unwrap()), [6]);
    assert!(source.next_batch().unwrap().is_empty());
}

#[test]
fn attach_maps_an_appended_turn_to_its_session_within_a_poll() {
    let sessions = TempDir::new().unwrap();
    let logs = TempDir::new().unwrap();
    let current = logs.path().join(CURRENT);
    write(&current, "");
    append(
        &sessions
            .path()
            .join("-home-user-project")
            .join(format!("{SESSION_A}.jsonl")),
        &session_start(SESSION_A, None),
    );
    let config = IngestConfig {
        factory_sessions_dir: sessions.path().to_string_lossy().into_owned(),
        factory_logs_dir: logs.path().to_string_lossy().into_owned(),
        jsonl_poll_ms: 200,
        ..IngestConfig::default()
    };
    let mut attach = Attach::start(&config).unwrap();
    attach.poll();
    assert_eq!(attach.watched().len(), 1);

    append(&current, pair("2026-10-23", SESSION_A, 12).as_bytes());
    // A turn of a session this attach does not watch is counted, not turned into an event.
    append(&current, pair("2026-10-23", SESSION_B, 99).as_bytes());
    let appended = Instant::now();
    let mut events: Vec<Event> = Vec::new();
    while events.is_empty() && appended.elapsed() < Duration::from_secs(10) {
        attach.wait();
        events.extend(attach.poll().events);
    }

    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].kind,
        EventKind::TurnUsage { input: 12, .. }
    ));
    assert_eq!(events[0].session_id, Some(SessionId(SESSION_A.to_owned())));
    assert_eq!(
        events[0].lane,
        lane("plan"),
        "the root session is in the Plan phase"
    );
    assert_eq!(attach.usage_counters().turns_elsewhere, 1);
}

// ---- The M0 fixture -------------------------------------------------------------------

/// Per role: turns, input, output, cache_read of the committed extract, summed outside the
/// source under test, from each Streaming result line's own `tags.sessionId`.
const M0_TOTALS: [(Role, u64, u64, u64, u64); 4] = [
    (Role::Orchestrator, 307, 77_540, 529_212, 38_166_855),
    (Role::Worker, 635, 75_403, 923_696, 45_998_989),
    (Role::ScrutinyValidator, 64, 154, 88_643, 3_198_549),
    (Role::UserTestingValidator, 109, 165_488, 121_197, 4_839_929),
];

fn m0_usage_log() -> PathBuf {
    let header = read_capture(&m0_root())
        .unwrap()
        .expect("the M0 capture has a header");
    let info = header
        .usage_log
        .expect("the header names the usage-log extract");
    m0_root().with_file_name(info.file)
}

/// The session events of the M0 tree plus the `turn.usage` events of the extract.
fn replay_m0_with_usage_log() -> (Vec<Event>, UsageLogSource) {
    let cfg = IngestConfig::default();
    let path = m0_root();
    let tree = SessionTree::discover(&path, &cfg).unwrap();
    let mut source = JsonlSource::open_tree(&tree, &cfg).unwrap();
    let mut mapper = TreeMapper::new(&tree, mapping_version_of(&path).unwrap());
    let mut events = Vec::new();
    while !source.at_eof() {
        for record in source.next_batch().unwrap() {
            events.extend(mapper.map(&record));
        }
    }
    for record in source.drain_partial() {
        events.extend(mapper.map(&record));
    }
    events.extend(mapper.finish());

    let mut usage = UsageLogSource::open_files(&[m0_usage_log()], &cfg).unwrap();
    for turn in usage.read_all().unwrap() {
        events.extend(mapper.map_usage(&turn));
    }
    (events, usage)
}

#[test]
fn replay_with_usage_log_sums_tokens_per_role_on_m0_fixture() {
    let (events, usage) = replay_m0_with_usage_log();

    assert_eq!(
        usage.unknown_turns(),
        0,
        "every extract line was selected by a joined id"
    );
    assert_eq!(
        usage.join_mismatches(),
        0,
        "the line's own tags agree with the join"
    );
    let mut sums: HashMap<Role, (u64, u64, u64, u64)> = HashMap::new();
    for event in &events {
        if let EventKind::TurnUsage {
            input,
            output,
            cache_read,
            ..
        } = &event.kind
        {
            let sum = sums.entry(event.role).or_default();
            *sum = (sum.0 + 1, sum.1 + input, sum.2 + output, sum.3 + cache_read);
        }
    }
    for (role, turns, input, output, cache_read) in M0_TOTALS {
        assert_eq!(
            sums.get(&role).copied(),
            Some((turns, input, output, cache_read)),
            "{role:?}"
        );
    }
    assert_eq!(sums.values().map(|sum| sum.0).sum::<u64>(), 1115);

    // The lane of a turn is the phase its session is in, as for the session's other role-neutral
    // events: `usage_reported` is emitted last, with the final phase. (An integration session is
    // only known to be one from its first prompt, so its `subagent_spawned` still says Build.)
    let phase: HashMap<_, _> = events
        .iter()
        .filter(|event| event.kind == EventKind::UsageReported)
        .map(|event| (event.session_id.clone(), event.lane))
        .collect();
    for event in events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::TurnUsage { .. }))
    {
        let lane = phase
            .get(&event.session_id)
            .copied()
            .unwrap_or(lane("plan"));
        assert_eq!(event.lane, lane, "{}", event.id);
        assert!(
            matches!(event.source, SourcePtr::UsageLog { .. }),
            "{}",
            event.id
        );
    }

    // The committed golden holds the same turns.
    let golden: Vec<Event> = serde_json::from_str(
        &std::fs::read_to_string(m0_root().with_extension("events.json")).unwrap(),
    )
    .unwrap();
    let in_golden = golden
        .iter()
        .filter(|event| matches!(event.kind, EventKind::TurnUsage { .. }))
        .count();
    assert_eq!(in_golden, 1115, "the golden carries the turn.usage events");
}

#[test]
fn the_m0_extract_holds_only_turns_and_their_session_lines() {
    let text = std::fs::read_to_string(m0_usage_log()).expect("the extract is committed");
    let header = read_capture(&m0_root())
        .unwrap()
        .unwrap()
        .usage_log
        .unwrap();
    let (mut turns, mut sessions) = (0_u64, 0_u64);
    for line in text.lines() {
        match parse_line(line.as_bytes()) {
            LogLine::Turn(_) => turns += 1,
            LogLine::Session(_) => sessions += 1,
            LogLine::Other => panic!("a line that is neither a turn nor a session line"),
        }
    }
    assert_eq!(Some(turns), header.turns);
    assert_eq!(Some(turns + sessions), header.lines);
    assert!(text.ends_with('\n'), "the last line ends in a newline");
}
