use serde::{Deserialize, Serialize};

use crate::model::{Event, EventKind, Role, SourcePtr};

/// State of the PR's checks, the header of the Gate lane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateBadge {
    /// No PR yet: the lane shows the gate commands of the Integrate phase instead.
    #[default]
    Pending,
    ChecksRunning,
    CiRed,
    CiGreen,
    /// A required check is missing or a ruleset blocks the merge.
    Blocked,
    /// `handed_to_cd` by a person.
    HumanMerged,
    /// `handed_to_cd` by the merge policy.
    PolicyMerged,
}

impl GateBadge {
    pub const ALL: [GateBadge; 7] = [
        GateBadge::Pending,
        GateBadge::ChecksRunning,
        GateBadge::CiRed,
        GateBadge::CiGreen,
        GateBadge::Blocked,
        GateBadge::HumanMerged,
        GateBadge::PolicyMerged,
    ];

    /// The wire text (`ci_green`).
    pub fn as_str(self) -> &'static str {
        match self {
            GateBadge::Pending => "pending",
            GateBadge::ChecksRunning => "checks_running",
            GateBadge::CiRed => "ci_red",
            GateBadge::CiGreen => "ci_green",
            GateBadge::Blocked => "blocked",
            GateBadge::HumanMerged => "human_merged",
            GateBadge::PolicyMerged => "policy_merged",
        }
    }

    pub fn parse(text: &str) -> Option<GateBadge> {
        GateBadge::ALL
            .into_iter()
            .find(|badge| badge.as_str() == text)
    }
}

/// Verdict of the last `gate_command.*` run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateVerdict {
    Passed,
    Failed,
}

/// The Gate lane header card.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateGroup {
    pub badge: GateBadge,
    pub pr_number: Option<u64>,
    /// `gate_command.run` events seen, shown before a PR exists.
    pub gate_runs: u64,
    pub last_gate_command: Option<GateVerdict>,
}

/// Text of the Gate lane before a PR exists (PRD §8).
const PENDING_TEXT: &str = "gate: pending (no PR)";
const CYCLE_CLOCK: &str = "engineering cycle time";
const LEAD_CLOCK: &str = "lead time to production: delivery platform's clock";
/// The placeholder of the branch pattern that stands for the mission name.
const NAME_PLACEHOLDER: &str = "<name>";

/// One of the two clocks the board names. `value` stays empty until M5 computes it (the lead
/// time to production is never computed here).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockLabel {
    pub label: String,
    pub value: Option<String>,
}

/// What the Gate lane shows while no PR exists: the pending state, the mission branch, the
/// verdict of the last `gate_command.*` run of Integrate and the two clock labels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatePlaceholder {
    pub text: String,
    pub branch: String,
    pub gate_runs: u64,
    pub last_gate_command: Option<GateVerdict>,
    pub clocks: [ClockLabel; 2],
}

/// The mission branch for a mission name (`[mission].branch_pattern`).
pub fn mission_branch(pattern: &str, name: &str) -> String {
    pattern.replace(NAME_PLACEHOLDER, name)
}

impl GateGroup {
    /// The placeholder, while the gate is pending and no PR number is known.
    pub fn placeholder(&self, branch: &str) -> Option<GatePlaceholder> {
        if self.badge != GateBadge::Pending || self.pr_number.is_some() {
            return None;
        }
        let clock = |label: &str| ClockLabel {
            label: label.to_owned(),
            value: None,
        };
        Some(GatePlaceholder {
            text: PENDING_TEXT.to_owned(),
            branch: branch.to_owned(),
            gate_runs: self.gate_runs,
            last_gate_command: self.last_gate_command,
            clocks: [clock(CYCLE_CLOCK), clock(LEAD_CLOCK)],
        })
    }

    pub(super) fn apply(&mut self, event: &Event) {
        match &event.kind {
            EventKind::GateCommandRun => self.gate_runs += 1,
            EventKind::GateCommandPassed => self.last_gate_command = Some(GateVerdict::Passed),
            EventKind::GateCommandFailed => self.last_gate_command = Some(GateVerdict::Failed),
            EventKind::PrOpened => {
                self.badge = GateBadge::ChecksRunning;
                self.pr_number = pr_number_of(&event.source).or(self.pr_number);
            }
            EventKind::CheckStarted => self.badge = GateBadge::ChecksRunning,
            EventKind::CheckFailed => self.badge = GateBadge::CiRed,
            EventKind::CheckPassed => self.badge = GateBadge::CiGreen,
            EventKind::CheckRequiredMissing | EventKind::RulesetBlocked => {
                self.badge = GateBadge::Blocked;
            }
            EventKind::HandedToCd { pr_number, .. } => {
                self.badge = if event.role == Role::Human {
                    GateBadge::HumanMerged
                } else {
                    GateBadge::PolicyMerged
                };
                self.pr_number = Some(*pr_number);
            }
            _ => {}
        }
    }
}

/// The PR number when the pointer is a GitHub pull URL (`.../pulls/41`).
fn pr_number_of(source: &SourcePtr) -> Option<u64> {
    let SourcePtr::GitHub { url } = source else {
        return None;
    };
    url.rsplit('/').next()?.parse().ok()
}
