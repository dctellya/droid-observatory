//! The launch screen (FR-L3): the form the human fills, its validation, and the `droid exec`
//! command a valid form stands for. The UI renders `LaunchForm` and shows `LaunchError`s; the
//! shell receives the `LaunchConfig`.

use serde::{Deserialize, Serialize};

use crate::config::MissionConfig;

/// The autonomy levels a hosted mission may run at. Droid's mission mode needs `high` (D-033).
pub const AUTONOMY_LEVELS: [&str; 1] = ["high"];

/// One value per role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerRole<T> {
    pub orchestrator: T,
    pub worker: T,
    pub validator: T,
}

/// The fields of the form, in the order they are shown and validated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchField {
    Repo,
    MissionFile,
    Branch,
    OrchestratorModel,
    WorkerModel,
    ValidatorModel,
    OrchestratorEffort,
    WorkerEffort,
    ValidatorEffort,
    Autonomy,
    Budget,
}

impl LaunchField {
    pub const ALL: [LaunchField; 11] = [
        Self::Repo,
        Self::MissionFile,
        Self::Branch,
        Self::OrchestratorModel,
        Self::WorkerModel,
        Self::ValidatorModel,
        Self::OrchestratorEffort,
        Self::WorkerEffort,
        Self::ValidatorEffort,
        Self::Autonomy,
        Self::Budget,
    ];

    /// The label shown on the form and named by the validation message.
    pub fn label(self) -> &'static str {
        match self {
            Self::Repo => "Repo",
            Self::MissionFile => "Mission file",
            Self::Branch => "Mission branch name",
            Self::OrchestratorModel => "Orchestrator model",
            Self::WorkerModel => "Worker model",
            Self::ValidatorModel => "Validator model",
            Self::OrchestratorEffort => "Orchestrator reasoning effort",
            Self::WorkerEffort => "Worker reasoning effort",
            Self::ValidatorEffort => "Validator reasoning effort",
            Self::Autonomy => "Autonomy",
            Self::Budget => "Budget (optional)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchError {
    pub field: LaunchField,
    pub message: String,
}

/// The form as typed: every field is text.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LaunchForm {
    pub repo: String,
    pub mission_file: String,
    pub branch: String,
    pub orchestrator_model: String,
    pub worker_model: String,
    pub validator_model: String,
    pub orchestrator_effort: String,
    pub worker_effort: String,
    pub validator_effort: String,
    pub autonomy: String,
    pub budget: String,
}

/// A form that passed validation: what the shell launches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub repo: String,
    pub mission_file: String,
    pub branch: String,
    pub models: PerRole<String>,
    pub efforts: PerRole<String>,
    pub autonomy: String,
    pub budget_usd: Option<f64>,
}

impl LaunchForm {
    /// Models, efforts and autonomy from `[mission]`; repo, mission file, branch and budget empty.
    pub fn defaults(mission: &MissionConfig) -> Self {
        Self {
            orchestrator_model: mission.orchestrator_model.clone(),
            worker_model: mission.worker_model.clone(),
            validator_model: mission.validator_model.clone(),
            orchestrator_effort: mission.orchestrator_reasoning_effort.clone(),
            worker_effort: mission.worker_reasoning_effort.clone(),
            validator_effort: mission.validator_reasoning_effort.clone(),
            autonomy: mission.autonomy.clone(),
            ..Self::default()
        }
    }

    pub fn value(&self, field: LaunchField) -> &str {
        match field {
            LaunchField::Repo => &self.repo,
            LaunchField::MissionFile => &self.mission_file,
            LaunchField::Branch => &self.branch,
            LaunchField::OrchestratorModel => &self.orchestrator_model,
            LaunchField::WorkerModel => &self.worker_model,
            LaunchField::ValidatorModel => &self.validator_model,
            LaunchField::OrchestratorEffort => &self.orchestrator_effort,
            LaunchField::WorkerEffort => &self.worker_effort,
            LaunchField::ValidatorEffort => &self.validator_effort,
            LaunchField::Autonomy => &self.autonomy,
            LaunchField::Budget => &self.budget,
        }
    }

