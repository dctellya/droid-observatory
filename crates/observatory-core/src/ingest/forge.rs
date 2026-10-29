//! The forge a mission's pull request lives on (architecture §4.6). `Forge` is what the gate
//! lane needs from it; `GitHubClient` is the one implementation. The types here carry no HTTP,
//! so the store and the views read them without the `github` feature.

use serde::{Deserialize, Serialize};

use crate::config::MergePolicy as ConfigMergePolicy;
use crate::model::{Event, EventId, MilestoneId, Timestamp};

/// The pull request of a mission branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pr {
    pub number: u64,
    pub head_sha: String,
    pub head: String,
    pub base: String,
    /// The API URL of the pull request, the pointer of the events that observe it.
    pub url: String,
    pub created_at: Timestamp,
    pub state: PrState,
    /// Auto-merge is enabled on the PR: GitHub merges it when the rules allow, nobody presses.
    pub auto_merge: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrState {
    Open,
    Merged {
        at: Timestamp,
        merge_sha: Option<String>,
        /// The login that merged it; the pulls list does not say, the single-PR endpoint does.
        merged_by: Option<String>,
    },
    /// Closed without a merge.
    Closed,
}

impl PrState {
    /// The text of `prs.state`.
    pub fn wire(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Merged { .. } => "merged",
            Self::Closed => "closed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub source: CheckSource,
    pub conclusion: CheckConclusion,
    pub url: String,
    /// When the forge last updated the run or status; `None` when it did not say.
    pub at: Option<Timestamp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckSource {
    ActionsRun { run_id: u64 },
    CommitStatus { context: String },
}

impl CheckSource {
    /// The text of `checks.source`.
    pub fn wire(&self) -> &'static str {
        match self {
            Self::ActionsRun { .. } => "actions_run",
            Self::CommitStatus { .. } => "commit_status",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckConclusion {
    Queued,
    Running,
    Passed,
    Failed,
    /// A required check no run or status reports yet.
    RequiredMissing,
}

impl CheckConclusion {
    /// The text of `checks.conclusion`.
    pub fn wire(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::RequiredMissing => "required_missing",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ForgeError {
    /// The token cannot do this: a merge needs `pull_requests:write`.
    #[error(
        "Merge of PR #{pr} into {target} refused: token lacks {missing_permission}. Merge in GitHub; the badge updates from the next poll."
    )]
    Forbidden {
        pr: u64,
        target: String,
        missing_permission: &'static str,
    },
    #[error("GitHub rate limit reached (resets at {reset_at:?})")]
    RateLimited { reset_at: Option<u64> },
    #[error("GitHub answered {status} for {url}")]
    Http { status: u16, url: String },
    #[error("{0}")]
    Other(String),
}

/// How a pull request is merged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    /// The text of `merge_method` in the merge request and of `allowed_merge_methods` in a rule.
    pub fn wire(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        [Self::Merge, Self::Squash, Self::Rebase]
            .into_iter()
            .find(|method| method.wire() == text)
    }
}

/// Which rule decides that a person merges (FR-G4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAttribution {
    Ruleset {
        id: u64,
        name: String,
    },
    BranchProtection,
    /// `[github].merge_policy` was set to something other than `auto`; carries its text.
    ConfigOverride(String),
    NoRule,
}

/// What the target branch asks of a merge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergePolicy {
    pub requires_human: bool,
    pub allowed_methods: Vec<MergeMethod>,
    pub decided_by: RuleAttribution,
    /// The status contexts the rules require, by name.
    pub required_checks: Vec<String>,
}

impl MergePolicy {
    /// The policy `[github].merge_policy` makes of what the forge said. `auto` keeps the forge's
    /// answer unless auto-merge is enabled on the PR; `human` and `policy` replace who decides and
    /// leave the required checks alone.
    #[must_use]
    pub fn with_override(mut self, config: ConfigMergePolicy, auto_merge: bool) -> Self {
        match config {
            ConfigMergePolicy::Auto => self.requires_human &= !auto_merge,
            ConfigMergePolicy::Human => {
                self.requires_human = true;
                self.decided_by = RuleAttribution::ConfigOverride("human".to_owned());
            }
            ConfigMergePolicy::Policy => {
                self.requires_human = false;
                self.decided_by = RuleAttribution::ConfigOverride("policy".to_owned());
            }
        }
        self
    }
}

/// A review of a pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    pub id: u64,
    /// `approved`, `commented`, `changes_requested`, `dismissed` or `pending`.
    pub state: String,
    pub url: String,
}

