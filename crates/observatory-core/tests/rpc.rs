//! The hosted JSON-RPC lane: stdout of `droid exec --input-format stream-jsonrpc
//! --output-format stream-jsonrpc` spooled verbatim, mapped by mapping v1 to the lane events
//! of the session file, and the control lines written back (architecture §4.5, D-033).
//!
//! The recorded stream is built from the M0 capture: the envelope shapes below are the ones a
//! real `droid` 0.247.0 printed on 2026-10-24 (a one-turn Haiku session, see D-033), with the
//! session file's own messages as the payload.

#![cfg(all(feature = "rpc", feature = "store"))]

mod common;

use std::collections::BTreeMap;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use observatory_core::config::IngestConfig;
use observatory_core::config::MissionConfig;
use observatory_core::ingest::rpc::{
    AckOutcome, ControlChannel, ControlError, HostControl, OpenRequests, RpcMapper, RpcSource,
    Spool, encode_initialize,
};
use observatory_core::ingest::{JsonlSource, RawRecord, Source};
use observatory_core::mapping::{Mapper, MappingVersion, SessionMeta};
use observatory_core::model::{
    Event, EventKind, Lane, Overlay, Role, SessionId, SourcePtr, Timestamp,
};
use observatory_core::store::{HostedRun, Store};
use observatory_core::view::{LaunchConfig, LaunchForm};
use rusqlite::Connection;
use serde_json::{Value, json};

const LAUNCH: &str = "launch-0001";

fn notification(session: &str, notification: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "type": "notification", "factoryApiVersion": "1.0.0",
        "factoryProtocolVersion": "1.274.0", "method": "droid.session_notification",
        "params": { "sessionId": session, "notification": notification },
    })
}

fn response(id: &str, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "type": "response", "factoryApiVersion": "1.0.0",
        "factoryProtocolVersion": "1.274.0", "id": id, "result": result,
    })
}

fn request(id: &str, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "type": "request", "factoryApiVersion": "1.0.0",
        "factoryProtocolVersion": "1.274.0", "id": id, "method": method, "params": params,
    })
}

fn line(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).unwrap();
    bytes.push(b'\n');
    bytes
}

fn millis(text: &str) -> i64 {
    let nanos = Timestamp::parse(text).unwrap().0.unix_timestamp_nanos();
    i64::try_from(nanos / 1_000_000).unwrap()
}

fn create_message(session: &str, record: &Value) -> Value {
    let mut message = record["message"].clone();
    let created = record["timestamp"].as_str().map(millis);
    message["id"] = record["id"].clone();
    message["parentId"] = record.get("parentId").cloned().unwrap_or(json!("root"));
    message["seq"] = record.get("seq").cloned().unwrap_or(json!(0));
    if let Some(created) = created {
        message["createdAt"] = json!(created);
        message["updatedAt"] = json!(created);
    }
    notification(
        session,
        json!({ "type": "create_message", "message": message }),
    )
}

/// What Droid prints for the same mission as `file`: the session's lines as notifications,
/// with the protocol chatter between them. `todo_state` and `compaction_state` have no known
/// notification, so they stay session records. Returns the stream and the number of chatter
/// lines (those with no lane event of their own).
fn hosted_stream_of(file: &Path) -> (Vec<u8>, u64) {
    let mut out = Vec::new();
    let mut chatter = 0;
    let mut chat = |out: &mut Vec<u8>, session: &str, kind: &str| {
        out.extend(line(&notification(session, json!({ "type": kind }))));
        chatter += 1;
    };
    chat(&mut out, "", "mcp_status_changed");
    chat(&mut out, "", "settings_updated");
    let mut session = String::new();
    for raw in std::fs::read_to_string(file).unwrap().lines() {
        let record: Value = serde_json::from_str(raw).unwrap();
        match record["type"].as_str() {
            Some("session_start") => {
                session = record["id"].as_str().unwrap().to_owned();
                out.extend(line(&response(
                    "1",
                    json!({ "sessionId": session, "settings": { "modelId": "m" } }),
                )));
            }
            Some("message") => {
                out.extend(line(&create_message(&session, &record)));
                chat(&mut out, &session, "session_token_usage_changed");
            }
            Some("agent_turn_outcome") => out.extend(line(&notification(
                &session,
                json!({
                    "type": "agent_turn_completed", "reason": record["reason"],
                    "turnId": record["turnId"], "tokenUsage": { "inputTokens": 0 },
                }),
            ))),
            _ => {
                out.extend_from_slice(raw.as_bytes());
                out.push(b'\n');
            }
        }
    }
    (out, chatter)
}

/// The fake process: `cat` prints the recorded stream on its stdout, and the spool takes it
/// from the pipe the way the host does.
fn spool_through_fake_process(stream: &Path, spool: &Path) {
    let mut child = Command::new("cat")
        .arg(stream)
        .stdout(Stdio::piped())
        .spawn()
        .expect("cat starts");
    let mut spool = Spool::create(spool).expect("the spool is created");
    spool
        .copy_from(BufReader::new(child.stdout.take().unwrap()))
        .expect("the pipe is copied");
    assert!(child.wait().unwrap().success());
}

