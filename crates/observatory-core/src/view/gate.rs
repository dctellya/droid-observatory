use serde::{Deserialize, Serialize};

use crate::ingest::forge::{CheckConclusion, GateFacts, RequiredCheck, RuleAttribution};
use crate::model::{CycleTemplate, Event, EventKind, MissionId, Role, SourcePtr};

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

    /// The text of the badge on the Gate header.
    pub fn label(self) -> &'static str {
        match self {
            GateBadge::Pending => "pending",
            GateBadge::ChecksRunning => "checks running",
            GateBadge::CiRed => "CI red",
            GateBadge::CiGreen => "CI green",
            GateBadge::Blocked => "blocked",
            GateBadge::HumanMerged | GateBadge::PolicyMerged => "handed to CD",
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
    /// The rule that decides whether a person merges, as the last poll read it.
    #[serde(default)]
    pub decided_by: Option<RuleAttribution>,
    #[serde(default)]
    pub requires_human: bool,
    /// Whether the Merge button is offered: a person decides, the PR is open and every
    /// required check is green.
    #[serde(default)]
    pub merge_offered: bool,
    #[serde(default)]
    pub required_checks: Vec<RequiredCheck>,
    #[serde(default)]
    pub reviews: u64,
    #[serde(default)]
    pub attestation_ids: Vec<String>,
    /// The merge commit, once `handed_to_cd` is observed.
    #[serde(default)]
    pub merge_sha: Option<String>,
    /// The branch the PR merges into, as the last poll read it.
    #[serde(default)]
    pub target: Option<String>,
}

/// Text of the Gate lane before a PR exists (PRD §8).
const PENDING_TEXT: &str = "gate: pending (no PR)";
/// The placeholder of the branch pattern that stands for the mission name.
const NAME_PLACEHOLDER: &str = "<name>";

/// One of the clocks the template names for the final lane. `value` stays empty until M5 computes it (the lead
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
    pub clocks: Vec<ClockLabel>,
}

/// What `gate_changed` tells the window: the Gate header of one mission as the last poll read it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GateChanged {
    pub mission_id: MissionId,
    pub gate: GateGroup,
}

/// Length of the merge sha the badge shows.
const SHORT_SHA_LEN: usize = 7;
const MERGE_LABEL: &str = "Merge";

fn short_sha(sha: &str) -> &str {
    sha.get(..SHORT_SHA_LEN).unwrap_or(sha)
}

/// A required check and what the poll reads for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateCheckRow {
    pub name: String,
    /// The `checks.conclusion` text (`passed`, `failed`, `running`, ...).
    pub conclusion: String,
}

/// The Merge button: offered only when a person decides and every required check is green.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeOffer {
    pub pr_number: u64,
    /// The branch the PR merges into.
    pub target: String,
    pub label: String,
}

/// The confirmation the Merge button opens: it names the PR and the target before any request is
/// sent (FR-G5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeConfirmation {
    pub text: String,
    pub confirm: String,
    pub cancel: String,
}

impl MergeOffer {
    pub fn confirmation(&self) -> MergeConfirmation {
        MergeConfirmation {
            text: format!("Merge PR #{} into {}?", self.pr_number, self.target),
            confirm: "Confirm merge".to_owned(),
            cancel: "Cancel".to_owned(),
        }
    }
}

/// What the `merge` command answers: the state the press's row took and the text to show.
pub type MergePressed = crate::control::merge::MergeOutcome;

/// One clock the template names for the final lane, and what the Gate header says about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateClockLine {
    pub label: String,
    pub note: String,
}

/// The Gate group as the lane draws it (FR-B3): checks, reviews, ruleset and attestations, each
/// with its text, the merge offer, the `handed to CD` reading and the two clocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateGroupView {
    pub badge: GateBadge,
    pub badge_text: String,
    pub pr_number: Option<u64>,
    pub checks: Vec<GateCheckRow>,
    pub reviews: u64,
    pub reviews_text: String,
    /// The ruleset's name, when a ruleset decides who merges.
    pub ruleset: Option<String>,
    pub ruleset_text: String,
    pub attestations: Vec<String>,
    pub attestations_text: String,
    pub merge: Option<MergeOffer>,
    /// Set once the merge is observed.
    pub handed_to_cd: Option<String>,
    pub clocks: Vec<GateClockLine>,
}

/// The mission branch for a mission name (`[mission].branch_pattern`).
pub fn mission_branch(pattern: &str, name: &str) -> String {
    pattern.replace(NAME_PLACEHOLDER, name)
}

impl GateGroup {
    /// The header of the Gate lane after `events`, in time order.
    pub fn of(events: &[Event]) -> Self {
        let mut group = Self::default();
        for event in events {
            group.apply(event);
        }
        group
    }

