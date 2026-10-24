//! Reads one stdout line and says what it is, in terms of the session-file records mapping v1
//! reads. The session file and the stream carry the same messages; only the envelope differs
//! (checked against a real one-turn capture, D-033).

use serde_json::{Map, Value, json};

use super::open::{OpenPermission, OpenQuestion, Prompt};
use super::protocol::method;
use crate::model::Timestamp;

/// Notifications that report state of the process and have no counterpart in the session
/// file. They are counted, not mapped: a lane event comes from a message, never from chatter.
const PROTOCOL_NOTIFICATIONS: &[&str] = &[
    "mcp_status_changed",
    "settings_updated",
    "droid_working_state_changed",
    "session_token_usage_changed",
    "session_title_updated",
    "assistant_text_delta",
    "assistant_text_complete",
    "thinking_text_delta",
    "thinking_text_complete",
    "tool_progress_update",
    "tool_execution_heartbeat",
    "tool_execution_phase_changed",
    "mission_heartbeat",
    "mission_state_changed",
    "mission_features_changed",
    "mission_progress_entry",
    "mission_worker_started",
    "mission_worker_completed",
];

/// Keys of a notification's message that the session file keeps beside it, not inside it.
const OUTER_MESSAGE_KEYS: &[&str] = &["id", "parentId", "seq", "createdAt", "updatedAt"];
/// `parentId` of a message with no parent in a notification; the file leaves the key out.
const NO_PARENT: &str = "root";
const ASK_USER_TOOL: &str = "AskUser";

pub(super) enum Incoming {
    /// Not an envelope: a session record, mapped as it is.
    Record,
    /// The response to `droid.initialize_session`, as the `session_start` line of a file.
    Session {
        id: String,
        record: Vec<u8>,
    },
    Message {
        record: Vec<u8>,
        /// `tool_use` ids of the `AskUser` calls in the message, in order.
        asks: Vec<String>,
        /// `tool_use` ids the message carries a result for.
        results: Vec<String>,
    },
    Outcome {
        record: Vec<u8>,
    },
    Question(OpenQuestion),
    Permission(OpenPermission),
    PermissionResolved {
        request_id: String,
    },
    /// A line counted under this name that yields no event.
    Protocol(String),
    /// An envelope the rules do not know: an `Unknown` event named this, never dropped.
    Unrecognised(String),
}

pub(super) fn parse(bytes: &[u8]) -> Incoming {
    let Ok(Value::Object(line)) = serde_json::from_slice::<Value>(bytes) else {
        return Incoming::Record;
    };
    if !line.get("jsonrpc").is_some_and(Value::is_string) {
        return Incoming::Record;
    }
    match line.get("type").and_then(Value::as_str) {
        Some("response") => response(&line),
        Some("request") => incoming_request(&line),
        Some("notification") => notification(&line),
        _ => unrecognised("envelope"),
    }
}

fn unrecognised(what: &str) -> Incoming {
    Incoming::Unrecognised(format!("rpc/{what}"))
}

fn response(line: &Map<String, Value>) -> Incoming {
    let result = line.get("result");
    let session = result
        .and_then(|result| result.get("sessionId"))
        .and_then(Value::as_str)
        .filter(|_| result.is_some_and(|result| result["settings"].is_object()));
    match session {
        Some(id) => Incoming::Session {
            id: id.to_owned(),
            record: json!({ "type": "session_start", "id": id })
                .to_string()
                .into_bytes(),
        },
        None => Incoming::Protocol("response".into()),
    }
}