struct Hosted {
    mapper: RpcMapper,
    events: Vec<Event>,
    records: Vec<RawRecord>,
    spool: PathBuf,
}

fn host(dir: &Path, stream: &[u8]) -> Hosted {
    let source_file = dir.join("recorded.jsonl");
    std::fs::write(&source_file, stream).unwrap();
    let spool = observatory_core::ingest::rpc::spool_path(dir, LAUNCH);
    spool_through_fake_process(&source_file, &spool);

    let mut source = RpcSource::open(&spool, &IngestConfig::default()).expect("the spool opens");
    let mut mapper = RpcMapper::new(LAUNCH, MappingVersion::LATEST);
    let (mut events, mut records) = (Vec::new(), Vec::new());
    loop {
        let mut batch = source.next_batch().expect("the spool reads");
        let at_eof = source.at_eof();
        if at_eof {
            batch.extend(source.drain_partial());
        }
        events.extend(batch.iter().flat_map(|record| mapper.map(record)));
        records.extend(batch);
        if at_eof {
            break;
        }
    }
    events.extend(mapper.finish());
    Hosted {
        mapper,
        events,
        records,
        spool,
    }
}

/// The attached view of the same mission: the session file tailed and mapped.
fn attach(file: &Path) -> Vec<Event> {
    let mut source = JsonlSource::open(file, &IngestConfig::default()).unwrap();
    let mut mapper = Mapper::new(
        SessionMeta::root(source.root_session_id().clone()),
        MappingVersion::LATEST,
    );
    let mut events = Vec::new();
    loop {
        let mut batch = source.next_batch().unwrap();
        let at_eof = source.at_eof();
        if at_eof {
            batch.extend(source.drain_partial());
        }
        events.extend(batch.iter().flat_map(|record| mapper.map(record)));
        if at_eof {
            break;
        }
    }
    events.extend(mapper.finish());
    events
}

type Lanes = (
    String,
    Lane,
    Option<Overlay>,
    Role,
    Option<SessionId>,
    Timestamp,
    Option<std::time::Duration>,
    Option<String>,
);

/// An event without its pointer and id, which name where the line sits and differ by design.
fn lane_event(event: &Event) -> Lanes {
    (
        event.kind.prd_name().into_owned(),
        event.lane,
        event.overlay,
        event.role,
        event.session_id.clone(),
        event.at,
        event.duration,
        event.feature_id.as_ref().map(ToString::to_string),
    )
}

#[test]
fn rpc_events_map_to_the_same_lane_events_as_the_session_file() {
    let dir = tempfile::tempdir().unwrap();
    let (stream, chatter) = hosted_stream_of(&common::m0_root());

    let hosted = host(dir.path(), &stream);
    let attached = attach(&common::m0_root());

    let hosted_lanes: Vec<_> = hosted.events.iter().map(lane_event).collect();
    let attached_lanes: Vec<_> = attached.iter().map(lane_event).collect();
    assert_eq!(hosted_lanes.len(), attached_lanes.len());
    // A turn outcome notification has no time of its own: the host stamps it with the time it
    // arrived, where the session file's line takes the time of the line before it, and the
    // turn's duration follows from that time.
    let untimed = |index: usize| {
        let SourcePtr::Rpc {
            path,
            byte_offset,
            byte_len,
        } = &hosted.events[index].source
        else {
            panic!("a hosted event must carry an rpc pointer");
        };
        let raw = JsonlSource::read_at(path, *byte_offset, *byte_len).unwrap();
        String::from_utf8_lossy(&raw).contains("agent_turn_completed")
    };
    for (index, (hosted_event, attached_event)) in
        hosted_lanes.iter().zip(&attached_lanes).enumerate()
    {
        if untimed(index) {
            let mut hosted_event = hosted_event.clone();
            (hosted_event.5, hosted_event.6) = (attached_event.5, attached_event.6);
            assert_eq!(&hosted_event, attached_event, "event {index}");
        } else {
            assert_eq!(hosted_event, attached_event, "event {index}");
        }
    }
    for event in &hosted.events {
        assert_eq!(event.mapping_version, MappingVersion::V1);
        let SourcePtr::Rpc {
            path,
            byte_offset,
            byte_len,
        } = &event.source
        else {
            panic!(
                "a hosted event must carry an rpc pointer: {:?}",
                event.source
            );
        };
        assert_eq!(path, &hosted.spool);
        let raw = JsonlSource::read_at(path, *byte_offset, *byte_len).unwrap();
        assert!(serde_json::from_slice::<Value>(&raw).is_ok());
    }
    let counted: u64 = hosted.mapper.protocol_counts().values().sum();
    assert!(counted >= chatter, "{counted} < {chatter}");
    assert!(
        !hosted
            .events
            .iter()
            .any(|event| matches!(event.kind, EventKind::Unknown { .. })),
        "the recorded stream has no unknown event"
    );
}

