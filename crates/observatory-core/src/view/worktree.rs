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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::ingest::git::Worktree;

    fn snapshot(worktrees: Vec<Worktree>) -> GitSnapshot {
        GitSnapshot {
            mission_branch: "mission/demo".to_owned(),
            mission_head: "abc".to_owned(),
            worktrees,
            milestone_branches: Vec::new(),
        }
    }

    fn listed(path: &str, milestone: Option<&str>) -> Worktree {
        Worktree {
            path: PathBuf::from(path),
            branch: milestone.map(|m| format!("wt/{m}")),
            head: "abc".to_owned(),
            milestone: milestone.map(|m| MilestoneId(m.to_owned())),
        }
    }

    #[test]
    fn a_milestone_without_a_worktree_has_no_name() {
        let worktrees = milestone_worktrees(&snapshot(vec![
            listed("/work/observatory", None),
            listed("/work/observatory-wt/m4", Some("m4")),
        ]));
        assert_eq!(
            worktrees,
            vec![MilestoneWorktree {
                milestone: MilestoneId("m4".to_owned()),
                name: "m4".to_owned(),
                path: "/work/observatory-wt/m4".to_owned(),
            }]
        );
        assert!(
            worktrees
                .iter()
                .all(|worktree| worktree.milestone.0 != "m3")
        );
        assert!(milestone_worktrees(&snapshot(Vec::new())).is_empty());
    }
}
