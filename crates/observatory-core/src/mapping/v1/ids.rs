//! Milestone and feature ids. A milestone is a row `| **M0** title | ... |` of the plan table in
//! the `ProposeMission` proposal; a feature id is a kebab-case word that starts with a
//! milestone id (`m0-mapping-v1`) in a `Task` description, or the quoted id in the runner's
//! worker prompt. Both are lower-cased.

use crate::model::MilestoneId;

/// Marker of the prompt the mission runner gives each session it starts.
const RUNNER_PROMPT: &str = "execute feature \"";
/// First line of the prompt a `Task` call gives its subagent.
const TASK_PROMPT: &str = "# Task Tool Invocation";
const TASK_DESCRIPTION: &str = "Task description:";

/// Milestone ids of the plan table, in order, without repeats.
pub(crate) fn proposal_milestones(proposal: &str) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for line in proposal.lines() {
        let Some(row) = line.trim_start().strip_prefix("| **") else {
            continue;
        };
        let Some((id, _)) = row.split_once("**") else {
            continue;
        };
        let id = id.to_ascii_lowercase();
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()) && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// The first word of `text` that is a feature id of one of `milestones`.
pub(super) fn feature_in(text: &str, milestones: &[MilestoneId]) -> Option<String> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
        .find(|word| {
            let kebab = word
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            kebab
                && milestones.iter().any(|milestone| {
                    word.strip_prefix(milestone.0.as_str())
                        .is_some_and(|rest| rest.starts_with('-') && rest.len() > 1)
                })
        })
        .map(str::to_owned)
}

/// The milestone a feature belongs to: the first word of its id (`m0-mapping-v1`), or the last
/// one for the runner's per-milestone validator features (`scrutiny-validator-s`).
pub(super) fn milestone_of(feature: &str, milestones: &[MilestoneId]) -> Option<MilestoneId> {
    let (first, last) = (
        feature.split('-').next().unwrap_or_default(),
        feature.rsplit('-').next().unwrap_or_default(),
    );
    [first, last]
        .into_iter()
        .find_map(|word| milestones.iter().find(|milestone| milestone.0 == word))
        .cloned()
}

/// `Some` when `text` is the prompt that starts a child on its task; the inner value is the
/// feature the prompt names, if any.
pub(super) fn assignment(text: &str, milestones: &[MilestoneId]) -> Option<Option<String>> {
    if let Some((_, after)) = text.split_once(RUNNER_PROMPT) {
        let id = after.split('"').next().unwrap_or_default();
        return Some((!id.is_empty()).then(|| id.to_owned()));
    }
    if text.trim_start().starts_with(TASK_PROMPT) {
        let description = text
            .lines()
            .find_map(|line| line.strip_prefix(TASK_DESCRIPTION))
            .unwrap_or_default();
        return Some(feature_in(description, milestones));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<MilestoneId> {
        names
            .iter()
            .map(|name| MilestoneId((*name).to_owned()))
            .collect()
    }

    #[test]
    fn milestones_come_from_the_plan_table_rows() {
        let proposal = "| Milestone | Delivers |\n|---|---|\n| **S** a | x |\n  | **M0** b | y |\n| **M0** again | z |\n| **no id here | w |\n";
        assert_eq!(proposal_milestones(proposal), ["s", "m0"]);
    }

    #[test]
    fn a_feature_id_starts_with_a_known_milestone() {
        let milestones = ids(&["s", "m0"]);
        assert_eq!(
            feature_in("Review s-cli-replay (round 2)", &milestones).as_deref(),
            Some("s-cli-replay")
        );
        assert_eq!(feature_in("Diagnose build-log output", &milestones), None);
        assert_eq!(feature_in("Review s-cli-replay", &[]), None);
        assert_eq!(feature_in("Test VAL-SKEL-004", &milestones), None);
    }

    #[test]
    fn a_feature_belongs_to_the_milestone_at_either_end_of_its_id() {
        let milestones = ids(&["s", "m0"]);
        assert_eq!(
            milestone_of("m0-fixture-freeze", &milestones),
            Some(ids(&["m0"]).remove(0))
        );
        assert_eq!(
            milestone_of("scrutiny-validator-s", &milestones),
            Some(ids(&["s"]).remove(0))
        );
        assert_eq!(milestone_of("misc-work", &milestones), None);
    }

    #[test]
    fn assignment_prompts_are_recognised_and_other_text_is_not() {
        let milestones = ids(&["s"]);
        assert_eq!(
            assignment(
                "<system-reminder>\nYou are a worker assigned to execute feature \"s-ci-wiring\".",
                &milestones
            ),
            Some(Some("s-ci-wiring".to_owned()))
        );
        assert_eq!(
            assignment(
                "# Task Tool Invocation\n\nTask description: Review s-core\n",
                &milestones
            ),
            Some(Some("s-core".to_owned()))
        );
        assert_eq!(
            assignment(
                "# Task Tool Invocation\n\nTask description: Draft notes\n",
                &milestones
            ),
            Some(None)
        );
        assert_eq!(assignment("hello", &milestones), None);
    }
}
