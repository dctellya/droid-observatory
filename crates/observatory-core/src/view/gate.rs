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

impl GateGroup {
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
