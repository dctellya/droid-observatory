use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::ids::ModelId;
use super::overlay::Overlay::{self, Human};
use super::timestamp::Timestamp;

/// Declares one variant per PRD §7 event kind with its dotted PRD name and its overlay (D-024).
/// The lane of a kind is not here: the cycle template assigns kinds to lanes (D-035).
/// The serde name of a variant is its snake_case form, which equals the PRD name with
/// `.` replaced by `_`.
macro_rules! event_kinds {
    ($($variant:ident => ($name:literal, $overlay:expr),)*) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum EventKind {
            $($variant,)*
            /// The cycle's last event: the PR merged with its required checks green. The
            /// merger is told by the event's role (`Human` or `External`); nothing after it
            /// belongs to the mission (D-024).
            HandedToCd {
                merge_sha: String,
                pr_number: u64,
                attestation_ids: Vec<String>,
            },
            /// One model turn of the usage log (`[Agent] Streaming result`), with the token
            /// counts of the turn. `model` is the log's `modelId`; `log_ts` is the log
            /// line's own time. The sidecar `usage_reported` stays as the session's total.
            TurnUsage {
                input: u64,
                output: u64,
                cache_read: u64,
                reasoning: u64,
                total_input: u64,
                model: Option<ModelId>,
                log_ts: Timestamp,
            },
            /// A raw record the mapping does not know. It is counted and shown, never
            /// dropped (rule 18); `raw_type` is the record's own `type` value.
            Unknown { raw_type: String },
        }

        impl EventKind {
            /// Every kind the PRD names, in PRD table order, `HandedToCd` last.
            pub const KNOWN: &'static [EventKind] = &[
                $(EventKind::$variant,)*
                EventKind::HandedToCd {
                    merge_sha: String::new(),
                    pr_number: 0,
                    attestation_ids: Vec::new(),
                },
                EventKind::TurnUsage {
                    input: 0,
                    output: 0,
                    cache_read: 0,
                    reasoning: 0,
                    total_input: 0,
                    model: None,
                    log_ts: Timestamp(time::OffsetDateTime::UNIX_EPOCH),
                },
            ];

            /// The dotted PRD name (`finding.blocking`), for display, the CLI and the
            /// store `kind` column.
            pub fn prd_name(&self) -> Cow<'_, str> {
                match self {
                    $(EventKind::$variant => Cow::Borrowed($name),)*
                    EventKind::HandedToCd { .. } => Cow::Borrowed("handed_to_cd"),
                    EventKind::TurnUsage { .. } => Cow::Borrowed("turn.usage"),
                    EventKind::Unknown { .. } => Cow::Borrowed("unknown.event"),
                }
            }

            /// `Some(Human)` for the kinds that pull a person in or record their word.
            pub fn overlay(&self) -> Option<Overlay> {
                match self {
                    $(EventKind::$variant => $overlay,)*
                    EventKind::HandedToCd { .. }
                    | EventKind::TurnUsage { .. }
                    | EventKind::Unknown { .. } => None,
                }
            }
        }
    };
}

impl EventKind {
    /// True for the kinds that only the Panel reads (`turn.usage`): they are not lane rows,
    /// card activity, agent activity, tray entries or scrubber stops (rule 24, 2026-10-24).
    pub fn is_panel_only(&self) -> bool {
        matches!(self, EventKind::TurnUsage { .. })
    }
}