#[test]
fn spool_is_byte_identical_to_the_process_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let (stream, _) = hosted_stream_of(&common::m0_root());

    let hosted = host(dir.path(), &stream);

    assert_eq!(std::fs::read(&hosted.spool).unwrap(), stream);
    let lines = stream.iter().filter(|byte| **byte == b'\n').count();
    assert_eq!(hosted.records.len(), lines);
    let mut offset = 0;
    for record in &hosted.records {
        let SourcePtr::Rpc {
            byte_offset,
            byte_len,
            ..
        } = &record.ptr
        else {
            panic!("an rpc source yields rpc pointers: {:?}", record.ptr);
        };
        assert_eq!(*byte_offset, offset);
        assert_eq!(record.bytes.len(), *byte_len as usize);
        offset += u64::from(*byte_len) + 1;
    }
}

#[test]
fn host_and_attach_of_the_same_mission_agree_on_event_count() {
    let dir = tempfile::tempdir().unwrap();
    let (stream, _) = hosted_stream_of(&common::m0_root());

    let hosted = host(dir.path(), &stream);
    let attached = attach(&common::m0_root());

    assert!(!attached.is_empty());
    assert_eq!(hosted.events.len(), attached.len());
    let kinds = |events: &[Event]| {
        let mut counts = BTreeMap::new();
        for event in events {
            *counts
                .entry(event.kind.prd_name().into_owned())
                .or_insert(0) += 1;
        }
        counts
    };
    assert_eq!(kinds(&hosted.events), kinds(&attached));
}

fn written_lines(channel: &ControlChannel<Vec<u8>>) -> Vec<Value> {
    channel
        .writer()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

#[test]
fn pause_request_is_written_as_jsonrpc_and_acknowledged() {
    let mut channel = ControlChannel::new(Vec::new());
    let mut open = OpenRequests::default();

    let sent = channel.send(&HostControl::Pause, &mut open).unwrap();

    let written = written_lines(&channel);
    assert_eq!(written.len(), 1);
    let request = &written[0];
    assert_eq!(request["jsonrpc"], "2.0");
    assert_eq!(request["type"], "request");
    assert_eq!(request["factoryApiVersion"], "1.0.0");
    assert_eq!(request["method"], "droid.interrupt_session");
    assert_eq!(request["params"], json!({}));
    // Droid 0.247.0 rejects a numeric id with -32700: the id is a string of the counter.
    let id = request["id"].as_str().expect("the id is a string");
    assert!(id.parse::<u64>().is_ok(), "{id}");
    assert_eq!(sent.id, id);
    assert!(channel.writer().ends_with(b"\n"));
    assert_eq!(channel.pending(), 1);

    let other = line(&response("999", json!({})));
    assert_eq!(channel.observe(&other), None);
    assert_eq!(channel.pending(), 1);

    let ack = channel
        .observe(&line(&response(id, json!({}))))
        .expect("the response with the same id acknowledges the request");
    assert_eq!(ack.id, id);
    assert_eq!(ack.method, "droid.interrupt_session");
    assert_eq!(ack.outcome, AckOutcome::Accepted);
    assert_eq!(channel.pending(), 0);
}

#[test]
fn a_rejected_request_is_acknowledged_with_the_error() {
    let mut channel = ControlChannel::new(Vec::new());
    let mut open = OpenRequests::default();
    let first = channel
        .send(
            &HostControl::Message {
                text: "go on".into(),
            },
            &mut open,
        )
        .unwrap();
    let second = channel.send(&HostControl::Resume, &mut open).unwrap();
    assert_ne!(first.id, second.id);

    let error = json!({
        "jsonrpc": "2.0", "type": "response", "factoryApiVersion": "1.0.0", "id": first.id,
        "error": { "code": -32602, "message": "Invalid params" },
    });
    let ack = channel.observe(&line(&error)).unwrap();

    assert_eq!(ack.method, "droid.add_user_message");
    assert_eq!(
        ack.outcome,
        AckOutcome::Rejected {
            code: -32602,
            message: "Invalid params".into()
        }
    );
    assert_eq!(
        written_lines(&channel)[0]["params"],
        json!({ "text": "go on" })
    );
    assert_eq!(channel.pending(), 1);
}

fn question_stream() -> Vec<u8> {
    let ask = |tool: &str, n: u32| {
        json!({
            "id": format!("msg-{tool}"), "timestamp": format!("2026-10-24T05:39:0{n}.000Z"),
            "message": { "role": "assistant", "content": [
                { "type": "tool_use", "id": tool, "name": "AskUser",
                  "input": { "questions": [{ "question": "Which one?" }] } } ] },
        })
    };
    let ask_request = |id: &str, tool: &str| {
        request(
            id,
            "droid.ask_user",
            json!({
                "toolCallId": tool,
                "questions": [{ "index": 1, "topic": "t", "question": "Which one?", "options": [] }],
            }),
        )
    };
    let mut out = Vec::new();
    out.extend(line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    )));
    out.extend(line(&create_message("s-1", &ask("toolu_A", 1))));
    out.extend(line(&ask_request("req-A", "toolu_A")));
    out.extend(line(&create_message("s-1", &ask("toolu_B", 2))));
    out.extend(line(&ask_request("req-B", "toolu_B")));
    out
}

fn question_events(hosted: &Hosted) -> Vec<String> {
    hosted
        .events
        .iter()
        .filter(|event| event.kind == EventKind::QuestionAsked)
        .map(|event| event.id.to_string())
        .collect()
}