/// A provenance attestation for an artifact the PR's runs produced. The forge returns the bundle
/// without verifying it here, so `verified` is false until something checks the signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attestation {
    pub id: String,
    /// The digest of the attested artifact (`sha256:...`).
    pub subject: String,
    pub verified: bool,
    pub url: String,
}

/// The answer to a merge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Merged {
    pub sha: String,
}

/// What the gate lane reads from a forge: the pull request of a branch, the results of its
/// checks and the rules around its merge. Check results come from Actions runs and commit
/// statuses only (planning note 6: the token has no Checks permission).
pub trait Forge {
    /// The newest pull request from `branch` into `target`, whatever its state.
    fn pr(&self, branch: &str, target: &str) -> Result<Option<Pr>, ForgeError>;

    /// The results of the checks on the head of `pr`, one per workflow or status context.
    fn checks(&self, pr: &Pr) -> Result<Vec<Check>, ForgeError>;

    /// The rules a merge into `target` is subject to: rulesets first, then branch protection.
    fn policy(&self, target: &str) -> Result<MergePolicy, ForgeError>;

    fn reviews(&self, pr: &Pr) -> Result<Vec<Review>, ForgeError>;

    /// Attestations for the artifacts of the runs on the head of `pr`; none is a normal answer.
    fn attestations(&self, pr: &Pr) -> Result<Vec<Attestation>, ForgeError>;

    /// The login the token belongs to; `None` when the forge does not say (no token).
    fn token_owner(&self) -> Result<Option<String>, ForgeError>;

    /// The one write: merges `pr`. A token without `pull_requests:write` gets `Forbidden`.
    fn merge(&self, pr: &Pr, method: MergeMethod) -> Result<Merged, ForgeError>;
}

/// A required check and what the forge reports for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredCheck {
    pub name: String,
    pub conclusion: CheckConclusion,
}

/// What one poll read around the merge: the policy in force, the required checks and the
/// counts the Gate header shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateFacts {
    pub policy: MergePolicy,
    pub required_checks: Vec<RequiredCheck>,
    pub reviews: u64,
    pub attestation_ids: Vec<String>,
    /// The pull request is open: a merged or closed one offers no Merge.
    pub pr_open: bool,
}

/// What one poll of the forge saw, and the events it adds.
#[derive(Clone, Debug, PartialEq)]
pub struct Poll {
    pub observed_at: Timestamp,
    /// The mission branch's pull request, as it is now.
    pub pr: Option<Pr>,
    pub checks: Vec<Check>,
    pub reviews: Vec<Review>,
    pub attestations: Vec<Attestation>,
    /// `None` when the PR does not exist yet.
    pub gate: Option<GateFacts>,
    /// New events only: a PR already announced yields none.
    pub events: Vec<Event>,
    /// The fix-cycle windows those events open and close.
    pub windows: Vec<WindowChange>,
}

/// A `gate` rework window (FR-G7): the red check opens it on the affected card, the green run of
/// the same check closes it. The store applies it after the events it names are recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowChange {
    Open {
        /// The `check.failed` event that opens the window.
        event: EventId,
        pr_number: u64,
        check: String,
        card: MilestoneId,
    },
    Close {
        /// The `check.passed` event that closes the window.
        event: EventId,
        pr_number: u64,
        check: String,
    },
}
