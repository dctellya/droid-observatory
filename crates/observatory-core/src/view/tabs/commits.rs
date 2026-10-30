//! Commits and integration (FR-D4): the commits of one milestone, its worktree, its integration
//! into the mission branch, conflicts and fast-check results.

use super::milestone_events;
use crate::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, MilestoneId, SourcePtr, Timestamp,
};
use crate::view::{MilestoneWorktree, pointer_text};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitRow {
    pub event_id: EventId,
    pub feature: Option<FeatureId>,
    pub at: Timestamp,
    /// The sha when the event points at git, else where the raw line lives.
    pub reference: String,
}

/// One event of the integration section, with the words the tab uses for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRow {
    pub event_id: EventId,
    pub feature: Option<FeatureId>,
    pub at: Timestamp,
    pub lane: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitsTab {
    pub milestone: MilestoneId,
    /// The mission branch the integration entries name.
    pub branch: String,
    /// The directory name of the milestone's worktree; none when git lists none.
    pub worktree: Option<String>,
    pub commits: Vec<CommitRow>,
    pub integration: Vec<EventRow>,
    pub conflicts: Vec<EventRow>,
    pub checks: Vec<EventRow>,
}

pub fn commits_tab(
    events: &[Event],
    milestone: &MilestoneId,
    template: &CycleTemplate,
    branch: &str,
    worktree: Option<&MilestoneWorktree>,
) -> CommitsTab {
    let mut tab = CommitsTab {
        milestone: milestone.clone(),
        branch: branch.to_owned(),
        worktree: worktree.map(|worktree| worktree.name.clone()),
        commits: Vec::new(),
        integration: Vec::new(),
        conflicts: Vec::new(),
        checks: Vec::new(),
    };
    for event in milestone_events(events, milestone) {
        let row = |text: String| EventRow {
            event_id: event.id.clone(),
            feature: event.feature_id.clone(),
            at: event.at,
            lane: template.name_of(&event.lane),
            text,
        };
        match event.kind {
            EventKind::CommitCreated => tab.commits.push(CommitRow {
                event_id: event.id.clone(),
                feature: event.feature_id.clone(),
                at: event.at,
                reference: match &event.source {
                    SourcePtr::Git { sha, .. } => sha.clone(),
                    other => pointer_text(other),
                },
            }),
            EventKind::IntegrationMerged => tab
                .integration
                .push(row(format!("integrated in mission {branch}"))),
            EventKind::IntegrationConflict => tab.conflicts.push(row("conflict".to_owned())),
            EventKind::IntegrationConflictResolved => {
                tab.conflicts.push(row("conflict resolved".to_owned()));
            }
            EventKind::GateCommandRun => tab.checks.push(row("fast check run".to_owned())),
            EventKind::GateCommandPassed => tab.checks.push(row("fast check passed".to_owned())),
            EventKind::GateCommandFailed => tab.checks.push(row("fast check failed".to_owned())),
            _ => {}
        }
    }
    tab
}