#[test]
fn answer_routes_to_the_open_question_id() {
    let dir = tempfile::tempdir().unwrap();
    let mut hosted = host(dir.path(), &question_stream());
    let questions = question_events(&hosted);
    assert_eq!(questions.len(), 2, "{:?}", hosted.events);
    let (older, newer) = (questions[0].clone(), questions[1].clone());
    let mut channel = ControlChannel::new(Vec::new());

    // The older question is answered by its own id, not the newest one's.
    let sent = channel
        .send(
            &HostControl::Answer {
                question_id: older.clone(),
                text: "the first".into(),
            },
            hosted.mapper.open_requests_mut(),
        )
        .unwrap();

    let written = written_lines(&channel);
    assert_eq!(sent.id, "req-A");
    assert_eq!(written[0]["type"], "response");
    assert_eq!(written[0]["jsonrpc"], "2.0");
    assert_eq!(written[0]["id"], "req-A");
    assert_eq!(
        written[0]["result"]["answers"],
        json!([{ "index": 1, "question": "Which one?", "answer": "the first" }])
    );
    // A response to a request is not itself acknowledged.
    assert_eq!(channel.pending(), 0);

    // Once answered, the same question is closed.
    let again = channel.send(
        &HostControl::Answer {
            question_id: older.clone(),
            text: "twice".into(),
        },
        hosted.mapper.open_requests_mut(),
    );
    assert!(
        matches!(again, Err(ControlError::NotOpen { .. })),
        "{again:?}"
    );
    let unknown = channel.send(
        &HostControl::Answer {
            question_id: "s-1:0:0".into(),
            text: "who".into(),
        },
        hosted.mapper.open_requests_mut(),
    );
    assert!(
        matches!(unknown, Err(ControlError::NotOpen { .. })),
        "{unknown:?}"
    );
    assert_eq!(written_lines(&channel).len(), 1);

    let sent = channel
        .send(
            &HostControl::Answer {
                question_id: newer,
                text: "the second".into(),
            },
            hosted.mapper.open_requests_mut(),
        )
        .unwrap();
    assert_eq!(sent.id, "req-B");
    assert_eq!(written_lines(&channel)[1]["id"], "req-B");
}

#[test]
fn a_question_answered_in_the_terminal_is_closed() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = question_stream();
    stream.extend(line(&create_message(
        "s-1",
        &json!({
            "id": "msg-result", "timestamp": "2026-10-24T05:39:09.000Z",
            "message": { "role": "user", "content": [
                { "type": "tool_result", "tool_use_id": "toolu_A", "content": "the first" } ] },
        }),
    )));
    let mut hosted = host(dir.path(), &stream);
    let older = question_events(&hosted)[0].clone();
    let mut channel = ControlChannel::new(Vec::new());

    let late = channel.send(
        &HostControl::Answer {
            question_id: older,
            text: "late".into(),
        },
        hosted.mapper.open_requests_mut(),
    );

    assert!(
        matches!(late, Err(ControlError::NotOpen { .. })),
        "{late:?}"
    );
    assert!(channel.writer().is_empty());
}

#[test]
fn approve_and_deny_answer_the_permission_request_by_its_id() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = Vec::new();
    stream.extend(line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    )));
    stream.extend(line(&request(
        "perm-1",
        "droid.request_permission",
        json!({
            "toolUses": [{ "toolUse": { "id": "toolu_P", "name": "Execute" } }],
            "options": [{ "value": "proceed_once" }, { "value": "cancel" }],
        }),
    )));
    stream.extend(line(&request(
        "perm-2",
        "droid.request_permission",
        json!({ "toolUses": [], "options": [{ "value": "cancel" }] }),
    )));
    let mut hosted = host(dir.path(), &stream);
    let mut channel = ControlChannel::new(Vec::new());

    channel
        .send(
            &HostControl::Approve {
                request_id: "perm-1".into(),
            },
            hosted.mapper.open_requests_mut(),
        )
        .unwrap();
    channel
        .send(
            &HostControl::Deny {
                request_id: "perm-2".into(),
            },
            hosted.mapper.open_requests_mut(),
        )
        .unwrap();
    let refused = channel.send(
        &HostControl::Approve {
            request_id: "perm-2".into(),
        },
        hosted.mapper.open_requests_mut(),
    );

    let written = written_lines(&channel);
    assert_eq!(written[0]["id"], "perm-1");
    assert_eq!(
        written[0]["result"],
        json!({ "selectedOption": "proceed_once" })
    );
    assert_eq!(written[1]["id"], "perm-2");
    assert_eq!(written[1]["result"], json!({ "selectedOption": "cancel" }));
    assert!(
        matches!(refused, Err(ControlError::NotOpen { .. })),
        "{refused:?}"
    );
}

