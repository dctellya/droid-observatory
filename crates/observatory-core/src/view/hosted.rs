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

/// The `hosted_status` payload and the `host_status` answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStatus {
    pub launch_id: String,
    pub state: HostedState,
    pub questions: Vec<QuestionRow>,
    pub acks: Vec<AckRow>,
}

impl HostedStatus {
    pub fn starting(launch_id: &str) -> Self {
        Self {
            launch_id: launch_id.to_owned(),
            state: HostedState::Starting,
            questions: Vec::new(),
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
