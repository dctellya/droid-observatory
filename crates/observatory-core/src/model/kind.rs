use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::lane::Lane;

/// Declares one unit variant per PRD §7 event kind with its dotted PRD name and lane.
/// The serde name of a variant is its snake_case form, which equals the PRD name with
/// `.` replaced by `_`.
macro_rules! event_kinds {
    ($($variant:ident => ($name:literal, $lane:ident),)*) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum EventKind {
            $($variant,)*
        }

        impl EventKind {
            /// Every kind the PRD names, in PRD table order.
            pub const KNOWN: &'static [EventKind] = &[$(EventKind::$variant,)*];

            /// The dotted PRD name (`finding.blocking`), for display, the CLI and the
            /// store `kind` column.
            pub fn prd_name(&self) -> Cow<'_, str> {
                match self {
                    $(EventKind::$variant => Cow::Borrowed($name),)*
                }
            }

            /// The PRD lane of a known kind.
            pub fn lane(&self) -> Option<Lane> {
                match self {
                    $(EventKind::$variant => Some(Lane::$lane),)*
                }
            }
        }
    };
}

event_kinds! {
    IntentCreated => ("intent.created", Intent),
    IntentAmended => ("intent.amended", Intent),
    QuestionAsked => ("question.asked", Intent),
    IntentAnswered => ("intent.answered", Intent),
    SpecVersioned => ("spec.versioned", Intent),

    PlanCreated => ("plan.created", Orchestrator),
    FeatureDefined => ("feature.defined", Orchestrator),
    MilestoneDefined => ("milestone.defined", Orchestrator),
    TaskAssigned => ("task.assigned", Orchestrator),
    TaskSkipped => ("task.skipped", Orchestrator),
    PlanRevised => ("plan.revised", Orchestrator),
    EscalationRaised => ("escalation.raised", Orchestrator),
    MilestoneCompleted => ("milestone.completed", Orchestrator),
    MissionPaused => ("mission.paused", Orchestrator),
    MissionResumed => ("mission.resumed", Orchestrator),
    MissionCompleted => ("mission.completed", Orchestrator),
    ContextCompacted => ("context.compacted", Orchestrator),

    SubagentSpawned => ("subagent.spawned", Workers),
    TaskStarted => ("task.started", Workers),
    FileTouched => ("file.touched", Workers),
    TestsRun => ("tests.run", Workers),
    CommitCreated => ("commit.created", Workers),
    TaskDone => ("task.done", Workers),
    SubagentFinished => ("subagent.finished", Workers),
    ToolError => ("tool.error", Workers),
    PermissionRequested => ("permission.requested", Workers),
    PermissionGranted => ("permission.granted", Workers),
    PermissionDenied => ("permission.denied", Workers),
    HookBlocked => ("hook.blocked", Workers),
    UsageReported => ("usage.reported", Workers),
    ModelChanged => ("model.changed", Workers),

    FindingBlocking => ("finding.blocking", Scrutiny),
    FindingNonBlocking => ("finding.non_blocking", Scrutiny),
    ReworkRequested => ("rework.requested", Scrutiny),
    ReworkResolved => ("rework.resolved", Scrutiny),
    ScrutinyPassed => ("scrutiny.passed", Scrutiny),

    QaScenarioPassed => ("qa.scenario.passed", UserTesting),
    QaScenarioFailed => ("qa.scenario.failed", UserTesting),
    QaEvidence => ("qa.evidence", UserTesting),

    IntegrationMerged => ("integration.merged", Integration),
    IntegrationConflict => ("integration.conflict", Integration),
    IntegrationConflictResolved => ("integration.conflict_resolved", Integration),
    GateCommandRun => ("gate_command.run", Integration),
    GateCommandPassed => ("gate_command.passed", Integration),
    GateCommandFailed => ("gate_command.failed", Integration),

    ApprovalRequested => ("approval.requested", Human),
    ApprovalGiven => ("approval.given", Human),
    QuestionAnswered => ("question.answered", Human),
    ReviewOutcome => ("review.outcome", Human),
    PrMergedByHuman => ("pr.merged_by_human", Human),

    PrOpened => ("pr.opened", Outside),
    PrReviewRequested => ("pr.review_requested", Outside),
    CheckStarted => ("check.started", Outside),
    CheckPassed => ("check.passed", Outside),
    CheckFailed => ("check.failed", Outside),
    CheckRequiredMissing => ("check.required_missing", Outside),
    RulesetBlocked => ("ruleset.blocked", Outside),
    AttestationRecorded => ("attestation.recorded", Outside),
    AttestationVerified => ("attestation.verified", Outside),
    PrMergedByPolicy => ("pr.merged_by_policy", Outside),
    DeployCompleted => ("deploy.completed", Outside),
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn serde_name_is_the_prd_name_with_underscores() {
        for kind in EventKind::KNOWN {
            let json = serde_json::to_string(kind).expect("kind serializes");
            let expected = format!("\"{}\"", kind.prd_name().replace('.', "_"));
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn prd_names_are_unique_and_every_kind_has_a_lane() {
        let names: HashSet<_> = EventKind::KNOWN.iter().map(EventKind::prd_name).collect();
        assert_eq!(names.len(), EventKind::KNOWN.len());
        assert!(
            EventKind::KNOWN
                .iter()
                .all(|kind| kind.lane().is_some_and(|lane| lane != Lane::TimeWasteCost))
        );
    }
}
