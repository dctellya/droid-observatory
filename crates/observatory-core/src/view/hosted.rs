//! What the screen shows of a hosted run: its state, the questions waiting for the human and
//! the acknowledgement of each control. Plain data, so the UI reads it in wasm; the shell fills
//! it from the control channel and the open requests.

use serde::{Deserialize, Serialize};

use super::launch::LaunchConfig;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum HostedState {
    /// Spawned; Droid has not answered `droid.initialize_session`.
    Starting,
    Running,
    Exited {
        code: Option<i32>,
    },
}

/// Droid's answer to one request of the host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AckRow {
    pub request_id: String,
    pub method: String,
    pub accepted: bool,
    /// Why Droid refused the request.
    pub message: Option<String>,
}

/// A `droid.ask_user` question that waits, by the id of its `QuestionAsked` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionRow {
    pub event_id: String,
    pub prompts: Vec<String>,
}

/// A `droid.request_permission` request that waits, with the answers Droid accepts for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRow {
    /// The JSON-RPC id of Droid's request: what the host answers.
    pub request_id: String,
    /// The `permission.requested` event the request became; what an approve or deny points at.
    pub event_id: String,
    pub tools: Vec<String>,
    pub can_approve: bool,
    pub can_deny: bool,
}

/// The answer of `host_launch`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launched {
    pub launch_id: String,
    /// The session Droid is told to use; the mission id of the board is the same string.
    pub session_id: String,
    pub spool: String,
    pub pid: u32,
    /// `droid` and its arguments as spawned.
    pub command: String,
}

/// The answer of `host_control`: the id of the JSON-RPC message written to the process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSent {
    pub request_id: String,
    pub method: String,
}

/// The `hosted_status` payload and the `host_status` answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStatus {
    pub launch_id: String,
    pub state: HostedState,
    pub questions: Vec<QuestionRow>,
    pub permissions: Vec<PermissionRow>,
    pub acks: Vec<AckRow>,
}

impl HostedStatus {
    pub fn starting(launch_id: &str) -> Self {
        Self {
            launch_id: launch_id.to_owned(),
            state: HostedState::Starting,
            questions: Vec::new(),
            permissions: Vec::new(),
            acks: Vec::new(),
        }
    }

    pub fn record_ack(&mut self, ack: AckRow) {
        self.acks.push(ack);
    }

    /// Droid accepted `droid.initialize_session`.
    pub fn initialised(&mut self) {
        if self.state == HostedState::Starting {
            self.state = HostedState::Running;
        }
    }

    pub fn exited(&mut self, code: Option<i32>) {
        self.state = HostedState::Exited { code };
    }
}

impl LaunchConfig {
    /// The first user message of the hosted session: the mission file as it is, then the
    /// facts of the form that the file does not carry. `--mission` would take the file as a
    /// prompt; stream-jsonrpc takes it as a message (D-034).
    pub fn first_message(&self, mission_text: &str) -> String {
        let mut message = mission_text.trim_end().to_owned();
        message.push_str(&format!("\n\nMission branch name: {}", self.branch));
        if let Some(budget) = self.budget_usd {
            message.push_str(&format!(
                "\nBudget: stop before spending more than ${budget:.2}."
            ));
        }
        message
    }
}
