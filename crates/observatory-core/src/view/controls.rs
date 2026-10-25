//! The control bar and the stall banner (FR-H1 to FR-H6): which controls a mission offers, in
//! which state, and what the stall flag says. Plain data; the UI renders it as it is.

use serde::{Deserialize, Serialize};

use super::hosted::{HostedState, HostedStatus, PermissionRow, QuestionRow};
use crate::control::stall::StallFlag;
use crate::control::{HumanAction, RunbookPrompt};
use crate::model::{Event, EventKind, FeatureId};

/// What an attached mission shows next to a control: nothing reaches the mission from the
/// app (D-008), so the person carries the text over (architecture §8).
pub const TERMINAL_INSTRUCTION: &str = "copy to the terminal running the mission";

/// Where the runbook the stall prompts name lives; the anchor of each prompt is appended.
pub const RUNBOOK_PATH: &str = "docs/runbooks/stalled-mission.md";

const SECONDS_PER_MINUTE: u64 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlMode {
    /// The app owns the process: controls reach it over stdin.
    Hosted,
    /// A session file is watched: a control is recorded and the text is shown to paste.
    Attached,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunState {
    Starting,
    Running,
    Paused,
    Exited,
    Attached,
}

impl RunState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Exited => "exited",
            Self::Attached => "attached",
        }
    }

    /// The kit's chip class for the state (`run`, `pause`, `done`).
    pub fn chip(self) -> &'static str {
        match self {
            Self::Running => "run",
            Self::Paused => "pause",
            Self::Starting | Self::Exited | Self::Attached => "done",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlBar {
    pub mode: ControlMode,
    pub state: RunState,
    pub pause_enabled: bool,
    pub resume_enabled: bool,
    pub message_enabled: bool,
    /// Set for an attached mission.
    pub terminal_instruction: Option<&'static str>,
    pub questions: Vec<QuestionRow>,
    pub permissions: Vec<PermissionRow>,
}

/// Whether the last pause or resume of the mission left it paused. The events are in time order.
fn paused_by(events: &[Event]) -> bool {
    events
        .iter()
        .rev()
        .find_map(|event| match event.kind {
            EventKind::MissionPaused => Some(true),
            EventKind::MissionResumed => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

impl ControlBar {
    /// The bar of a hosted mission. `events` are the mission's events, which say whether it is
    /// paused; `None` where they are not at hand, and both pause and resume stay available.
    pub fn hosted(status: &HostedStatus, events: Option<&[Event]>) -> Self {
        let paused = events.is_some_and(paused_by);
        let (state, live) = match status.state {
            HostedState::Starting => (RunState::Starting, false),
            HostedState::Exited { .. } => (RunState::Exited, false),
            HostedState::Running if paused => (RunState::Paused, true),
            HostedState::Running => (RunState::Running, true),
        };
        let unknown = events.is_none();
        Self {
            mode: ControlMode::Hosted,
            state,
            pause_enabled: live && (unknown || !paused),
            resume_enabled: live && (unknown || paused),
            message_enabled: live,
            terminal_instruction: None,
            questions: status.questions.clone(),
            permissions: status.permissions.clone(),
        }
    }

    /// The bar of an attached mission: every control records its event; the open questions are
    /// the ones the stream asked and nobody answered.
    pub fn attached(events: &[Event]) -> Self {
        let answered_after = |asked: &Event| {
            events.iter().any(|event| {
                matches!(
                    event.kind,
                    EventKind::QuestionAnswered | EventKind::IntentAnswered
                ) && event.at >= asked.at
                    && event.id != asked.id
            })
        };
        let questions = events
            .iter()
            .filter(|event| event.kind == EventKind::QuestionAsked && !answered_after(event))
            .map(|event| QuestionRow {
                event_id: event.id.to_string(),
                prompts: Vec::new(),
            })
            .collect();
        Self {
            mode: ControlMode::Attached,
            state: RunState::Attached,
            pause_enabled: true,
            resume_enabled: true,
            message_enabled: true,
            terminal_instruction: Some(TERMINAL_INSTRUCTION),
            questions,
            permissions: Vec::new(),
        }
    }
}

/// The text to paste into the terminal that runs an attached mission for `action`; `text` is
/// what the person typed.
pub fn terminal_text(action: &HumanAction, text: &str) -> String {
    match action {
        HumanAction::Pause => "Pause the mission.".to_owned(),
        HumanAction::Resume => "Resume the mission.".to_owned(),
        HumanAction::Message { .. } | HumanAction::Answer { .. } => text.to_owned(),
        HumanAction::MarkSkipped { feature_id, .. } => {
            let mut instruction = format!(
                "Skip feature {feature_id}: mark it skipped and do not assign it again in this mission."
            );
            if !text.is_empty() {
                instruction.push_str(" Reason: ");
                instruction.push_str(text);
            }
            instruction
        }
        HumanAction::MarkComplete { feature_id } => {
            format!("Feature {feature_id} is complete; mark it done in the plan.")
        }
        HumanAction::Approve { .. } | HumanAction::Deny { .. } => {
            "Answer the permission request in the terminal that runs the mission.".to_owned()
        }
    }
}

/// A stall flag as it travels to the window and into the log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StallFlagView {
    pub feature_id: FeatureId,
    pub idle_secs: u64,
    pub prompt: String,
    pub anchor: String,
}

impl From<&StallFlag> for StallFlagView {
    fn from(flag: &StallFlag) -> Self {
        let RunbookPrompt { prompt, anchor } = flag.runbook;
        Self {
            feature_id: flag.feature_id.clone(),
            idle_secs: flag.idle_for.as_secs(),
            prompt: prompt.to_owned(),
            anchor: anchor.to_owned(),
        }
    }
}

/// What `human_action` answers: the `human_actions` row and the event it wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRecorded {
    pub action_id: u64,
    pub event_id: String,
}

/// The flags of one mission, as the window carries them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StallBatch {
    pub mission_id: String,
    pub flags: Vec<StallFlagView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StallRow {
    pub feature_id: FeatureId,
    pub idle_minutes: u64,
    pub prompt: String,
    pub link: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StallBanner {
    pub rows: Vec<StallRow>,
}

impl StallBanner {
    pub fn of(flags: &[StallFlagView]) -> Self {
        Self {
            rows: flags
                .iter()
                .map(|flag| StallRow {
                    feature_id: flag.feature_id.clone(),
                    idle_minutes: flag.idle_secs / SECONDS_PER_MINUTE,
                    prompt: flag.prompt.clone(),
                    link: format!("{RUNBOOK_PATH}{}", flag.anchor),
                })
                .collect(),
        }
    }
}
