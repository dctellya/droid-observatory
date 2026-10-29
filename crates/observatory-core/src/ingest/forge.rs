//! The forge a mission's pull request lives on (architecture §4.6). `Forge` is what the gate
//! lane needs from it; `GitHubClient` is the one implementation. The types here carry no HTTP,
//! so the store and the views read them without the `github` feature.

use serde::{Deserialize, Serialize};

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrState {
    Open,
    Merged {
        at: Timestamp,
        merge_sha: Option<String>,
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

/// What the gate lane reads from a forge: the pull request of a branch and the results of its
/// checks. Check results come from Actions runs and commit statuses only (planning note 6: the
/// token has no Checks permission).
pub trait Forge {
    /// The newest pull request from `branch` into `target`, whatever its state.
    fn pr(&self, branch: &str, target: &str) -> Result<Option<Pr>, ForgeError>;

    /// The results of the checks on the head of `pr`, one per workflow or status context.
    fn checks(&self, pr: &Pr) -> Result<Vec<Check>, ForgeError>;
}

/// What one poll of the forge saw, and the events it adds.
#[derive(Clone, Debug, PartialEq)]
pub struct Poll {
    pub observed_at: Timestamp,
    /// The mission branch's pull request, as it is now.
    pub pr: Option<Pr>,
    pub checks: Vec<Check>,
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
