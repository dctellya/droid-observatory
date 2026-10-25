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

impl Role {
    /// Every role, in the order the UI lists them.
    pub const ALL: [Role; 7] = [
        Role::Orchestrator,
        Role::Worker,
        Role::ScrutinyValidator,
        Role::UserTestingValidator,
        Role::Human,
        Role::External,
        Role::Unresolved,
    ];

    /// The wire text (`scrutiny_validator`).
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Orchestrator => "orchestrator",
            Role::Worker => "worker",
            Role::ScrutinyValidator => "scrutiny_validator",
            Role::UserTestingValidator => "user_testing_validator",
            Role::Human => "human",
            Role::External => "external",
            Role::Unresolved => "unresolved",
        }
    }

    /// The roles that review work: they name the features they review in their own lane (D-032).
    pub fn is_validator(self) -> bool {
        matches!(self, Role::ScrutinyValidator | Role::UserTestingValidator)
    }

    pub fn parse(text: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|role| role.as_str() == text)
    }
}