#[test]
fn store_rows_of_hosted_events_carry_the_rpc_pointer_and_mapping_v1() {
    let dir = tempfile::tempdir().unwrap();
    let (stream, _) = hosted_stream_of(&common::m0_root());
    let hosted = host(dir.path(), &stream);
    let session = hosted
        .mapper
        .session_id()
        .cloned()
        .expect("the stream starts a session");
    let db = dir.path().join("observatory.sqlite");
    let mut store = Store::open(&db).unwrap();

    store
        .record_hosted(&HostedRun {
            mission_name: "hosted".into(),
            session: session.clone(),
            spool: hosted.spool.clone(),
            droid_version: Some("0.247.0".into()),
            started_at: "2026-10-24T05:39:04.000Z".into(),
        })
        .unwrap();
    let added = store.insert_events(&hosted.events).unwrap();
    // Replaying the spool twice adds nothing.
    assert_eq!(store.insert_events(&hosted.events).unwrap(), 0);

    assert_eq!(added, hosted.events.len());
    let conn = Connection::open(&db).unwrap();
    let rows: Vec<(String, String)> = conn
        .prepare("SELECT DISTINCT source, mapping_version FROM events WHERE path = ?1")
        .unwrap()
        .query_map([hosted.spool.to_str().unwrap()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows, vec![("rpc".to_owned(), "v1".to_owned())]);
    let unpointed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE source = 'rpc' AND (byte_offset IS NULL OR byte_len IS NULL)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(unpointed, 0);
    let first = &hosted.events[0];
    assert_eq!(store.pointer(&first.id).unwrap(), first.source);
    let mission: String = conn
        .query_row("SELECT mission_id FROM events LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(mission, session.to_string());
}

#[test]
fn spool_appends_lines_as_given_and_refuses_to_clobber_a_launch() {
    let dir = tempfile::tempdir().unwrap();
    let path = observatory_core::ingest::rpc::spool_path(dir.path(), "launch-x");
    assert_eq!(path, dir.path().join("hosted").join("launch-x.jsonl"));

    let mut spool = Spool::create(&path).unwrap();
    spool.append_line(br#"{"a":1}"#).unwrap();
    spool.append_line(br#"{"b":2}"#).unwrap();

    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"{\"a\":1}\n{\"b\":2}\n");
    assert_eq!(spool.bytes_written(), bytes.len() as u64);
    assert!(Spool::create(&path).is_err());
}

fn launch_config() -> LaunchConfig {
    let mut form = LaunchForm::defaults(&MissionConfig::default());
    form.repo = "/work/repo".into();
    form.mission_file = "/work/repo/mission.md".into();
    form.branch = "mission/hello".into();
    form.worker_model = "worker-model".into();
    form.validator_model = "validator-model".into();
    form.validator_effort = "low".into();
    form.validate().unwrap()
}

#[test]
fn initialize_session_carries_the_launch_settings_and_is_acknowledged() {
    let config = launch_config();

    let request = encode_initialize(&config, "11111111-2222-4333-8444-555555555555", 1);

    let wire = serde_json::to_value(&request).unwrap();
    assert_eq!(wire["type"], "request");
    assert_eq!(wire["method"], "droid.initialize_session");
    assert_eq!(wire["id"], "1");
    let params = &wire["params"];
    assert_eq!(params["cwd"], "/work/repo");
    assert_eq!(params["sessionId"], "11111111-2222-4333-8444-555555555555");
    assert_eq!(params["modelId"], "claude-fable-5.1");
    assert_eq!(params["reasoningEffort"], "high");
    assert_eq!(params["autonomyLevel"], "high");
    assert_eq!(params["interactionMode"], "mission");
    assert_eq!(params["decompSessionType"], "orchestrator");
    assert!(
        params["machineId"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(
        params["missionSettings"],
        json!({
            "workerModel": "worker-model", "workerReasoningEffort": "high",
            "validationWorkerModel": "validator-model", "validationWorkerReasoningEffort": "low",
        })
    );

    let mut channel = ControlChannel::new(Vec::new());
    let sent = channel
        .initialize(&config, "11111111-2222-4333-8444-555555555555")
        .unwrap();
    assert_eq!(sent.method, "droid.initialize_session");
    assert_eq!(written_lines(&channel)[0]["id"], sent.id);
    let ack = channel
        .observe(&line(&response(&sent.id, json!({ "sessionId": "x" }))))
        .expect("the response acknowledges the initialize request");
    assert_eq!(ack.outcome, AckOutcome::Accepted);
}

#[test]
fn a_mission_proposal_is_told_from_other_permission_requests() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = Vec::new();
    stream.extend(line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    )));
    for (id, confirmation) in [("p-1", "propose_mission"), ("p-2", "exec")] {
        stream.extend(line(&request(
            id,
            "droid.request_permission",
            json!({
                "toolUses": [], "confirmationType": confirmation,
                "options": [{ "value": "proceed_once" }, { "value": "cancel" }],
            }),
        )));
    }

    let hosted = host(dir.path(), &stream);

    let proposals: Vec<&str> = hosted
        .mapper
        .open_requests()
        .permissions()
        .iter()
        .filter(|permission| permission.is_mission_proposal())
        .map(|permission| permission.request_id.as_str())
        .collect();
    assert_eq!(proposals, ["p-1"]);
}

#[test]
fn a_permission_request_for_the_propose_mission_tool_is_a_mission_proposal() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = Vec::new();
    stream.extend(line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    )));
    for (id, tool) in [("p-1", "ProposeMission"), ("p-2", "Execute")] {
        stream.extend(line(&request(
            id,
            "droid.request_permission",
            json!({
                "toolUses": [{ "toolUse": { "type": "tool_use", "id": "t", "name": tool, "input": {} } }],
                "options": [{ "value": "proceed_once" }, { "value": "cancel" }],
            }),
        )));
    }

    let hosted = host(dir.path(), &stream);

    let proposals: Vec<&str> = hosted
        .mapper
        .open_requests()
        .permissions()
        .iter()
        .filter(|permission| permission.is_mission_proposal())
        .map(|permission| permission.request_id.as_str())
        .collect();
    assert_eq!(proposals, ["p-1"]);
}