    pub fn set(&mut self, field: LaunchField, value: String) {
        *match field {
            LaunchField::Repo => &mut self.repo,
            LaunchField::MissionFile => &mut self.mission_file,
            LaunchField::Branch => &mut self.branch,
            LaunchField::OrchestratorModel => &mut self.orchestrator_model,
            LaunchField::WorkerModel => &mut self.worker_model,
            LaunchField::ValidatorModel => &mut self.validator_model,
            LaunchField::OrchestratorEffort => &mut self.orchestrator_effort,
            LaunchField::WorkerEffort => &mut self.worker_effort,
            LaunchField::ValidatorEffort => &mut self.validator_effort,
            LaunchField::Autonomy => &mut self.autonomy,
            LaunchField::Budget => &mut self.budget,
        } = value;
    }

    /// Every field but the budget must hold something other than spaces; the budget is empty
    /// or a positive number; the autonomy is one a mission can run at. All errors are returned,
    /// in field order, so the screen can mark each one.
    pub fn validate(&self) -> Result<LaunchConfig, Vec<LaunchError>> {
        let mut errors = Vec::new();
        for field in LaunchField::ALL {
            if field == LaunchField::Budget {
                continue;
            }
            if self.value(field).trim().is_empty() {
                errors.push(LaunchError {
                    field,
                    message: format!("{} is required", field.label()),
                });
            }
        }
        if !self.autonomy.trim().is_empty() && !AUTONOMY_LEVELS.contains(&self.autonomy.trim()) {
            errors.push(LaunchError {
                field: LaunchField::Autonomy,
                message: format!(
                    "{} must be one of {}: a mission needs it",
                    LaunchField::Autonomy.label(),
                    AUTONOMY_LEVELS.join(", ")
                ),
            });
        }
        let budget = match self.budget.trim() {
            "" => None,
            text => match text.parse::<f64>() {
                Ok(value) if value.is_finite() && value > 0.0 => Some(value),
                _ => {
                    errors.push(LaunchError {
                        field: LaunchField::Budget,
                        message: format!(
                            "{} must be a number above 0",
                            LaunchField::Budget.label()
                        ),
                    });
                    None
                }
            },
        };
        if !errors.is_empty() {
            errors.sort_by_key(|error| {
                LaunchField::ALL
                    .iter()
                    .position(|field| *field == error.field)
            });
            return Err(errors);
        }
        let text = |field| self.value(field).trim().to_owned();
        Ok(LaunchConfig {
            repo: text(LaunchField::Repo),
            mission_file: text(LaunchField::MissionFile),
            branch: text(LaunchField::Branch),
            models: PerRole {
                orchestrator: text(LaunchField::OrchestratorModel),
                worker: text(LaunchField::WorkerModel),
                validator: text(LaunchField::ValidatorModel),
            },
            efforts: PerRole {
                orchestrator: text(LaunchField::OrchestratorEffort),
                worker: text(LaunchField::WorkerEffort),
                validator: text(LaunchField::ValidatorEffort),
            },
            autonomy: text(LaunchField::Autonomy),
            budget_usd: budget,
        })
    }
}

impl LaunchConfig {
    /// The arguments after `droid`. In stream-jsonrpc mode Droid 0.247.0 rejects `--mission`,
    /// `-f` and the per-role mission flags and ignores `-m`, `--auto` and `-r` (D-034): the
    /// session is configured by the `droid.initialize_session` request and the mission file is
    /// sent as the first message. The flags that remain keep the process line readable.
    pub fn command_args(&self) -> Vec<String> {
        [
            "exec",
            "--input-format",
            "stream-jsonrpc",
            "--output-format",
            "stream-jsonrpc",
            "--auto",
            &self.autonomy,
            "--model",
            &self.models.orchestrator,
            "--reasoning-effort",
            &self.efforts.orchestrator,
        ]
        .map(str::to_owned)
        .to_vec()
    }
}

/// A version 4 UUID built from 16 random bytes: the id of a hosted session, chosen by the app so
/// that attach can be told to skip the session file before Droid writes it.
pub fn session_uuid(mut bytes: [u8; 16]) -> String {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}
