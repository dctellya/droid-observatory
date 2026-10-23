use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::ids::ModelId;
use super::lane::Lane::{self, Contract, Gate, Integrate, Plan, Scrutiny, UserTesting};
use super::overlay::Overlay::{self, Human};
use super::role::Role;
use super::timestamp::Timestamp;

/// Declares one variant per PRD §7 event kind with its dotted PRD name, the phase that fixes
/// its lane (`None`: the role decides, see `EventKind::lane`) and its overlay (D-024).
/// The serde name of a variant is its snake_case form, which equals the PRD name with
/// `.` replaced by `_`.
macro_rules! event_kinds {
    ($($variant:ident => ($name:literal, $fixed:expr, $overlay:expr),)*) => {
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

            /// The phase this kind always has, whoever emits it. `None` for the role-neutral
            /// kinds, where the role (or the session) decides.
            pub fn fixed_lane(&self) -> Option<Lane> {
                match self {
                    $(EventKind::$variant => $fixed,)*
                    EventKind::HandedToCd { .. } => Some(Gate),
                    EventKind::TurnUsage { .. } | EventKind::Unknown { .. } => None,
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
    /// The phase of an event of this kind by `role` (D-024 rules 1, 2 and 5): the fixed phase
    /// of the kind; the orchestrator's commits, which land on the mission branch, in
    /// Integrate; otherwise the role's home. Session facts (an integration or contract
    /// session) are applied by the mapping on top of this.
    pub fn lane(&self, role: Role) -> Lane {
        match (self, role) {
            (EventKind::CommitCreated, Role::Orchestrator) => Integrate,
            _ => self.fixed_lane().unwrap_or_else(|| Lane::home(role)),
        }
    }
}

event_kinds! {
    IntentCreated => ("intent.created", Some(Plan), Some(Human)),
    IntentAmended => ("intent.amended", Some(Plan), Some(Human)),
    QuestionAsked => ("question.asked", Some(Plan), Some(Human)),
    IntentAnswered => ("intent.answered", Some(Plan), Some(Human)),
    SpecVersioned => ("spec.versioned", Some(Contract), None),

    PlanCreated => ("plan.created", Some(Plan), None),
    FeatureDefined => ("feature.defined", Some(Plan), None),
    MilestoneDefined => ("milestone.defined", Some(Plan), None),
    TaskAssigned => ("task.assigned", None, None),
    TaskSkipped => ("task.skipped", None, None),
    PlanRevised => ("plan.revised", Some(Plan), None),
    EscalationRaised => ("escalation.raised", Some(Plan), None),
    MilestoneCompleted => ("milestone.completed", Some(Plan), None),
    MissionPaused => ("mission.paused", Some(Plan), Some(Human)),
    MissionResumed => ("mission.resumed", Some(Plan), Some(Human)),
    MissionCompleted => ("mission.completed", Some(Plan), None),
    ContextCompacted => ("context.compacted", None, None),

    SubagentSpawned => ("subagent.spawned", None, None),
    TaskStarted => ("task.started", None, None),
    FileTouched => ("file.touched", None, None),
    TestsRun => ("tests.run", None, None),
    CommitCreated => ("commit.created", None, None),
    TaskDone => ("task.done", None, None),
    SubagentFinished => ("subagent.finished", None, None),
    ToolError => ("tool.error", None, None),
    PermissionRequested => ("permission.requested", None, Some(Human)),
    PermissionGranted => ("permission.granted", None, Some(Human)),
    PermissionDenied => ("permission.denied", None, Some(Human)),
    HookBlocked => ("hook.blocked", None, None),
    UsageReported => ("usage.reported", None, None),
    ModelChanged => ("model.changed", None, None),

    FindingBlocking => ("finding.blocking", Some(Scrutiny), None),
    FindingNonBlocking => ("finding.non_blocking", Some(Scrutiny), None),
    ReworkRequested => ("rework.requested", Some(Scrutiny), None),
    ReworkResolved => ("rework.resolved", Some(Scrutiny), None),
    ScrutinyPassed => ("scrutiny.passed", Some(Scrutiny), None),

    QaScenarioPassed => ("qa.scenario.passed", Some(UserTesting), None),
    QaScenarioFailed => ("qa.scenario.failed", Some(UserTesting), None),
    QaEvidence => ("qa.evidence", Some(UserTesting), None),

    IntegrationMerged => ("integration.merged", Some(Integrate), None),
    IntegrationConflict => ("integration.conflict", Some(Integrate), None),
    IntegrationConflictResolved => ("integration.conflict_resolved", Some(Integrate), None),
    GateCommandRun => ("gate_command.run", Some(Integrate), None),
    GateCommandPassed => ("gate_command.passed", Some(Integrate), None),
    GateCommandFailed => ("gate_command.failed", Some(Integrate), None),

    ApprovalRequested => ("approval.requested", Some(Plan), Some(Human)),
    ApprovalGiven => ("approval.given", Some(Plan), Some(Human)),
    QuestionAnswered => ("question.answered", Some(Plan), Some(Human)),
    ReviewOutcome => ("review.outcome", Some(Gate), Some(Human)),

    PrOpened => ("pr.opened", Some(Integrate), None),
    PrReviewRequested => ("pr.review_requested", Some(Gate), None),
    CheckStarted => ("check.started", Some(Gate), None),
    CheckPassed => ("check.passed", Some(Gate), None),
    CheckFailed => ("check.failed", Some(Gate), None),
    CheckRequiredMissing => ("check.required_missing", Some(Gate), None),
    RulesetBlocked => ("ruleset.blocked", Some(Gate), None),
    AttestationRecorded => ("attestation.recorded", Some(Gate), None),
    AttestationVerified => ("attestation.verified", Some(Gate), None),
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::model::Role;

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
    fn prd_names_are_unique_and_every_kind_has_a_lane() {
        let names: HashSet<_> = EventKind::KNOWN.iter().map(EventKind::prd_name).collect();
        assert_eq!(names.len(), EventKind::KNOWN.len());
        assert_eq!(EventKind::KNOWN.len(), 60);
        for kind in EventKind::KNOWN {
            for role in [Role::Orchestrator, Role::Worker, Role::External] {
                assert!(Lane::ALL.contains(&kind.lane(role)), "{}", kind.prd_name());
            }
        }
    }
}