#[test]
fn open_questions_are_listed_by_the_event_that_asked_them() {
    let dir = tempfile::tempdir().unwrap();
    let mut hosted = host(dir.path(), &question_stream());
    let asked = question_events(&hosted);
    assert_eq!(
        hosted
            .mapper
            .open_requests()
            .open_questions()
            .iter()
            .map(|question| question.event_id.clone())
            .collect::<Vec<_>>(),
        asked
    );
    let mut channel = ControlChannel::new(Vec::new());
    channel
        .send(
            &HostControl::Answer {
                question_id: asked[0].clone(),
                text: "a".into(),
            },
            hosted.mapper.open_requests_mut(),
        )
        .unwrap();

    let left = hosted.mapper.open_requests().open_questions();

    assert_eq!(left.len(), 1);
    assert_eq!(left[0].event_id, asked[1]);
    assert_eq!(left[0].prompts[0].question, "Which one?");
}

#[test]
fn tool_call_and_tool_result_notifications_are_counted_not_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = Vec::new();
    stream.extend(line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    )));
    for kind in ["tool_call", "tool_result"] {
        stream.extend(line(&notification("s-1", json!({ "type": kind }))));
    }

    let hosted = host(dir.path(), &stream);

    assert!(
        hosted
            .events
            .iter()
            .all(|event| !matches!(event.kind, EventKind::Unknown { .. })),
        "{:?}",
        hosted.events
    );
    assert_eq!(hosted.mapper.protocol_counts()["tool_call"], 1);
    assert_eq!(hosted.mapper.protocol_counts()["tool_result"], 1);
}

#[test]
fn a_message_discarded_by_a_pause_is_counted_not_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = Vec::new();
    stream.extend(line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    )));
    stream.extend(line(&notification(
        "s-1",
        json!({ "type": "queued_messages_discarded", "text": "hi", "requestId": "3" }),
    )));

    let hosted = host(dir.path(), &stream);

    assert!(
        hosted
            .events
            .iter()
            .all(|event| !matches!(event.kind, EventKind::Unknown { .. })),
        "{:?}",
        hosted.events
    );
    assert_eq!(
        hosted.mapper.protocol_counts()["queued_messages_discarded"],
        1
    );
}

/// What Droid 0.247.0 streams for an answered question: the assistant message with the call, the
/// request, and the result as a `tool_result` notification (the user message that carries it
/// in the session file arrives with empty content).
fn answered_question_stream(result_also_in_message: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    )));
    out.extend(line(&create_message(
        "s-1",
        &json!({
            "id": "msg-A", "timestamp": "2026-10-24T05:39:01.000Z",
            "message": { "role": "assistant", "content": [
                { "type": "tool_use", "id": "toolu_A", "name": "AskUser",
                  "input": { "questions": [{ "question": "Which one?" }] } } ] },
        }),
    )));
    out.extend(line(&notification(
        "s-1",
        json!({ "type": "tool_result", "toolUseId": "toolu_A", "isError": false,
                "content": "1. [question] Which one?\n[answer] salute" }),
    )));
    if result_also_in_message {
        out.extend(line(&create_message(
            "s-1",
            &json!({
                "id": "msg-B", "timestamp": "2026-10-24T05:39:02.000Z",
                "message": { "role": "user", "content": [
                    { "type": "tool_result", "tool_use_id": "toolu_A", "content": "salute" } ] },
            }),
        )));
    }
    out
}

fn answered(hosted: &Hosted) -> usize {
    hosted
        .events
        .iter()
        .filter(|event| event.kind == EventKind::IntentAnswered)
        .count()
}

#[test]
fn a_tool_result_notification_maps_like_the_result_in_the_session_file() {
    let dir = tempfile::tempdir().unwrap();

    let hosted = host(dir.path(), &answered_question_stream(false));

    assert_eq!(answered(&hosted), 1, "{:?}", hosted.events);
    assert!(hosted.mapper.open_requests().open_questions().is_empty());
}

#[test]
fn a_result_delivered_twice_maps_once() {
    let dir = tempfile::tempdir().unwrap();

    let hosted = host(dir.path(), &answered_question_stream(true));

    assert_eq!(answered(&hosted), 1, "{:?}", hosted.events);
}

