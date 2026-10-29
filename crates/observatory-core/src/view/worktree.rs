use serde::{Deserialize, Serialize};

use crate::ingest::git::GitSnapshot;
use crate::model::MilestoneId;

/// The worktree git lists for a milestone's `wt/<m>` branch, as its card names it (FR-G1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneWorktree {
    pub milestone: MilestoneId,
    /// The directory name of the worktree.
    pub name: String,
    /// Where it is, for the tooltip of the name.
    pub path: String,
}

/// The worktrees of the milestones, in the order git lists them. A worktree on any other branch
/// (the mission branch's own) belongs to no card.
pub fn milestone_worktrees(snapshot: &GitSnapshot) -> Vec<MilestoneWorktree> {
    snapshot
        .worktrees
        .iter()
        .filter_map(|worktree| {
            Some(MilestoneWorktree {
                milestone: worktree.milestone.clone()?,
                name: worktree.path.file_name()?.to_string_lossy().into_owned(),
                path: worktree.path.display().to_string(),
            })
        })
        .collect()
}