event_kinds! {
    IntentCreated => ("intent.created", Some(Human)),
    IntentAmended => ("intent.amended", Some(Human)),
    QuestionAsked => ("question.asked", Some(Human)),
    IntentAnswered => ("intent.answered", Some(Human)),
    SpecVersioned => ("spec.versioned", None),

    PlanCreated => ("plan.created", None),
    FeatureDefined => ("feature.defined", None),
    MilestoneDefined => ("milestone.defined", None),
    TaskAssigned => ("task.assigned", None),
    TaskSkipped => ("task.skipped", None),
    PlanRevised => ("plan.revised", None),
    EscalationRaised => ("escalation.raised", None),
    MilestoneCompleted => ("milestone.completed", None),
    MissionPaused => ("mission.paused", Some(Human)),
    MissionResumed => ("mission.resumed", Some(Human)),
    MissionCompleted => ("mission.completed", None),
    ContextCompacted => ("context.compacted", None),

    SubagentSpawned => ("subagent.spawned", None),
    TaskStarted => ("task.started", None),
    FileTouched => ("file.touched", None),
    TestsRun => ("tests.run", None),
    CommitCreated => ("commit.created", None),
    TaskDone => ("task.done", None),
    SubagentFinished => ("subagent.finished", None),
    ToolError => ("tool.error", None),
    PermissionRequested => ("permission.requested", Some(Human)),
    PermissionGranted => ("permission.granted", Some(Human)),
    PermissionDenied => ("permission.denied", Some(Human)),
    HookBlocked => ("hook.blocked", None),
    UsageReported => ("usage.reported", None),
    ModelChanged => ("model.changed", None),

    FindingBlocking => ("finding.blocking", None),
    FindingNonBlocking => ("finding.non_blocking", None),
    ReworkRequested => ("rework.requested", None),
    ReworkResolved => ("rework.resolved", None),
    ScrutinyPassed => ("scrutiny.passed", None),

    QaScenarioPassed => ("qa.scenario.passed", None),
    QaScenarioFailed => ("qa.scenario.failed", None),
    QaEvidence => ("qa.evidence", None),

    IntegrationMerged => ("integration.merged", None),
    IntegrationConflict => ("integration.conflict", None),
    IntegrationConflictResolved => ("integration.conflict_resolved", None),
    GateCommandRun => ("gate_command.run", None),
    GateCommandPassed => ("gate_command.passed", None),
    GateCommandFailed => ("gate_command.failed", None),

    ApprovalRequested => ("approval.requested", Some(Human)),
    ApprovalGiven => ("approval.given", Some(Human)),
    QuestionAnswered => ("question.answered", Some(Human)),
    ReviewOutcome => ("review.outcome", Some(Human)),

    PrOpened => ("pr.opened", None),
    PrReviewRequested => ("pr.review_requested", None),
    CheckStarted => ("check.started", None),
    CheckPassed => ("check.passed", None),
    CheckFailed => ("check.failed", None),
    CheckRequiredMissing => ("check.required_missing", None),
    RulesetBlocked => ("ruleset.blocked", None),
    AttestationRecorded => ("attestation.recorded", None),
    AttestationVerified => ("attestation.verified", None),
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::model::{CycleTemplate, Role};

    #[test]
    fn serde_name_is_the_prd_name_with_underscores() {
        for kind in EventKind::KNOWN {
            let json = serde_json::to_value(kind).expect("kind serializes");
            let expected = kind.prd_name().replace('.', "_");
            match json {
                serde_json::Value::String(name) => assert_eq!(name, expected),
                serde_json::Value::Object(fields) => {
                    assert_eq!(fields.keys().collect::<Vec<_>>(), [&expected]);
                }
                other => panic!("unexpected wire form {other}"),
            }
        }
    }

    #[test]
    fn prd_names_are_unique_and_every_kind_has_a_lane_in_the_default_template() {
        let template = CycleTemplate::engineering();
        let names: HashSet<_> = EventKind::KNOWN.iter().map(EventKind::prd_name).collect();
        assert_eq!(names.len(), EventKind::KNOWN.len());
        assert_eq!(EventKind::KNOWN.len(), 60);
        for kind in EventKind::KNOWN {
            for role in [Role::Orchestrator, Role::Worker, Role::External] {
                assert!(
                    template.lane(&template.lane_of(kind, role)).is_some(),
                    "{}",
                    kind.prd_name()
                );
            }
        }
    }
}