#[test]
fn retry_and_retraction_notifications_are_counted_not_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = Vec::new();
    stream.extend(line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    )));
    stream.extend(line(&notification(
        "s-1",
        json!({ "type": "llm_retry", "attempt": 1, "reason": "timeout" }),
    )));
    stream.extend(line(&notification(
        "s-1",
        json!({ "type": "assistant_message_retracted", "messageId": "m" }),
    )));

    let hosted = host(dir.path(), &stream);

    assert!(
        hosted
            .events
            .iter()
            .all(|event| !matches!(event.kind, EventKind::Unknown { .. })),
        "{:?}",
        hosted.events
    );
    assert_eq!(hosted.mapper.protocol_counts()["llm_retry"], 1);
    assert_eq!(
        hosted.mapper.protocol_counts()["assistant_message_retracted"],
        1
    );
}

#[test]
fn host_controls_travel_as_tagged_json() {
    let control: HostControl =
        serde_json::from_value(json!({ "kind": "answer", "question_id": "q", "text": "t" }))
            .unwrap();
    assert_eq!(
        control,
        HostControl::Answer {
            question_id: "q".into(),
            text: "t".into()
        }
    );
    assert_eq!(
        serde_json::from_value::<HostControl>(json!({ "kind": "pause" })).unwrap(),
        HostControl::Pause
    );
}

fn at_millis(timestamp: Timestamp) -> i64 {
    i64::try_from(timestamp.0.unix_timestamp_nanos() / 1_000_000).unwrap()
}

fn now_millis() -> i64 {
    at_millis(Timestamp(time::OffsetDateTime::now_utc()))
}

fn cancelled_turn() -> Value {
    notification(
        "s-1",
        json!({ "type": "agent_turn_completed", "reason": "cancelled", "turnId": "t-1" }),
    )
}

fn of_kind(events: &[Event], kind: EventKind) -> &Event {
    events
        .iter()
        .find(|event| event.kind == kind)
        .unwrap_or_else(|| panic!("no {kind:?} in {events:?}"))
}

#[test]
fn a_pause_and_an_answer_carry_the_arrival_time_of_their_lines() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = answered_question_stream(false);
    stream.extend(line(&cancelled_turn()));

    let before = now_millis();
    let hosted = host(dir.path(), &stream);
    let after = now_millis();

    let question = of_kind(&hosted.events, EventKind::QuestionAsked).at;
    assert_eq!(at_millis(question), millis("2026-10-24T05:39:01.000Z"));
    for kind in [EventKind::IntentAnswered, EventKind::MissionPaused] {
        let at = at_millis(of_kind(&hosted.events, kind.clone()).at);
        assert!(
            at > at_millis(question),
            "{kind:?} inherited the question's time"
        );
        assert!(
            (before..=after).contains(&at),
            "{kind:?} at {at} outside {before}..{after}"
        );
    }
}

/// The notification Droid 0.247.0 writes when the host closes the window and reaps the child
/// (spool 2036e3c5, line at byte 585173, `sessionId` replaced).
fn process_exit_turn() -> Value {
    notification(
        "s-1",
        json!({
            "type": "agent_turn_completed", "reason": "process_exit",
            "turnId": "d6e6ec5f-e50e-41da-8266-747ff8942a4f",
            "tokenUsage": {
                "inputTokens": 10, "outputTokens": 201, "cacheCreationTokens": 363,
                "cacheReadTokens": 35035, "thinkingTokens": 74, "factoryCredits": 1989,
            },
            "childTokenUsage": {
                "inputTokens": 0, "outputTokens": 0, "cacheCreationTokens": 0,
                "cacheReadTokens": 0, "thinkingTokens": 0, "factoryCredits": 0,
            },
            "durationMs": 4655,
        }),
    )
}

#[test]
fn a_turn_ended_by_the_host_reaping_the_child_is_a_known_pause_that_keeps_its_reason() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = answered_question_stream(false);
    stream.extend(line(&process_exit_turn()));

    let hosted = host(dir.path(), &stream);

    let unknown: Vec<_> = hosted
        .events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
        .collect();
    assert!(unknown.is_empty(), "unknown events: {unknown:?}");
    let paused = of_kind(&hosted.events, EventKind::MissionPaused);
    let SourcePtr::Rpc {
        path,
        byte_offset,
        byte_len,
    } = &paused.source
    else {
        panic!("not a spool pointer: {:?}", paused.source);
    };
    let spooled = std::fs::read(path).unwrap();
    let start = usize::try_from(*byte_offset).unwrap();
    let raw = &spooled[start..start + usize::try_from(*byte_len).unwrap()];
    assert!(
        String::from_utf8_lossy(raw).contains("\"reason\":\"process_exit\""),
        "the pointer shows a host kill, not a user cancel: {}",
        String::from_utf8_lossy(raw)
    );
}

