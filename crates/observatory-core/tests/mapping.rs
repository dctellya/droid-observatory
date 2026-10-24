//! `mapping::v1` over the M0 capture (golden file, hard gate, version selection) and over
//! hand-made records (one test per rule of the table in `mapping/v1/kinds.rs`).
//!
//! The golden file `fixtures/streams/<slug>.events.json` changes only through
//! `OBSERVATORY_UPDATE_GOLDEN=1 cargo test -p observatory-core --test mapping`; read the diff
//! and explain it in the commit body.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::usage_log::UsageLogSource;
use observatory_core::ingest::{
    JsonlSource, RawRecord, SessionTree, Source, mapping_version_of, read_capture,
};
use observatory_core::mapping::{
    Mapper, MappingVersion, SessionMeta, TreeMapper, Warning, select_version,
};
use observatory_core::model::{
    Event, EventKind, Lane, MilestoneId, MissionId, ModelId, Overlay, Role, SessionId, SourcePtr,
    TokenUsage,
};
use serde_json::{Value, json};

const SLUG: &str = "2026-10-22-observatory-v1-m0";
const UPDATE_GOLDEN: &str = "OBSERVATORY_UPDATE_GOLDEN";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root exists")
}

fn fixture(extension: &str) -> PathBuf {
    repo_root()
        .join("fixtures/streams")
        .join(format!("{SLUG}.{extension}"))
}

/// The tree replay of the M0 capture as the CLI runs it, sorted by
/// `(at, session_id, byte_offset, sub)` with paths relative to the repository root.
fn map_m0() -> (usize, Vec<Event>) {
    map_m0_with(false)
}

/// `map_m0` plus the `turn.usage` events of the extract the capture header names.
fn map_m0_with_usage_log() -> (usize, Vec<Event>) {
    map_m0_with(true)
}

fn map_m0_with(usage_log: bool) -> (usize, Vec<Event>) {
    let path = fixture("jsonl");
    let cfg = IngestConfig::default();
    let tree = SessionTree::discover(&path, &cfg).expect("the M0 capture opens");
    let mut source = JsonlSource::open_tree(&tree, &cfg).expect("every session file opens");
    let version = mapping_version_of(&path).expect("capture header is readable");
    let mut mapper = TreeMapper::new(&tree, version);

    let mut records = 0;
    let mut events = Vec::new();
    for _ in 0..100_000 {
        let mut batch = source.next_batch().expect("batch reads");
        let at_eof = source.at_eof();
        if at_eof {
            batch.extend(source.drain_partial());
        }
        records += batch.len();
        for record in &batch {
            events.extend(mapper.map(record));
        }
        if at_eof {
            break;
        }
    }
    events.extend(mapper.finish());
    if usage_log {
        let header = read_capture(&path)
            .expect("capture header is readable")
            .expect("the M0 capture has a header");
        let info = header
            .usage_log
            .expect("the header names the usage-log extract");
        let extract = path.with_file_name(info.file);
        let mut usage = UsageLogSource::open_files(&[extract], &cfg).expect("the extract opens");
        for turn in usage.read_all().expect("the extract reads") {
            events.extend(mapper.map_usage(&turn));
        }
    }

    let root = repo_root();
    for event in &mut events {
        if let SourcePtr::Jsonl { path, .. } | SourcePtr::UsageLog { path, .. } = &mut event.source
        {
            *path = path
                .canonicalize()
                .expect("the pointer names a fixture file")
                .strip_prefix(&root)
                .expect("the fixture lives in the repository")
                .to_owned();
        }
    }
    events.sort_by_key(sort_key);
    (records, events)
}

fn sort_key(event: &Event) -> (observatory_core::model::Timestamp, String, u64, u32) {
    let (offset, sub) = offset_and_sub(event);
    (
        event.at,
        event
            .session_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        offset,
        sub,
    )
}

fn offset_and_sub(event: &Event) -> (u64, u32) {
    let (SourcePtr::Jsonl { byte_offset, .. } | SourcePtr::UsageLog { byte_offset, .. }) =
        &event.source
    else {
        panic!("the M0 capture holds only file pointers");
    };
    let sub = event
        .id
        .0
        .rsplit(':')
        .next()
        .and_then(|sub| sub.parse().ok())
        .expect("an event id ends in its sub index");
    (*byte_offset, sub)
}

fn golden_path() -> PathBuf {
    fixture("events.json")
}

