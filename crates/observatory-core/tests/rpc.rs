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
    assert_eq!(hosted_lanes, attached_lanes);
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