    /// The header with what the last poll read around the merge.
    #[must_use]
    pub fn with_facts(mut self, facts: &GateFacts) -> Self {
        self.decided_by = Some(facts.policy.decided_by.clone());
        self.requires_human = facts.policy.requires_human;
        self.required_checks.clone_from(&facts.required_checks);
        self.reviews = facts.reviews;
        self.attestation_ids.clone_from(&facts.attestation_ids);
        self.target = Some(facts.target.clone()).filter(|target| !target.is_empty());
        self.merge_offered = facts.pr_open
            && facts.policy.requires_human
            && self.badge == GateBadge::CiGreen
            && facts
                .required_checks
                .iter()
                .all(|check| check.conclusion == CheckConclusion::Passed);
        self
    }

    /// The header of `self` (read from the events a scrubber keeps) with what the last poll read
    /// around the merge. Merge stays withheld unless these events are green too.
    #[must_use]
    pub fn with_polled(mut self, polled: &GateGroup) -> Self {
        self.decided_by.clone_from(&polled.decided_by);
        self.requires_human = polled.requires_human;
        self.required_checks.clone_from(&polled.required_checks);
        self.reviews = polled.reviews;
        self.target.clone_from(&polled.target);
        if self.attestation_ids.is_empty() {
            self.attestation_ids.clone_from(&polled.attestation_ids);
        }
        self.merge_offered = polled.merge_offered && self.badge == GateBadge::CiGreen;
        self
    }

    /// What the Gate lane draws for this header: the group, the merge offer, the
    /// `handed to CD` reading and the clocks the template names for its final lane.
    pub fn view(&self, template: &CycleTemplate) -> GateGroupView {
        let merged_by = match self.badge {
            GateBadge::HumanMerged => Some("human"),
            GateBadge::PolicyMerged => Some("policy"),
            _ => None,
        };
        let handed_to_cd = merged_by.zip(self.pr_number).map(|(by, pr)| {
            let sha = self.merge_sha.as_deref().map_or("", short_sha);
            let attestations = if self.attestation_ids.is_empty() {
                "none".to_owned()
            } else {
                self.attestation_ids.join(", ")
            };
            format!("handed to CD: {sha} · PR #{pr} · attestations {attestations} · merged by {by}")
        });
        let merge = self
            .merge_offered
            .then_some(self.pr_number.zip(self.target.clone()))
            .flatten()
            .map(|(pr_number, target)| MergeOffer {
                pr_number,
                target,
                label: MERGE_LABEL.to_owned(),
            });
        let (ruleset, ruleset_text) = match &self.decided_by {
            Some(RuleAttribution::Ruleset { name, .. }) => {
                (Some(name.clone()), format!("ruleset: {name}"))
            }
            Some(RuleAttribution::ConfigOverride(text)) => {
                (None, format!("decided by: [github].merge_policy = {text}"))
            }
            Some(RuleAttribution::BranchProtection) => {
                (None, "decided by: branch protection".to_owned())
            }
            Some(RuleAttribution::NoRule) => (None, "decided by: no rule".to_owned()),
            None => (None, "ruleset: not read yet".to_owned()),
        };
        let sha = self.merge_sha.as_deref().map(short_sha);
        let clocks = template
            .lane(template.final_lane())
            .into_iter()
            .flat_map(|lane| &lane.clocks)
            .filter_map(|id| template.clocks.iter().find(|clock| clock.id == *id))
            .map(|clock| GateClockLine {
                label: clock.label.clone(),
                note: match (&clock.end, sha) {
                    (Some(end), _) => format!("closed at {end}"),
                    (None, Some(sha)) => format!("starts at {sha}; not computed here"),
                    (None, None) => "starts at the merge sha; not computed here".to_owned(),
                },
            })
            .collect();
        GateGroupView {
            badge: self.badge,
            badge_text: self.badge.label().to_owned(),
            pr_number: self.pr_number,
            checks: self
                .required_checks
                .iter()
                .map(|check| GateCheckRow {
                    name: check.name.clone(),
                    conclusion: check.conclusion.wire().to_owned(),
                })
                .collect(),
            reviews: self.reviews,
            reviews_text: format!("reviews: {}", self.reviews),
            ruleset,
            ruleset_text,
            attestations: self.attestation_ids.clone(),
            attestations_text: format!("attestations: {}", self.attestation_ids.len()),
            merge,
            handed_to_cd,
            clocks,
        }
    }

    /// The placeholder, while the gate is pending and no PR number is known.
    pub fn placeholder(&self, branch: &str, template: &CycleTemplate) -> Option<GatePlaceholder> {
        if self.badge != GateBadge::Pending || self.pr_number.is_some() {
            return None;
        }
        let clocks = template
            .lane(template.final_lane())
            .into_iter()
            .flat_map(|lane| &lane.clocks)
            .filter_map(|id| template.clocks.iter().find(|clock| clock.id == *id))
            .map(|clock| ClockLabel {
                label: clock.label.clone(),
                value: None,
            })
            .collect();
        Some(GatePlaceholder {
            text: PENDING_TEXT.to_owned(),
            branch: branch.to_owned(),
            gate_runs: self.gate_runs,
            last_gate_command: self.last_gate_command,
            clocks,
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
            EventKind::HandedToCd {
                merge_sha,
                pr_number,
                attestation_ids,
            } => {
                self.merge_sha = Some(merge_sha.clone());
                self.attestation_ids.clone_from(attestation_ids);
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
