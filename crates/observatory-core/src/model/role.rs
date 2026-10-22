use serde::{Deserialize, Serialize};

/// The sub-droid (or person) behind an event.
///
/// `Unresolved` is not in the PRD: a child session whose role cannot be resolved stays
/// visible with its own count instead of being guessed (rule 18).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Orchestrator,
    Worker,
    ScrutinyValidator,
    UserTestingValidator,
    Human,
    External,
    Unresolved,
}