fn write_golden(events: &[Event]) {
    let mut text = String::from("[\n");
    for (index, event) in events.iter().enumerate() {
        text.push_str("  ");
        text.push_str(&serde_json::to_string(event).expect("an event serializes"));
        text.push_str(if index + 1 == events.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    text.push_str("]\n");
    std::fs::write(golden_path(), text).expect("the golden file is writable");
}

#[test]
fn mapping_v1_matches_golden_lane_events() {
    let (_, events) = map_m0_with_usage_log();
    if std::env::var_os(UPDATE_GOLDEN).is_some() {
        write_golden(&events);
    }

    let text = std::fs::read_to_string(golden_path()).expect("the golden file is committed");
    let golden: Vec<Event> =
        serde_json::from_str(&text).expect("the golden file is a list of events");

    assert_eq!(events.len(), golden.len(), "event count");
    for (index, (got, want)) in events.iter().zip(&golden).enumerate() {
        assert_eq!(got, want, "event {index} ({})", want.id);
    }
    assert!(
        golden
            .iter()
            .all(|event| event.mapping_version == MappingVersion::V1
                && !matches!(event.kind, EventKind::Unknown { .. })),
        "the golden file holds v1 events and no Unknown"
    );
}

/// PRD §7 kinds the M0 capture produces. A kind added or lost here changes the decision
/// file's "defined, not observed" list.
const OBSERVED_IN_M0: &[&str] = &[
    "intent.created",
    "intent.amended",
    "question.asked",
    "intent.answered",
    "plan.created",
    "feature.defined",
    "milestone.defined",
    "task.assigned",
    "task.skipped",
    "plan.revised",
    "escalation.raised",
    "context.compacted",
    "subagent.spawned",
    "task.started",
    "file.touched",
    "tests.run",
    "commit.created",
    "task.done",
    "subagent.finished",
    "tool.error",
    "hook.blocked",
    "usage.reported",
    "model.changed",
    "finding.non_blocking",
    "scrutiny.passed",
    "qa.scenario.passed",
    "qa.evidence",
    "gate_command.run",
    "gate_command.passed",
    "gate_command.failed",
    "turn.usage",
];

#[test]
fn every_prd_kind_present_in_capture_is_mapped() {
    let (_, events) = map_m0_with_usage_log();

    let observed: BTreeSet<String> = events
        .iter()
        .map(|event| {
            assert!(
                !matches!(event.kind, EventKind::Unknown { .. }),
                "{} is unmapped",
                event.id
            );
            event.kind.prd_name().into_owned()
        })
        .collect();
    let expected: BTreeSet<String> = OBSERVED_IN_M0
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    let defined: BTreeSet<String> = EventKind::KNOWN
        .iter()
        .map(|kind| kind.prd_name().into_owned())
        .collect();

    assert!(observed.is_subset(&defined));
    assert_eq!(
        observed,
        expected,
        "defined, not observed: {:?}",
        defined.difference(&observed).collect::<Vec<_>>()
    );
}

#[test]
fn unknown_count_is_zero_on_m0_capture() {
    let (records, events) = map_m0();

    let unknown: Vec<_> = events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
        .map(|event| event.id.to_string())
        .collect();

    assert_eq!(unknown, Vec::<String>::new());
    assert_eq!(
        records, 8735,
        "the tree replay reads every line of 56 sessions"
    );
    assert!(events.len() > 1000);
}

#[test]
fn m0_events_fall_into_the_seven_phases_with_human_overlays() {
    let (_, events) = map_m0();
    let count = |lane: Lane| events.iter().filter(|event| event.lane == lane).count();

    let phases: Vec<usize> = Lane::ALL.into_iter().map(count).collect();
    assert_eq!(phases, [437, 69, 424, 93, 122, 134, 0], "Plan..Gate");
    assert_eq!(phases.iter().sum::<usize>(), events.len());

    let overlays: Vec<&Event> = events
        .iter()
        .filter(|event| event.overlay.is_some())
        .collect();
    assert_eq!(overlays.len(), 70);
    assert!(
        overlays
            .iter()
            .all(|event| event.overlay == Some(Overlay::Human) && event.lane == Lane::Plan)
    );
    assert!(
        events
            .iter()
            .all(|event| event.overlay == event.kind.overlay()),
        "the overlay is the kind's"
    );
}

/// The scrutiny validator that reads the feature list and names the features it reviews.
const SCRUTINY_SESSION: &str = "6eb88099-278d-4ddb-9f5d-23f1255cdbc5";

#[test]
fn a_feature_defined_by_a_scrutiny_validator_lands_in_the_scrutiny_lane() {
    let (_, events) = map_m0();
    let defined: Vec<&Event> = events
        .iter()
        .filter(|event| event.kind == EventKind::FeatureDefined)
        .collect();
    let by_session = |event: &&Event| {
        event
            .session_id
            .as_ref()
            .is_some_and(|id| id.0 == SCRUTINY_SESSION)
    };
    let of_scrutiny: Vec<&Event> = defined.iter().copied().filter(by_session).collect();

    assert_eq!(
        of_scrutiny.len(),
        6,
        "the capture holds six, all of 6eb88099"
    );
    assert_eq!(defined.len(), 6, "no other session defines a feature");
    assert!(
        of_scrutiny
            .iter()
            .all(|event| event.role == Role::ScrutinyValidator && event.lane == Lane::Scrutiny),
        "scrutiny: {:?}",
        of_scrutiny.iter().map(|e| e.lane).collect::<Vec<_>>()
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.lane == Lane::Plan && by_session(event))
            .count(),
        0,
        "the session has no event in Plan"
    );
}

#[test]
fn a_feature_defined_follows_the_phase_of_the_session_that_emits_it() {
    let task = assistant(
        1,
        "m1",
        json!([tool_use(
            "t",
            "Task",
            json!({"description": "Review m0-fixture-freeze", "subagent_type": "worker"})
        )]),
    );
    let lanes = |role: Role| {
        let mut session = Session::new(SessionMeta {
            milestones: vec![MilestoneId("m0".to_owned())],
            ..meta(role)
        });
        let events = session.feed(&task);
        assert_eq!(kinds(&events), ["task.assigned", "feature.defined"]);
        (events[0].lane, events[1].lane)
    };

    assert_eq!(lanes(Role::Orchestrator), (Lane::Plan, Lane::Plan));
    assert_eq!(
        lanes(Role::ScrutinyValidator),
        (Lane::Scrutiny, Lane::Scrutiny)
    );
    assert_eq!(
        lanes(Role::UserTestingValidator),
        (Lane::UserTesting, Lane::UserTesting)
    );
    assert_eq!(lanes(Role::Worker), (Lane::Build, Lane::Plan));
}

#[test]
fn mapping_version_is_selected_by_droid_version() {
    let header = read_capture(&fixture("jsonl"))
        .expect("the header is readable")
        .expect("the M0 capture has a header");
    assert_eq!(header.droid_version.as_deref(), Some("0.247.0"));

    assert_eq!(
        select_version(header.droid_version.as_deref()),
        (MappingVersion::V1, None)
    );
    assert_eq!(select_version(Some("0.247.0")), (MappingVersion::V1, None));
    assert_eq!(select_version(Some("0.247.9")), (MappingVersion::V1, None));
}

#[test]
fn unknown_droid_version_falls_back_to_latest_with_one_warning() {
    let (version, warning) = select_version(Some("0.300.0"));

    assert_eq!(version, MappingVersion::LATEST);
    let warning = warning.expect("an unknown version warns");
    assert_eq!(
        warning,
        Warning::UnknownDroidVersion {
            seen: Some("0.300.0".to_owned()),
            used: MappingVersion::LATEST,
        }
    );
    let line = warning.to_string();
    assert!(line.contains("0.300.0") && line.contains("v1"), "{line}");
    assert!(!line.contains('\n'), "one line: {line:?}");

    let (version, warning) = select_version(None);
    assert_eq!(version, MappingVersion::LATEST);
    assert!(warning.is_some(), "a capture without a version warns too");
}

// --- rules, record by record -------------------------------------------------------------

const SESSION: &str = "11111111-1111-4111-8111-111111111111";

fn meta(role: Role) -> SessionMeta {
    SessionMeta {
        id: SessionId(SESSION.to_owned()),
        mission_id: MissionId("mission-1".to_owned()),
        role,
        model: Some(ModelId("sidecar-model".to_owned())),
        ..SessionMeta::root(SessionId(SESSION.to_owned()))
    }
}

/// Maps `lines` (one JSON value each) as consecutive lines of one session file.
struct Session {
    mapper: Mapper,
    offset: u64,
    last_offset: u64,
    events: Vec<Event>,
}

impl Session {
    fn new(meta: SessionMeta) -> Self {
        Self {
            mapper: Mapper::new(meta, MappingVersion::V1),
            offset: 0,
            last_offset: 0,
            events: Vec::new(),
        }
    }

    fn feed(&mut self, line: &Value) -> Vec<Event> {
        self.feed_bytes(line.to_string().into_bytes())
    }

    fn feed_bytes(&mut self, bytes: Vec<u8>) -> Vec<Event> {
        let record = RawRecord {
            ptr: SourcePtr::Jsonl {
                path: PathBuf::from("session.jsonl"),
                byte_offset: self.offset,
                byte_len: u32::try_from(bytes.len()).unwrap(),
            },
            session_id: SessionId(SESSION.to_owned()),
            bytes,
            arrived_at: None,
        };
        self.last_offset = self.offset;
        self.offset += record.bytes.len() as u64 + 1;
        let events = self.mapper.map(&record);
        self.events.extend(events.clone());
        events
    }

    fn finish(&mut self) -> Vec<Event> {
        let events = self.mapper.finish();
        self.events.extend(events.clone());
        events
    }
}

fn kinds(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .map(|event| event.kind.prd_name().into_owned())
        .collect()
}

fn at(second: u32) -> String {
    format!("2026-10-22T10:00:{second:02}.000Z")
}

fn assistant(second: u32, model: &str, parts: Value) -> Value {
    json!({"type": "message", "id": "m", "seq": 1, "timestamp": at(second),
        "message": {"role": "assistant", "modelId": model, "content": parts}})
}

fn user(second: u32, parts: Value) -> Value {
    json!({"type": "message", "id": "m", "seq": 1, "timestamp": at(second),
        "message": {"role": "user", "content": parts}})
}

fn tool_use(id: &str, name: &str, input: Value) -> Value {
    json!({"type": "tool_use", "id": id, "name": name, "input": input})
}

fn tool_result(id: &str, content: &str, is_error: bool) -> Value {
    json!({"type": "tool_result", "tool_use_id": id, "content": content, "is_error": is_error})
}

#[test]
fn session_start_takes_the_time_of_the_next_timestamped_record() {
    let mut session = Session::new(meta(Role::Orchestrator));

    let first = session.feed(&json!({"type": "session_start", "id": SESSION}));
    assert!(first.is_empty(), "the time is not known yet: {first:?}");

    let second = session.feed(&user(7, json!([{"type": "text", "text": "hello"}])));
    assert_eq!(kinds(&second), ["intent.created"]);
    assert_eq!(second[0].at.to_rfc3339_millis(), at(7));
    assert_eq!(second[0].id.0, format!("{SESSION}:0:0"));
    assert_eq!(second[0].role, Role::Orchestrator);
    assert_eq!(second[0].lane, Lane::Plan);
}

#[test]
fn a_child_session_start_is_a_subagent_spawned_event_of_that_child() {
    let mut session = Session::new(meta(Role::Worker));
    session.feed(&json!({"type": "session_start", "id": SESSION,
        "callingSessionId": "root", "callingToolUseId": "toolu_1"}));
    let after = session.feed(&user(3, json!([])));

    assert_eq!(
        kinds(&after),
        ["subagent.spawned"],
        "the empty message adds nothing"
    );
    let finished = session.finish();
    assert_eq!(
        kinds(&session.events),
        ["subagent.spawned", "subagent.finished"]
    );
    let spawned = &session.events[0];
    assert_eq!(spawned.session_id, Some(SessionId(SESSION.to_owned())));
    assert_eq!(spawned.mission_id, MissionId("mission-1".to_owned()));
    assert_eq!(spawned.role, Role::Worker);
    assert_eq!(spawned.lane, Lane::Build);
    assert_eq!(spawned.at.to_rfc3339_millis(), at(3));
    assert_eq!(finished.len(), 1);
}

/// A child worker session that runs `feature` through the runner's prompt, edits a file,
/// commits, runs the tests and ends.
fn child_run(meta: SessionMeta, feature: &str) -> Vec<Event> {
    let mut session = Session::new(meta);
    session.feed(&json!({"type": "session_start", "id": SESSION,
        "callingSessionId": "root", "callingToolUseId": "toolu_1"}));
    session.feed(&user(
        1,
        json!([{"type": "text",
            "text": format!("You are a worker assigned to execute feature \"{feature}\".")}]),
    ));
    session.feed(&assistant(
        2,
        "m1",
        json!([
            tool_use("e", "Edit", json!({"file_path": "a.rs"})),
            tool_use("c", "Execute", json!({"command": "git commit -m x"})),
            tool_use("t", "Execute", json!({"command": "cargo test"})),
        ]),
    ));
    session.feed(&json!({"type": "agent_turn_outcome", "reason": "completed", "timestamp": at(3)}));
    session.finish();
    session.events
}

fn worker_with_milestones() -> SessionMeta {
    SessionMeta {
        milestones: vec![MilestoneId("m0".to_owned())],
        usage: Some(TokenUsage {
            input: 1,
            output: 2,
            cache_read: 3,
            cache_write: 4,
            credits: None,
        }),
        ..meta(Role::Worker)
    }
}

#[test]
fn integration_session_events_land_in_integrate() {
    let events = child_run(worker_with_milestones(), "m0-integrate-s");

    assert_eq!(
        kinds(&events),
        [
            "subagent.spawned",
            "task.started",
            "file.touched",
            "commit.created",
            "tests.run",
            "task.done",
            "usage.reported",
            "subagent.finished"
        ]
    );
    for event in &events {
        assert_eq!(event.lane, Lane::Integrate, "{}", event.kind.prd_name());
        assert_eq!(event.role, Role::Worker, "the role stays the session's");
    }
    let final_gate = child_run(worker_with_milestones(), "final-integrate-mission");
    assert!(final_gate.iter().all(|event| event.lane == Lane::Integrate));

    let build = child_run(worker_with_milestones(), "m0-mapping-v1");
    assert!(build.iter().all(|event| event.lane == Lane::Build));
}

#[test]
fn contract_session_events_land_in_contract() {
    let contract = SessionMeta {
        contract: true,
        ..worker_with_milestones()
    };
    let events = child_run(contract, "m0-validation-contract");

    assert_eq!(events.len(), 8);
    assert!(
        events.iter().all(|event| event.lane == Lane::Contract),
        "{:?}",
        events
            .iter()
            .map(|event| (event.kind.prd_name().into_owned(), event.lane))
            .collect::<Vec<_>>()
    );
    assert!(events.iter().all(|event| event.role == Role::Worker));
}

#[test]
fn human_prompts_in_the_root_amend_the_intent() {
    let mut session = Session::new(meta(Role::Orchestrator));
    let typed = json!({"type": "message", "id": "m", "timestamp": at(1),
        "message": {"role": "user", "userMessageSource": "cli_tui",
            "content": [{"type": "text", "text": "build it"}]}});
    let events = session.feed(&typed);

    assert_eq!(kinds(&events), ["intent.amended"]);
    assert_eq!(
        events[0].role,
        Role::Orchestrator,
        "the session's role, not Human"
    );
    assert_eq!(events[0].lane, Lane::Plan);

    let injected = json!({"type": "message", "id": "m", "timestamp": at(2),
        "message": {"role": "user", "visibility": "llm_only",
            "content": [{"type": "text", "text": "<system-reminder>x</system-reminder>"}]}});
    assert!(session.feed(&injected).is_empty());
}

#[test]
fn git_commit_and_test_commands_map_to_commit_and_tests_kinds() {
    let mut session = Session::new(meta(Role::Worker));
    let commit_and_test = assistant(
        1,
        "m1",
        json!([tool_use(
            "t1",
            "Execute",
            json!({"command": "git commit -m x && cargo test"})
        )]),
    );
    let events = session.feed(&commit_and_test);

    assert_eq!(kinds(&events), ["commit.created", "tests.run"]);
    assert_eq!(events[1].id.0, format!("{SESSION}:0:1"));
    assert!(events.iter().all(|event| event.lane == Lane::Build));

    let script = assistant(
        2,
        "m1",
        json!([tool_use(
            "t2",
            "Script",
            json!({"script": "await Execute({command: 'make test'})"})
        )]),
    );
    assert!(
        session.feed(&script).is_empty(),
        "a Script body only mentions the command; its inner Execute call is its own record"
    );
}

#[test]
fn make_check_is_a_gate_command_whose_result_decides_passed_or_failed() {
    let mut session = Session::new(meta(Role::Worker));
    let run = assistant(
        1,
        "m1",
        json!([
            tool_use("ok", "Execute", json!({"command": "make check"})),
            tool_use("bad", "Execute", json!({"command": "make check"})),
        ]),
    );
    assert_eq!(
        kinds(&session.feed(&run)),
        [
            "tests.run",
            "gate_command.run",
            "tests.run",
            "gate_command.run"
        ]
    );

    let results = user(
        4,
        json!([
            tool_result("ok", "all green", false),
            tool_result("bad", "Error: Command failed (exit code: 2)", true),
        ]),
    );
    let events = session.feed(&results);
    assert_eq!(
        kinds(&events),
        ["gate_command.passed", "gate_command.failed"]
    );
    assert!(events.iter().all(|event| event.lane == Lane::Integrate));
    assert_eq!(events[0].duration.unwrap().as_secs(), 3);
}

/// A result as Droid 0.247 writes it for an `Execute`: no `is_error` key at all.
fn tool_result_without_flag(id: &str, content: &str) -> Value {
    json!({"type": "tool_result", "tool_use_id": id, "content": content})
}

/// Feeds one assistant message holding `Execute` calls and returns the kinds of its events.
fn run_executes(session: &mut Session, second: u32, commands: &[(&str, &str)]) -> Vec<String> {
    let calls: Vec<Value> = commands
        .iter()
        .map(|(id, command)| tool_use(id, "Execute", json!({ "command": command })))
        .collect();
    kinds(&session.feed(&assistant(second, "m1", Value::Array(calls))))
}

#[test]
fn a_failing_make_check_with_is_error_null_maps_to_gate_command_failed() {
    let mut session = Session::new(meta(Role::Worker));
    run_executes(
        &mut session,
        1,
        &[
            (
                "line41",
                "cd /home/user/code/EllyaOrg/projects/dalimaray/observatory-wt/current && \
                 make check > /tmp/scrutiny-s-check.log 2>&1",
            ),
            ("short", "make check"),
            ("tail", "make check 2>&1 | tail -5"),
        ],
    );

    let results = user(
        4,
        json!([
            tool_result_without_flag(
                "line41",
                "Command failed (exit code: 2)\n\n[Process exited with code 2]"
            ),
            tool_result_without_flag(
                "short",
                "make: *** [check] Error 2\n\n\n[Process exited with code 2]"
            ),
            tool_result_without_flag("tail", "Command failed (exit code: 1)"),
        ]),
    );
    let events = session.feed(&results);
    assert_eq!(
        kinds(&events),
        [
            "gate_command.failed",
            "gate_command.failed",
            "gate_command.failed"
        ]
    );
    assert!(events.iter().all(|event| event.lane == Lane::Integrate));
    assert_eq!(events[0].duration.unwrap().as_secs(), 3);

    let mut flagged = Session::new(meta(Role::Worker));
    run_executes(&mut flagged, 1, &[("e", "make check")]);
    let events = flagged.feed(&user(2, json!([tool_result("e", "boom", true)])));
    assert_eq!(
        kinds(&events),
        ["gate_command.failed"],
        "is_error alone, with no exit code in the text, still fails the gate"
    );
}

#[test]
fn make_check_dash_targets_and_script_mentions_are_not_gate_runs() {
    let mut session = Session::new(meta(Role::Worker));
    let not_runs = [
        ("todos", "make check-todos"),
        ("links", "make check-links 2>&1 | tail -3"),
        ("docs", "cd wt && make check-docs"),
        ("echo", "echo \"run make check before git commit\""),
        ("mine", "xmake check"),
        ("tests", "make test-all && cargo test-helper"),
        (
            "heredoc",
            "cat > notes.md <<'EOF'\nmake check\ngit commit -m x\nEOF",
        ),
    ];
    assert!(run_executes(&mut session, 1, &not_runs).is_empty());

    let script = assistant(
        2,
        "m1",
        json!([tool_use(
            "script",
            "Script",
            json!({"script": "const r = await Execute({command: 'make check'});\n\
                // git commit and cargo test and make test stay in the Script's inner calls"})
        )]),
    );
    assert!(session.feed(&script).is_empty());

    let results = user(
        4,
        json!([
            tool_result_without_flag("todos", "ok\n\n[Process exited with code 0]"),
            tool_result_without_flag("script", "Command failed (exit code: 2)"),
        ]),
    );
    assert!(
        session.feed(&results).is_empty(),
        "no gate call is pending, so no verdict"
    );
}

#[test]
fn a_make_check_exit_zero_maps_to_gate_command_passed() {
    let mut session = Session::new(meta(Role::Worker));
    let run = run_executes(
        &mut session,
        1,
        &[
            ("plain", "make check"),
            ("redirect", "make check>/tmp/log 2>&1"),
            ("chain", "make fmt; make check && echo done"),
            ("env", "CI=1 make check"),
            ("noise", "make check-docs"),
        ],
    );
    assert_eq!(
        run.iter()
            .filter(|kind| *kind == "gate_command.run")
            .count(),
        4,
        "make check-docs is not a gate run: {run:?}"
    );

    let results = user(
        5,
        json!([
            tool_result_without_flag("plain", "all green\n\n\n[Process exited with code 0]"),
            tool_result("redirect", "", false),
            tool_result_without_flag(
                "chain",
                "log mentions exit code: 2 and Error 1\n\n\n[Process exited with code 0]"
            ),
            tool_result_without_flag("env", "test result: ok\n\n[Process exited with code 0]"),
            tool_result_without_flag("noise", "ok\n\n[Process exited with code 0]"),
        ]),
    );
    let events = session.feed(&results);
    assert_eq!(kinds(&events), ["gate_command.passed"; 4]);
    assert_eq!(events[0].duration.unwrap().as_secs(), 4);
}

#[test]
fn error_results_are_tool_errors_and_hook_blocks_are_named() {
    let mut session = Session::new(meta(Role::Worker));
    let calls = assistant(
        1,
        "m1",
        json!([
            tool_use("r", "Read", json!({"file_path": "/x"})),
            tool_use("e", "Edit", json!({"file_path": "/y"})),
        ]),
    );
    assert_eq!(kinds(&session.feed(&calls)), ["file.touched"]);

    let results = user(
        2,
        json!([
            tool_result("r", "Error reading file: ENOENT", true),
            tool_result(
                "e",
                "Script policy rejected this call: blocked by .factory/hooks/guard-git.sh",
                true
            ),
            tool_result("fine", "ok", false),
        ]),
    );
    let events = session.feed(&results);
    assert_eq!(kinds(&events), ["tool.error", "hook.blocked"]);
    assert!(events.iter().all(|event| event.lane == Lane::Build));
}

#[test]
fn a_new_model_id_is_reported_once_per_switch() {
    let mut session = Session::new(meta(Role::Worker));
    let text = |second, model| assistant(second, model, json!([{"type": "text", "text": "hi"}]));

    assert!(session.feed(&text(1, "a")).is_empty());
    assert!(session.feed(&text(2, "a")).is_empty());
    let switched = session.feed(&text(3, "b"));

    assert_eq!(kinds(&switched), ["model.changed"]);
    assert_eq!(switched[0].model, Some(ModelId("b".to_owned())));
    assert!(session.feed(&text(4, "b")).is_empty());
}

#[test]
fn compaction_and_changed_todos_are_orchestrator_events() {
    let mut session = Session::new(meta(Role::Orchestrator));
    let todos = |status: &str| {
        json!({"type": "todo_state", "id": "t", "timestamp": at(1),
            "todos": {"todos": [{"id": "1", "status": status, "content": "x"}]}})
    };

    assert_eq!(kinds(&session.feed(&todos("pending"))), ["plan.revised"]);
    assert!(
        session.feed(&todos("pending")).is_empty(),
        "same todos, no revision"
    );
    assert_eq!(kinds(&session.feed(&todos("completed"))), ["plan.revised"]);

    let compacted = session.feed(&json!({"type": "compaction_state", "id": "c",
        "timestamp": at(9), "summaryKind": "llm_summary"}));
    assert_eq!(kinds(&compacted), ["context.compacted"]);
    assert_eq!(compacted[0].lane, Lane::Plan);
}

#[test]
fn child_turn_outcomes_end_the_task_and_end_of_file_finishes_the_subagent_with_its_usage() {
    let usage = TokenUsage {
        input: 11,
        output: 22,
        cache_read: 33,
        cache_write: 44,
        credits: Some(1.5),
    };
    let mut session = Session::new(SessionMeta {
        usage: Some(usage),
        ..meta(Role::Worker)
    });
    session.feed(&user(1, json!([{"type": "text", "text": "start"}])));
    let outcome = |reason: &str| json!({"type": "agent_turn_outcome", "turnId": "t", "reason": reason, "resultKind": "text"});

    let done = session.feed(&outcome("completed"));
    assert_eq!(kinds(&done), ["task.done"]);
    assert_eq!(
        done[0].at.to_rfc3339_millis(),
        at(1),
        "an outcome has no time of its own"
    );
    assert_eq!(
        kinds(&session.feed(&outcome("cancelled"))),
        ["task.skipped"]
    );
    assert_eq!(kinds(&session.feed(&outcome("error"))), ["tool.error"]);

    let end = session.finish();
    assert_eq!(kinds(&end), ["usage.reported", "subagent.finished"]);
    assert_eq!(end[0].usage, Some(usage));
    assert_eq!(end[0].model, Some(ModelId("sidecar-model".to_owned())));
    assert!(end.iter().all(|event| {
        matches!(&event.source, SourcePtr::Jsonl { byte_offset, .. } if *byte_offset == session.last_offset)
    }));
    let ids: BTreeSet<_> = session
        .events
        .iter()
        .map(|event| event.id.0.clone())
        .collect();
    assert_eq!(ids.len(), session.events.len(), "event ids stay unique");
}

#[test]
fn root_turn_outcomes_pause_or_escalate_and_a_finished_turn_is_quiet() {
    let mut session = Session::new(meta(Role::Orchestrator));
    session.feed(&user(1, json!([])));
    let outcome = |reason: &str| json!({"type": "agent_turn_outcome", "turnId": "t", "reason": reason, "resultKind": null});

    assert!(session.feed(&outcome("completed")).is_empty());
    assert_eq!(
        kinds(&session.feed(&outcome("cancelled"))),
        ["mission.paused"]
    );
    assert_eq!(
        kinds(&session.feed(&outcome("model_usage_exhausted"))),
        ["escalation.raised"]
    );
    assert!(
        session.finish().is_empty(),
        "no sidecar usage, no child to finish"
    );
}

#[test]
fn a_turn_the_host_ended_with_process_exit_maps_like_a_cancel_in_child_and_root() {
    let outcome = json!({"type": "agent_turn_outcome", "turnId": "t", "reason": "process_exit"});

    let mut child = Session::new(meta(Role::Worker));
    child.feed(&user(1, json!([{"type": "text", "text": "start"}])));
    assert_eq!(kinds(&child.feed(&outcome)), ["task.skipped"]);

    let mut root = Session::new(meta(Role::Orchestrator));
    root.feed(&user(1, json!([])));
    assert_eq!(kinds(&root.feed(&outcome)), ["mission.paused"]);
}

#[test]
fn the_plan_the_questions_and_the_features_come_from_the_orchestrators_tool_calls() {
    let mut session = Session::new(meta(Role::Orchestrator));
    let proposal =
        "| Milestone | Delivers |\n|---|---|\n| **S** skeleton | a |\n| **M0** mapping | b |\n";
    let plan = assistant(
        1,
        "m1",
        json!([tool_use(
            "p",
            "ProposeMission",
            json!({"title": "t", "proposal": proposal})
        )]),
    );
    let events = session.feed(&plan);
    assert_eq!(
        kinds(&events),
        ["plan.created", "milestone.defined", "milestone.defined"]
    );
    assert_eq!(events[1].milestone_id, Some(MilestoneId("s".to_owned())));
    assert_eq!(events[2].milestone_id, Some(MilestoneId("m0".to_owned())));
    assert!(events.iter().all(|event| event.lane == Lane::Plan));

    let task = |id: &str, description: &str| {
        assistant(
            2,
            "m1",
            json!([tool_use(
                id,
                "Task",
                json!({"description": description, "subagent_type": "worker"})
            )]),
        )
    };
    let named = session.feed(&task("a", "Review m0-fixture-freeze"));
    assert_eq!(kinds(&named), ["task.assigned", "feature.defined"]);
    assert!(named.iter().all(|event| {
        event
            .feature_id
            .as_ref()
            .map(ToString::to_string)
            .as_deref()
            == Some("m0-fixture-freeze")
            && event.milestone_id == Some(MilestoneId("m0".to_owned()))
    }));
    assert_eq!(
        kinds(&session.feed(&task("b", "Review m0-fixture-freeze"))),
        ["task.assigned"]
    );
    assert_eq!(
        kinds(&session.feed(&task("c", "Draft the notes"))),
        ["task.assigned"]
    );

    let ask = assistant(
        3,
        "m1",
        json!([tool_use("q", "AskUser", json!({"questionnaire": "?"}))]),
    );
    let asked = session.feed(&ask);
    assert_eq!(kinds(&asked), ["question.asked"]);
    assert_eq!(asked[0].lane, Lane::Plan);
    let answered = session.feed(&user(4, json!([tool_result("q", "yes", false)])));
    assert_eq!(kinds(&answered), ["intent.answered"]);
    assert_eq!(answered[0].role, Role::Orchestrator);
}

#[test]
fn a_worker_prompt_names_its_feature_and_milestone() {
    let mut session = Session::new(SessionMeta {
        milestones: vec![MilestoneId("s".to_owned()), MilestoneId("m0".to_owned())],
        ..meta(Role::Worker)
    });
    let prompt = user(
        1,
        json!([{"type": "text",
            "text": "<system-reminder>\nYou are a worker assigned to execute feature \"m0-mapping-v1\".\n"}]),
    );
    let started = session.feed(&prompt);
    assert_eq!(kinds(&started), ["task.started"]);

    let edit = assistant(
        2,
        "m1",
        json!([tool_use("e", "Edit", json!({"file_path": "/x"}))]),
    );
    let touched = session.feed(&edit);
    assert_eq!(kinds(&touched), ["file.touched"]);
    assert_eq!(
        touched[0]
            .feature_id
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("m0-mapping-v1")
    );
    assert_eq!(touched[0].milestone_id, Some(MilestoneId("m0".to_owned())));
    assert_eq!(touched[0].model, Some(ModelId("m1".to_owned())));
}

#[test]
fn scrutiny_reports_become_findings_and_a_pass() {
    let mut session = Session::new(meta(Role::ScrutinyValidator));
    let report = "**Result: pass.** No blocking issues.\n\n**Non-blocking issues**\n\
        1. first thing\n2. second thing\n- third thing\n\n**Shared-state observations**\n- not a finding\n";
    let events = session.feed(&assistant(
        1,
        "m1",
        json!([{"type": "text", "text": report}]),
    ));

    assert_eq!(
        kinds(&events),
        [
            "finding.non_blocking",
            "finding.non_blocking",
            "finding.non_blocking",
            "scrutiny.passed"
        ]
    );
    assert!(events.iter().all(|event| event.lane == Lane::Scrutiny));

    let failing = "**Blocking issues**\n1. broken\n";
    let events = session.feed(&assistant(
        2,
        "m1",
        json!([{"type": "text", "text": failing}]),
    ));
    assert_eq!(kinds(&events), ["finding.blocking"]);

    let chatter = "I will now read the diff and look for blocking problems.";
    assert!(
        session
            .feed(&assistant(
                3,
                "m1",
                json!([{"type": "text", "text": chatter}])
            ))
            .is_empty()
    );
}

#[test]
fn user_testing_reports_become_scenarios_and_evidence() {
    let mut session = Session::new(meta(Role::UserTestingValidator));
    let report = "| Assertion | Status |\n|---|---|\n| VAL-A-001 | pass | ok |\n| VAL-A-002 | fail | no |\n\n\
        **VAL-A-003 (pass): shown**\n**VAL-A-004: FAIL**\n\nEvidence folder: /x/evidence/s/flow/\n";
    let events = session.feed(&assistant(
        1,
        "m1",
        json!([{"type": "text", "text": report}]),
    ));

    assert_eq!(
        kinds(&events),
        [
            "qa.scenario.passed",
            "qa.scenario.failed",
            "qa.scenario.passed",
            "qa.scenario.failed",
            "qa.evidence"
        ]
    );
    assert!(events.iter().all(|event| event.lane == Lane::UserTesting));
}

#[test]
fn lines_and_parts_the_rules_do_not_know_stay_unknown_and_counted() {
    let mut session = Session::new(meta(Role::Worker));

    let garbage = session.feed_bytes(b"not json".to_vec());
    let future = session.feed(&json!({"type": "future_kind"}));
    let part = session.feed(&assistant(1, "m1", json!([{"type": "audio", "data": "x"}])));
    let tool = session.feed(&assistant(
        2,
        "m1",
        json!([
            tool_use("z", "TeleportFile", json!({})),
            tool_use("e", "Edit", json!({}))
        ]),
    ));

    let raw = |events: &[Event]| -> Vec<String> {
        events
            .iter()
            .map(|event| match &event.kind {
                EventKind::Unknown { raw_type } => raw_type.clone(),
                known => known.prd_name().into_owned(),
            })
            .collect()
    };
    assert_eq!(raw(&garbage), ["<unparseable>"]);
    assert_eq!(raw(&future), ["future_kind"]);
    assert_eq!(raw(&part), ["message/audio"]);
    assert_eq!(
        raw(&tool),
        ["message/tool_use/TeleportFile", "file.touched"]
    );
    assert!(
        garbage
            .iter()
            .chain(&tool)
            .all(|event| event.lane == Lane::Build)
    );
}

#[test]
fn chatter_that_carries_no_prd_event_maps_to_no_event() {
    let mut session = Session::new(meta(Role::Worker));
    let chatter = assistant(
        1,
        "m1",
        json!([
            {"type": "thinking", "thinking": "hmm"},
            {"type": "text", "text": "working"},
            tool_use("r", "Read", json!({"file_path": "/x"})),
            tool_use("g", "Grep", json!({"pattern": "x"})),
            tool_use("t", "TodoWrite", json!({"todos": []})),
        ]),
    );

    assert!(session.feed(&chatter).is_empty());
    assert!(
        session
            .feed(&user(2, json!([tool_result("r", "contents", false)])))
            .is_empty()
    );
}

#[test]
fn a_system_note_such_as_request_interrupted_by_user_maps_to_no_event() {
    let mut session = Session::new(meta(Role::Orchestrator));
    let note = json!({"type": "message", "id": "m", "seq": 1, "timestamp": at(1),
        "message": {"role": "system", "visibility": "llm_only",
            "content": [{"type": "text", "text": "Request interrupted by user"}]}});

    assert!(
        session.feed(&note).is_empty(),
        "a message of the system role is Droid's own annotation, not an unknown line"
    );
}