#[test]
fn lines_that_arrive_apart_get_different_times() {
    let dir = tempfile::tempdir().unwrap();
    let path = observatory_core::ingest::rpc::spool_path(dir.path(), LAUNCH);
    let mut spool = Spool::create(&path).unwrap();
    let mut source = RpcSource::open(&path, &IngestConfig::default()).unwrap();
    let mut mapper = RpcMapper::new(LAUNCH, MappingVersion::LATEST);
    let mut events = Vec::new();
    let mut step = |spool_line: Vec<u8>, spool: &mut Spool| {
        spool.append_line(spool_line.trim_ascii_end()).unwrap();
        for record in source.next_batch().unwrap() {
            events.extend(mapper.map(&record));
        }
    };

    for stream_line in answered_question_stream(false).split_inclusive(|byte| *byte == b'\n') {
        step(stream_line.to_vec(), &mut spool);
    }
    std::thread::sleep(std::time::Duration::from_millis(60));
    step(line(&cancelled_turn()), &mut spool);

    let answered = at_millis(of_kind(&events, EventKind::IntentAnswered).at);
    let paused = at_millis(of_kind(&events, EventKind::MissionPaused).at);
    assert!(
        paused - answered >= 55,
        "answered {answered}, paused {paused}"
    );
}

#[test]
fn a_permission_request_that_waits_says_what_the_human_can_choose() {
    let dir = tempfile::tempdir().unwrap();
    let mut stream = line(&response(
        "1",
        json!({ "sessionId": "s-1", "settings": {} }),
    ));
    stream.extend(line(&request(
        "perm-1",
        "droid.request_permission",
        json!({
            "toolUses": [{ "toolUse": { "id": "toolu_P", "name": "Execute" } }],
            "options": [{ "value": "proceed_once" }, { "value": "cancel" }],
        }),
    )));
    stream.extend(line(&request(
        "perm-2",
        "droid.request_permission",
        json!({ "toolUses": [], "options": [{ "value": "cancel" }] }),
    )));
    stream.extend(line(&request(
        "perm-3",
        "droid.request_permission",
        json!({ "toolUses": [{ "toolUse": { "name": "Edit" } }] }),
    )));
    let hosted = host(dir.path(), &stream);

    let waiting = hosted.mapper.open_requests().waiting_permissions();

    let summary: Vec<_> = waiting
        .iter()
        .map(|permission| {
            (
                permission.request_id.as_str(),
                permission.tools.join(","),
                permission.can_approve,
                permission.can_deny,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("perm-1", "Execute".to_owned(), true, true),
            ("perm-2", String::new(), false, true),
            ("perm-3", "Edit".to_owned(), true, true),
        ]
    );
}

fn initialize_params(mission: &MissionConfig) -> Value {
    let mut form = LaunchForm::defaults(mission);
    form.repo = "/work/repo".into();
    form.mission_file = "/work/repo/mission.md".into();
    form.branch = "mission/hello".into();
    let config = form.validate().unwrap();
    serde_json::to_value(encode_initialize(&config, "s-1", 1)).unwrap()["params"].clone()
}

#[test]
fn session_tags_and_skipped_validation_travel_in_the_initialize_request() {
    let mission = MissionConfig {
        session_tags: vec!["scratch".into()],
        skip_scrutiny: true,
        skip_user_testing: true,
        ..MissionConfig::default()
    };

    let params = initialize_params(&mission);

    assert_eq!(params["tags"], json!([{ "name": "scratch" }]));
    assert_eq!(params["missionSettings"]["skipScrutiny"], true);
    assert_eq!(params["missionSettings"]["skipUserTesting"], true);
}

#[test]
fn the_default_config_adds_no_tags_and_skips_no_validation() {
    let params = initialize_params(&MissionConfig::default());

    assert!(params.get("tags").is_none(), "{params}");
    assert!(params["missionSettings"].get("skipScrutiny").is_none());
    assert!(params["missionSettings"].get("skipUserTesting").is_none());
}

#[test]
fn the_mission_models_are_set_with_an_update_after_initialize() {
    // Droid 0.247.0 takes `missionSettings` of `droid.initialize_session` into the session
    // but writes the mission's model-settings.json from the user's own defaults; the update
    // request is what reaches the mission (a hosted scratch mission ran a worker on Opus).
    let mission = MissionConfig {
        worker_model: "worker-model".into(),
        validator_model: "validator-model".into(),
        skip_scrutiny: true,
        ..MissionConfig::default()
    };
    let mut form = LaunchForm::defaults(&mission);
    form.repo = "/work/repo".into();
    form.mission_file = "/work/repo/mission.md".into();
    form.branch = "mission/hello".into();
    let config = form.validate().unwrap();
    let mut channel = ControlChannel::new(Vec::new());
    channel.initialize(&config, "s-1").unwrap();

    let sent = channel.mission_settings(&config).unwrap();

    let written = written_lines(&channel);
    assert_eq!(written.len(), 2);
    let update = &written[1];
    assert_eq!(update["method"], "droid.update_session_settings");
    assert_eq!(sent.method, "droid.update_session_settings");
    assert_eq!(update["id"], sent.id.as_str());
    assert_eq!(written[0]["id"], "1");
    assert_eq!(update["id"], "2");
    assert_eq!(
        update["params"],
        json!({ "missionSettings": written[0]["params"]["missionSettings"] })
    );
    assert_eq!(
        update["params"]["missionSettings"]["workerModel"],
        "worker-model"
    );
    assert_eq!(
        update["params"]["missionSettings"]["validationWorkerModel"],
        "validator-model"
    );
    let ack = channel
        .observe(&line(&response("2", json!({}))))
        .expect("the update is acknowledged");
    assert_eq!(ack.method, "droid.update_session_settings");
}
