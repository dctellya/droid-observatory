//! The wire format of `droid exec --input-format stream-jsonrpc --output-format
//! stream-jsonrpc` (Droid 0.247.0, D-033): names, the envelope and what the host writes.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::view::LaunchConfig;

/// Method names, as printed by Droid or accepted by it. Confirmed against 0.247.0 on 2026-10-24.
pub mod method {
    /// Host to Droid: interrupts the running turn. The pause control.
    pub const INTERRUPT_SESSION: &str = "droid.interrupt_session";
    /// Host to Droid: a user message. The message and resume controls.
    pub const ADD_USER_MESSAGE: &str = "droid.add_user_message";
    /// Droid to host, in a response to `droid.initialize_session`: the session id.
    pub const INITIALIZE_SESSION: &str = "droid.initialize_session";
    /// Droid to host: every event of the session, `params.notification.type` says which.
    pub const SESSION_NOTIFICATION: &str = "droid.session_notification";
    /// Droid to host: a question for the human. Answered by a response with the same id.
    pub const ASK_USER: &str = "droid.ask_user";
    /// Droid to host: a permission prompt. Answered by a response with the same id.
    pub const REQUEST_PERMISSION: &str = "droid.request_permission";
}

/// `factoryApiVersion` of every envelope; Droid rejects an envelope without it.
pub const FACTORY_API_VERSION: &str = "1.0.0";
const JSONRPC: &str = "2.0";

/// `selectedOption` of a permission response that lets the call run once.
pub const OPTION_APPROVE: &str = "proceed_once";
/// `selectedOption` of a permission response that refuses the call.
pub const OPTION_DENY: &str = "cancel";

/// What a resume sends: an interrupted session continues on its next user message.
pub const RESUME_TEXT: &str = "Resume the mission from where it was paused.";

/// What the app asks of the hosted orchestrator. Travels from the UI as `{"kind": "pause"}`,
/// `{"kind": "answer", "question_id": ..., "text": ...}` and so on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostControl {
    Pause,
    Resume,
    Message {
        text: String,
    },
    /// `question_id` is the id of the `QuestionAsked` event being answered.
    Answer {
        question_id: String,
        text: String,
    },
    /// `request_id` is the JSON-RPC id of the `droid.request_permission` request.
    Approve {
        request_id: String,
    },
    Deny {
        request_id: String,
    },
}

/// A request the host sends. Droid validates `id` as a string: a number is answered with
/// -32700 and no id.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RpcRequest {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub jsonrpc: &'static str,
    #[serde(rename = "factoryApiVersion")]
    pub factory_api_version: &'static str,
    pub id: String,
    pub method: &'static str,
    pub params: Value,
}

/// The host's answer to a request of Droid's (`droid.ask_user`, `droid.request_permission`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RpcResponse {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub jsonrpc: &'static str,
    #[serde(rename = "factoryApiVersion")]
    pub factory_api_version: &'static str,
    pub id: String,
    pub result: Value,
}

impl RpcRequest {
    pub fn new(id: u64, method: &'static str, params: Value) -> Self {
        Self {
            kind: "request",
            jsonrpc: JSONRPC,
            factory_api_version: FACTORY_API_VERSION,
            id: id.to_string(),
            method,
            params,
        }
    }
}

impl RpcResponse {
    pub fn new(id: &str, result: Value) -> Self {
        Self {
            kind: "response",
            jsonrpc: JSONRPC,
            factory_api_version: FACTORY_API_VERSION,
            id: id.to_owned(),
            result,
        }
    }
}

/// `machineId` of the initialize request: any non-empty name; Droid keeps it with the session.
const MACHINE_ID: &str = "droid-observatory";

/// The `droid.initialize_session` request that makes a hosted session the orchestrator of a
/// mission. In stream-jsonrpc mode the launch flags have no effect (D-033, D-034), so models,
/// efforts and autonomy travel here: `modelId` and `reasoningEffort` are the orchestrator's,
/// `missionSettings` those of the workers and validators, `decompSessionType` makes the session
/// an orchestrator (what `--mission` does on the command line). `session_id` is chosen by the
/// host, so the session file is known before Droid writes it.
pub fn encode_initialize(config: &LaunchConfig, session_id: &str, id: u64) -> RpcRequest {
    let mut mission_settings = json!({
        "workerModel": config.models.worker,
        "workerReasoningEffort": config.efforts.worker,
        "validationWorkerModel": config.models.validator,
        "validationWorkerReasoningEffort": config.efforts.validator,
    });
    // Left out unless set, so a request without them is what Droid's own defaults answer to.
    for (key, skipped) in [
        ("skipScrutiny", config.skip_scrutiny),
        ("skipUserTesting", config.skip_user_testing),
    ] {
        if skipped {
            mission_settings[key] = json!(true);
        }
    }
    let mut params = json!({
        "machineId": MACHINE_ID,
        "cwd": config.repo,
        "sessionId": session_id,
        "modelId": config.models.orchestrator,
        "reasoningEffort": config.efforts.orchestrator,
        "autonomyLevel": config.autonomy,
        "interactionMode": "mission",
        "decompSessionType": "orchestrator",
        "missionSettings": mission_settings,
    });
    if !config.tags.is_empty() {
        params["tags"] = config
            .tags
            .iter()
            .map(|tag| json!({ "name": tag }))
            .collect();
    }
    RpcRequest::new(id, method::INITIALIZE_SESSION, params)
}

/// The request a control sends, or `None` for the controls that answer a request of Droid's.
pub fn encode(control: &HostControl, id: u64) -> Option<RpcRequest> {
    match control {
        HostControl::Pause => Some(RpcRequest::new(id, method::INTERRUPT_SESSION, json!({}))),
        HostControl::Resume => Some(RpcRequest::new(
            id,
            method::ADD_USER_MESSAGE,
            json!({ "text": RESUME_TEXT }),
        )),
        HostControl::Message { text } => Some(RpcRequest::new(
            id,
            method::ADD_USER_MESSAGE,
            json!({ "text": text }),
        )),
        HostControl::Answer { .. } | HostControl::Approve { .. } | HostControl::Deny { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_carry_the_envelope_droid_requires() {
        let request = encode(&HostControl::Pause, 7).unwrap();
        let wire = serde_json::to_value(&request).unwrap();
        assert_eq!(
            wire,
            json!({
                "type": "request", "jsonrpc": "2.0", "factoryApiVersion": "1.0.0",
                "id": "7", "method": "droid.interrupt_session", "params": {},
            })
        );
    }

    #[test]
    fn resume_and_message_are_user_messages() {
        let resume = encode(&HostControl::Resume, 1).unwrap();
        assert_eq!(resume.method, method::ADD_USER_MESSAGE);
        assert_eq!(resume.params["text"], RESUME_TEXT);
        let message = encode(&HostControl::Message { text: "hi".into() }, 2).unwrap();
        assert_eq!(message.params, json!({ "text": "hi" }));
    }

    #[test]
    fn answers_and_approvals_are_not_requests() {
        for control in [
            HostControl::Answer {
                question_id: "q".into(),
                text: "t".into(),
            },
            HostControl::Approve {
                request_id: "r".into(),
            },
            HostControl::Deny {
                request_id: "r".into(),
            },
        ] {
            assert_eq!(encode(&control, 1), None);
        }
    }

    #[test]
    fn a_response_repeats_the_request_id() {
        let wire = serde_json::to_value(RpcResponse::new("req-1", json!({}))).unwrap();
        assert_eq!(wire["id"], "req-1");
        assert_eq!(wire["type"], "response");
    }
}
