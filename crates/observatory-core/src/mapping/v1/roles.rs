//! Who a session is, for Droid 0.247 session files. The rule order is frozen by the M0 capture.

use crate::model::Role;

/// `tool_use.input.subagent_type` values the M0 capture shows, and the role each stands for.
/// An explorer is a read-only worker; the capture headers label it `worker`.
const SUBAGENT_ROLES: [(&str, Role); 4] = [
    ("worker", Role::Worker),
    ("explorer", Role::Worker),
    ("scrutiny-feature-reviewer", Role::ScrutinyValidator),
    ("user-testing-flow-validator", Role::UserTestingValidator),
];

/// `Skill` names with which a runner-started session says what it validates.
const VALIDATOR_SKILLS: [(&str, Role); 2] = [
    ("scrutiny-validator", Role::ScrutinyValidator),
    ("user-testing-validator", Role::UserTestingValidator),
];

const TAG_ROLE_ORCHESTRATOR: &str = "orchestrator";
const TAG_ROLE_WORKER: &str = "worker";

/// What the files say about one session. Nothing here is guessed: absent evidence stays absent.
#[derive(Clone, Copy, Debug, Default)]
pub struct RoleEvidence<'a> {
    /// `session_start.callingSessionId`.
    pub calling_session_id: Option<&'a str>,
    /// `session_start.callingToolUseId`.
    pub calling_tool_use_id: Option<&'a str>,
    /// `input.subagent_type` of the parent's `tool_use` whose `id` is `calling_tool_use_id`;
    /// `None` when that part is missing or carries no type.
    pub subagent_type: Option<&'a str>,
    /// `metadata.role` of the sidecar `mission-session` tag.
    pub tag_role: Option<&'a str>,
    /// `input.skill` of every `Skill` tool_use in the session's own transcript.
    pub skills: &'a [String],
}

/// In order: (1) the `mission-session` tag says orchestrator; (2) no caller; (3) a spawning
/// `Task` names its `subagent_type`; (4) a runner-started session (tag role `worker`, no
/// spawning tool_use) is told apart by the validator `Skill` it invokes; (5) `Unresolved`.
pub(in crate::mapping) fn resolve_role(evidence: &RoleEvidence<'_>) -> Role {
    if evidence.tag_role == Some(TAG_ROLE_ORCHESTRATOR) || evidence.calling_session_id.is_none() {
        return Role::Orchestrator;
    }
    if evidence.calling_tool_use_id.is_some() {
        return evidence
            .subagent_type
            .and_then(|seen| lookup(&SUBAGENT_ROLES, seen))
            .unwrap_or(Role::Unresolved);
    }
    if evidence.tag_role == Some(TAG_ROLE_WORKER) {
        return evidence
            .skills
            .iter()
            .find_map(|skill| lookup(&VALIDATOR_SKILLS, skill))
            .unwrap_or(Role::Worker);
    }
    Role::Unresolved
}

/// `Task.description` of the session that writes the validation contract: it starts with the
/// word `Contract`, or names the `validation contract` anywhere.
pub(in crate::mapping) fn is_contract_task(description: &str) -> bool {
    let starts = description.strip_prefix("Contract").is_some_and(|rest| {
        !rest
            .chars()
            .next()
            .is_some_and(|next| next.is_alphanumeric() || next == '_')
    });
    starts || description.contains("alidation contract")
}

/// A feature id of the integration session of a milestone or of the mission:
/// `m<digits>-integrate-...` or `final-integrate-...`.
pub(super) fn is_integration_feature(feature: &str) -> bool {
    let Some((head, _)) = feature.split_once("-integrate-") else {
        return false;
    };
    head == "final"
        || head
            .strip_prefix('m')
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn lookup(table: &[(&str, Role)], seen: &str) -> Option<Role> {
    table
        .iter()
        .find_map(|(name, role)| (*name == seen).then_some(*role))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller() -> RoleEvidence<'static> {
        RoleEvidence {
            calling_session_id: Some("root"),
            ..RoleEvidence::default()
        }
    }

    #[test]
    fn contract_tasks_and_integration_features_are_told_by_their_words() {
        for yes in [
            "Contract: draft the assertions",
            "Contract",
            "Write the validation contract for M1",
            "Review Validation contract",
        ] {
            assert!(is_contract_task(yes), "{yes}");
        }
        for no in ["Contractor review", "Subcontract", "Draft the notes", ""] {
            assert!(!is_contract_task(no), "{no}");
        }
        for yes in [
            "m0-integrate-s",
            "m12-integrate-m1",
            "final-integrate-mission",
        ] {
            assert!(is_integration_feature(yes), "{yes}");
        }
        for no in [
            "s-integrate-x",
            "m-integrate-s",
            "m0-integrated",
            "m0-mapping-v1",
        ] {
            assert!(!is_integration_feature(no), "{no}");
        }
    }

    #[test]
    fn the_orchestrator_tag_wins_even_with_a_caller() {
        let evidence = RoleEvidence {
            tag_role: Some("orchestrator"),
            ..caller()
        };
        assert_eq!(resolve_role(&evidence), Role::Orchestrator);
    }

    #[test]
    fn no_caller_is_the_orchestrator() {
        assert_eq!(resolve_role(&RoleEvidence::default()), Role::Orchestrator);
    }

    #[test]
    fn a_spawning_tool_use_decides_by_subagent_type() {
        let spawned = |kind| RoleEvidence {
            calling_tool_use_id: Some("t"),
            subagent_type: Some(kind),
            ..caller()
        };
        assert_eq!(resolve_role(&spawned("worker")), Role::Worker);
        assert_eq!(resolve_role(&spawned("explorer")), Role::Worker);
        assert_eq!(
            resolve_role(&spawned("scrutiny-feature-reviewer")),
            Role::ScrutinyValidator
        );
        assert_eq!(
            resolve_role(&spawned("user-testing-flow-validator")),
            Role::UserTestingValidator
        );
        assert_eq!(resolve_role(&spawned("invented")), Role::Unresolved);
    }

    #[test]
    fn a_spawning_tool_use_that_cannot_be_read_is_unresolved_not_a_worker() {
        let evidence = RoleEvidence {
            calling_tool_use_id: Some("t"),
            tag_role: Some("worker"),
            ..caller()
        };
        assert_eq!(resolve_role(&evidence), Role::Unresolved);
    }

    #[test]
    fn a_runner_started_session_is_a_worker_unless_it_invokes_a_validator_skill() {
        let skills = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
        let resolve = |names: Vec<String>| {
            resolve_role(&RoleEvidence {
                tag_role: Some("worker"),
                skills: &names,
                ..caller()
            })
        };
        assert_eq!(resolve(skills(&["core-worker"])), Role::Worker);
        assert_eq!(resolve(skills(&[])), Role::Worker);
        assert_eq!(
            resolve(skills(&["mission-worker-base", "scrutiny-validator"])),
            Role::ScrutinyValidator
        );
        assert_eq!(
            resolve(skills(&["user-testing-validator"])),
            Role::UserTestingValidator
        );
    }

    #[test]
    fn a_child_without_tool_use_or_worker_tag_is_unresolved() {
        assert_eq!(resolve_role(&caller()), Role::Unresolved);
    }
}