fn incoming_request(line: &Map<String, Value>) -> Incoming {
    let (Some(id), Some(name)) = (
        line.get("id").and_then(Value::as_str),
        line.get("method").and_then(Value::as_str),
    ) else {
        return unrecognised("request");
    };
    let params = line.get("params").unwrap_or(&Value::Null);
    match name {
        method::ASK_USER => match params["toolCallId"].as_str() {
            Some(call) => Incoming::Question(OpenQuestion {
                request_id: id.to_owned(),
                tool_call_id: call.to_owned(),
                prompts: params["questions"]
                    .as_array()
                    .map(|questions| {
                        questions
                            .iter()
                            .map(|question| Prompt {
                                index: question["index"].as_u64().unwrap_or_default(),
                                question: question["question"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_owned(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            }),
            None => unrecognised(name),
        },
        method::REQUEST_PERMISSION => Incoming::Permission(OpenPermission {
            request_id: id.to_owned(),
            options: params["options"]
                .as_array()
                .map(|options| {
                    options
                        .iter()
                        .filter_map(|option| option["value"].as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
        }),
        other => unrecognised(other),
    }
}

fn notification(line: &Map<String, Value>) -> Incoming {
    if line.get("method").and_then(Value::as_str) != Some(method::SESSION_NOTIFICATION) {
        return unrecognised("notification");
    }
    let body = &line.get("params").unwrap_or(&Value::Null)["notification"];
    let Some(kind) = body["type"].as_str() else {
        return unrecognised("notification");
    };
    match kind {
        "create_message" => message(body).unwrap_or_else(|| unrecognised(kind)),
        "agent_turn_completed" => Incoming::Outcome {
            record: json!({
                "type": "agent_turn_outcome", "turnId": body["turnId"], "reason": body["reason"],
            })
            .to_string()
            .into_bytes(),
        },
        "permission_resolved" => match body["requestId"].as_str() {
            Some(id) => Incoming::PermissionResolved {
                request_id: id.to_owned(),
            },
            None => unrecognised(kind),
        },
        known if PROTOCOL_NOTIFICATIONS.contains(&known) => Incoming::Protocol(known.to_owned()),
        other => unrecognised(other),
    }
}

/// The `message` line of a session file for a `create_message` notification.
fn message(body: &Value) -> Option<Incoming> {
    let mut inner = body.get("message")?.as_object()?.clone();
    let mut record = Map::new();
    record.insert("type".into(), "message".into());
    for (key, value) in &inner {
        if matches!(key.as_str(), "id" | "seq") {
            record.insert(key.clone(), value.clone());
        }
        if key == "parentId" && value.as_str() != Some(NO_PARENT) {
            record.insert(key.clone(), value.clone());
        }
    }
    if let Some(at) = inner.get("createdAt").and_then(Value::as_i64) {
        record.insert("timestamp".into(), Value::String(from_millis(at)?));
    }
    for key in OUTER_MESSAGE_KEYS {
        inner.remove(*key);
    }
    let (mut asks, mut results) = (Vec::new(), Vec::new());
    for part in inner
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match part["type"].as_str() {
            Some("tool_use") if part["name"] == ASK_USER_TOOL => {
                asks.extend(part["id"].as_str().map(str::to_owned));
            }
            Some("tool_result") => results.extend(part["tool_use_id"].as_str().map(str::to_owned)),
            _ => {}
        }
    }
    record.insert("message".into(), Value::Object(inner));
    Some(Incoming::Message {
        record: Value::Object(record).to_string().into_bytes(),
        asks,
        results,
    })
}

fn from_millis(millis: i64) -> Option<String> {
    let nanos = i128::from(millis).checked_mul(1_000_000)?;
    let at = time::OffsetDateTime::from_unix_timestamp_nanos(nanos).ok()?;
    Some(Timestamp(at).to_rfc3339_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notified(notification: &Value) -> Vec<u8> {
        json!({
            "jsonrpc": "2.0", "type": "notification", "method": method::SESSION_NOTIFICATION,
            "params": { "sessionId": "s", "notification": notification },
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn a_session_record_and_garbage_pass_through() {
        assert!(matches!(
            parse(br#"{"type":"todo_state"}"#),
            Incoming::Record
        ));
        assert!(matches!(parse(b"not json"), Incoming::Record));
        assert!(matches!(parse(b"[1]"), Incoming::Record));
    }

    #[test]
    fn a_message_becomes_the_record_the_file_has() {
        let line = notified(&json!({ "type": "create_message", "message": {
            "id": "m1", "parentId": "root", "seq": 3, "createdAt": 1_792_820_344_488_i64,
            "updatedAt": 1, "role": "user",
            "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "x" }],
        }}));

        let Incoming::Message {
            record,
            asks,
            results,
        } = parse(&line)
        else {
            panic!("a message");
        };

        let record: Value = serde_json::from_slice(&record).unwrap();
        assert_eq!(
            record,
            json!({
                "type": "message", "id": "m1", "seq": 3, "timestamp": "2026-10-24T05:39:04.488Z",
                "message": { "role": "user",
                    "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "x" }] },
            })
        );
        assert!(asks.is_empty());
        assert_eq!(results, ["t1"]);
    }

    #[test]
    fn a_parent_is_kept_and_ask_user_calls_are_listed() {
        let line = notified(&json!({ "type": "create_message", "message": {
            "id": "m1", "parentId": "m0", "role": "assistant",
            "content": [{ "type": "tool_use", "id": "t9", "name": "AskUser", "input": {} },
                        { "type": "tool_use", "id": "t8", "name": "Read", "input": {} }],
        }}));

        let Incoming::Message { record, asks, .. } = parse(&line) else {
            panic!("a message");
        };

        let record: Value = serde_json::from_slice(&record).unwrap();
        assert_eq!(record["parentId"], "m0");
        assert!(record.get("timestamp").is_none());
        assert_eq!(asks, ["t9"]);
    }

    #[test]
    fn a_turn_completion_is_an_outcome_record() {
        let line = notified(&json!({
            "type": "agent_turn_completed", "reason": "cancelled", "turnId": "u1",
        }));
        let Incoming::Outcome { record } = parse(&line) else {
            panic!("an outcome");
        };
        let record: Value = serde_json::from_slice(&record).unwrap();
        assert_eq!(
            record,
            json!({ "type": "agent_turn_outcome", "turnId": "u1", "reason": "cancelled" })
        );
    }

    #[test]
    fn chatter_is_protocol_and_the_rest_is_unrecognised() {
        let chatter = notified(&json!({ "type": "assistant_text_delta" }));
        assert!(
            matches!(parse(&chatter), Incoming::Protocol(name) if name == "assistant_text_delta")
        );
        let odd = notified(&json!({ "type": "brand_new" }));
        assert!(matches!(parse(&odd), Incoming::Unrecognised(name) if name == "rpc/brand_new"));
        let no_body = notified(&json!({}));
        assert!(matches!(parse(&no_body), Incoming::Unrecognised(_)));
        let other_method = br#"{"jsonrpc":"2.0","type":"notification","method":"x"}"#;
        assert!(matches!(parse(other_method), Incoming::Unrecognised(_)));
        let strange = br#"{"jsonrpc":"2.0","type":"what"}"#;
        assert!(matches!(parse(strange), Incoming::Unrecognised(name) if name == "rpc/envelope"));
    }

    #[test]
    fn responses_are_protocol_unless_they_start_a_session() {
        let ack = br#"{"jsonrpc":"2.0","type":"response","id":"2","result":{}}"#;
        assert!(matches!(parse(ack), Incoming::Protocol(name) if name == "response"));
        let start = br#"{"jsonrpc":"2.0","type":"response","id":"1","result":{"sessionId":"s-1","settings":{}}}"#;
        let Incoming::Session { id, record } = parse(start) else {
            panic!("a session");
        };
        assert_eq!(id, "s-1");
        assert_eq!(
            serde_json::from_slice::<Value>(&record).unwrap(),
            json!({ "type": "session_start", "id": "s-1" })
        );
    }

    #[test]
    fn requests_of_droid_are_parsed_for_the_human_to_answer() {
        let ask = json!({ "jsonrpc": "2.0", "type": "request", "id": "r1", "method": "droid.ask_user",
            "params": { "toolCallId": "t1", "questions": [{ "index": 2, "question": "Which?" }] } });
        let Incoming::Question(question) = parse(ask.to_string().as_bytes()) else {
            panic!("a question");
        };
        assert_eq!(question.request_id, "r1");
        assert_eq!(question.tool_call_id, "t1");
        assert_eq!(
            question.prompts,
            [Prompt {
                index: 2,
                question: "Which?".into()
            }]
        );

        let permission = json!({ "jsonrpc": "2.0", "type": "request", "id": "p1",
            "method": "droid.request_permission", "params": { "options": [{ "value": "cancel" }] } });
        let Incoming::Permission(permission) = parse(permission.to_string().as_bytes()) else {
            panic!("a permission");
        };
        assert_eq!(permission.options, ["cancel"]);

        for bad in [
            json!({ "jsonrpc": "2.0", "type": "request", "id": "r", "method": "droid.ask_user", "params": {} }),
            json!({ "jsonrpc": "2.0", "type": "request", "method": "droid.ask_user" }),
            json!({ "jsonrpc": "2.0", "type": "request", "id": "r", "method": "droid.other" }),
        ] {
            assert!(matches!(
                parse(bad.to_string().as_bytes()),
                Incoming::Unrecognised(_)
            ));
        }
        let resolved = notified(&json!({ "type": "permission_resolved", "requestId": "p1" }));
        assert!(
            matches!(parse(&resolved), Incoming::PermissionResolved { request_id } if request_id == "p1")
        );
        let broken = notified(&json!({ "type": "permission_resolved" }));
        assert!(matches!(parse(&broken), Incoming::Unrecognised(_)));
    }
}
