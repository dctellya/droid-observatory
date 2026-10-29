//! The Merge press (FR-G5, architecture §4.6): one request to the forge per confirmed press and
//! what its answer means for the `human_actions` row. A token that cannot merge is not an
//! error of the app: the row is `refused`, the message is the one architecture §4.6 gives, and
//! nothing is retried.

use serde::{Deserialize, Serialize};

use crate::ingest::forge::{Forge, ForgeError, MergeMethod, Pr};

/// Length of the merge sha the message shows.
const SHORT_SHA_LEN: usize = 7;

/// The state a `merge` row takes from the answer of the forge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeState {
    /// The forge merged it.
    Merged,
    /// The token lacks the permission (HTTP 403).
    Refused,
    /// Any other answer: the rule refused the merge, the network failed, the rate limit was hit.
    Failed,
}

impl MergeState {
    /// The text of `human_actions.state`.
    pub fn wire(self) -> &'static str {
        match self {
            Self::Merged => "merged",
            Self::Refused => "refused",
            Self::Failed => "failed",
        }
    }
}

/// What a press came to: the state of its row and the text the window shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeOutcome {
    pub state: MergeState,
    pub message: String,
}

/// The method of the merge request: squash when the rule allows it, else the first method it
/// allows, else squash (a rule that names none leaves the choice open).
pub fn merge_method(allowed: &[MergeMethod]) -> MergeMethod {
    if allowed.is_empty() || allowed.contains(&MergeMethod::Squash) {
        MergeMethod::Squash
    } else {
        allowed[0]
    }
}

/// Asks the forge once to merge `pr`. No retry on any answer: the poll reads the PR's real state
/// at its normal interval.
pub fn press<F: Forge>(forge: &F, pr: &Pr, allowed: &[MergeMethod]) -> MergeOutcome {
    match forge.merge(pr, merge_method(allowed)) {
        Ok(merged) => MergeOutcome {
            state: MergeState::Merged,
            message: format!(
                "Merged PR #{} into {} as {}. The badge updates from the next poll.",
                pr.number,
                pr.base,
                merged.sha.get(..SHORT_SHA_LEN).unwrap_or(&merged.sha)
            ),
        },
        Err(error @ ForgeError::Forbidden { .. }) => MergeOutcome {
            state: MergeState::Refused,
            message: error.to_string(),
        },
        Err(error) => MergeOutcome {
            state: MergeState::Failed,
            message: format!(
                "Merge of PR #{} into {} failed: {error}. Merge in GitHub; the badge updates from the next poll.",
                pr.number, pr.base
            ),
        },
    }
}
